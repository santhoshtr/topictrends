-include .env
SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -euo pipefail -c
export $(shell sed 's/=.*//' .env 2>/dev/null)

# Date parsing - use DATE environment variable if set, otherwise use yesterday
DATE ?= $(shell date -d "yesterday" +%Y-%m-%d)
YEAR := $(shell echo $(DATE) | cut -d'-' -f1)
MONTH := $(shell echo $(DATE) | cut -d'-' -f2)
DAY := $(shell echo $(DATE) | cut -d'-' -f3)
# Exclusive upper bound for the single-day replica edit query (YYYYMMDD).
NEXT_DATE := $(shell date -d "$(DATE) +1 day" +%Y%m%d)

CARGO_RELEASE := target/release

DATA_DIR ?= data
GSC_DIR ?= $(DATA_DIR)/gsc_page_date
QUERIES_DIR := queries

# Raw pageview dumps. Read from the clouddumps NFS mount on the VPS; override
# for local runs. Layout mirrors dumps.wikimedia.org: <dir>/$Y/$Y-$M/pageviews-$Y$M$D-user.bz2
PAGEVIEW_DIR ?= /mnt/nfs/dumps-clouddumps1001.wikimedia.org/other/pageview_complete

# Vector store (zvec) configuration. Embeddings and search run in-process via
# topictrend_taxonomy (fastembed + zvec-rust).
ZVEC_DIR ?= $(DATA_DIR)/embedding_store/zvec
WIKI ?= enwiki
EMBED_ENV := DATA_DIR=$(abspath $(DATA_DIR)) ZVEC_DIR=$(abspath $(ZVEC_DIR))
# zvec-sys's build.rs fetches libzvec_c_api.so as a prebuilt with no embedded
# rpath, so it must be on the loader path at runtime. Resolved lazily from the
# build output (empty until `cargo build` has run).
ZVEC_LIB_DIR = $(abspath $(shell find $(CARGO_RELEASE)/build -name libzvec_c_api.so -printf '%h\n' 2>/dev/null | head -1))

# Deferred so the list is re-read after `init` produces it on a fresh checkout.
WIKIS = $(shell cat $(DATA_DIR)/wikipedia.list 2>/dev/null)

.DEFAULT_GOAL := run

.PHONY: run init clean help monthly index-wiki index-clean web gsc coverage canonical _run-wikis topology-refresh

# Help target
help:
	@echo "This Makefile is intended for the wmcloud VPS (replica access + GNU date)."
	@echo ""
	@echo "Available targets:"
	@echo "  run              - Build binaries and process all wikis for DATE"
	@echo "  monthly          - Process all wikis for the calendar month containing END_DATE"
	@echo "                     (1..LAST_DAY of that month, capped at END_DATE's day)"
	@echo "  gsc              - Process Google Search Console data for all wikis (single DATE)"
	@echo "  coverage         - Build the coverage matrix (direct_coverage + qid_overlap) for all"
	@echo "                     wikis as a dated snapshot (data/<wiki>/coverage/<DATE>.parquet);"
	@echo "                     needs the canonical projections, so run 'make canonical' first"
	@echo "  canonical        - Union all wikis' article_category into the canonical relation"
	@echo "                     with per-edge wiki counts (data/canonical/<DATE>/), then project"
	@echo "                     it per wiki (article_category_canonical + categories_canonical)"
	@echo "  topology-refresh - Re-fetch topology (articles/categories/graph) from the replica"
	@echo "                     for all wikis; scope with WIKIS=enwiki. Restart the server after."
	@echo "  web              - Start the web server (no rebuild)"
	@echo "  init             - Build release binaries and fetch the Wikipedia list"
	@echo "  index-wiki       - Index a wiki's categories into zvec (WIKI=enwiki)"
	@echo "  index-clean      - Remove the zvec index store"
	@echo "  clean            - Remove all generated data files"
	@echo "  help             - Show this help message"
	@echo ""
	@echo "Environment variables:"
	@echo "  DATE          - Date to process (YYYY-MM-DD, defaults to yesterday)"
	@echo "  END_DATE      - Last day for 'monthly' (YYYY-MM-DD, defaults to yesterday)"
	@echo "  GSC_DIR       - Path to GSC source root (defaults to data/gsc_page_date)"
	@echo "  WIKI          - Wiki for index-wiki (defaults to enwiki)"
	@echo ""
	@echo "Examples:"
	@echo "  make run DATE=2025-01-15"
	@echo "  make monthly END_DATE=2025-08-30"
	@echo "  make gsc DATE=2026-03-03"
	@echo "  make gsc DATE=2026-03-03 GSC_DIR=/mnt/gsc_data"
	@echo "  make coverage DATE=2026-06-09                                   # coverage snapshot, all wikis"
	@echo "  make canonical DATE=2026-06-11                                  # canonical relation snapshot"
	@echo "  make data/mlwiki/pageedits/2026/05/26.parquet DATE=2026-05-26   # one wiki, one day (replica)"
	@echo "  make topology-refresh WIKIS=enwiki                              # refresh one wiki's topology"

