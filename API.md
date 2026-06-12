# API.md: TopicTrends REST API Documentation

This document specifies the REST API endpoints provided by the TopicTrends web server.

## Base URL

```
http://localhost:8765
```

All endpoints are prefixed with `/api/`. Override the default port with the `PORT` environment variable:

```bash
PORT=8000 ./target/release/topictrend_web
```

## Authentication

No authentication is required. The system is designed for internal use within Wikimedia Cloud.

## Common Parameters

### `wiki` Parameter

Specifies the Wikipedia language edition. Examples:
- `enwiki` - English Wikipedia
- `frwiki` - French Wikipedia
- `dewiki` - German Wikipedia
- `zhwiki` - Chinese Wikipedia

See `data/wikipedia.list` for complete list of supported editions.

### Date Formats

Dates are ISO 8601 format: `YYYY-MM-DD` (e.g., `2025-01-14`)

### Title vs. QID

The API accepts titles as input (human-readable) and translates them to QIDs internally. Responses include both QIDs and titles for clarity.

**Example:** Query "Physics" (title) → internal translation to QID `42` → processing → response with both `category_qid: 42` and `category_title: "Physics"`

## Endpoints

### 1. GET /api/pageviews/category

Returns pageview statistics for a category and all its subcategories.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `title` | String | Yes | Category name (exact match) |
| `wiki` | String | Yes | Wikipedia edition (e.g., `enwiki`) |
| `start_date` | String | No | Start date (ISO 8601), default: 30 days ago |
| `end_date` | String | No | End date (ISO 8601), default: yesterday |

**Example Request:**

```bash
curl "http://localhost:8765/api/pageviews/category?wiki=enwiki&title=Physics"

curl "http://localhost:8765/api/pageviews/category?wiki=frwiki&title=Physique&start_date=2025-01-01&end_date=2025-01-14"
```

**Response:**

```json
{
  "category": {
    "title": "Physics",
    "qid": 42,
    "total_views": 15234567,
    "daily_breakdown": [
      {
        "date": "2025-01-14",
        "views": 524381
      },
      {
        "date": "2025-01-13",
        "views": 512643
      }
    ]
  }
}
```

**Complexity:** $O(E)$ where $E$ is the number of edges in the category subgraph. Typical execution time: 15-25 milliseconds.

**Error Responses:**

- `404 Not Found`: Category does not exist in the specified wiki
- `400 Bad Request`: Invalid date format or wiki parameter

---

### 2. GET /api/pageviews/article

Returns daily pageview data for a specific article.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `qid` | Integer | Yes | Wikidata QID (numeric, e.g., `42`) |
| `wiki` | String | Yes | Wikipedia edition |
| `start_date` | String | No | Start date, default: 30 days ago |
| `end_date` | String | No | End date, default: yesterday |

**Example Request:**

```bash
curl "http://localhost:8765/api/pageviews/article?wiki=enwiki&qid=42&start_date=2025-01-01"
```

**Response:**

```json
{
  "article": {
    "qid": 42,
    "title": "Douglas Adams",
    "daily_views": [
      {
        "date": "2025-01-14",
        "views": 1523
      },
      {
        "date": "2025-01-13",
        "views": 1412
      }
    ],
    "total_views": 45230
  }
}
```

**Complexity:** $O(\log n)$ — binary search over the day's sparse dense-ID array. Execution time: <1 millisecond.

**Error Responses:**

- `404 Not Found`: Article does not exist
- `400 Bad Request`: Invalid QID or wiki parameter

---

### 3. GET /api/list/sub_categories

Lists immediate child categories of a given parent.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `title` | String | Yes | Parent category name |
| `wiki` | String | Yes | Wikipedia edition |
| `limit` | Integer | No | Max results, default: 100, max: 1000 |

**Example Request:**

```bash
curl "http://localhost:8765/api/list/sub_categories?wiki=enwiki&title=Science&limit=20"
```

**Response:**

```json
{
  "parent": {
    "title": "Science",
    "qid": 336
  },
  "children": [
    {
      "title": "Physics",
      "qid": 42,
      "child_count": 523
    },
    {
      "title": "Chemistry",
      "qid": 2329,
      "child_count": 412
    },
    {
      "title": "Biology",
      "qid": 5844,
      "child_count": 789
    }
  ],
  "total_count": 156
}
```

