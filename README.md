##  WikiGraph Struct:

- children: Vec<Vec<u32>>: This is your primary navigation tool. It maps Dense_Category_ID -> List<Dense_Category_ID>. Access is O(1)O(1).
- cat_articles: Vec<RoaringBitmap>: This maps Dense_Category_ID -> CompressedSet<Dense_Article_ID>. This is the secret sauce.
- *_original_to_dense: This HashMap is used only at the boundary (when you receive a request with a Wiki Page ID). Internally, the graph never touches this map, ensuring pure integer speed.

## GraphBuilder::build:
- It loads categories.parquet first to establish a dense ID space (00 to NN). This allows us to use Vec instead of HashMap for the graph structure, saving massive amounts of memory and CPU cycles during traversal.
- It iterates through Polars columns. While Polars is columnar, we need to perform row-wise logic to build the Adjacency List, so we zip the iterators.
-    get_articles_in_category:     It performs a BFS (Breadth-First Search).         It uses RoaringBitmap::union_with to aggregate results. If you have a subcategory with 1 million articles and another with 1 million articles, merging them takes microseconds because Roaring operates on bitwise chunks (SIMD), not individual elements.