# Main run target.
# First invocation may need to bootstrap wikipedia.list (so that WIKIS is
# non-empty), then re-enter Make to expand the per-wiki dependency list.
run:
	@if [ ! -f $(DATA_DIR)/wikipedia.list ]; then \
		echo "Bootstrapping wikipedia.list..."; \
		$(MAKE) -s init; \
	fi
	@$(MAKE) -s _run-wikis DATE=$(DATE)

_run-wikis: init $(WIKIS)

$(DATA_DIR):
	@mkdir -p $@

# Initialize directory structure
init: $(DATA_DIR)/wikipedia.list
	cargo build --release
	@mkdir -p $(DATA_DIR)

# Per-wiki targets. The trailing recipe just prints a completion line so the
# operator can see progress while the run grinds through hundreds of wikis.
$(WIKIS): %: \
	$(DATA_DIR)/%/articles.parquet \
	$(DATA_DIR)/%/categories.parquet \
	$(DATA_DIR)/%/article_category.parquet \
	$(DATA_DIR)/%/category_graph.parquet \
	$(DATA_DIR)/%/pageviews/$(YEAR)/$(MONTH)/$(DAY).parquet \
	$(DATA_DIR)/%/pageedits/$(YEAR)/$(MONTH)/$(DAY).parquet
	@echo "✓ $* ($(DATE))"

# Helper function for database queries
dbquery = mariadb --quick --host $*.analytics.db.svc.wikimedia.cloud --database $*_p

# Article data
$(DATA_DIR)/%/articles.parquet: $(QUERIES_DIR)/articles.sql
	@mkdir -p $(dir $@)
	@echo "Fetching articles for $*..."
	@cat $< | $(call dbquery) | $(CARGO_RELEASE)/get-articles $@

# Category data
$(DATA_DIR)/%/categories.parquet: $(QUERIES_DIR)/categories.sql
	@mkdir -p $(dir $@)
	@echo "Fetching categories for $*..."
	@cat $< | $(call dbquery) | $(CARGO_RELEASE)/get-categories $@

# Category graph
$(DATA_DIR)/%/category_graph.parquet: $(QUERIES_DIR)/category-graph.sql $(DATA_DIR)/%/categories.parquet
	@mkdir -p $(dir $@)
	@echo "Fetching category graph for $*..."
	@cat $< | $(call dbquery) | $(CARGO_RELEASE)/get-categorygraph $(DATA_DIR)/$*/categories.parquet $@

# Article-category mapping
$(DATA_DIR)/%/article_category.parquet: $(QUERIES_DIR)/article-category.sql $(DATA_DIR)/%/articles.parquet
	@mkdir -p $(dir $@)
	@echo "Fetching article-category mapping for $*..."
	@cat $< | $(call dbquery) | \
		$(CARGO_RELEASE)/get-article_category $(DATA_DIR)/$*/articles.parquet $(DATA_DIR)/$*/categories.parquet  $@

