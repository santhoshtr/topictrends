# TopicTrends: Architecture

This document describes the core architectural principles, data structures, and algorithms that power TopicTrends.

For why these choices were made over alternatives, see [DESIGN.md](DESIGN.md).
For semantic search architecture specifically, see [SEMANTIC_SEARCH.md](SEMANTIC_SEARCH.md).

## Core Principles

### QID-Centric Numerics

TopicTrends enforces a strict "no strings in the core engine" principle. The system operates exclusively on Wikidata QIDs — universal, language-agnostic identifiers representing concepts across Wikipedia. The `Q` prefix is stripped; QIDs are stored as native `u32` integers (e.g., `Q42` becomes `42`).

This has two practical consequences. First, `u32` operations are atomic, cache-friendly, and SIMD-compatible — comparing strings or parsing titles is eliminated from the hot path. Second, QIDs provide stability: the same identifier represents the same concept across all 345 languages. An article about "Douglas Adams" is `Q42` in every Wikipedia.

The boundary between strings and numbers is pushed to the system's edge. The Axum web layer translates incoming titles to QIDs via database lookup and translates outgoing QIDs back to titles. The core engine never sees strings.

### Data-Oriented Design

Rather than modeling entities as objects with methods and properties, TopicTrends represents the world as flat, contiguous arrays of numbers. A category is not an object; it is an index into vectors. The article-category relationship is not an edge object; it is a position in a Compressed Sparse Row matrix.

This maximizes memory locality. Modern CPUs achieve peak performance when data fits in cache and is accessed sequentially. Pointer-chasing through object graphs causes cache misses; arrays of numbers are prefetched efficiently.

### Edge-Layer Separation

String handling, title resolution, and human-readable output are pushed to the edge — the Axum web server. This separation preserves the core engine's purity (algorithms operate on integers only) and enables independent optimization. The web layer can be replicated, cached, or replaced without affecting core logic.

## Data Structures

### Compressed Sparse Row for Massive Graphs

English Wikipedia has 196 million article-category relationships. A naive representation using nested vectors (`Vec<Vec<u32>>`) would incur 24 bytes of overhead per article for vector metadata, totaling 168 MB just in headers. With heap fragmentation, this quickly becomes prohibitive.

TopicTrends uses Compressed Sparse Row (CSR) representation — a classical sparse matrix format. The structure requires two arrays: `offsets` (one entry per article) and `targets` (one entry per relationship). For English Wikipedia, this totals approximately 28 MB for offsets and 784 MB for targets, consuming under 1 GB RAM total. Both arrays are contiguous in memory, enabling prefetching and cache efficiency that generic data structures cannot match.

CSR representations are immutable once constructed; dynamic insertion requires reconstruction. This constraint maps perfectly to TopicTrends' batch-oriented update model — topology is refreshed monthly as a complete operation, not incrementally.

### Per-Day Pageview Parquet, Densified On Load

Pageview data arrives daily for 345 wikis. Each `(wiki, date)` is stored as a small Parquet file with schema `(qid: u32, views: u32)`, sorted by `qid` and sparse — only articles with non-zero views for the date appear. This is refresh-stable: the on-disk key is the stable Wikidata QID, not a positional dense index, so rebuilding `articles.parquet` (additions, deletions) leaves historical files valid.

At load time the engine translates each QID to the current dense article ID via `art_original_to_dense` and materializes a `Vec<u32>` indexed by dense ID, cached per date in process memory. Subsequent reads — per-article lookup in `get_article_trend`, RoaringBitmap-driven traversal in `get_category_trend`, full-vector SIMD aggregation in `get_top_categories` / `get_top_articles` — all operate on the dense in-memory layout exactly as before. The Parquet decompression is paid once per cold date load; the warm path is unchanged from the prior format.

### Sparse In-Memory Pageedit Data

Pageedit data has a different profile from pageviews: article edit activity is significantly sparser (most articles have zero edits on any given day), and the full history is processed as a batch rather than daily increments.

Pageedits are stored as a single Parquet file per wiki (`data/{wiki}/pageedits/pageedits.parquet`) with columns `article_qid`, `date`, and `edit_count`. At startup, the engine loads this file and builds a `HashMap<NaiveDate, DailyEditData>` in memory.