**Complexity:** $O(D)$ where $D$ is the degree (number of children). Execution time: <5 milliseconds.

**Error Responses:**

- `404 Not Found`: Category does not exist
- `400 Bad Request`: Invalid parameters

---

### 4. GET /api/list/top_categories

Discovers trending categories within a time range. Returns categories with highest pageview growth or absolute traffic.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `wiki` | String | Yes | Wikipedia edition |
| `metric` | String | No | `total_views` (default) or `trend_score` |
| `start_date` | String | No | Start date, default: 30 days ago |
| `end_date` | String | No | End date, default: yesterday |
| `limit` | Integer | No | Max results, default: 100, max: 1000 |

**Example Requests:**

```bash
# Top 50 categories by total pageviews
curl "http://localhost:8765/api/list/top_categories?wiki=enwiki&limit=50"

# Trending categories (growth metric)
curl "http://localhost:8765/api/list/top_categories?wiki=frwiki&metric=trend_score&limit=20&start_date=2025-01-01&end_date=2025-01-14"
```

**Response:**

```json
{
  "wiki": "enwiki",
  "metric": "total_views",
  "period": {
    "start_date": "2024-12-15",
    "end_date": "2025-01-14"
  },
  "categories": [
    {
      "rank": 1,
      "title": "United States",
      "qid": 30,
      "views": 12567890,
      "daily_average": 418929
    },
    {
      "rank": 2,
      "title": "Science",
      "qid": 336,
      "views": 11234567,
      "daily_average": 374485
    }
  ],
  "total_categories_analyzed": 2567123
}
```

**Complexity:** $O(N)$ where $N$ is the number of articles with pageview data. Execution time: 20-50 milliseconds.

**Error Responses:**

- `400 Bad Request`: Invalid metric, date range, or wiki parameter
- `422 Unprocessable Entity`: Date range exceeds available data

---

### 5. GET /api/search/categories

Performs semantic search across categories using neural embeddings.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | String | Yes | Search query (English only) |
| `wiki` | String | Yes | Target Wikipedia edition for result titles |
| `match_threshold` | Float | No | Min. similarity score (0.0-1.0), default: 0.6 |
| `limit` | Integer | No | Max results, default: 1000, max: 10000 |

**Important:** The `query` parameter must be in English. The embedding model understands English semantics exclusively. Results are translated to the target `wiki` language via QID mapping.

**Example Requests:**

```bash
# Search for "machine learning" concepts in English
curl "http://localhost:8765/api/search/categories?wiki=enwiki&query=machine+learning&limit=10"

# Same search, return results in French
curl "http://localhost:8765/api/search/categories?wiki=frwiki&query=machine+learning&limit=10"

# High-confidence results only
curl "http://localhost:8765/api/search/categories?wiki=dewiki&query=quantum+computing&match_threshold=0.8&limit=5"
```

**Response:**

```json
{
  "query": "machine learning",
  "wiki": "enwiki",
  "match_threshold": 0.6,
  "categories": [
    {
      "category_qid": 11019,
      "category_title_en": "Artificial intelligence",
      "category_title": "Artificial intelligence",
      "match_score": 0.951
    },
    {
      "category_qid": 5952,
      "category_title_en": "Machine learning",
      "category_title": "Machine learning",
      "match_score": 0.887
    },
    {
      "category_qid": 11300,
      "category_title_en": "Deep learning",
      "category_title": "Deep learning",
      "match_score": 0.843
    }
  ],
  "total_matched": 47,
  "execution_time_ms": 87
}
```

**Cross-Lingual Example (French):**

```json
{
  "query": "machine learning",
  "wiki": "frwiki",
  "match_threshold": 0.6,
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
  ],
  "total_matched": 42,
  "execution_time_ms": 95
}
```

**Key Observations:**
- `category_title_en` is always in English (from embeddings)
- `category_title` is translated to the target wiki language
- `match_score` represents cosine similarity (0.0 = opposite, 1.0 = identical)
- Categories without translations in target wiki are filtered out
- Execution time includes embedding generation and vector search

**Complexity:** Dominated by zvec HNSW search, $O(\log N)$ where $N$ = 2.5M categories. Execution time: 50-150 milliseconds.

**Error Responses:**