# Force-refetch every wiki's topology from the replica. Derives the target list
# from $(WIKIS) (not a filesystem glob) so newly-added wikis are covered and
# excluded/stale dirs are skipped. Per-day pageview/pageedit/gsc files are left
# untouched — they take articles.parquet as an order-only prerequisite. Scope to
# a subset with a command-line override, e.g. `make topology-refresh WIKIS=enwiki`.
# Topology is loaded only at server startup, so restart topictrend_web afterward.
TOPOLOGY_FILES = articles.parquet categories.parquet \
                 article_category.parquet category_graph.parquet
TOPOLOGY = $(foreach w,$(WIKIS),$(addprefix $(DATA_DIR)/$(w)/,$(TOPOLOGY_FILES)))

topology-refresh: init
	@if [ -z "$(strip $(TOPOLOGY))" ]; then \
		echo "No wikis to refresh (empty WIKIS / $(DATA_DIR)/wikipedia.list)" >&2; exit 1; \
	fi
	@$(MAKE) -B $(TOPOLOGY)
	@echo "Topology refreshed. Restart topictrend_web to load it."

# Daily pageviews for a specific wiki/date.
# Expands to data/enwiki/pageviews/2025/12/30.parquet (example).
# Bound to the current $(DATE); to build a different date, pass DATE=YYYY-MM-DD.
# The raw dump is a direct prerequisite (built once, shared by all wikis on a
# given date) — avoids a recursive sub-make per wiki.
$(DATA_DIR)/%/pageviews/$(YEAR)/$(MONTH)/$(DAY).parquet: $(DATA_DIR)/pageviews/$(YEAR)/$(MONTH)/$(DAY).parquet
	@mkdir -p $(dir $@)
	@echo "Processing pageviews for $* on $(DATE) -> $@"
	$(CARGO_RELEASE)/get-per_day_wiki_stats --wiki $* --year $(YEAR) --month $(MONTH) --day $(DAY) -o $@

# Raw pageview data from the clouddumps NFS mount ($(PAGEVIEW_DIR)).
# Expands to data/pageviews/2025/12/30.parquet (example).
# A missing file (e.g. date not yet published) is skipped with a warning;
# any failure decoding/processing an existing file is a hard error.
$(DATA_DIR)/pageviews/%.parquet:
	@YEAR=$$(echo $* | cut -d'/' -f1); \
	MONTH=$$(echo $* | cut -d'/' -f2); \
	DAY=$$(basename $@ .parquet); \
	mkdir -p $$(dirname $@); \
	SRC="$(PAGEVIEW_DIR)/$$YEAR/$$YEAR-$$MONTH/pageviews-$$YEAR$$MONTH$$DAY-user.bz2"; \
	if [ ! -f "$$SRC" ]; then \
		echo "WARNING: pageviews not found for $$YEAR-$$MONTH-$$DAY: $$SRC" >&2; exit 0; \
	fi; \
	bzip2 -dc "$$SRC" \
		| $(CARGO_RELEASE)/get-pageviews $@ || { echo "Error processing pageviews from $$SRC"; exit 1; }

# Daily pageedits for a specific wiki/date, filled from the MediaWiki replica.
# Expands to data/enwiki/pageedits/2026/05/26.parquet (example).
# Mirrors the pageviews per-day layout so edits stay as current as views. Only
# complete (past) days are recorded — today's partial count is skipped so it is
# never frozen. articles.parquet is an order-only prerequisite
# (the `|`): it must exist for page_id→qid mapping, but its mtime must not force
# a re-query of existing day files, keeping per-day files idempotent across
# topology refreshes.
$(DATA_DIR)/%/pageedits/$(YEAR)/$(MONTH)/$(DAY).parquet: $(QUERIES_DIR)/day-pageedits.sql | $(DATA_DIR)/%/articles.parquet
	@today=$$(date +%Y-%m-%d); \
	if [[ "$(DATE)" > "$$today" || "$(DATE)" == "$$today" ]]; then \
		echo "Skipping pageedits for $* on $(DATE): only complete past days are recorded" >&2; \
		exit 0; \
	fi
	@mkdir -p $(dir $@)
	@echo "Fetching pageedits for $* on $(DATE) -> $@"
	sed -e 's/@NEXTDAY/$(NEXT_DATE)/' -e 's/@DAY/$(YEAR)$(MONTH)$(DAY)/' $< \
		| $(call dbquery) \
		| $(CARGO_RELEASE)/get-day-pageedits $* $@

