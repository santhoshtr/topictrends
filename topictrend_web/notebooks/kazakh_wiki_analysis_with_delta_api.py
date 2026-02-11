# -*- coding: utf-8 -*-
"""Kazakh wiki analysis using new Delta API

Demonstrates the simplified workflow using the new gRPC delta analysis endpoints.
"""

!uv pip install grpcio grpcio-tools pandas matplotlib

"""# Setup gRPC client for the TopicTrends server"""

!uv run python -m grpc_tools.protoc -I./proto --python_out=. --grpc_python_out=. ./proto/topictrend.proto

import grpc
from grpc import StatusCode
import logging
import topictrend_pb2
import topictrend_pb2_grpc
import pandas as pd
import matplotlib.pyplot as plt
from datetime import datetime, timedelta

channel_options = [
    ('grpc.keepalive_time_ms', 30000),
    ('grpc.keepalive_timeout_ms', 5000),
    ('grpc.keepalive_permit_without_calls', True),
    ('grpc.http2.max_pings_without_data', 0),
    ('grpc.http2.min_time_between_pings_ms', 10000),
]

class TopicTrendClient:
  def __init__(self, host='localhost', port=50051):
    self.channel = grpc.insecure_channel(f'{host}:{port}', options=channel_options)
    self.stub = topictrend_pb2_grpc.TopicTrendServiceStub(self.channel)
    self.logger = logging.getLogger(__name__)
  def close(self):
    self.channel.close()

  def __enter__(self):
    return self

  def __exit__(self, exc_type, exc_val, exc_tb):
    self.close()

  def _handle_grpc_error(self, e: grpc.RpcError):
    """Handle common gRPC errors"""
    error_mapping = {
        StatusCode.NOT_FOUND: "Resource not found",
        StatusCode.INVALID_ARGUMENT: "Invalid request parameters",
        StatusCode.INTERNAL: "Internal server error",
        StatusCode.UNAVAILABLE: "Service unavailable",
        StatusCode.DEADLINE_EXCEEDED: "Request timeout",
    }

    error_msg = error_mapping.get(e.code(), f"Unknown error: {e.details()}")
    self.logger.error(f"gRPC Error [{e.code()}]: {error_msg}")
    raise Exception(f"gRPC Error: {error_msg}")

# Initialize client
client = TopicTrendClient()

"""# Category Delta Analysis - Single API Call

Now we can get the complete delta analysis with just one API call instead of 200+ calls!
"""

def get_category_delta_analysis(wiki, baseline_start, baseline_end, impact_start, impact_end, limit=100):
    """
    Get category delta analysis between two time periods using the new gRPC API.
    """
    request = topictrend_pb2.CategoryDeltaRequest(
        wiki=wiki,
        baseline_start_date=baseline_start,
        baseline_end_date=baseline_end,
        impact_start_date=impact_start,
        impact_end_date=impact_end,
        limit=limit
    )

    try:
        response = client.stub.GetCategoryDelta(request)
        
        # Convert to DataFrame for easy analysis
        data = []
        for category in response.categories:
            data.append({
                'category_qid': category.category_qid,
                'category_name': category.category_title,
                'baseline_views': category.baseline_views,
                'impact_views': category.impact_views,
                'delta_percentage': category.delta_percentage,
                'absolute_delta': category.absolute_delta
            })
        
        df = pd.DataFrame(data)
        
        print(f"Analysis Period:")
        print(f"Baseline: {response.baseline_period}")
        print(f"Impact: {response.impact_period}")
        print(f"Categories analyzed: {len(df)}")
        
        return df
        
    except grpc.RpcError as e:
        client._handle_grpc_error(e)

# Perform the delta analysis
delta_df = get_category_delta_analysis(
    wiki="kkwiki",
    baseline_start="2025-09-01",
    baseline_end="2025-09-30", 
    impact_start="2025-08-01",
    impact_end="2025-08-31",
    limit=100
)

# Display top 10 most increased categories
print("\nTop 10 Most Increased Categories:")
delta_df.head(10)

"""# Visualization"""

# Plot the top 20 categories with biggest percentage increase
plt.figure(figsize=(15, 8))
top_20 = delta_df.head(20)
plt.barh(top_20['category_name'], top_20['delta_percentage'])
plt.xlabel('Pageview Change (%)')
plt.title('Top 20 Most Increased Categories (kkwiki Sept vs Aug 2025)')
plt.axvline(x=0, color='red', linestyle='--')
plt.tight_layout()
plt.gca().invert_yaxis()  # Highest at top
plt.show()

