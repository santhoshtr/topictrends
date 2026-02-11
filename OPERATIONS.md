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
- **MariaDB client tools**: For database access to Wikimedia SQL replicas
- **Docker** (optional): For running Qdrant vector database
- **Access to Wikimedia infrastructure**: Required for data ingestion from SQL replicas and pageview dumps
- **Network connectivity**: To https://dumps.wikimedia.org for pageview data

## Quick Start

### Building the Project

```bash
cd /path/to/topictrend
cargo build --release
```

This produces four binaries in `target/release/`:
- `wikigraph`: CLI for graph analysis
- `topictrend_web`: Web server (Axum)
- `get-pageviews`: Pageview data processor
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
# - data/pageviews/{YEAR}/{MONTH}/{DAY}.bin (daily pageviews)
```

### Running the Web Server

```bash
# Ensure Qdrant is running (for semantic search)
make qdrant

# Start the web server
make web

# Server listens on http://localhost:8765
```

## Architecture Overview

### System Components

The system consists of several distinct components that operate independently:

#### 1. ETL Pipeline (Batch Processing)
Runs via Makefile targets and system cron jobs. Fetches topology from Wikimedia SQL replicas and pageview dumps from public archives.

#### 2. Core Engine (In-Memory)
Loads processed data at startup into memory. Performs pure numeric operations on CSR graphs and mmap'd time series.

#### 3. Web Server (Axum)
Thin translation layer. Handles HTTP requests, translates titles to QIDs via MariaDB, invokes core engine, translates results back to titles.

#### 4. Semantic Search (Microservices)
Optional component for semantic search:
- **Embedding Service**: Python gRPC server running a sentence transformer model
- **Vector Database (Qdrant)**: Persistent storage for 384-dimensional embeddings with HNSW indexing

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
# Database credentials (REQUIRED)
DB_USERNAME=<wikimedia replica user>
DB_PASSWORD=<wikimedia replica password>

# Embedding service endpoint (optional, only for semantic search)
EMBEDDING_SERVER=http://localhost:50051

# Qdrant vector database endpoint (optional, only for semantic search)
QDRANT_SERVER=http://localhost:6333

# Web server port (optional, defaults to 8765)
PORT=8765

# gRPC server port (optional, defaults to 50051)
GRPC_PORT=50051

# Data directory path (optional, defaults to "data")
DATA_DIR=data
```

**Required Variables:**
- `DB_USERNAME` and `DB_PASSWORD` are required for database access to Wikimedia SQL replicas
- Server will fail to start without these credentials

**Optional Variables:**
- `EMBEDDING_SERVER` and `QDRANT_SERVER` are only needed if using semantic search endpoints
- `PORT` and `GRPC_PORT` override defaults if needed

### Database Replica Access

TopicTrends assumes access to Wikimedia's public SQL replicas. The system queries these replicas for:
- Article metadata and QID mappings
- Category metadata
- Category graph structure
- Article-category relationships

Queries are defined in `queries/` directory:
- `articles.sql`: Fetch articles with QID mappings
- `categories.sql`: Fetch category metadata
- `category-graph.sql`: Fetch category parent relationships
- `article-category.sql`: Fetch article-to-category assignments
- `get_qid_by_title.sql`: Translate title to QID
- `get_titles_by_qids.sql`: Batch translate QIDs to titles

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
6. Write binary vectors: `data/{wiki}/pageviews/{YEAR}/{MONTH}/{DAY}.bin`

**Binary format:** Per-day files containing a vector where index is QID, value is pageview count. This enables O(1) lookup and mmap access.

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
# Ensure environment variables are set
export DB_USERNAME=<user>
export DB_PASSWORD=<password>

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
2. Mmaps daily pageview binaries
3. Starts HTTP server on `0.0.0.0:8765` (or custom `PORT`)
4. Starts gRPC server on `0.0.0.0:50051` (or custom `GRPC_PORT`)
5. Establishes connection pool to MariaDB replica for title translation

### Dependencies

The web server requires:
- **MariaDB replica access** (hard requirement): Used for all title↔QID translation
- **Qdrant service** (optional): Only if using semantic search endpoints

If MariaDB is unavailable, the server will fail to start. If Qdrant is unavailable, semantic search endpoints will return errors, but other APIs function normally.

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

### Prerequisites

- **Docker** (to run Qdrant)
- **Python 3.9+** (to run embedding service)
- **Embedding service**: Included in `services/embedding/`

### Step 1: Start Qdrant Vector Database

```bash
make qdrant

# Or manually:
docker run -d --rm \
  -p 6333:6333 \
  --name qdrant \
  qdrant/qdrant

# Verify: http://localhost:6333/health
```

**Configuration:**
- **Port 6333**: HTTP API
- **Storage**: In-container (ephemeral, or use volume for persistence)

### Step 2: Start Embedding Service

