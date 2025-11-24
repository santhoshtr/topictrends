# =========================================================
# NOTE: This Makefile is optimized for parallel execution.
# Run with `make -j` to use all CPU cores.
# This makefile is designed to run on WMF analystics servers (stat machines)
# It assumes the presence of analytics-mysql tool to query production databases
#
# Example usage:
#
# make eswiki
# make data/eswiki/pageviews/2025/10/20.bin
# =========================================================


WIKIS := $(shell cat wikipedia.list)
TODAY := $(shell date +%Y-%m-%d)
YEAR := $(shell date +%Y)
MONTH := $(shell date +%m)
DAY := $(shell date +%d)

.PHONY: run

run: init $(WIKIS)
	cargo run --release --bin wikigraph

init: wikipedia.list
	mkdir -p data

.PHONY: $(WIKIS)
$(WIKIS): %:
	$(MAKE) data/$*/articles.parquet
	$(MAKE) data/$*/categories.parquet
	$(MAKE) data/$*/article_category.parquet
	$(MAKE) data/$*/category_graph.parquet

data/%/articles.parquet:
	@mkdir -p data/$*
	cat queries/articles.sql | analytics-mysql $* | cargo run --release --bin get-articles $@

data/%/categories.parquet:
	@mkdir -p data/$*
	cat queries/categories.sql | analytics-mysql $* | cargo run --release --bin get-categories $@

data/%/category_graph.parquet:
	@mkdir -p data/$*

	cat queries/category-graph.sql | analytics-mysql $* | cargo run --release --bin get-categorygraph $@

data/%/article_category.parquet:
	@mkdir -p data/$*

	cat queries/article-category.sql | analytics-mysql $* | cargo run --release --bin get-article_category $@

# Expands to data/enwiki/pageviews/2025/12/30.bin (example)
data/%/pageviews/%/%/%.bin:
	@WIKI=$$(echo $@ | cut -d'/' -f2); \
	YEAR=$$(echo $@ | cut -d'/' -f4); \
	MONTH=$$(echo $@ | cut -d'/' -f5); \
	DAY=$$(echo $@ | cut -d'/' -f6);

	@mkdir -p data/$$WIKI/pageviews/$$YEAR/$$MONTH

	$(MAKE) data/pageviews/$$YEAR/$$MONTH/$$DAY.parquet
	cargo run --release --bin get-daily-pageviews $$WIKI $$YEAR $$MONTH $$DAY $@

# Expands to data/pageviews/2025/12/30 (example)
data/pageviews/%/%/%.parquet:
	@YEAR=$$(echo $@ | cut -d'/' -f3); \
	MONTH=$$(echo $@ | cut -d'/' -f4); \
	DAY=$$(echo $@ | cut -d'/' -f5)

	@mkdir -p data/pageviews/$$YEAR/$$MONTH
	curl https://dumps.wikimedia.org/other/pageview_complete/$$YEAR/$$YEAR-$$MONTH-$$DAY/pageviews-$$YEAR$$MONTH$$DAY-user.bz2 \
    | bzip2 -dc \
	| cargo run --release --bin get-pageviews $@

wikipedia.list:
	curl -s https://noc.wikimedia.org/conf/dblists/closed.dblist > closed.dblist
	curl -s https://noc.wikimedia.org/conf/dblists/wikipedia.dblist | grep -E 'wiki$$' | grep -v '^#' | grep -v -f closed.dblist > $@
	sed -i '/^arbcom/d' $@
	sed -i '/^sysop/d' $@
	sed -i '/^wg_en/d' $@
	sed -i '/^cebwiki/d' $@
	sed -i '/^warwiki/d' $@
	sed -i '/^be_x_old/d' $@
	rm closed.dblist

