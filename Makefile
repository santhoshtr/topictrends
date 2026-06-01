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

CARGO_RELEASE := target/release

DATA_DIR ?= data
GSC_DIR ?= $(DATA_DIR)/gsc_page_date
QUERIES_DIR := queries

# Page edit defaults are derived from END_DATE (for `monthly`) or DATE so that
# backfills pick the right MediaWiki history snapshot rather than always
# defaulting to last month relative to today.
REFERENCE_DATE := $(if $(END_DATE),$(END_DATE),$(DATE))
EDIT_SNAPSHOT ?= $(shell date -d "$(REFERENCE_DATE) -1 month" +%Y-%m)
MIN_EDIT_YEAR ?= $(shell date -d "$(REFERENCE_DATE) -1 year" +%Y)

# Embedding service configuration
EMBEDDING_SERVER ?= localhost:50051
ZVEC_DIR ?= $(DATA_DIR)/embedding_store/zvec
WIKI ?= enwiki
EMBED_ENV := DATA_DIR=$(abspath $(DATA_DIR)) ZVEC_DIR=$(abspath $(ZVEC_DIR))

# Deferred so the list is re-read after `init` produces it on a fresh checkout.
WIKIS = $(shell cat $(DATA_DIR)/wikipedia.list 2>/dev/null)
PAGEEDITS_FILES = $(addsuffix /pageedits/pageedits.parquet,$(addprefix $(DATA_DIR)/,$(WIKIS)))

.DEFAULT_GOAL := run

.PHONY: run init clean help monthly index-wiki index-clean embedding-server web gsc _run-wikis pageedits _pageedits-wikis

# Help target
help:
	@echo "This Makefile is intended for the wmcloud VPS (replica access + GNU date)."
	@echo ""
	@echo "Available targets:"
	@echo "  run              - Build binaries and process all wikis for DATE"
	@echo "  monthly          - Process all wikis for the calendar month containing END_DATE"
	@echo "                     (1..LAST_DAY of that month, capped at END_DATE's day)"
	@echo "  gsc              - Process Google Search Console data for all wikis (single DATE)"
	@echo "  pageedits        - Refresh page edits for all wikis at EDIT_SNAPSHOT (deletes"
	@echo "                     existing parquets first so the new snapshot is re-fetched)"
	@echo "  web              - Start the web server (no rebuild)"
	@echo "  init             - Build release binaries and fetch the Wikipedia list"
	@echo "  embedding-server - Start the embedding gRPC server"
	@echo "  index-wiki       - Index a wiki's categories into zvec (WIKI=enwiki)"
	@echo "  index-clean      - Remove the zvec index store"
	@echo "  clean            - Remove all generated data files"
	@echo "  help             - Show this help message"
	@echo ""
	@echo "Environment variables:"
	@echo "  DATE          - Date to process (YYYY-MM-DD, defaults to yesterday)"
	@echo "  END_DATE      - Last day for 'monthly' (YYYY-MM-DD, defaults to yesterday)"
	@echo "  EDIT_SNAPSHOT - MediaWiki history dump snapshot (defaults to the month before"
	@echo "                  END_DATE/DATE, e.g. END_DATE=2024-01-31 -> 2023-12)"
	@echo "  MIN_EDIT_YEAR - Minimum year for page edit files (defaults to the year before"
	@echo "                  END_DATE/DATE)"
	@echo "  GSC_DIR       - Path to GSC source root (defaults to data/gsc_page_date)"
	@echo "  WIKI          - Wiki for index-wiki (defaults to enwiki)"
	@echo ""
	@echo "Examples:"
	@echo "  make run DATE=2025-01-15"
	@echo "  make monthly END_DATE=2025-08-30"
	@echo "  make gsc DATE=2026-03-03"
	@echo "  make gsc DATE=2026-03-03 GSC_DIR=/mnt/gsc_data"
	@echo "  make data/mlwiki/pageedits/pageedits.parquet EDIT_SNAPSHOT=2025-12"
	@echo "  make data/arwiki/pageedits/pageedits.parquet MIN_EDIT_YEAR=2015"
	@echo ""
	@echo "Note: Page edits processing handles both single-file and multi-part dumps"
	@echo "      (e.g., arwiki with year-split files). Files with 'all-time' are"
	@echo "      always processed regardless of MIN_EDIT_YEAR."

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

# Refresh page edits for all wikis at EDIT_SNAPSHOT.
# The pageedits file rule has no prerequisites, so an existing parquet is always
# considered up-to-date — changing EDIT_SNAPSHOT alone won't rebuild it. We
# delete the existing files first, then re-enter Make so the now-missing targets
# re-fire at the new snapshot. Pass EDIT_SNAPSHOT=YYYY-MM (and optionally
# MIN_EDIT_YEAR=YYYY) on the command line.
# Caveat: a wiki whose new snapshot 404s is skipped with a warning, leaving it
# with no pageedits file until the next successful fetch.
pageedits:
	@if [ ! -f $(DATA_DIR)/wikipedia.list ]; then \
		echo "Bootstrapping wikipedia.list..."; \
		$(MAKE) -s init; \
	fi
	rm -f $(PAGEEDITS_FILES)
	@$(MAKE) -s _pageedits-wikis