- `400 Bad Request`: Invalid parameters or empty query
- `503 Service Unavailable`: Embedding service or vector store unavailable
- `422 Unprocessable Entity`: Query too long (max ~1000 characters)

---

### 6. GET /api/pageviews/delta/categories

Compares pageview trends between two time periods to identify categories with significant changes.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `wiki` | String | Yes | Wikipedia edition (e.g., `enwiki`) |
| `baseline_start_date` | String | Yes | Baseline period start (ISO 8601) |
| `baseline_end_date` | String | Yes | Baseline period end (ISO 8601) |
| `impact_start_date` | String | Yes | Impact period start (ISO 8601) |
| `impact_end_date` | String | Yes | Impact period end (ISO 8601) |
| `limit` | Integer | No | Max results, default: 100, max: 1000 |

**Example Request:**

```bash
curl "http://localhost:8765/api/pageviews/delta/categories?wiki=enwiki&baseline_start_date=2024-12-01&baseline_end_date=2024-12-31&impact_start_date=2025-01-01&impact_end_date=2025-01-14&limit=50"
```

**Response:**

```json
{
  "categories": [
    {
      "category_qid": 42,
      "category_title": "Science",
      "baseline_total_views": 1000000,
      "impact_total_views": 1500000,
      "view_change": 500000,
      "percent_change": 50.0
    }
  ],
  "total_analyzed": 2567123
}
```

**Error Responses:**

- `400 Bad Request`: Invalid date range or wiki parameter
- `422 Unprocessable Entity`: Date range exceeds available data

---

### 7. GET /api/pageviews/delta/articles

Compares article pageview changes within a category between two time periods.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `wiki` | String | Yes | Wikipedia edition |
| `category_qid` | Integer | Yes | Parent category QID |
| `baseline_start_date` | String | Yes | Baseline period start |
| `baseline_end_date` | String | Yes | Baseline period end |
| `impact_start_date` | String | Yes | Impact period start |
| `impact_end_date` | String | Yes | Impact period end |
| `limit` | Integer | No | Max results, default: 100 |

**Example Request:**

```bash
curl "http://localhost:8765/api/pageviews/delta/articles?wiki=enwiki&category_qid=336&baseline_start_date=2024-12-01&baseline_end_date=2024-12-31&impact_start_date=2025-01-01&impact_end_date=2025-01-14&limit=20"
```

**Response:**

```json
{
  "articles": [
    {
      "article_qid": 5,
      "article_title": "Physics",
      "baseline_views": 100000,
      "impact_views": 150000,
      "view_change": 50000,
      "percent_change": 50.0
    }
  ]
}
```

**Error Responses:**

- `400 Bad Request`: Invalid parameters
- `404 Not Found`: Category QID not found

---

### 8. GET /api/pageedits/delta/categories

Compares page edit activity in categories between two time periods.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `wiki` | String | Yes | Wikipedia edition |
| `baseline_start_date` | String | Yes | Baseline period start |
| `baseline_end_date` | String | Yes | Baseline period end |
| `impact_start_date` | String | Yes | Impact period start |
| `impact_end_date` | String | Yes | Impact period end |
| `limit` | Integer | No | Max results, default: 100 |

**Example Request:**

```bash
curl "http://localhost:8765/api/pageedits/delta/categories?wiki=enwiki&baseline_start_date=2024-12-01&baseline_end_date=2024-12-31&impact_start_date=2025-01-01&impact_end_date=2025-01-14&limit=50"
```

**Response:**

```json
{
  "categories": [
    {
      "category_qid": 42,
      "category_title": "Science",
      "baseline_total_edits": 5000,
      "impact_total_edits": 8000,
      "edit_change": 3000,
      "percent_change": 60.0
    }
  ]
}
```

**Error Responses:**

- `400 Bad Request`: Invalid parameters
- `422 Unprocessable Entity`: Date range exceeds available data

---

### 9. GET /api/pageedits/delta/articles

Compares article edit activity within a category between two time periods.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `wiki` | String | Yes | Wikipedia edition |
| `category_qid` | Integer | Yes | Parent category QID |
| `baseline_start_date` | String | Yes | Baseline period start |
| `baseline_end_date` | String | Yes | Baseline period end |
| `impact_start_date` | String | Yes | Impact period start |
| `impact_end_date` | String | Yes | Impact period end |
| `limit` | Integer | No | Max results, default: 100 |

