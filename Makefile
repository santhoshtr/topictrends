SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -euo pipefail -c
YEAR := $(shell date -d "yesterday" +%Y)
MONTH := $(shell date -d "yesterday" +%m)
DAY := $(shell date -d "yesterday" +%d)

CARGO := cargo
CARGO_RELEASE := target/release

DATA_DIR ?= data
WIKIS := $(shell cat $(DATA_DIR)/wikipedia.list 2>/dev/null)
QUERIES_DIR := queries
PAGEVIEWS_DIR := $(DATA_DIR)/pageviews

.DEFAULT_GOAL := run

.PHONY: run init clean help $(WIKIS) qdrant

# Help target
help:
	@echo "This Makefile can only be used in a wmcloud VPS."
	@echo "Available targets:"
	@echo "  run     - Process all wikis and run wikigraph cli"
	@echo "  web     - Start webserver"
	@echo "  init    - Initialize data directory and wikipedia list"
	@echo "  clean   - Remove generated data files"
	@echo "  help    - Show this help message"

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
	$(DATA_DIR)/%/pageviews/$(YEAR)/$(MONTH)/$(DAY).bin

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
	@cat $< | $(call dbquery) | $(CARGO_RELEASE)/get-categorygraph $@

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

# Wikipedia list
$(DATA_DIR)/wikipedia.list: | $(DATA_DIR)
	@echo "Fetching Wikipedia list..."
	@mkdir -p $(DATA_DIR)
	@curl -fsSL https://noc.wikimedia.org/conf/dblists/closed.dblist > closed.dblist
	@curl -fsSL https://noc.wikimedia.org/conf/dblists/wikipedia.dblist \
		| grep -E 'wiki$$' \
		| grep -v '^#' \
		| grep -v -f closed.dblist > $@
	@sed -i '/^arbcom/d; /^sysop/d; /^wg_en/d; /^cebwiki/d; /^warwiki/d; /^be_x_old/d' $@
	@rm -f closed.dblist

# Clean target
clean:
	@echo "Cleaning generated data..."
	@rm -rf $(DATA_DIR)
	@echo "Done!"


web: init
	 $(CARGO_RELEASE)/topictrend_web

qdrant:
	# Port 6334 is GRPC and that is what rust will use.
	docker run -d --rm -p 6333:6333 -p 6334:6334 --name qdrant qdrant/qdrant

# Prevent deletion of intermediate files
.PRECIOUS: $(DATA_DIR)/%/articles.parquet \
           $(DATA_DIR)/%/categories.parquet \
           $(DATA_DIR)/%/category_graph.parquet \
           $(DATA_DIR)/%/article_category.parquet \
           $(DATA_DIR)/pageviews/%.parquet
# Prevent parallel issues with shared resources
.NOTPARALLEL: $(DATA_DIR)/pageviews/%.parquet