`DailyEditData` stores only articles with non-zero edit counts for that date as two parallel sorted `Vec<u32>` arrays — article dense IDs and corresponding counts. Lookup is O(log n) via binary search. This structure uses far less memory than a dense array, which would be wasteful given the sparsity of edit activity.

## Algorithms

### Recursive Aggregation with Cycle Handling

Computing the total pageviews for a category including all its subcategories requires traversing a potentially 20-level-deep tree. The category graph contains cycles — a category can appear under multiple parents, and those parents might eventually reference the original category indirectly. Naive recursion would infinite-loop; naive visited-set tracking would over-count.

The solution is level-wise propagation. The graph depth is analyzed once during startup. At query time, scores from leaf categories (depth N) are propagated upward to parents (depth N-1), then to their parents (depth N-2), and so on. Propagation always moves toward the root, eliminating infinite loops. Cycles are handled by tracking visited nodes, ensuring each category contributes its value exactly once.

This achieves O(E) complexity where E is the number of edges (196 million for English Wikipedia). The entire aggregation for a category tree completes in approximately 20 milliseconds.

### Trending Discovery via Reverse Scatter

Finding which categories are trending without checking all 2.5 million of them requires a smarter approach than computing and sorting all category scores.

Instead, the system performs a reverse scatter operation. Rather than pushing from categories to articles, it starts from articles (which contain pageview or pageedit data) and scatters values to their parent categories via the CSR adjacency. Because only articles with non-zero data participate, the algorithm's cost is proportional to article traffic, not category count. For typical Wikipedia traffic, this operation completes in milliseconds.

## Cross-Lingual Unification

### QID as Universal Identifier

Wikidata assigns a unique, immutable identifier to every concept in human knowledge. The identifier `Q11019` represents "artificial intelligence" in all 345 Wikipedia languages and all time periods. In English Wikipedia it maps to "Artificial intelligence", in French to "Intelligence artificielle", in German to "Künstliche Intelligenz".

Most systems treat language editions as siloed. With QID unification, the system can correlate trends across languages. If a topic's QID is accumulating pageviews in French, German, Spanish, and Japanese simultaneously, that is a global trend independent of which language edition surfaced it first.

### Current State

TopicTrends currently analyzes each Wikipedia independently and uses QIDs to enable semantic search results to be translated across languages. The architecture supports per-language analytics with cross-lingual title resolution at the web layer.

### Future Possibility

The architecture supports aggregating pageview vectors across languages:

```
GlobalViews(Q42) = EnglishViews(Q42) + FrenchViews(Q42) + GermanViews(Q42) + ...
```

This would reveal which topics are genuinely globally viral versus locally popular. It requires no architectural changes — only the aggregation logic.

## Performance Characteristics

The following latencies are observed in production on commodity hardware:

| Operation | Complexity | Typical Latency |
|---|---|---|
| Category pageview aggregation | O(E), E = edges in subgraph | 15–25 ms |
| Trending category discovery | O(N), N = articles with traffic | 20–50 ms |
| Article pageview lookup | O(1), direct mmap | < 1 ms |
| Article pageedit lookup | O(log n), binary search | < 1 ms |
| Semantic search (encode + search + translate) | O(log N) vector search | 50–150 ms |
| Title translation (batch) | Database round-trip | 5–10 ms |

The architecture scales with the number of categories (2.5 million for English), not exponentially with language count. Adding a new Wikipedia edition requires ingesting its topology and time series, but does not increase the semantic search footprint.

## Crate Structure & Testing

TopicTrends follows Unix philosophy: small, focused crates with single responsibilities, each tested in isolation.

- **`topictrend_core`**: Pure numeric algorithms — CSR traversal, level-wise aggregation, visited-set cycle handling. Tested with synthetic graphs of varying size and structure, no external dependencies.
- **`topictrend_taxonomy`**: Semantic search integration — embedding service client, zvec queries. Tested against the live embedding service and zvec.
- **`topictrend_web`**: HTTP routing and title-to-QID translation. Tested against the real MariaDB replica.

This separation enables rapid iteration. Algorithmic improvements to `topictrend_core` can be validated without involving external services.