# Wikipedia list. Downloads the active wikipedia list and strips closed /
# excluded wikis. closed.dblist is staged in a temp dir so an interrupted run
# never leaves an orphan in the repo root.
$(DATA_DIR)/wikipedia.list: | $(DATA_DIR)
	@echo "Fetching Wikipedia list..."
	@TMPDIR=$$(mktemp -d -t topictrend-wikilist.XXXXXX); \
	trap 'rm -rf "$$TMPDIR"' EXIT; \
	curl -fsSL https://noc.wikimedia.org/conf/dblists/closed.dblist > "$$TMPDIR/closed.dblist"; \
	curl -fsSL https://noc.wikimedia.org/conf/dblists/wikipedia.dblist \
		| grep -E 'wiki$$' \
		| grep -v '^#' \
		| grep -v -f "$$TMPDIR/closed.dblist" > $@; \
	sed -i '/^arbcom/d; /^test/d; /^sysop/d; /^wg_en/d; /^cebwiki/d; /^warwiki/d; /^be_x_old/d' $@

# GSC per-wiki-per-date target
# Expands to data/enwiki/gsc/2026/03/03.parquet (example)
# Depends on the GSC source parquet and the wiki's articles.parquet (for QID mapping)
$(DATA_DIR)/%/gsc/$(YEAR)/$(MONTH)/$(DAY).parquet: $(DATA_DIR)/%/articles.parquet
	@mkdir -p $(dir $@)
	@GSC_SRC="$(GSC_DIR)/date=$(DATE)/data.parquet"; \
	if [ ! -f "$$GSC_SRC" ]; then \
		echo "GSC source not found: $$GSC_SRC" >&2; exit 1; \
	fi
	$(CARGO_RELEASE)/get-gsc-qid-date \
		--wiki $* \
		--date $(DATE) \
		--gsc-dir $(GSC_DIR) \
		--output $@

# Process GSC data for all wikis for a single date
# Usage: make gsc DATE=2026-03-03
# Falls back to yesterday if DATE is not set.
gsc: init $(foreach w,$(WIKIS),$(DATA_DIR)/$(w)/gsc/$(YEAR)/$(MONTH)/$(DAY).parquet)

# Coverage matrix for one wiki, dated snapshot: depth-0 direct_coverage from
# the local relation plus qid_overlap_coverage from the canonical projection
# (article_category_canonical.parquet, produced by `make canonical` — run it
# first against the same topology state).
# Both inputs are order-only prerequisites (|): they must exist for the
# derivation, but their mtimes must not force a rebuild of an existing dated
# snapshot, keeping snapshots idempotent across topology refreshes.
$(DATA_DIR)/%/coverage/$(DATE).parquet: | $(DATA_DIR)/%/article_category.parquet $(DATA_DIR)/%/article_category_canonical.parquet
	@mkdir -p $(dir $@)
	@echo "Building coverage matrix for $* ($(DATE)) -> $@"
	$(CARGO_RELEASE)/coverage-matrix $(DATA_DIR)/$*/article_category.parquet $(DATA_DIR)/$*/article_category_canonical.parquet $@ $(DATA_DIR)/$*/pageviews $(DATE)

# Build the full coverage matrix for all wikis as a dated snapshot.
# Usage: make coverage DATE=2026-06-09  (falls back to yesterday if DATE unset)
coverage: init $(foreach w,$(WIKIS),$(DATA_DIR)/$(w)/coverage/$(DATE).parquet)

# Canonical cross-wiki article->category relation, dated snapshot.
# Stage 1 (canonical-membership) unions every wiki's article_category.parquet
# with per-edge wiki counts into data/canonical/$(DATE)/article_category.parquet
# (+ manifest.tsv, which gates the next run against truncated inputs).
# Stage 2 (canonical-projection) intersects the union with each wiki's article
# set, writing data/<wiki>/article_category_canonical.parquet and the category
# node universe data/<wiki>/categories_canonical.parquet.
# Usage: make canonical DATE=2026-06-11  (falls back to yesterday if DATE unset)
canonical: init
	DATA_DIR=$(DATA_DIR) $(CARGO_RELEASE)/canonical-membership --date $(DATE)
	DATA_DIR=$(DATA_DIR) $(CARGO_RELEASE)/canonical-projection --date $(DATE)
	DATA_DIR=$(DATA_DIR) $(CARGO_RELEASE)/canonical-labels --date $(DATE)

