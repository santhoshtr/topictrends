WIKIS := $(shell cat wikipedia.list)

.PHONY: lint run data

run: init data
	cargo run --release --bin wikigraph

init: wikipedia.list
	mkdir -p data

data: init $(addprefix data/articles.,$(addsuffix .parquet,$(WIKIS))) \
	$(addprefix data/categories.,$(addsuffix .parquet,$(WIKIS))) \
	$(addprefix data/category_graph.,$(addsuffix .parquet,$(WIKIS))) \
	$(addprefix data/article_category.,$(addsuffix .parquet,$(WIKIS)))

data/articles.%.parquet:
	cat queries/articles.sql | analytics-mysql $* | cargo run --release --bin get-articles $@

data/categories.%.parquet:
	cat queries/categories.sql | analytics-mysql $* | cargo run --release --bin get-categories $@

data/category_graph.%.parquet:
	cat queries/category-graph.sql | analytics-mysql $* | cargo run --release --bin get-categorygraph $@

data/article_category.%.parquet:
	cat queries/article-category.sql | analytics-mysql $* | cargo run --release --bin get-article_category $@

data/pageviews-2025-11-20.parquet:
	curl https://dumps.wikimedia.org/other/pageview_complete/2025/2025-11/pageviews-20251120-user.bz2 \
    | bzip2 -dc \
	| cargo run --release --bin get-pageviews $@

lint:
	cargo clippy

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

