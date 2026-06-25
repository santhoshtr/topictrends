use super::{CoreServiceError, EngineService, QidService};
use crate::models::{AppState, ArticleClusterDto, ArticleItem, ClusterArticlesResponse};
use std::sync::Arc;

pub struct ClusterService;

impl ClusterService {
    /// Group `articles` (titles) into category-topics on the shared in-memory
    /// graph, then resolve every result QID back to titles. Each article lands
    /// in its single best topic; titles that resolve to no QID are returned in
    /// `unresolved`, resolved articles with no local category in `unclustered`.
    pub async fn cluster(
        state: Arc<AppState>,
        wiki: &str,
        articles: Vec<String>,
        max_clusters: Option<usize>,
        min_agreement: u16,
    ) -> Result<ClusterArticlesResponse, CoreServiceError> {
        // Resolve article titles -> QIDs (namespace 0). Titles that resolve
        // nowhere are echoed back so callers can spot typos or stale names.
        let resolved =
            QidService::get_qids_by_titles(Arc::clone(&state), wiki, articles.clone(), 0).await?;
        let unresolved: Vec<String> = articles
            .iter()
            .filter(|t| !resolved.contains_key(*t))
            .cloned()
            .collect();

        // Sort the QIDs for deterministic output: cluster membership is
        // order-independent, but within-cluster article order follows input
        // order, and HashMap iteration order is not stable.
        let mut qids: Vec<u32> = resolved.into_values().collect();
        qids.sort_unstable();

        let graph = EngineService::get_or_build_graph_engine(Arc::clone(&state), wiki).await?;
        let outcome = graph.cluster_articles(&qids, max_clusters, min_agreement);

        // One title lookup covering every category and article QID in the result.
        let mut all_qids: Vec<u32> = Vec::new();
        for c in &outcome.clusters {
            all_qids.push(c.category_qid);
            all_qids.extend(&c.article_qids);
        }
        all_qids.extend(&outcome.unclustered_qids);
        let titles = QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids).await?;
        let en = QidService::get_english_titles(Arc::clone(&state), wiki, &all_qids).await;

        let item = |qid: u32| ArticleItem {
            qid,
            title: titles.get(&qid).cloned().unwrap_or_else(|| format!("Q{}", qid)),
            title_en: en.get(&qid).cloned(),
        };

        let clusters = outcome
            .clusters
            .into_iter()
            .map(|c| ArticleClusterDto {
                category: titles
                    .get(&c.category_qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", c.category_qid)),
                category_en: en.get(&c.category_qid).cloned(),
                category_qid: c.category_qid,
                size: c.size,
                articles: c.article_qids.iter().map(|&q| item(q)).collect(),
            })
            .collect();
        let unclustered = outcome.unclustered_qids.iter().map(|&q| item(q)).collect();

        Ok(ClusterArticlesResponse {
            wiki: wiki.to_string(),
            clusters,
            unclustered,
            unresolved,
        })
    }
}
