use crate::models::AppState;
use crate::services::core::{CoreServiceError, EngineService};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributionOrigin {
    Direct,
    Fallback,
}

impl AttributionOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Fallback => "fallback",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedSourceCategory {
    pub category_qid: u32,
    pub origin: AttributionOrigin,
}

fn pick_source_category(
    direct_categories: &[u32],
    topic_category_qid_set: &HashSet<u32>,
    per_category_metrics: Option<&HashMap<u32, u64>>,
    fallback_source_category_qid: Option<u32>,
) -> Option<ResolvedSourceCategory> {
    let mut best_direct: Option<(u64, u32)> = None;

    if let Some(metrics_map) = per_category_metrics {
        for category_qid in direct_categories {
            if !topic_category_qid_set.contains(category_qid) {
                continue;
            }

            let metric = metrics_map.get(category_qid).copied().unwrap_or(0);
            if metric == 0 {
                continue;
            }

            match best_direct {
                None => best_direct = Some((metric, *category_qid)),
                Some((best_metric, best_category_qid)) => {
                    if metric > best_metric
                        || (metric == best_metric && *category_qid < best_category_qid)
                    {
                        best_direct = Some((metric, *category_qid));
                    }
                }
            }
        }
    }

    if let Some((_, category_qid)) = best_direct {
        return Some(ResolvedSourceCategory {
            category_qid,
            origin: AttributionOrigin::Direct,
        });
    }

    fallback_source_category_qid.map(|category_qid| ResolvedSourceCategory {
        category_qid,
        origin: AttributionOrigin::Fallback,
    })
}

pub async fn resolve_source_categories(
    state: Arc<AppState>,
    wiki: &str,
    article_qids: &[u32],
    topic_category_qid_set: &HashSet<u32>,
    article_category_metrics: &HashMap<u32, HashMap<u32, u64>>,
    fallback_source_by_article: &HashMap<u32, u32>,
) -> Result<HashMap<u32, ResolvedSourceCategory>, CoreServiceError> {
    if article_qids.is_empty() {
        return Ok(HashMap::new());
    }

    let graph = EngineService::get_or_build_graph_engine(state, wiki).await?;
    let graph_lock = graph.read().map_err(|e| {
        CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
    })?;

    let mut resolved = HashMap::new();

    for article_qid in article_qids {
        let direct_categories = graph_lock
            .get_categories_for_article(*article_qid)
            .map_err(|e| {
                CoreServiceError::EngineError(format!(
                    "Failed to get categories for article Q{}: {}",
                    article_qid, e
                ))
            })?;

        let source = pick_source_category(
            &direct_categories,
            topic_category_qid_set,
            article_category_metrics.get(article_qid),
            fallback_source_by_article.get(article_qid).copied(),
        );

        if let Some(source) = source {
            resolved.insert(*article_qid, source);
        }
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::{AttributionOrigin, pick_source_category};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn picks_direct_category_with_positive_metric() {
        let topic_set = HashSet::from([10, 11, 12]);
        let direct_categories = vec![11, 12];
        let metrics = HashMap::from([(11, 50_u64), (12, 75_u64)]);

        let resolved =
            pick_source_category(&direct_categories, &topic_set, Some(&metrics), Some(10));

        assert_eq!(resolved.expect("source").category_qid, 12);
        assert_eq!(resolved.expect("source").origin, AttributionOrigin::Direct);
    }

    #[test]
    fn ignores_zero_metric_direct_and_uses_fallback() {
        let topic_set = HashSet::from([10, 11]);
        let direct_categories = vec![11];
        let metrics = HashMap::from([(11, 0_u64)]);

        let resolved =
            pick_source_category(&direct_categories, &topic_set, Some(&metrics), Some(10));

        assert_eq!(resolved.expect("source").category_qid, 10);
        assert_eq!(
            resolved.expect("source").origin,
            AttributionOrigin::Fallback
        );
    }

    #[test]
    fn no_direct_intersection_uses_fallback() {
        let topic_set = HashSet::from([10]);
        let direct_categories = vec![11, 12];
        let metrics = HashMap::from([(11, 42_u64), (12, 99_u64)]);

        let resolved =
            pick_source_category(&direct_categories, &topic_set, Some(&metrics), Some(10));

        assert_eq!(resolved.expect("source").category_qid, 10);
        assert_eq!(
            resolved.expect("source").origin,
            AttributionOrigin::Fallback
        );
    }

    #[test]
    fn tie_breaks_by_lower_category_qid() {
        let topic_set = HashSet::from([10, 11]);
        let direct_categories = vec![11, 10];
        let metrics = HashMap::from([(10, 20_u64), (11, 20_u64)]);

        let resolved = pick_source_category(&direct_categories, &topic_set, Some(&metrics), None);

        assert_eq!(resolved.expect("source").category_qid, 10);
        assert_eq!(resolved.expect("source").origin, AttributionOrigin::Direct);
    }
}