The embedding service runs as a gRPC server on port 50051 (default).

```bash
cd services/embedding
docker-compose up -d

# Or manually (requires Python venv):
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt
python embedding_server.py
```

**Configuration:**
- **Port 50051**: gRPC endpoint
- **Model**: `sentence-transformers/all-MiniLM-L12-v2` (384-dimensional)
- **First run**: Downloads model from Hugging Face (~100MB)

**Set environment variable for web server:**
```bash
export EMBEDDING_SERVER=http://localhost:50051
```

### Step 3: Index English Wikipedia Categories

This is a one-time operation that builds the Qdrant collection. Use the topictrend_taxonomy binary:

```bash
cargo build --release
./target/release/topictrend_taxonomy

# Or check the Makefile for available targets
make help | grep -i embedding
```

**Process:**
1. Loads English Wikipedia categories from `data/enwiki/categories.parquet`
2. Batches categories in groups of 100
3. Encodes each batch via gRPC to embedding service
4. Inserts vectors into Qdrant collection `enwiki-categories`
5. Creates HNSW index on Qdrant

**Runtime**: ~30 minutes (depends on network latency to embedding service)

**Output:**
- Qdrant collection: `enwiki-categories` with 2.5M points
- Each point: {id: QID, vector: 384-dim, payload: {qid, page_title}}

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

# Check MariaDB connectivity
curl http://localhost:8765/api/db-status
```

### Performance Baselines

Expected latencies (in milliseconds):

- **Category pageview aggregation**: 15-25ms
- **Top categories trending**: 10-20ms
- **Semantic search (encoding + search)**: 50-150ms
- **Title translation (batch)**: 5-10ms

If latencies exceed these significantly, check:
- Memory pressure (is mmap causing page swaps?)
- CPU utilization (saturated?)
- Network latency (database replica slow?)
- Disk I/O (pageview binaries on slow storage?)

### Logging

By default, Axum logs to stderr. Set environment variable to control verbosity:

```bash
RUST_LOG=debug ./target/release/topictrend_web
RUST_LOG=info ./target/release/topictrend_web
RUST_LOG=warn ./target/release/topictrend_web
```

### Database Connection Pool

The web server maintains a connection pool to MariaDB replica (default: 5-10 connections). Monitor these:

```bash
# From MariaDB:
SHOW PROCESSLIST;

# Look for connections from topictrend_web host
```

If pool is exhausted, increase pool size in configuration.

## Troubleshooting

### Issue: Web Server Won't Start

**Symptom**: `Error connecting to database` or `Connection refused`

**Diagnosis**:
```bash
# Test MariaDB connectivity
mariadb --host enwiki.analytics.db.svc.wikimedia.cloud --user ... -e "SELECT 1"

# Check if Qdrant is required
grep -r "QUADRANT_SERVER" src/
```

**Solution**:
- Verify MariaDB replica is accessible from your network
- Check `.env` has correct database credentials
- If Qdrant is optional, ensure semantic search endpoints aren't required

### Issue: Semantic Search Returns Errors

**Symptom**: `{"error": "Embedding service unavailable"}` or `{"error": "Vector database error"}`

**Diagnosis**:
```bash
# Check embedding service
curl http://localhost:50051/health  # (gRPC, may not respond to HTTP)

# Check Qdrant
curl http://localhost:6333/health

# Verify collection exists
curl http://localhost:6333/collections/enwiki-categories
```

**Solution**:
- Restart embedding service: `docker-compose restart` in `services/embedding/`
- Restart Qdrant: `docker restart qdrant`
- Re-index: `./target/release/topictrend_web --init-embeddings`

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
- Increase shared memory if using container: `docker run --shm-size=2g qdrant/qdrant`

### Issue: Pageview Data Won't Ingest

**Symptom**: `make run` fails with "Failed to download pageviews" or "Parse error"

**Diagnosis**:
```bash
# Check if URL is reachable
curl -I https://dumps.wikimedia.org/other/pageview_complete/2025/2025-01/pageviews-20250114-user.bz2

# Check wiki list
cat data/wikipedia.list

# Manual test with single wiki
make data/enwiki/pageviews/2025/01/14.bin
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
./target/release/topictrend_web --init-embeddings

# 6. Test endpoints
curl http://localhost:8765/api/pageviews/category?qid=42&wiki=enwiki
```

### Daily Monitoring

```bash
# Check server is healthy
curl http://localhost:8765/health

# Monitor database connections
mariadb -e "SHOW PROCESSLIST" | grep topictrend

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
- **Qdrant storage**: 2.5M embeddings at 384-dim uses ~10GB disk

To scale further, consider:
- Sharding by language or category prefix
- Caching frequently accessed translations
- Adding read replicas for database queries

---

## Support & Further Reading

For architectural context and design decisions, see [README.md](README.md).

For REST API endpoint documentation, see [API.md](API.md).
