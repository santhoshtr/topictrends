-include .env
SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -euo pipefail -c
export $(shell sed 's/=.*//' .env)

# Date parsing - use DATE environment variable if set, otherwise use yesterday
DATE ?= $(shell date -d "yesterday" +%Y-%m-%d)
YEAR := $(shell echo $(DATE) | cut -d'-' -f1)
MONTH := $(shell echo $(DATE) | cut -d'-' -f2)
DAY := $(shell echo $(DATE) | cut -d'-' -f3)

CARGO := cargo
CARGO_RELEASE := target/release

DATA_DIR ?= data
GSC_DIR ?= $(DATA_DIR)/gsc_page_date
WIKIS := $(shell cat $(DATA_DIR)/wikipedia.list 2>/dev/null)
QUERIES_DIR := queries
PAGEVIEWS_DIR := $(DATA_DIR)/pageviews
EDIT_SNAPSHOT ?= 2026-01
MIN_EDIT_YEAR ?= 2020

.DEFAULT_GOAL := run

.PHONY: run init clean help $(WIKIS) monthly notebook index-wiki index-clean embedding-server web gsc

# Help target
help:
	@echo "This Makefile can only be used in a wmcloud VPS."
	@echo "Available targets:"
	@echo "  run     - Process all wikis and run wikigraph cli"
	@echo "  monthly - Process all wikis for the last 30 days"
	@echo "  gsc     - Process Google Search Console data for all wikis (single date)"
	@echo "  web     - Start webserver"
	@echo "  init    - Initialize data directory and wikipedia list"
	@echo "  clean   - Remove generated data files"
	@echo "  help    - Show this help message"
	@echo ""
	@echo "Environment variables:"
	@echo "  DATE          - Date to process (YYYY-MM-DD format, defaults to yesterday)"
	@echo "  EDIT_SNAPSHOT - MediaWiki history dump version (defaults to 2026-01)"
	@echo "  MIN_EDIT_YEAR - Minimum year for page edit files (defaults to 2020)"
	@echo "  GSC_DIR       - Path to GSC source root (defaults to data/gsc_page_date)"
	@echo "  Example: make run DATE=2025-01-15"
	@echo "  Example: make gsc DATE=2026-03-03"
	@echo "  Example: make gsc DATE=2026-03-03 GSC_DIR=/mnt/gsc_data"
	@echo "  Example: make data/mlwiki/pageedits/pageedits.parquet EDIT_SNAPSHOT=2025-12"
	@echo "  Example: make data/arwiki/pageedits/pageedits.parquet MIN_EDIT_YEAR=2015"
	@echo ""
	@echo "Note: Page edits processing automatically handles both single-file and"
	@echo "      multi-part dumps (e.g., arwiki with year-split files)."
	@echo "      Files with 'all-time' are always processed regardless of MIN_EDIT_YEAR."

# Main run target
run: init $(WIKIS)
	$(CARGO_RELEASE)/wikigraph

$(DATA_DIR):
	@mkdir -p $@

# Initialize directory structure
init: $(DATA_DIR)/wikipedia.list
	cargo build --release
	@mkdir -p $(DATA_DIR)

# Per-wiki targets
$(WIKIS): %: \
	$(DATA_DIR)/%/articles.parquet \
	$(DATA_DIR)/%/categories.parquet \
	$(DATA_DIR)/%/article_category.parquet \
	$(DATA_DIR)/%/category_graph.parquet \
	$(DATA_DIR)/%/pageviews/$(YEAR)/$(MONTH)/$(DAY).bin \
	$(DATA_DIR)/%/pageedits/pageedits.parquet

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

# Daily pageviews for specific wiki
# Expands to data/enwiki/pageviews/2025/12/30.bin (example)
$(DATA_DIR)/%.bin:
	@WIKI=$$(echo $@ | cut -d'/' -f2); \
	YEAR=$$(echo $@ | cut -d'/' -f4); \
	MONTH=$$(echo $@ | cut -d'/' -f5); \
	DAY=$$(basename $@ .bin); \
	echo "Processing pageviews for $$WIKI on $$YEAR-$$MONTH-$$DAY..."; \
	mkdir -p $$(dirname $@); \
	$(MAKE) $(DATA_DIR)/pageviews/$$YEAR/$$MONTH/$$DAY.parquet; \
	$(CARGO_RELEASE)/get-per_day_wiki_stats --wiki $$WIKI --year $$YEAR --month $$MONTH --day $$DAY -o $@

# Raw pageview data from Wikimedia
# Expands to data/pageviews/2025/12/30.parquet (example)
$(DATA_DIR)/pageviews/%.parquet:
	@YEAR=$$(echo $@ | cut -d'/' -f3); \
	MONTH=$$(echo $@ | cut -d'/' -f4); \
	DAY=$$(basename $@ .parquet); \
	mkdir -p $$(dirname $@); \
	URL="https://dumps.wikimedia.org/other/pageview_complete/$$YEAR/$$YEAR-$$MONTH/pageviews-$$YEAR$$MONTH$$DAY-user.bz2"; \
	curl -fsSL "$$URL" | bzip2 -dc \
		| $(CARGO_RELEASE)/get-pageviews $@ || { echo "Error downloading pageviews"; exit 1; }