# Show the most impacted category
top_category = delta_df.iloc[0]
print(f"\nMost Impacted Category:")
print(f"Name: {top_category['category_name']}")
print(f"QID: Q{top_category['category_qid']}")
print(f"Baseline views: {top_category['baseline_views']:,}")
print(f"Impact views: {top_category['impact_views']:,}")
print(f"Percentage change: {top_category['delta_percentage']:.1f}%")
print(f"Absolute change: {top_category['absolute_delta']:,}")

"""# Article Delta Analysis within Top Category

Now let's analyze which articles within the top category caused the change.
"""

def get_article_delta_analysis(wiki, category_qid, baseline_start, baseline_end, impact_start, impact_end, limit=50):
    """
    Get article delta analysis within a category between two time periods.
    """
    request = topictrend_pb2.ArticleDeltaRequest(
        wiki=wiki,
        category_qid=category_qid,
        baseline_start_date=baseline_start,
        baseline_end_date=baseline_end,
        impact_start_date=impact_start,
        impact_end_date=impact_end,
        limit=limit,
        depth=2
    )

    try:
        response = client.stub.GetArticleDelta(request)
        
        # Convert to DataFrame for easy analysis
        data = []
        for article in response.articles:
            data.append({
                'article_qid': article.article_qid,
                'article_title': article.article_title,
                'baseline_views': article.baseline_views,
                'impact_views': article.impact_views,
                'delta_percentage': article.delta_percentage,
                'absolute_delta': article.absolute_delta
            })
        
        df = pd.DataFrame(data)
        
        print(f"Article Analysis for Category: {response.category_title} (Q{response.category_qid})")
        print(f"Baseline: {response.baseline_period}")
        print(f"Impact: {response.impact_period}")
        print(f"Articles analyzed: {len(df)}")
        
        return df
        
    except grpc.RpcError as e:
        client._handle_grpc_error(e)

# Get the top category QID from our previous analysis
top_category_qid = delta_df.iloc[0]['category_qid']

# Analyze articles within that category
article_delta_df = get_article_delta_analysis(
    wiki="kkwiki",
    category_qid=top_category_qid,
    baseline_start="2025-09-01",
    baseline_end="2025-09-30",
    impact_start="2025-08-01", 
    impact_end="2025-08-31",
    limit=50
)

# Display top 10 articles with biggest change
print("\nTop 10 Articles with Biggest Change:")
article_delta_df.head(10)

"""# Plot Individual Article Trends

Let's plot the daily trend for the top article to see the change pattern.
"""

def plot_article_trend(wiki, article_qid, start_date, end_date, title=""):
    """
    Plot daily pageview trend for an article.
    """
    request = topictrend_pb2.ArticleViewsRequest(
        wiki=wiki,
        article_qid=article_qid,
        start_date=start_date,
        end_date=end_date
    )

    try:
        response = client.stub.GetArticleViews(request)
        
        # Convert to DataFrame
        data = []
        for view in response.views:
            data.append({
                'Date': view.date,
                'Views': view.views
            })

        df = pd.DataFrame(data)
        df['Date'] = pd.to_datetime(df['Date'])
        df.set_index('Date', inplace=True)

        # Create the plot
        plt.figure(figsize=(12, 6))
        df['Views'].plot(
            kind='line',
            marker='o',
            markersize=4,
            linestyle='-',
            color='#3498db',
            linewidth=2
        )

        plt.title(f'Daily Views: {title}', fontsize=16, pad=15)
        plt.xlabel('Date', fontsize=12)
        plt.ylabel('Daily Views', fontsize=12)
        plt.xticks(rotation=45)
        plt.ticklabel_format(style='plain', axis='y')
        plt.grid(axis='y', linestyle=':', alpha=0.6)
        plt.tight_layout()
        plt.show()

        return df

    except grpc.RpcError as e:
        client._handle_grpc_error(e)

# Plot the trend for the top article
if not article_delta_df.empty:
    top_article = article_delta_df.iloc[0]
    print(f"Plotting trend for: {top_article['article_title']}")
    
    article_trend_df = plot_article_trend(
        wiki="kkwiki",
        article_qid=top_article['article_qid'],
        start_date="2025-08-01",
        end_date="2025-09-30",
        title=top_article['article_title']
    )

"""# Summary

The new delta analysis API provides:

1. **Single API call** instead of 200+ individual calls
2. **Server-side calculation** of deltas with proper zero-handling
3. **Consistent results** across different clients
4. **Better performance** through optimized database queries
5. **Cleaner code** with less client-side data manipulation

Key findings from this analysis:
- Most impacted category and its percentage change
- Top articles within that category causing the change
- Visual trends showing the exact timing of pageview spikes

This makes differential analysis much more practical for regular use!
"""

# Close the gRPC connection
client.close()
