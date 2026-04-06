pub mod tools;

use std::sync::Arc;

use rmcp::{ServerHandler, model::ServerInfo, tool_handler};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{Implementation, ServerCapabilities};

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
            .with_route((Self::get_category_pageview_trend_tool_attr(), Self::get_category_pageview_trend))
            .with_route((Self::get_article_pageview_trend_tool_attr(), Self::get_article_pageview_trend))
            .with_route((Self::get_topic_pageview_trend_tool_attr(), Self::get_topic_pageview_trend))
            .with_route((Self::get_pageviews_top_categories_tool_attr(), Self::get_pageviews_top_categories))
            .with_route((Self::get_pageviews_top_articles_tool_attr(), Self::get_pageviews_top_articles))
            .with_route((Self::get_categories_pageview_trend_tool_attr(), Self::get_categories_pageview_trend))
            // pageedits (5)
            .with_route((Self::get_category_pageedit_trend_tool_attr(), Self::get_category_pageedit_trend))
            .with_route((Self::get_article_pageedit_trend_tool_attr(), Self::get_article_pageedit_trend))
            .with_route((Self::get_topic_pageedit_trend_tool_attr(), Self::get_topic_pageedit_trend))
            .with_route((Self::get_pageedits_top_categories_tool_attr(), Self::get_pageedits_top_categories))
            .with_route((Self::get_pageedits_top_articles_tool_attr(), Self::get_pageedits_top_articles))
            // googlesearch (5)
            .with_route((Self::get_category_googlesearch_trend_tool_attr(), Self::get_category_googlesearch_trend))
            .with_route((Self::get_article_googlesearch_trend_tool_attr(), Self::get_article_googlesearch_trend))
            .with_route((Self::get_topic_googlesearch_trend_tool_attr(), Self::get_topic_googlesearch_trend))
            .with_route((Self::get_googlesearch_top_categories_tool_attr(), Self::get_googlesearch_top_categories))
            .with_route((Self::get_googlesearch_top_articles_tool_attr(), Self::get_googlesearch_top_articles))
            // delta (6)
            .with_route((Self::get_category_pageview_delta_tool_attr(), Self::get_category_pageview_delta))
            .with_route((Self::get_article_pageview_delta_tool_attr(), Self::get_article_pageview_delta))
            .with_route((Self::get_category_pageedit_delta_tool_attr(), Self::get_category_pageedit_delta))
            .with_route((Self::get_article_pageedit_delta_tool_attr(), Self::get_article_pageedit_delta))
            .with_route((Self::get_category_googlesearch_delta_tool_attr(), Self::get_category_googlesearch_delta))
            .with_route((Self::get_article_googlesearch_delta_tool_attr(), Self::get_article_googlesearch_delta))
            // search (1)
            .with_route((Self::search_categories_tool_attr(), Self::search_categories))
            // lists (3)
            .with_route((Self::list_subcategories_tool_attr(), Self::list_subcategories))
            .with_route((Self::list_articles_in_category_tool_attr(), Self::list_articles_in_category))
            .with_route((Self::get_content_gap_topic_tool_attr(), Self::get_content_gap_topic))
    }
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
