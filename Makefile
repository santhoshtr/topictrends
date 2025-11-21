run: data
	cargo run --bin wikigraph

data: data/articles.parquet \
	data/categories.parquet \
	data/cat_children.parquet \
	data/cat_parents.parquet \
	data/article_category.parquet

data/articles.parquet:
	cargo run --bin get-articles

data/categories.parquet:
	cargo run --bin get-categories

data/cat_children.parquet:
	cargo run --bin get-categorygraph

data/cat_parents.parquet:
	cargo run --bin get-categorygraph

data/article_category.parquet:
	cargo run --bin get-article_category

default: run