_pageedits-wikis: init $(PAGEEDITS_FILES)
	@echo "✓ pageedits refreshed for $(words $(WIKIS)) wikis at snapshot $(EDIT_SNAPSHOT)"

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
	$(DATA_DIR)/%/pageedits/pageedits.parquet
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
$(DATA_DIR)/%/category_graph.parquet: $(QUERIES_DIR)/category-graph.sql
	@mkdir -p $(dir $@)
	@echo "Fetching category graph for $*..."
	@cat $< | $(call dbquery) | $(CARGO_RELEASE)/get-categorygraph $(DATA_DIR)/$*/categories.parquet $@

# Article-category mapping
$(DATA_DIR)/%/article_category.parquet: $(QUERIES_DIR)/article-category.sql $(DATA_DIR)/%/articles.parquet
	@mkdir -p $(dir $@)
	@echo "Fetching article-category mapping for $*..."
	@cat $< | $(call dbquery) | \
		$(CARGO_RELEASE)/get-article_category $(DATA_DIR)/$*/articles.parquet $(DATA_DIR)/$*/categories.parquet  $@

# Daily pageviews for a specific wiki/date.
# Expands to data/enwiki/pageviews/2025/12/30.parquet (example).
# Bound to the current $(DATE); to build a different date, pass DATE=YYYY-MM-DD.
# The raw dump is a direct prerequisite (built once, shared by all wikis on a
# given date) — avoids a recursive sub-make per wiki.
$(DATA_DIR)/%/pageviews/$(YEAR)/$(MONTH)/$(DAY).parquet: $(DATA_DIR)/pageviews/$(YEAR)/$(MONTH)/$(DAY).parquet
	@mkdir -p $(dir $@)
	@echo "Processing pageviews for $* on $(DATE) -> $@"
	$(CARGO_RELEASE)/get-per_day_wiki_stats --wiki $* --year $(YEAR) --month $(MONTH) --day $(DAY) -o $@

# Raw pageview data from Wikimedia
# Expands to data/pageviews/2025/12/30.parquet (example).
# Uses a HEAD request to detect 404 (e.g. date not yet published) and skip
# without erroring; any other failure during the actual download is a hard error.
$(DATA_DIR)/pageviews/%.parquet:
	@YEAR=$$(echo $* | cut -d'/' -f1); \
	MONTH=$$(echo $* | cut -d'/' -f2); \
	DAY=$$(basename $@ .parquet); \
	mkdir -p $$(dirname $@); \
	URL="https://dumps.wikimedia.org/other/pageview_complete/$$YEAR/$$YEAR-$$MONTH/pageviews-$$YEAR$$MONTH$$DAY-user.bz2"; \
	if ! curl -fsSI -o /dev/null "$$URL"; then \
		echo "WARNING: pageviews not found (404) for $$YEAR-$$MONTH-$$DAY: $$URL" >&2; exit 0; \
	fi; \
	curl -fsSL "$$URL" | bzip2 -dc \
		| $(CARGO_RELEASE)/get-pageviews $@ || { echo "Error processing pageviews from $$URL"; exit 1; }

# Page edits from MediaWiki history dumps
# Expands to data/mlwiki/pageedits/pageedits.parquet (example).
# The fetch/decompress pipeline lives in scripts/fetch_pageedits.sh; this rule
# only orchestrates a 404 check + the stream-to-binary pipe.
$(DATA_DIR)/%/pageedits/pageedits.parquet:
	@mkdir -p $(dir $@)
	@if ! scripts/fetch_pageedits.sh --check $* "$(EDIT_SNAPSHOT)"; then \
		echo "WARNING: pageedits not found (404) for $* at snapshot $(EDIT_SNAPSHOT)" >&2; \
		exit 0; \
	fi
	scripts/fetch_pageedits.sh $* "$(EDIT_SNAPSHOT)" "$(MIN_EDIT_YEAR)" \
		| $(CARGO_RELEASE)/get-pageedits $* $@

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

# Clean target
clean:
	@echo "Cleaning generated data..."
	@rm -rf $(DATA_DIR)
	@echo "Done!"

# Run the web server. Depends only on the wiki list so that starting the server
# does not re-invoke cargo build; build the binary explicitly via `make init`
# when needed.
web: $(DATA_DIR)/wikipedia.list
	@echo "Checking embedding server at $(EMBEDDING_SERVER)..."
	@(cd services/embedding && EMBEDDING_SERVER=$(EMBEDDING_SERVER) uv run python healthcheck.py)
	@echo "Embedding server OK"
	$(CARGO_RELEASE)/topictrend_web

# Start the embedding gRPC server from the project root so DATA_DIR and
# ZVEC_DIR resolve as absolute paths regardless of shell CWD.
embedding-server:
	@echo "Starting embedding server..."
	@cd services/embedding && $(EMBED_ENV) uv run python embedding_server.py

# Index a wiki's categories into zvec (usage: make index-wiki WIKI=enwiki)
index-wiki: $(DATA_DIR)/$(WIKI)/categories.parquet
	@echo "Indexing $(WIKI) categories into zvec..."
	@cd services/embedding && $(EMBED_ENV) \
		uv run python index_categories.py --wiki $(WIKI) --server $(EMBEDDING_SERVER)

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
           $(DATA_DIR)/%/pageedits/pageedits.parquet \
           $(DATA_DIR)/%/gsc/$(YEAR)/$(MONTH)/$(DAY).parquet
# Prevent parallel issues with shared resources
.NOTPARALLEL: $(DATA_DIR)/pageviews/%.parquet
