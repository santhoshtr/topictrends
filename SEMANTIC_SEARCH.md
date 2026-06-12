# TopicTrends: Semantic Search

This document describes how semantic category search works in TopicTrends — the motivation, architecture, indexing strategy, and cross-lingual query flow.

For the QID translation mechanism that makes cross-lingual results possible, see [ARCHITECTURE.md](ARCHITECTURE.md#cross-lingual-unification). For the decision to use English-only embeddings and zvec, see [DESIGN.md](DESIGN.md).

## Motivation: Concept Over Keywords

Traditional keyword-based search assumes exact phrase matching. Searching for "machine learning" returns only categories with those exact words, missing "neural networks," "deep learning," and "artificial intelligence" — concepts that are semantically related.

Semantic search uses neural embeddings — dense vector representations of text where similarity in vector space correlates with semantic similarity. A query and a candidate category are both converted to high-dimensional vectors, and cosine distance measures semantic relatedness. This enables finding "neural networks" when searching for "machine learning."

## Architecture

### English-Only Embeddings

Rather than maintaining separate embedding collections for all 345 languages (which would require ~3.3 TB), TopicTrends indexes a single English category collection using an English sentence transformer model. Cross-lingual coverage is achieved through QID translation at query time, not through multilingual models.

See [DESIGN.md](DESIGN.md#why-a-single-english-embedding-model) for the full rationale.

### Vector Database: zvec with HNSW

TopicTrends stores 2.3 million 384-dimensional embeddings (one per English Wikipedia category) in zvec. zvec implements Hierarchical Navigable Small World (HNSW) indexing, an approximate nearest-neighbor algorithm that achieves O(log N) query complexity while returning the true top-k results with high probability.

Key parameters:
- **Dimensionality**: 384 (model: `sentence-transformers/all-MiniLM-L12-v2`)
- **Distance metric**: Cosine similarity — invariant to vector magnitude, so verbosity differences don't affect relevance
- **Storage**: On-disk vectors with quantized data in RAM, balancing memory efficiency with search speed
- **Index size**: ~3.8 GB on disk for 2.3M English category embeddings

### Indexing Pipeline

The index is built once and refreshed after each monthly topology update:

1. Load English Wikipedia categories from `data/enwiki/categories.parquet`
2. Batch categories in groups of 100
3. Encode each batch via the gRPC embedding service
4. Insert vectors into zvec collection `enwiki-categories` (keyed by QID)
5. Build HNSW index

Runtime: ~30 minutes. See [OPERATIONS.md](OPERATIONS.md#semantic-search-setup) for the full setup procedure.

## Query Flow

When a semantic search request arrives (e.g., `query=machine learning`, `wiki=frwiki`):

1. **Encode**: The English query string is sent to the embedding service and converted to a 384-dimensional vector.
2. **Search**: The vector is searched against the `enwiki-categories` zvec collection, returning the top-k results with cosine similarity scores.
3. **Filter & translate** (if target wiki ≠ enwiki): Results are kept only if the category is populated in the target wiki's canonical graph (`qid_overlap > 0` in its coverage snapshot) — a local category page is not required, since the canonical projection populates categories the wiki never created. Titles resolve from the target wiki's parquet title store, falling back to the canonical label table (usually the English label) for categories with no local page.
4. **Return**: Results include both the English title (from embeddings) and the translated title, plus the similarity score.

**Example** — query "machine learning", target `frwiki`:

```json
{
  "query": "machine learning",
  "wiki": "frwiki",
  "categories": [
    {
      "category_qid": 11019,
      "category_title_en": "Artificial intelligence",
      "category_title": "Intelligence artificielle",
      "match_score": 0.951
    },
    {
      "category_qid": 5952,
      "category_title_en": "Machine learning",
      "category_title": "Apprentissage automatique",
      "match_score": 0.887
    }
  ]
}
```

The semantic understanding comes from the English embedding model; the cross-lingual coverage comes from QID universality.

## Constraints

- **Query language**: Queries must be in English. The embedding model understands English semantics only. Queries in other languages will return results, but semantic matching quality is degraded because the model was not trained on those languages.
- **Coverage**: Categories that exist in the target wiki but have no English equivalent (no QID link to enwiki) are not reachable through semantic search.
- **Embedding service dependency**: Semantic search endpoints require the embedding gRPC service to be running. Other API endpoints function normally without it.

## Latency Profile

| Step | Typical Time |
|---|---|
| Query encoding (embedding service) | 10–50 ms |
| HNSW nearest-neighbor search (zvec) | 5–20 ms |
| Coverage filter + QID→title translation (parquet stores, in-memory after first load) | 1–5 ms |
| **Total** | **50–150 ms** |