**Example Request:**

```bash
curl "http://localhost:8765/api/pageedits/delta/articles?wiki=enwiki&category_qid=336&baseline_start_date=2024-12-01&baseline_end_date=2024-12-31&impact_start_date=2025-01-01&impact_end_date=2025-01-14&limit=20"
```

**Response:**

```json
{
  "articles": [
    {
      "article_qid": 5,
      "article_title": "Physics",
      "baseline_edits": 50,
      "impact_edits": 120,
      "edit_change": 70,
      "percent_change": 140.0
    }
  ]
}
```

**Error Responses:**

- `400 Bad Request`: Invalid parameters
- `404 Not Found`: Category QID not found

---

### 10. GET /api/list/articles

Lists articles within a category.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `wiki` | String | Yes | Wikipedia edition |
| `category` | String | No | Category name (title) |
| `category_qid` | Integer | No | Category QID (numeric) |
| `min_agreement` | Integer | No | Keep only members at least this many wikis agree on (default 1; meaningful on canonical topology) |
| `limit` | Integer | No | Max results, default: 100, max: 10000 |

**Note:** Either `category` (title) or `category_qid` must be provided.

**Example Requests:**

```bash
# By category title
curl "http://localhost:8765/api/list/articles?wiki=enwiki&category=Physics&limit=50"

# By QID
curl "http://localhost:8765/api/list/articles?wiki=enwiki&category_qid=42&limit=50"
```

**Response:**

```json
{
  "category": {
    "title": "Physics",
    "qid": 42
  },
  "articles": [
    {
      "article_qid": 100,
      "article_title": "Classical mechanics",
      "latest_views": 5234
    },
    {
      "article_qid": 101,
      "article_title": "Quantum mechanics",
      "latest_views": 8923
    }
  ],
  "total_count": 5234
}
```

**Error Responses:**

- `404 Not Found`: Category does not exist
- `400 Bad Request`: Invalid parameters

---

### 11. GET /api/pageviews/categories

Performs semantic search on categories and returns aggregated pageview trends for matching categories.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `wiki` | String | Yes | Wikipedia edition |
| `category_query` | String | Yes | Semantic search query (English) |
| `start_date` | String | No | Start date, default: 30 days ago |
| `end_date` | String | No | End date, default: yesterday |
| `match_threshold` | Float | No | Min. similarity score (0.0-1.0), default: 0.6 |
| `limit` | Integer | No | Max results, default: 100 |

**Example Request:**

```bash
curl "http://localhost:8765/api/pageviews/categories?wiki=enwiki&category_query=machine+learning&limit=10&match_threshold=0.7"
```

**Response:**

```json
{
  "query": "machine learning",
  "wiki": "enwiki",
  "period": {
    "start_date": "2024-12-15",
    "end_date": "2025-01-14"
  },
  "categories": [
    {
      "category_qid": 11019,
      "category_title": "Artificial intelligence",
      "match_score": 0.95,
      "total_views": 2500000,
      "daily_average": 83333
    },
    {
      "category_qid": 5952,
      "category_title": "Machine learning",
      "match_score": 0.88,
      "total_views": 1800000,
      "daily_average": 60000
    }
  ]
}
```

**Error Responses:**

- `400 Bad Request`: Invalid parameters or empty query
- `503 Service Unavailable`: Embedding service unavailable

---

### 12. GET /api/content_gap/categories

Compares one category's coverage across multiple wikis. Each wiki's
`article_count` is the category's `qid_overlap_coverage` from the monthly
coverage snapshot — how many of the category's globally-known articles exist
in that wiki — the same measure the gap-discovery ranking uses.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `category` | String | No | Category title in enwiki (exact match) |
| `category_qid` | Integer | No | Category QID (numeric) |
| `wikis` | String | Yes | Comma-separated wiki list (e.g., `enwiki,mlwiki,tawiki`) |

**Note:** Either `category` or `category_qid` must be provided.

**Example Request:**

```bash
curl "http://localhost:8765/api/content_gap/categories?category=Quantum_physics&wikis=enwiki,mlwiki,tawiki"
```

**Response:**

