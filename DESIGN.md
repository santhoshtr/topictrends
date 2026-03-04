# TopicTrends: Design Decisions

This document records the key design decisions in TopicTrends and the tradeoffs accepted. For the data structures and algorithms that result from these decisions, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Why Rust

Performance-critical systems require predictable, low-level control. Rust provides memory safety without garbage collection pauses, enabling deterministic latency. SIMD operations are accessible through libraries. The type system eliminates entire classes of concurrency bugs at compile time. For a system where 20 milliseconds is the performance target and the core engine is purely numeric, these properties are essential.

## Why CSR Over Graph Databases

Graph databases like Neo4j provide query convenience at the cost of runtime overhead — network round-trips, garbage collection, and pointer-chasing through heap allocations. Edge lists require pointer-chasing through memory. Adjacency matrices waste space for sparse graphs.

CSR is the classical approach for this exact problem: large sparse graphs with static structure and frequent traversals. The tradeoff is immutability — dynamic insertion requires full reconstruction — but this matches TopicTrends' batch-update model. Topology is refreshed monthly, not incrementally.

## Why Memory-Mapped Files

Keeping gigabytes of time series in RAM wastes memory for cold data (historical dates rarely queried). Keeping data entirely on disk incurs I/O latency on every access. Memory-mapped files let the OS manage this boundary automatically. The kernel's page cache handles hot/cold data more efficiently than any application-level caching strategy. The application gains simplicity; the OS gains full visibility into access patterns for prefetch optimization.

## Why a Single English Embedding Model

A naive approach would embed category titles in all 345 languages using a multilingual model. This would require 345 separate vector collections, each with 2.5 million embeddings, totaling approximately 3.3 terabytes of storage.

TopicTrends uses a single English embedding model for all languages. The cross-lingual capability is preserved through QID translation: when a user searches for a concept, the English query is encoded, the English category index is searched, and results are translated to the target language via Wikidata QID mapping.

This reduces storage from 3.3 TB to ~9.6 GB — a factor of ~345 — at the cost of one constraint: queries must be formulated in English. Results can be returned in any language. English is the semantic bottleneck, but its linguistic richness and the density of its Wikidata links make it a strong anchor for cross-lingual understanding.

## Why zvec for Vector Search

Vector search requires approximate nearest-neighbor indexing. The main alternatives considered:

- **FAISS**: Requires lower-level C/Python integration and does not provide persistent storage out of the box.
- **Qdrant**: Requires running a separate service, adding operational complexity.
- **Cloud solutions** (Pinecone, etc.): Introduce vendor lock-in and network latency.

zvec provides an in-process solution with a straightforward Python API, persistent on-disk storage, and HNSW indexing — without running a separate service. The embedding pipeline stays within a single operational boundary.

## What Was Sacrificed

These tradeoffs are intentional, not oversights:

- **Write flexibility**: Topology cannot be updated incrementally. Monthly refreshes require full reconstruction and a brief reload window.
- **Query language**: Semantic search queries must be in English. Other languages require reformulation.
- **Real-time topology**: New Wikipedia articles and categories added between monthly refreshes are not reflected in analytics. Wikipedia's structure changes slowly, making this acceptable for trend analysis.
- **Dynamic graph modifications**: Structural changes require taking the system offline for a reload.

The use case — analytics on a relatively stable knowledge graph with high-frequency time series queries — makes all of these acceptable.

## Known Limitations & Future Work

### Static Topology Refresh

The category graph is refreshed monthly. Between refreshes, new articles and categories are invisible to the analytics engine. For breaking-news trend analysis this is a meaningful gap; for longer-term topic research it is generally acceptable.

### English Query Requirement

Semantic search requires English queries. Supporting other query languages would require either multilingual embedding models (storage explosion) or machine translation (latency and error propagation). Neither tradeoff is currently warranted.

### Incomplete Cross-Wikipedia Aggregation

Trending analysis currently runs within each Wikipedia independently. The architecture supports global aggregation across QID boundaries — identifying topics that trend across multiple language editions simultaneously — but this is not yet implemented in the query layer. It is a natural extension requiring no structural changes.