# Clean target
clean:
	@echo "Cleaning generated data..."
	@rm -rf $(DATA_DIR)
	@echo "Done!"

# Run the web server. Depends only on the wiki list so that starting the server
# does not re-invoke cargo build; build the binary explicitly via `make init`
# when needed. Semantic search runs in-process, so no extra service is needed.
web: $(DATA_DIR)/wikipedia.list
	@test -n "$(ZVEC_LIB_DIR)" || { echo "libzvec_c_api.so not found under $(CARGO_RELEASE)/build; run 'make init' first" >&2; exit 1; }
	LD_LIBRARY_PATH=$(ZVEC_LIB_DIR):$${LD_LIBRARY_PATH:-} $(CARGO_RELEASE)/topictrend_web

# Index a wiki's categories into zvec, in-process via fastembed + zvec-rust
# (usage: make index-wiki WIKI=enwiki).
index-wiki: $(DATA_DIR)/$(WIKI)/categories.parquet
	@test -n "$(ZVEC_LIB_DIR)" || { echo "libzvec_c_api.so not found under $(CARGO_RELEASE)/build; run 'make init' first" >&2; exit 1; }
	@echo "Indexing $(WIKI) categories into zvec..."
	$(EMBED_ENV) LD_LIBRARY_PATH=$(ZVEC_LIB_DIR):$${LD_LIBRARY_PATH:-} \
		$(CARGO_RELEASE)/topictrend_taxonomy index $(WIKI)

# Clean zvec indexes
index-clean:
	@echo "Cleaning zvec indexes..."
	@rm -rf $(ZVEC_DIR)
	@echo "Done"

# Monthly processing target
# make monthly END_DATE=2025-08-30  # Processes all dates from 2025-08-01 to 2025-08-30
# make monthly END_DATE=2025-02-15  # Processes all dates from 2025-02-01 to 2025-02-15
monthly: init
	@END_DATE_VAR=$${END_DATE:-$$(date -d "yesterday" +%Y-%m-%d)}; \
	echo "Processing month containing END_DATE=$$END_DATE_VAR..."; \
	YEAR=$$(echo $$END_DATE_VAR | cut -d'-' -f1); \
	MONTH=$$(echo $$END_DATE_VAR | cut -d'-' -f2); \
	LAST_DAY=$$(date -d "$$YEAR-$$MONTH-01 +1 month -1 day" +%d); \
	END_DAY=$$(echo $$END_DATE_VAR | cut -d'-' -f3 | sed 's/^0//'); \
	if [ "$$END_DAY" -lt "$$LAST_DAY" ]; then LAST_DAY=$$END_DAY; fi; \
	echo "Processing $$YEAR-$$MONTH (1 to $$LAST_DAY)..."; \
	for DAY in $$(seq 1 $$LAST_DAY); do \
		PROCESS_DATE=$$(printf "%s-%02d-%02d" $$YEAR $$((10#$$MONTH)) $$DAY); \
		echo "Processing date: $$PROCESS_DATE"; \
		$(MAKE) -k run DATE="$$PROCESS_DATE" || true; \
	done; \
	rm -rf $(DATA_DIR)/pageviews; \
	echo "Monthly processing complete for $$YEAR-$$MONTH!"

# Prevent deletion of intermediate files
.PRECIOUS: $(DATA_DIR)/%/articles.parquet \
           $(DATA_DIR)/%/categories.parquet \
           $(DATA_DIR)/%/category_graph.parquet \
           $(DATA_DIR)/%/article_category.parquet \
           $(DATA_DIR)/pageviews/%.parquet \
           $(DATA_DIR)/%/gsc/$(YEAR)/$(MONTH)/$(DAY).parquet
# Prevent parallel issues with shared resources
.NOTPARALLEL: $(DATA_DIR)/pageviews/%.parquet
