use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use once_cell::sync::Lazy;
use tera::{Context, Tera};

static TEMPLATES: Lazy<Tera> =
    Lazy::new(|| match Tera::new("topictrend_web/templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Template parsing error: {}", e);
            std::process::exit(1);
        }
    });

#[derive(Debug)]
pub enum TemplateError {
    RenderError(tera::Error),
}

impl IntoResponse for TemplateError {
    fn into_response(self) -> Response {
        match self {
            TemplateError::RenderError(e) => {
                eprintln!("Template render error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
            }
        }
    }
}

pub struct PageContext {
    pub page_title: String,
    pub is_pageviews_active: bool,
    pub is_pageedits_active: bool,
    pub is_search_active: bool,
    pub is_content_gap_active: bool,
    pub is_trends_pageviews: bool,
    pub is_delta_pageviews: bool,
    pub is_trends_pageedits: bool,
    pub is_delta_pageedits: bool,
    pub form_id: Option<String>,
}

impl PageContext {
    pub fn home() -> Self {
        Self {
            page_title: "Topic Trends".to_string(),
            is_pageviews_active: false,
            is_pageedits_active: false,
            is_search_active: false,
            is_content_gap_active: false,
            is_trends_pageviews: false,
            is_delta_pageviews: false,
            is_trends_pageedits: false,
            is_delta_pageedits: false,
            form_id: None,
        }
    }

    pub fn pageview_trends() -> Self {
        Self {
            page_title: "Topic Trends - Pageviews".to_string(),
            is_pageviews_active: true,
            is_pageedits_active: false,
            is_search_active: false,
            is_content_gap_active: false,
            is_trends_pageviews: true,
            is_delta_pageviews: false,
            is_trends_pageedits: false,
            is_delta_pageedits: false,
            form_id: Some("trend-form".to_string()),
        }
    }

    pub fn pageview_delta() -> Self {
        Self {
            page_title: "Topic Trends - Pageviews".to_string(),
            is_pageviews_active: true,
            is_pageedits_active: false,
            is_search_active: false,
            is_content_gap_active: false,
            is_trends_pageviews: false,
            is_delta_pageviews: true,
            is_trends_pageedits: false,
            is_delta_pageedits: false,
            form_id: Some("delta-form".to_string()),
        }
    }

    pub fn pageedit_trends() -> Self {
        Self {
            page_title: "Topic Trends - Page Edits".to_string(),
            is_pageviews_active: false,
            is_pageedits_active: true,
            is_search_active: false,
            is_content_gap_active: false,
            is_trends_pageviews: false,
            is_delta_pageviews: false,
            is_trends_pageedits: true,
            is_delta_pageedits: false,
            form_id: Some("trend-form".to_string()),
        }
    }

    pub fn pageedit_delta() -> Self {
        Self {
            page_title: "Topic Trends - Page Edits".to_string(),
            is_pageviews_active: false,
            is_pageedits_active: true,
            is_search_active: false,
            is_content_gap_active: false,
            is_trends_pageviews: false,
            is_delta_pageviews: false,
            is_trends_pageedits: false,
            is_delta_pageedits: true,
            form_id: Some("delta-form".to_string()),
        }
    }

    pub fn search() -> Self {
        Self {
            page_title: "Topic Trends - Search".to_string(),
            is_pageviews_active: false,
            is_pageedits_active: false,
            is_search_active: true,
            is_content_gap_active: false,
            is_trends_pageviews: false,
            is_delta_pageviews: false,
            is_trends_pageedits: false,
            is_delta_pageedits: false,
            form_id: Some("search-form".to_string()),
        }
    }

    pub fn content_gap() -> Self {
        Self {
            page_title: "Topic Trends - Content Gap".to_string(),
            is_pageviews_active: false,
            is_pageedits_active: false,
            is_search_active: false,
            is_content_gap_active: true,
            is_trends_pageviews: false,
            is_delta_pageviews: false,
            is_trends_pageedits: false,
            is_delta_pageedits: false,
            form_id: Some("content-gap-form".to_string()),
        }
    }
}

pub fn render_template(
    template_name: &str,
    page_context: PageContext,
) -> Result<Html<String>, TemplateError> {
    let mut context = Context::new();
    context.insert("page_title", &page_context.page_title);
    context.insert("is_pageviews_active", &page_context.is_pageviews_active);
    context.insert("is_pageedits_active", &page_context.is_pageedits_active);
    context.insert("is_search_active", &page_context.is_search_active);
    context.insert("is_content_gap_active", &page_context.is_content_gap_active);
    context.insert("is_trends_pageviews", &page_context.is_trends_pageviews);
    context.insert("is_delta_pageviews", &page_context.is_delta_pageviews);
    context.insert("is_trends_pageedits", &page_context.is_trends_pageedits);
    context.insert("is_delta_pageedits", &page_context.is_delta_pageedits);
    context.insert("form_id", &page_context.form_id);

    TEMPLATES
        .render(template_name, &context)
        .map(Html)
        .map_err(TemplateError::RenderError)
}