# Page edits from MediaWiki history dumps
# Supports both single all-time file and multi-part dumps
# Expands to data/mlwiki/pageedits/pageedits.parquet (example)
$(DATA_DIR)/%/pageedits/pageedits.parquet:
	@WIKI=$*; \
	mkdir -p $$(dirname $@); \
	BASE_URL="https://dumps.wikimedia.org/other/mediawiki_history/$(EDIT_SNAPSHOT)/$$WIKI/"; \
	TEMP_DIR="/tmp/topictrend-$$WIKI-pageedits-$$$$"; \
	mkdir -p "$$TEMP_DIR"; \
	echo "Processing page edits for $$WIKI from snapshot $(EDIT_SNAPSHOT)..."; \
	echo "Fetching file list from $$BASE_URL"; \
	FILELIST="$$TEMP_DIR/filelist.txt"; \
	wget -q -O - "$$BASE_URL" | grep -oP 'href="\K[^"]*\.bz2(?=")' > "$$FILELIST" || { \
		echo "Error fetching file list from $$BASE_URL" >&2; \
		rm -rf "$$TEMP_DIR"; \
		exit 1; \
	}; \
	if [ ! -s "$$FILELIST" ]; then \
		echo "No .bz2 files found at $$BASE_URL" >&2; \
		rm -rf "$$TEMP_DIR"; \
		exit 1; \
	fi; \
	echo "Filtering files by year (MIN_EDIT_YEAR=$(MIN_EDIT_YEAR))..."; \
	FILTERED_LIST="$$TEMP_DIR/filtered.txt"; \
	SKIPPED=0; \
	while IFS= read -r filename; do \
		year_part="$$(echo "$$filename" | cut -d'.' -f3)"; \
		if [ "$$year_part" = "all-time" ]; then \
			echo "$$filename" >> "$$FILTERED_LIST"; \
		else \
			year="$${year_part:0:4}"; \
			if [[ "$$year" =~ ^[0-9]{4}$$ ]] && [ "$$year" -ge "$(MIN_EDIT_YEAR)" ]; then \
				echo "$$filename" >> "$$FILTERED_LIST"; \
			else \
				SKIPPED=$$((SKIPPED+1)); \
			fi; \
		fi; \
	done < "$$FILELIST"; \
	if [ ! -s "$$FILTERED_LIST" ]; then \
		echo "No files remain after year filtering (MIN_EDIT_YEAR=$(MIN_EDIT_YEAR))" >&2; \
		rm -rf "$$TEMP_DIR"; \
		exit 1; \
	fi; \
	if [ "$$SKIPPED" -gt 0 ]; then \
		echo "Skipped $$SKIPPED file(s) before year $(MIN_EDIT_YEAR)"; \
	fi; \
	TOTAL=$$(wc -l < "$$FILTERED_LIST"); \
	echo "Found $$TOTAL dump file(s) to download and process"; \
	i=0; \
	{ \
		while IFS= read -r filename; do \
			i=$$((i+1)); \
			echo "[$$i/$$TOTAL] Downloading $$filename..." >&2; \
			FILE_PATH="$$TEMP_DIR/$$filename"; \
			wget -q --show-progress -O "$$FILE_PATH" "$$BASE_URL$$filename" || { \
				echo "Error downloading $$filename" >&2; \
				rm -rf "$$TEMP_DIR"; \
				exit 1; \
			}; \
			echo "[$$i/$$TOTAL] Decompressing $$filename..." >&2; \
			bzip2 -dc "$$FILE_PATH" || { \
				echo "Error decompressing $$filename" >&2; \
				rm -rf "$$TEMP_DIR"; \
				exit 1; \
			}; \
			rm -f "$$FILE_PATH"; \
			echo "Deleted $$filename to free disk space" >&2; \
		done < "$$FILTERED_LIST"; \
	} | $(CARGO_RELEASE)/get-pageedits $$WIKI $@ || { \
		echo "Error processing page edits for $$WIKI" >&2; \
		rm -rf "$$TEMP_DIR"; \
		exit 1; \
	}; \
	rm -rf "$$TEMP_DIR"; \
	echo "Cleaned up temporary files from $$TEMP_DIR"

# Wikipedia list
$(DATA_DIR)/wikipedia.list: | $(DATA_DIR)
	@echo "Fetching Wikipedia list..."
	@mkdir -p $(DATA_DIR)
	@curl -fsSL https://noc.wikimedia.org/conf/dblists/closed.dblist > closed.dblist
	@curl -fsSL https://noc.wikimedia.org/conf/dblists/wikipedia.dblist \
		| grep -E 'wiki$$' \
		| grep -v '^#' \
		| grep -v -f closed.dblist > $@
	@sed -i '/^arbcom/d; /^test/d; /^sysop/d; /^wg_en/d; /^cebwiki/d; /^warwiki/d; /^be_x_old/d' $@
	@rm -f closed.dblist

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
.PHONY: gsc
gsc: init $(foreach w,$(WIKIS),$(DATA_DIR)/$(w)/gsc/$(YEAR)/$(MONTH)/$(DAY).parquet)

