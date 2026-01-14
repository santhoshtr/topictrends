# API.md: TopicTrends REST API Documentation

This document specifies the REST API endpoints provided by the TopicTrends web server.

## Base URL

```
http://localhost:8000
```

All endpoints are prefixed with `/api/`.

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
curl "http://localhost:8000/api/pageviews/category?wiki=enwiki&title=Physics"

curl "http://localhost:8000/api/pageviews/category?wiki=frwiki&title=Physique&start_date=2025-01-01&end_date=2025-01-14"
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
curl "http://localhost:8000/api/pageviews/article?wiki=enwiki&qid=42&start_date=2025-01-01"
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

**Complexity:** $O(1)$ — Direct array lookup via mmap. Execution time: <1 millisecond.

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
curl "http://localhost:8000/api/list/sub_categories?wiki=enwiki&title=Science&limit=20"
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
curl "http://localhost:8000/api/list/top_categories?wiki=enwiki&limit=50"

# Trending categories (growth metric)
curl "http://localhost:8000/api/list/top_categories?wiki=frwiki&metric=trend_score&limit=20&start_date=2025-01-01&end_date=2025-01-14"
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
curl "http://localhost:8000/api/search/categories?wiki=enwiki&query=machine+learning&limit=10"

# Same search, return results in French
curl "http://localhost:8000/api/search/categories?wiki=frwiki&query=machine+learning&limit=10"

# High-confidence results only
curl "http://localhost:8000/api/search/categories?wiki=dewiki&query=quantum+computing&match_threshold=0.8&limit=5"
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

**Complexity:** Dominated by Qdrant HNSW search, $O(\log N)$ where $N$ = 2.5M categories. Execution time: 50-150 milliseconds.

**Error Responses:**

- `400 Bad Request`: Invalid parameters or empty query
- `503 Service Unavailable`: Embedding service or Qdrant unavailable
- `422 Unprocessable Entity`: Query too long (max ~1000 characters)

---

## Utility Endpoints

### GET /api/health

Health check endpoint. Returns system status.

**Response:**

```json
{
  "status": "healthy",
  "timestamp": "2025-01-14T10:30:45Z",
  "components": {
    "topology_loaded": true,
    "pageviews_available": true,
    "mariadb_connected": true,
    "qdrant_available": true
  }
}
```

### GET /api/stats

System statistics and loaded data info.

**Response:**

```json
{
  "wikis_loaded": 345,
  "total_categories": 2567123,
  "total_articles": 45231567,
  "total_edges": 196000000,
  "memory_usage_mb": 3456,
  "pageview_days_available": 365,
  "latest_pageview_date": "2025-01-14",
  "topology_refresh_date": "2024-12-15"
}
```

---

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
- `SERVICE_UNAVAILABLE` (503): Database or Qdrant unavailable
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
    url = 'http://localhost:8000/api/list/top_categories'
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
curl "http://localhost:8000/api/search/categories?wiki=enwiki&query=machine+learning&limit=5" \
  | jq '.categories[] | {title: .category_title, score: .match_score}'

# Get category pageviews
curl "http://localhost:8000/api/pageviews/category?wiki=enwiki&title=Physics&start_date=2025-01-01" \
  | jq '.category | {title, total_views}'

# Trending categories
curl "http://localhost:8000/api/list/top_categories?wiki=frwiki&metric=trend_score&limit=10" \
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
3. Verify MariaDB connectivity: `./target/release/topictrend_web --check-db`
4. Test Qdrant: `curl http://localhost:6333/health`

For deployment and operational questions, see [OPERATIONS.md](OPERATIONS.md).
For architectural context, see [README.md](README.md).
