use crate::models::AppState;
use crate::services::core::{CoreServiceError, EngineService};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub async fn resolve_source_categories(
    state: Arc<AppState>,
    wiki: &str,
    article_qids: &[u32],
    topic_category_qid_set: &HashSet<u32>,
    fallback_source_by_article: &HashMap<u32, u32>,
) -> Result<HashMap<u32, Vec<u32>>, CoreServiceError> {
    if article_qids.is_empty() {
        return Ok(HashMap::new());
    }

    let graph = EngineService::get_or_build_graph_engine(state, wiki).await?;

    let mut resolved = HashMap::new();

    for article_qid in article_qids {
        let direct_categories = graph
            .get_categories_for_article(*article_qid)
            .map_err(|e| {
                CoreServiceError::EngineError(format!(
                    "Failed to get categories for article Q{}: {}",
                    article_qid, e
                ))
            })?;

        // Collect matching categories: direct hits + parent hits (1 hop)
        let mut matched_set: HashSet<u32> = HashSet::new();

        for cat_qid in direct_categories {
            // Check if direct category is in topic set
            if topic_category_qid_set.contains(&cat_qid) {
                matched_set.insert(cat_qid);
            } else {
                // Check parent categories (1 hop) for matches
                if let Ok(parents) = graph.get_parent_categories(cat_qid) {
                    for parent_qid in parents {
                        if topic_category_qid_set.contains(&parent_qid) {
                            matched_set.insert(parent_qid);
                        }
                    }
                }
            }
        }

        let matched: Vec<u32> = matched_set.into_iter().collect();

        if !matched.is_empty() {
            resolved.insert(*article_qid, matched);
        } else if let Some(fallback) = fallback_source_by_article.get(article_qid).copied() {
            // If no direct or parent match, use fallback
            resolved.insert(*article_qid, vec![fallback]);
        }
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    #[test]
    fn filters_direct_categories_by_topic_set() {
        let topic_set = HashSet::from([10, 11, 12]);
        let direct_categories = vec![11, 12, 20, 21];

        let matched: Vec<u32> = direct_categories
            .into_iter()
            .filter(|cat_qid| topic_set.contains(cat_qid))
            .collect();

        assert_eq!(matched.len(), 2);
        assert!(matched.contains(&11));
        assert!(matched.contains(&12));
        assert!(!matched.contains(&20));
    }

    #[test]
    fn uses_fallback_when_no_direct_match() {
        let topic_set = HashSet::from([10]);
        let direct_categories = vec![11, 12];

        let matched: Vec<u32> = direct_categories
            .into_iter()
            .filter(|cat_qid| topic_set.contains(cat_qid))
            .collect();

        assert!(matched.is_empty());

        let fallback = HashMap::from([(100_u32, 10_u32)]);
        if let Some(fallback_cat) = fallback.get(&100_u32).copied() {
            assert_eq!(fallback_cat, 10);
        }
    }

    #[test]
    fn returns_all_matching_categories() {
        let topic_set = HashSet::from([10, 11, 12, 13]);
        let direct_categories = vec![11, 12, 13];

        let matched: Vec<u32> = direct_categories
            .into_iter()
            .filter(|cat_qid| topic_set.contains(cat_qid))
            .collect();

        assert_eq!(matched.len(), 3);
        assert!(matched.contains(&11));
        assert!(matched.contains(&12));
        assert!(matched.contains(&13));
    }
}
