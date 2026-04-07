pub mod tools;

use std::sync::Arc;

use rmcp::{ServerHandler, model::ServerInfo, tool_handler};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{Implementation, ServerCapabilities, Tool};

use crate::models::AppState;

#[derive(Clone)]
pub struct TopicTrendMcpServer {
    pub state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
}

impl TopicTrendMcpServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    fn tool_router() -> ToolRouter<Self> {
        ToolRouter::<Self>::new()
            // pageviews (6)
            .with_route((clean(Self::get_category_pageview_trend_tool_attr()), Self::get_category_pageview_trend))
            .with_route((clean(Self::get_article_pageview_trend_tool_attr()), Self::get_article_pageview_trend))
            .with_route((clean(Self::get_topic_pageview_trend_tool_attr()), Self::get_topic_pageview_trend))
            .with_route((clean(Self::get_pageviews_top_categories_tool_attr()), Self::get_pageviews_top_categories))
            .with_route((clean(Self::get_pageviews_top_articles_tool_attr()), Self::get_pageviews_top_articles))
            .with_route((clean(Self::get_categories_pageview_trend_tool_attr()), Self::get_categories_pageview_trend))
            // pageedits (5)
            .with_route((clean(Self::get_category_pageedit_trend_tool_attr()), Self::get_category_pageedit_trend))
            .with_route((clean(Self::get_article_pageedit_trend_tool_attr()), Self::get_article_pageedit_trend))
            .with_route((clean(Self::get_topic_pageedit_trend_tool_attr()), Self::get_topic_pageedit_trend))
            .with_route((clean(Self::get_pageedits_top_categories_tool_attr()), Self::get_pageedits_top_categories))
            .with_route((clean(Self::get_pageedits_top_articles_tool_attr()), Self::get_pageedits_top_articles))
            // googlesearch (5)
            .with_route((clean(Self::get_category_googlesearch_trend_tool_attr()), Self::get_category_googlesearch_trend))
            .with_route((clean(Self::get_article_googlesearch_trend_tool_attr()), Self::get_article_googlesearch_trend))
            .with_route((clean(Self::get_topic_googlesearch_trend_tool_attr()), Self::get_topic_googlesearch_trend))
            .with_route((clean(Self::get_googlesearch_top_categories_tool_attr()), Self::get_googlesearch_top_categories))
            .with_route((clean(Self::get_googlesearch_top_articles_tool_attr()), Self::get_googlesearch_top_articles))
            // delta (6)
            .with_route((clean(Self::get_category_pageview_delta_tool_attr()), Self::get_category_pageview_delta))
            .with_route((clean(Self::get_article_pageview_delta_tool_attr()), Self::get_article_pageview_delta))
            .with_route((clean(Self::get_category_pageedit_delta_tool_attr()), Self::get_category_pageedit_delta))
            .with_route((clean(Self::get_article_pageedit_delta_tool_attr()), Self::get_article_pageedit_delta))
            .with_route((clean(Self::get_category_googlesearch_delta_tool_attr()), Self::get_category_googlesearch_delta))
            .with_route((clean(Self::get_article_googlesearch_delta_tool_attr()), Self::get_article_googlesearch_delta))
             // search (1)
            .with_route((clean(Self::search_categories_tool_attr()), Self::search_categories))
            // lists (4)
            .with_route((clean(Self::list_subcategories_tool_attr()), Self::list_subcategories))
            .with_route((clean(Self::list_articles_in_category_tool_attr()), Self::list_articles_in_category))
            .with_route((clean(Self::list_article_categories_tool_attr()), Self::list_article_categories))
            .with_route((clean(Self::get_content_gap_topic_tool_attr()), Self::get_content_gap_topic))
    }
}

/// Recursively remove all "format" keys from a JSON schema object.
/// schemars emits non-standard format values ("uint32", "uint64", "float") for Rust
/// integer/float primitives. These are not part of JSON Schema and cause "unknown format"
/// warnings in strict validators like OpenCode's MCP schema checker.
fn strip_format(obj: &serde_json::Map<String, serde_json::Value>) -> serde_json::Map<String, serde_json::Value> {
    obj.iter()
        .filter(|(k, _)| *k != "format")
        .map(|(k, v)| (k.clone(), strip_format_value(v)))
        .collect()
}

fn strip_format_value(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(obj) => serde_json::Value::Object(strip_format(obj)),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(strip_format_value).collect())
        }
        other => other.clone(),
    }
}

/// Strip non-standard "format" annotations from a tool's input and output schemas.
fn clean(mut tool: Tool) -> Tool {
    tool.input_schema = Arc::new(strip_format(&tool.input_schema));
    tool.output_schema = tool.output_schema.map(|s| Arc::new(strip_format(&s)));
    tool
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TopicTrendMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new("topictrends", env!("CARGO_PKG_VERSION")))
    }
}
