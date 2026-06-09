use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::models::{AppState, RelatedArticleItem, RelatedArticlesParams, RelatedArticlesResponse};
use crate::services::core::{CoreServiceError, QidService, RelatedService};

use super::ApiError;

/// Find articles related to the given article by shared-category overlap.
/// Resolves the input title to a QID via MariaDB, runs the scatter on the
/// shared in-memory graph, then resolves result QIDs back to titles and builds
/// a Wikipedia URL for each — title, link, and overlap score.
pub async fn get_related_articles_handler(
    Query(params): Query<RelatedArticlesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<RelatedArticlesResponse>, ApiError> {
    let top = params.limit.unwrap_or(20);

    let article_qid = if let Some(qid) = params.article_qid {
        qid
    } else {
        let article = params.article.ok_or_else(|| {
            CoreServiceError::InternalError(
                "Either article or article_qid must be provided".to_string(),
            )
        })?;
        // MediaWiki page_title uses underscores; accept spaces from the caller.
        let title = article.replace(' ', "_");
        QidService::get_qid_by_title(Arc::clone(&state), params.wiki.as_str(), &title, 0).await?
    };

    let related = RelatedService::get_related_articles(
        Arc::clone(&state),
        params.wiki.as_str(),
        article_qid,
        top,
    )
    .await?;

    let qids: Vec<u32> = related.iter().map(|(qid, _)| *qid).collect();
    let titles_map =
        QidService::get_titles_by_qids(Arc::clone(&state), params.wiki.as_str(), &qids).await?;

    let host = wiki_to_host(&params.wiki);
    let articles: Vec<RelatedArticleItem> = related
        .into_iter()
        .filter_map(|(qid, score)| {
            titles_map.get(&qid).map(|title| RelatedArticleItem {
                qid,
                title: title.clone(),
                url: format!("https://{}/wiki/{}", host, title),
                score,
            })
        })
        .collect();

    Ok(Json(RelatedArticlesResponse { articles }))
}

/// Wikipedia dbname -> hostname: strip the trailing `wiki`, underscores to
/// dashes. `enwiki` -> `en.wikipedia.org`, `be_x_oldwiki` -> `be-x-old.wikipedia.org`.
fn wiki_to_host(wiki: &str) -> String {
    let lang = wiki.strip_suffix("wiki").unwrap_or(wiki).replace('_', "-");
    format!("{}.wikipedia.org", lang)
}