```json
{
  "category": "Quantum_physics",
  "category_qid": 49833,
  "wikis": [
    {
      "wiki": "enwiki",
      "article_count": 100
    },
    {
      "wiki": "mlwiki",
      "article_count": 2
    }
  ]
}
```

**Error Responses:**

- `404 Not Found`: Category does not exist in enwiki
- `400 Bad Request`: Invalid parameters
- `500 Internal Server Error`: No coverage snapshot for a requested wiki

---

## Utility Endpoints

**Note:** The following endpoints are documented for reference but are not currently implemented in the codebase. Consider implementing them in future versions if needed.

### GET /api/health (Not Implemented)

Health check endpoint. Currently, use the `/api/pageviews/category` endpoint with a known category to verify server availability.

### GET /api/stats (Not Implemented)

System statistics endpoint. Currently, check system status by invoking `/api/pageviews/category` or other working endpoints.

## Error Handling

All errors follow a consistent format:

```json
{
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "Category 'InvalidCategory' does not exist in enwiki",
    "timestamp": "2025-01-14T10:30:45Z"
  }
}
```

**Common Error Codes:**

- `RESOURCE_NOT_FOUND` (404): Title or QID not found
- `INVALID_PARAMETER` (400): Invalid wiki, date, or metric
- `SERVICE_UNAVAILABLE` (503): Database or embedding service unavailable
- `UNPROCESSABLE_ENTITY` (422): Query parameters out of valid range
- `INTERNAL_ERROR` (500): Unexpected server error (rare)

---

## Client Code Examples

### JavaScript/TypeScript

```javascript
async function searchCategories(wiki, query) {
  const params = new URLSearchParams({
    wiki,
    query,
    match_threshold: 0.6,
    limit: 10
  });

  const response = await fetch(`/api/search/categories?${params}`);
  if (!response.ok) {
    throw new Error(`API error: ${response.status}`);
  }

  return response.json();
}

// Usage
searchCategories('enwiki', 'artificial intelligence')
  .then(results => {
    results.categories.forEach(cat => {
      console.log(`${cat.category_title} (${cat.match_score.toFixed(3)})`);
    });
  })
  .catch(err => console.error(err));
```

### Python

```python
import requests
import json

def get_top_categories(wiki, metric='total_views', limit=50):
    url = 'http://localhost:8765/api/list/top_categories'
    params = {
        'wiki': wiki,
        'metric': metric,
        'limit': limit
    }
    
    response = requests.get(url, params=params)
    response.raise_for_status()
    return response.json()

# Usage
results = get_top_categories('enwiki', limit=20)
for cat in results['categories']:
    print(f"{cat['rank']}. {cat['title']}: {cat['views']:,} views")
```

### cURL

```bash
# Search semantic index
curl "http://localhost:8765/api/search/categories?wiki=enwiki&query=machine+learning&limit=5" \
  | jq '.categories[] | {title: .category_title, score: .match_score}'

# Get category pageviews
curl "http://localhost:8765/api/pageviews/category?wiki=enwiki&title=Physics&start_date=2025-01-01" \
  | jq '.category | {title, total_views}'

# Trending categories
curl "http://localhost:8765/api/list/top_categories?wiki=frwiki&metric=trend_score&limit=10" \
  | jq '.categories[] | {rank, title, views}'
```

---

## Rate Limiting & Performance

The system is designed for internal use without rate limiting. However, be mindful of:

- **Trending queries** ($O(N)$): Can take 20-50ms. Cache results if polling frequently.
- **Semantic search** ($O(\log N)$): 50-150ms. Embedding service is the bottleneck.
- **Database queries**: Title translation adds 5-10ms. Batch requests where possible.

For high-frequency queries, consider:
- Caching results client-side
- Reducing date ranges
- Increasing `match_threshold` to reduce result set size

---

## Backward Compatibility

This API is considered stable. Breaking changes will increment the version number and be announced in advance. Additions to response objects are backward compatible.

For API versioning, prefix future versions with `/api/v2/`.

---

## Support & Debugging

For API issues:
1. Check `/api/health` for component status
2. Review logs: `RUST_LOG=debug ./topictrend_web`
4. Test embedding service: `cd services/embedding && EMBEDDING_SERVER=localhost:50051 uv run python healthcheck.py`

For deployment and operational questions, see [OPERATIONS.md](OPERATIONS.md).
For architectural context, see [README.md](README.md).
