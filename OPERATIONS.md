# OPERATIONS.md: TopicTrends Deployment & Operational Procedures

This document covers deployment, configuration, data ingestion, and operational procedures for TopicTrends.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Architecture Overview](#architecture-overview)
4. [Configuration](#configuration)
5. [Data Ingestion Pipeline](#data-ingestion-pipeline)
6. [Running the Web Server](#running-the-web-server)
7. [Semantic Search Setup](#semantic-search-setup)
8. [Monitoring & Health Checks](#monitoring--health-checks)
9. [Troubleshooting](#troubleshooting)

## Prerequisites

- **Rust toolchain** (1.70+): Install from https://rustup.rs/
- **MariaDB client tools**: For the ETL pipeline's replica queries (not needed to run the web server)
- **Access to Wikimedia infrastructure**: Required for data ingestion from SQL replicas and pageview dumps
- **Network connectivity**: To https://dumps.wikimedia.org for pageview data

## Quick Start

### Building the Project

```bash
cd /path/to/topictrend
cargo build --release
```

This produces binaries in `target/release/`:
- `wikigraph`: CLI for graph analysis
- `topictrend_web`: Web server (Axum)
- `get-pageviews`: Pageview data processor
- `get-day-pageedits`: Pageedits ETL — one day's edit counts from the replica
- `get-gsc-qid-date`: Google Search Console data processor (maps pages to QIDs per wiki per date)
- `get-articles`, `get-categories`, `get-categorygraph`, `get-article_category`, `get-per_day_wiki_stats`: Data extraction utilities

### Initial Setup

```bash
# Initialize data directories and fetch Wikipedia list
make init

# This creates:
# - data/wikipedia.list (all 345 Wikipedia editions)
# - data/{wiki}/articles.parquet (for each wiki)
# - data/{wiki}/categories.parquet (for each wiki)
# - data/{wiki}/article_category.parquet (for each wiki)
# - data/{wiki}/category_graph.parquet (for each wiki)
# - data/{wiki}/pageviews/{YEAR}/{MONTH}/{DAY}.parquet (daily pageviews, per wiki)
```

### Running the Web Server

```bash
# Start the web server (semantic search runs in-process)
make web

# Server listens on http://localhost:8765
```

## Architecture Overview

### System Components

The system consists of several distinct components that operate independently:

#### 1. ETL Pipeline (Batch Processing)
Runs via Makefile targets and system cron jobs. Fetches topology from Wikimedia SQL replicas and pageview dumps from public archives.

#### 2. Core Engine (In-Memory)
Loads topology at startup into memory (CSR graphs); loads time series (pageviews, pageedits, GSC) on demand per date into a bounded in-memory cache. Performs pure numeric operations on these structures.

#### 3. Web Server (Axum)
Thin translation layer. Handles HTTP requests, translates titles to QIDs from the topology parquets, invokes core engine, translates results back to titles.

#### 4. Semantic Search (In-Process)
- **Embedding inference**: `fastembed` runs the ONNX `all-MiniLM-L12-v2` model in-process
- **Vector Database (zvec)**: In-process storage for 384-dimensional embeddings with HNSW indexing, via the `zvec` Rust SDK

### Data Flow

```
Wikipedia SQL Replicas
        ↓
    [Extract]
        ↓
    Parquet Files (topology)
        ↓
    [Load at startup]
        ↓
    In-Memory CSR Graphs
        ↓
    [Query via Web API]
        ↓
    Results
```

## Configuration

### Environment Variables

Create a `.env` file in the project root or set these environment variables before running:

```bash
# zvec vector store directory (in-process, no separate service)
ZVEC_DIR=data/embedding_store/zvec

# Web server port (optional, defaults to 8765)
PORT=8765

# Data directory path (optional, defaults to "data")
DATA_DIR=data

# Topology source for graph building (optional). The default builds each
# wiki's graph from the cross-wiki canonical projection
# (article_category_canonical.parquet + categories_canonical.parquet,
# produced by `make canonical`); article->category edges carry cross-wiki
# agreement weights and the category hierarchy stays local. "local" selects
# the wiki's own relation instead (rollback path).
TOPICTREND_TOPOLOGY=local

# Maximum number of distinct dates the pageview engine keeps in its
# in-memory cache, per wiki (optional, defaults to 120). Each cached
# day for enwiki costs ~6-12 MB with the sparse representation
# (~720 MB - 1.4 GB worst case per wiki at the default).
# Lower this on memory-constrained hosts; raise it (or set 0 = unlimited)
# only if you know your workload fits in RAM.
TOPICTREND_PAGEVIEW_CACHE_DAYS=120

# Maximum number of wikis whose title maps (articles + categories, both
# directions) stay resident for title<->QID resolution (default 4; enwiki
# costs ~1GB). FIFO eviction, reloaded from parquet on demand.
TOPICTREND_TITLE_WIKIS=4
```

**Optional Variables:**
- `ZVEC_DIR` overrides the zvec vector store directory (semantic search)
- `PORT` overrides default if needed
- `TOPICTREND_PAGEVIEW_CACHE_DAYS` bounds the pageview engine's per-date cache to control RSS. The cap is per wiki; the cache evicts in FIFO insertion order when full. Setting it below the largest expected single-query range is safe — concurrent requests get an `Arc`-snapshot of their range, so mid-query eviction does not corrupt results, but recent dates may need to be re-loaded from disk more often.

### Database Replica Access (ETL only)

The ETL pipeline queries Wikimedia's public SQL replicas for:
- Article metadata and QID mappings
- Category metadata
- Category graph structure
- Article-category relationships

Queries are defined in `queries/` directory:
- `articles.sql`: Fetch articles with QID mappings
- `categories.sql`: Fetch category metadata
- `category-graph.sql`: Fetch category parent relationships
- `article-category.sql`: Fetch article-to-category assignments (hidden
  maintenance/tracking categories excluded)

The web server itself never touches the database: title↔QID resolution reads
the topology parquets (per-wiki bounded title stores) with the canonical
`category_labels.parquet` as fallback for categories that have no local page.

### Makefile Configuration

Key variables in `Makefile`:

```makefile
# Data directory (must contain wikipedia.list)
DATA_DIR ?= data

# Process date (defaults to yesterday)
DATE ?= $(shell date -d "yesterday" +%Y-%m-%d)

# Release binary directory
CARGO_RELEASE := target/release
```

Override at runtime:
```bash
make run DATE=2025-01-15
make monthly END_DATE=2025-01-31
```

## Data Ingestion Pipeline

### Overview

The ingestion process is separated into topology (structural data) and pageviews (time series).

### Topology Refresh (Monthly)

**Frequency**: Monthly (or on-demand)
**Runtime**: ~1 hour
**Operation**: Fetches complete Wikipedia topology for all 345 languages

```bash
make init
```

This target:
1. Fetches the list of active Wikipedia editions from Wikimedia
2. For each wiki, runs SQL queries against the Wikimedia replica
3. Pipes results through Rust processors to extract QID, titles, and graph structure
4. Writes compressed Parquet files to `data/{wiki}/`

**Output files per wiki:**
- `articles.parquet`: 7M rows (English), columns: page_id, qid, page_title
- `categories.parquet`: 2.5M rows (English), columns: page_id, qid, page_title
- `article_category.parquet`: 196M rows (English), article-category relationships
- `category_graph.parquet`: Parent-child category relationships

Parquet format is chosen for:
- Columnar compression (QIDs compress extremely well)
- Lazy loading via Polars
- Language-agnostic encoding (UTF-8)
- Archival and reproducibility

### Daily Pageview Ingestion

**Frequency**: Daily at 10:00 UTC
**Runtime**: ~10 minutes
**Operation**: Processes yesterday's pageview data for all 345 wikis

```bash
# Process a single date
make run DATE=2025-01-14

# Process entire month
make monthly END_DATE=2025-01-31
```

**Pipeline:**
1. Fetch compressed pageview dump from Wikimedia (`pageviews-YYYYMMDD-user.bz2`)
2. Stream decompress with bzip2
3. Parse TSV format: `domain_code page_title count_views bytes_sent`
4. Map titles to QIDs using articles.parquet
5. Aggregate views by QID
6. Write per-day Parquet: `data/{wiki}/pageviews/{YEAR}/{MONTH}/{DAY}.parquet`

**File format:** Per-day Parquet with schema `(qid: u32, views: u32)`, sorted by `qid`, sparse (only articles with non-zero views appear). On load the engine translates each QID to the current dense article ID via `articles.parquet` and produces an in-memory `Vec<u32>` indexed by dense ID for SIMD-friendly aggregation. The QID-keyed on-disk format is refresh-stable: a topology refresh (`articles.parquet` rebuild) does not invalidate historical pageview files; deleted articles' QIDs simply drop out of analytics, and added articles default to zero in pre-existing files.

### Page Edit Ingestion (Daily replica fill)

Page edits are written as **per-day** Parquet files at
`data/{wiki}/pageedits/{Y}/{M}/{D}.parquet` (schema `(qid: u32, edit_count: u32)`),
the same layout as pageviews. They are filled from the replica **idempotently
(write-if-missing)** as part of `make run` (alongside pageviews). For each wiki
and the run's date, `queries/day-pageedits.sql` aggregates one complete day of
`revision` rows (namespace-0, non-redirect) on the analytics replica;
`get-day-pageedits` maps `page_id → qid` via `articles.parquet` and writes the
day file. Only complete (past) days are recorded — today's partial count is
skipped. Cheap: one indexed `rev_timestamp` range scan per wiki per day.

```bash
make data/mlwiki/pageedits/2026/05/26.parquet DATE=2026-05-26    # one wiki, one day
```

To backfill historical dates, loop `make run` (or `make monthly`) over the
missing dates — one cheap single-day replica query per wiki per day.

### Google Search Console (GSC) Ingestion

**Frequency**: Daily (or whenever new GSC data is deposited)
**Runtime**: ~1 minute per wiki
**Operation**: Maps GSC page URLs to QIDs and writes per-wiki, per-date parquets

**Prerequisites:**
GSC source data is not fetched by the pipeline — it must be deposited externally into:
```
data/gsc_page_date/date=YYYY-MM-DD/data.parquet
```
Schema of source files: `date`, `page` (URL), `clicks`, `impressions`, `ctr`, `position`.

**Run for a specific date (all wikis):**
```bash
make gsc DATE=2026-03-03
```

**Run for a single wiki/date manually:**
```bash
./target/release/get-gsc-qid-date \
    --wiki enwiki \
    --date 2026-03-03 \
    --gsc-dir data/gsc_page_date \
    --output data/enwiki/gsc/2026/03/03.parquet
```

**Pipeline:**
1. Read `data/gsc_page_date/date=<DATE>/data.parquet`
2. Parse wiki language and article title from page URL (`https://<lang>.wikipedia.org/wiki/<title>`)
3. Filter rows to the target wiki; drop non-article URLs (portal, mobile subdomains, variant paths)
4. URL-decode titles; join against `data/<wiki>/articles.parquet` to resolve QID
5. Aggregate `(qid, clicks, impressions)` per date; recompute `ctr` and weighted-average `position`
6. Write to `data/<wiki>/gsc/<YEAR>/<MONTH>/<DAY>.parquet`

**Output schema per file:**

| column      | type  | notes                             |
|-------------|-------|-----------------------------------|
| qid         | u32   | Wikidata QID                      |
| clicks      | i64   | total clicks from Google Search   |
| impressions | i64   | total search impressions          |
| ctr         | f64   | clicks / impressions              |
| position    | f64   | impression-weighted avg position  |

Date is encoded in the path; no `date` column in the file.

**Stats output:** The binary prints per-run counters: input rows, URL parse failures, wrong-wiki rows, unmapped titles, and output rows. Use these to monitor mapping coverage.

**Coverage note:** GSC data covers only Wikipedia languages that appear in search results. Expect ~98% URL parse rate; QID mapping coverage depends on wiki size (large wikis >90%, small wikis lower).

**Makefile variable:**
```bash
GSC_DIR ?= data/gsc_page_date   # override if source is elsewhere
```

### Coverage Matrix (cross-wiki content gap)

**Frequency**: Monthly (chained after `make canonical` in the canonical systemd unit)
**Runtime**: minutes (two group-bys per wiki)
**Operation**: Materializes a dated coverage snapshot per wiki — the precomputed,
scannable form of the content-gap query. Consumes the canonical projection, so
`make canonical` must have run against the same topology state first.

```bash
make canonical coverage DATE=2026-06-09   # as the systemd unit runs it
make coverage DATE=2026-06-09             # alone, if projections are current
```

**Two depth-0 measures** (recursive/subtree coverage is deliberately omitted — the
correct depth is per-category and can't be precomputed):

- **`direct_coverage(C, W)`** — distinct articles filed *directly* under category C in
  wiki W. Divergence across wikis = a structure/categorization gap.
- **`qid_overlap_coverage(C, W)`** — `|{ a ∈ ∪_wikis directmembers(C) : a exists as an
  article in W }|`. The pure content gap: how many of a category's globally-known
  articles W has, independent of how W categorizes them.

**Pipeline:** one binary (`coverage-matrix`), one pass per wiki, two group-bys:

- `direct_coverage` ← `article_category.parquet` (the local relation, already
  node-filtered).
- `qid_overlap_coverage` ← `article_category_canonical.parquet` — the canonical
  projection already materializes `canonical_set(C) ∩ articles(W)` (built by
  `canonical-projection` from the gated canonical snapshot), so the overlap
  measure is a plain per-category count of it.

Every local direct edge is also a canonical edge over the wiki's own articles,
so `direct ≤ overlap` per category and the merge is a left join onto the
overlap key set, zero-filling `direct_coverage` (a category can have
`qid_overlap > 0` with `direct_coverage = 0` when it is absent in W but some of
its canonical articles exist there). Written to `data/<wiki>/coverage/<DATE>.parquet`.

**Output schema per file** `data/<wiki>/coverage/<DATE>.parquet`, sorted by `category_qid`:

| column                | type | notes                                            |
|-----------------------|------|--------------------------------------------------|
| category_qid          | u32  | Wikidata QID of the category                     |
| direct_coverage       | u32  | distinct direct member articles in this wiki     |
| qid_overlap_coverage  | u32  | canonical articles present as articles in this wiki |

Wiki and snapshot date are encoded in the path; no `wiki`/`date` columns in the file.
Dated snapshots are retained to support coverage-delta over time.

**Scheduling (systemd):** coverage has no unit of its own — it runs chained
after the canonical snapshot in `topictrend-canonical.service` (see the next
section), so it always sees freshly-projected inputs and never runs when the
canonical manifest gate aborts.

### Canonical Article→Category Relation (cross-wiki union)

**Frequency**: Monthly (systemd timer; the coverage snapshot runs chained after it)
**Runtime**: ~15 seconds for the union; minutes for the per-wiki projection
**Operation**: Unions every wiki's `article_category.parquet` into one global
relation, then projects it back onto each wiki. Articles and categories share
Wikidata QIDs across editions, so the union carries a per-edge count of how
many wikis assert each assignment — the categorization-consensus signal.

```bash
make canonical DATE=2026-06-11

# Or stage by stage:
target/release/canonical-membership --date 2026-06-11 [--force] [wiki ...]
target/release/canonical-projection --date 2026-06-11 [wiki ...]
target/release/canonical-labels --date 2026-06-11 [wiki ...]
```

**Stage 1 output** `data/canonical/<DATE>/article_category.parquet`, sorted by
`(article_qid, category_qid)`:

| column       | type | notes                                          |
|--------------|------|------------------------------------------------|
| article_qid  | u32  | Wikidata QID of the article                    |
| category_qid | u32  | Wikidata QID of the category                   |
| wiki_count   | u32  | number of wikis asserting this assignment      |

(`wiki_count` is u32, not u16: Polars cannot scan UInt16 Parquet columns, and
all downstream consumers read via Polars.)

**Stage 2 outputs**, per wiki (not dated — refreshed in place like the rest of
the per-wiki topology):

- `data/<wiki>/article_category_canonical.parquet` — the canonical edges whose
  article exists in this wiki; same columns as stage 1. The drop-in
  alternative to `article_category.parquet` for graph building.
- `data/<wiki>/categories_canonical.parquet` — `(qid: u32)`, sorted: the
  wiki's category node universe under the canonical relation (local category
  QIDs ∪ categories appearing in the projection). QIDs only — titles for
  non-local categories resolve at the web edge.

Expect the projection to be much wider than the local relation: a small wiki
inherits every category any edition assigns to articles it has (mlwiki:
206K local edges → 3.0M projected; 23K local categories → 522K).

**Stage 3 output** `data/canonical/<DATE>/category_labels.parquet`
`(qid: u32, label: str)`, sorted by qid — one label per category QID across
all editions, enwiki-first then `wikipedia.list` order (~4.9M rows). The web
server loads the latest snapshot's table lazily as the title fallback for
categories that have no page in the requested wiki.

**Manifest & sanity gate:** each snapshot also writes `manifest.tsv` (per-wiki
input row counts). The next run compares its inputs against the most recent
previous manifest and **aborts (exit 2) if any wiki's `article_category` row
count dropped below 50%**. This guards the known failure mode where a failed
replica fetch silently truncates one wiki's parquet — which would otherwise
shrink *every* wiki's canonical sets in the snapshot. Re-run with `--force`
after confirming the drop is legitimate. The manifest is written only after a
successful build, so a crashed run never becomes a gate baseline.

**Expected one-time gate trip:** `queries/article-category.sql` excludes
hidden (maintenance/tracking) categories. The first topology refresh after
that change legitimately shrinks `article_category` row counts (enwiki loses
"Living people", stub and tracking categories), so the first `make canonical`
on the refreshed topology will trip the gate and needs `--force` once.

**Scheduling (systemd):** the timer fires monthly at 02:00 on the 1st and the
service runs `make canonical coverage` — the coverage matrix consumes the
canonical projections, so the two run chained in dependency order (edit `User`
and `WorkingDirectory` to match the checkout):

```bash
sudo cp deploy/systemd/topictrend-canonical.{service,timer} /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now topictrend-canonical.timer
systemctl list-timers topictrend-canonical.timer   # verify next run
```

### Incremental Updates

The system does not support incremental topology updates. Complete monthly refreshes are required because:
- Categories may be deleted, merged, or recategorized
- Recomputing CSR structure requires all data
- Monthly refresh cycle aligns with Wikipedia's relatively stable structure

For critical fixes, topology can be manually refreshed:

```bash
# Force refresh for single wiki
make data/enwiki/articles.parquet --always-make
make data/enwiki/categories.parquet --always-make
make data/enwiki/article_category.parquet --always-make
make data/enwiki/category_graph.parquet --always-make
```

## Running the Web Server

### Startup

```bash
# Build and run
make web

# Or manually:
cargo build --release
./target/release/topictrend_web

# Run on a custom port
PORT=8000 ./target/release/topictrend_web
```

The server:
1. Loads topology from Parquet files into memory (CSR structure)
2. Starts HTTP server on `0.0.0.0:8765` (or custom `PORT`)
3. Loads each wiki's title maps from parquet on first use (bounded cache)

Daily pageview, pageedit, and GSC Parquet files are not loaded at startup; each date is read on first request into a bounded FIFO cache.

### Dependencies

The web server requires:
- **The `data/` tree** (topology parquets; canonical artifacts for v2 topology and label fallback)
- **The zvec native library** on the loader path and **the zvec category collections** under `ZVEC_DIR` — both only for semantic search (see [Semantic Search Setup](#semantic-search-setup))

If the zvec collections are absent, semantic search endpoints return errors, but other APIs function normally. No database is needed.

### Health Checks

```bash
# Test a working endpoint to verify server is running
curl http://localhost:8765/api/pageviews/category?wiki=enwiki&title=Science

# Expected response: Category data with views

# If using a custom port:
curl http://localhost:8765/api/pageviews/category?wiki=enwiki&title=Science
```

### Shutdown

```bash
# Via signal (graceful)
pkill -TERM topictrend_web

# Via keyboard interrupt
# Press Ctrl+C in the terminal where the server is running
```

The server closes database connections cleanly on shutdown.

## Semantic Search Setup

Semantic search runs in-process: query/title encoding via `fastembed` (ONNX `all-MiniLM-L12-v2`) and vector search via the `zvec` Rust SDK. There is no separate service to run.

### Prerequisites

- **The zvec native library.** `zvec-sys`'s `build.rs` fetches a prebuilt `libzvec_c_api.so` during `cargo build`, into `target/release/build/zvec-sys-*/out/zvec-prebuilt/`. It has no embedded rpath, so that directory must be on `LD_LIBRARY_PATH` at runtime — `make web` and `make index-wiki` set this for you.
- **The fastembed model.** `all-MiniLM-L12-v2` (384-dimensional) is downloaded from Hugging Face on first use, cached locally under `.fastembed_cache/`, and loaded once per process.

### Index English Wikipedia Categories

A one-time operation that builds the zvec collection:

```bash
# Ensure the categories parquet exists
make data/enwiki/categories.parquet

# Index into zvec (in-process)
make index-wiki WIKI=enwiki
```

**Process:**
1. Loads English Wikipedia categories from `data/enwiki/categories.parquet`
2. Batches categories in groups of 100
3. Encodes each batch in-process with fastembed
4. Inserts vectors into zvec collection `enwiki-categories`
5. Creates the HNSW index on zvec

**Runtime**: ~30 minutes.

**Output:**
- zvec collection: `data/embedding_store/zvec/enwiki-categories/` with ~2.5M points
- Each point: {id: QID, vector: 384-dim, fields: {qid, page_title}}

### Step 4: Verify Semantic Search

```bash
# Test semantic search
curl "http://localhost:8765/api/search/categories?wiki=enwiki&query=artificial+intelligence&limit=5"

# Expected response:
# {
#   "categories": [
#     {"category_qid": 11019, "category_title": "Artificial intelligence", "match_score": 0.951},
#     {"category_qid": 5952, "category_title": "Machine learning", "match_score": 0.887},
#     ...
#   ]
# }
```

## Monitoring & Health Checks

### Startup Verification

```bash
# Check server is running
curl http://localhost:8765/health

# Check topology loaded
curl http://localhost:8765/api/stats
```

### Performance Baselines

Expected latencies (in milliseconds):

- **Category pageview aggregation**: 15-25ms
- **Top categories trending**: 10-20ms
- **Semantic search (encoding + search)**: 50-150ms
- **Title translation (batch)**: 5-10ms

If latencies exceed these significantly, check:
- Memory pressure (cache size — tune `TOPICTREND_PAGEVIEW_CACHE_DAYS` / `TOPICTREND_PAGEEDIT_CACHE_DAYS`)
- CPU utilization (saturated?)
- Network latency (database replica slow?)
- Disk I/O (per-day Parquet files on slow storage?)

### Logging

By default, Axum logs to stderr. Set environment variable to control verbosity:

```bash
RUST_LOG=debug ./target/release/topictrend_web
RUST_LOG=info ./target/release/topictrend_web
RUST_LOG=warn ./target/release/topictrend_web
```

### Title Store Memory

Each wiki's title maps (articles + categories, both directions) load on first
use and stay resident, FIFO-bounded by `TOPICTREND_TITLE_WIKIS` (default 4;
enwiki ≈ 1GB). Lower the cap on memory-constrained hosts; titles for evicted
wikis reload from parquet on the next request.

## Troubleshooting

### Issue: Web Server Won't Start

**Symptom**: panic or missing-file errors at startup

**Diagnosis**:
```bash
# Topology parquets present for the wikis you serve?
ls data/enwiki/{articles,categories,article_category,category_graph}.parquet

# Canonical artifacts present? (the default topology needs them; run `make canonical`)
ls data/enwiki/article_category_canonical.parquet data/canonical/
```

**Solution**:
- Re-run topology ETL for missing wikis (`make topology-refresh WIKIS=<wiki>` on the VPS)
- Run `make canonical` before serving canonical topology

### Issue: Semantic Search Returns Errors

**Symptom**: `{"error": "Vector database error"}`, or a startup failure with `libzvec_c_api.so: cannot open shared object file`

**Diagnosis**:
```bash
# Is the zvec native library on the loader path? (make web / make index-wiki set this)
find target/release/build -name libzvec_c_api.so

# Verify the zvec collection exists
ls -la data/embedding_store/zvec/enwiki-categories/
```

**Solution**:
- Ensure `LD_LIBRARY_PATH` includes the directory holding `libzvec_c_api.so` (use `make web`, which sets it)
- Re-index if the collection is missing: `make index-wiki WIKI=enwiki`

### Issue: High Latency on Pageview Queries

**Symptom**: Category pageview queries take >100ms

**Diagnosis**:
```bash
# Monitor memory usage during queries
watch -n 0.1 'ps aux | grep topictrend_web'

# Check if system is swapping
watch -n 0.1 'free -h'

# Monitor CPU cache misses
perf stat -e cache-misses,cache-references ./target/release/topictrend_web
```

**Solution**:
- Ensure sufficient RAM is available (at least 4GB for topology)
- Move `data/` directory to faster storage if on HDD

### Issue: Pageview Data Won't Ingest

**Symptom**: `make run` fails with "Failed to download pageviews" or "Parse error"

**Diagnosis**:
```bash
# Check if URL is reachable
curl -I https://dumps.wikimedia.org/other/pageview_complete/2025/2025-01/pageviews-20250114-user.bz2

# Check wiki list
cat data/wikipedia.list

# Manual test with single wiki
make data/enwiki/pageviews/2025/01/14.parquet DATE=2025-01-14
```

**Solution**:
- Verify internet connectivity
- Check if date is valid (dumps are published 1 day behind)
- Check system disk space
- Verify bzip2 is installed: `which bzip2`

### Issue: Out of Memory During Initialization

**Symptom**: `make init` crashes during `get-categorygraph`

**Diagnosis**:
```bash
# Check available memory
free -h

# Check file sizes
du -h data/{wiki}/

# Monitor memory during run
watch -n 0.1 'ps aux | grep get-categorygraph'
```

**Solution**:
- Increase available RAM
- Process one wiki at a time: `make data/enwiki/category_graph.parquet`
- Reduce batch size in `topictrend_core/src/graph.rs` if applicable

---

## Operational Checklist

### Monthly Topology Refresh

```bash
# 1. Schedule downtime window (optional, but recommended)
# 2. Run initialization
make init

# 3. Verify data quality
curl http://localhost:8765/api/stats

# 4. Check for errors
tail -f /var/log/topictrend_web.log

# 5. If semantic search is enabled, re-index
make index-wiki

# 6. Test endpoints
curl http://localhost:8765/api/pageviews/category?qid=42&wiki=enwiki
```

### Daily Monitoring

```bash
# Check server is healthy
curl http://localhost:8765/health

# Check disk usage
du -h data/

# Review logs for errors
grep -i error /var/log/topictrend_web.log
```

### Scaling Considerations

The architecture scales to:
- **345 Wikipedia editions**: Current production deployment
- **10+ million categories**: Tested and working
- **500+ million article-category edges**: Architecture supports this

Bottlenecks emerge at:
- **Available RAM**: For large wikis, CSR topology can exceed available memory
- **Network latency**: Title translation is database-bound
- **zvec storage**: 2.5M embeddings at 384-dim uses ~10GB disk

To scale further, consider:
- Sharding by language or category prefix
- Caching frequently accessed translations
- Adding read replicas for database queries

---

## Support & Further Reading

For architectural context and design decisions, see [README.md](README.md).

For REST API endpoint documentation, see [API.md](API.md).