# Clean target
clean:
	@echo "Cleaning generated data..."
	@rm -rf $(DATA_DIR)
	@echo "Done!"


web: init
	@echo "Checking embedding server at $(EMBEDDING_SERVER)..."
	@(cd services/embedding && EMBEDDING_SERVER=$(EMBEDDING_SERVER) uv run python healthcheck.py)
	@echo "Embedding server OK"
	$(CARGO_RELEASE)/topictrend_web

# Start the embedding gRPC server from the project root so DATA_DIR and
# ZVEC_DIR resolve as absolute paths regardless of shell CWD.
embedding-server:
	@echo "Starting embedding server..."
	@cd services/embedding && \
		DATA_DIR=$(abspath $(DATA_DIR)) \
		ZVEC_DIR=$(abspath $(ZVEC_DIR)) \
		uv run python embedding_server.py

# Embedding DB indexing configuration
EMBEDDING_SERVER ?= localhost:50051
ZVEC_DIR ?= $(DATA_DIR)/embedding_store/zvec
WIKI ?= enwiki

# Index enwiki categories into zvec embedding database
# Usage: make index-wiki
.PHONY: index-wiki index-clean

index-wiki: $(DATA_DIR)/$(WIKI)/categories.parquet
	@echo "Indexing $(WIKI) categories into zvec..."
	@cd services/embedding && DATA_DIR=$(abspath $(DATA_DIR)) ZVEC_DIR=$(abspath $(ZVEC_DIR)) uv run python index_categories.py --wiki $(WIKI) --server $(EMBEDDING_SERVER)

# Clean zvec indexes
index-clean:
	@echo "Cleaning zvec indexes..."
	@rm -rf $(ZVEC_DIR)
	@echo "Done"

# Ensure categories parquet exists before indexing
$(DATA_DIR)/%/categories.parquet: 
	@$(MAKE) $(DATA_DIR)/$*/categories.parquet

# Monthly processing target
# make monthly END_DATE=2025-08-30  # Processes all dates from 2025-08-01 to 2025-08-31
# make monthly END_DATE=2025-02-15  # Processes all dates from 2025-02-01 to 2025-02-28
monthly: init
	@END_DATE_VAR=$${END_DATE:-$$(date +%Y-%m-%d)}; \
	echo "Processing month containing END_DATE=$$END_DATE_VAR..."; \
	YEAR=$$(echo $$END_DATE_VAR | cut -d'-' -f1); \
	MONTH=$$(echo $$END_DATE_VAR | cut -d'-' -f2); \
	LAST_DAY=$$(date -d "$$YEAR-$$MONTH-01 +1 month -1 day" +%d); \
	echo "Processing $$YEAR-$$MONTH (1 to $$LAST_DAY)..."; \
	for DAY in $$(seq 1 $$LAST_DAY); do \
		PROCESS_DATE=$$(printf "%s-%02d-%02d" $$YEAR $$((10#$$MONTH)) $$DAY); \
		echo "Processing date: $$PROCESS_DATE"; \
		$(MAKE) run DATE="$$PROCESS_DATE" || true; \
	done; \
	rm -rf $(DATA_DIR)/pageviews; \
	echo "Monthly processing complete for $$YEAR-$$MONTH!"

# Start a jupyter server from topictrend_web folder.
# Allow google colab as trusted origin so that we can use
# google colab to connect with this runtime.
notebook:
	@cd topictrend_web; \
	command -v uv >/dev/null 2>&1 || { echo >&2 "uv is required but not installed. Aborting."; exit 1; }; \
	uv run --with jupyter jupyter lab \
		--ServerApp.allow_origin='https://colab.research.google.com' \
		--ServerApp.allow_credentials=True \
		--ServerApp.port_retries=0 \
		--ServerApp.disable_check_xsrf=True \
		--ServerApp.allow_remote_access=True \
		--port=8888 \
		--no-browser \
		--ip=0.0.0.0

# Prevent deletion of intermediate files
.PRECIOUS: $(DATA_DIR)/%/articles.parquet \
           $(DATA_DIR)/%/categories.parquet \
           $(DATA_DIR)/%/category_graph.parquet \
           $(DATA_DIR)/%/article_category.parquet \
           $(DATA_DIR)/pageviews/%.parquet \
           $(DATA_DIR)/%/pageedits/pageedits.parquet \
           $(DATA_DIR)/%/gsc/%.parquet
# Prevent parallel issues with shared resources
.NOTPARALLEL: $(DATA_DIR)/pageviews/%.parquet
