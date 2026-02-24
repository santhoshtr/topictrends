use crate::models::{
	AppState, ArticleItem, ContentGapMissingResult, ContentGapResult, ContentGapWikiResult,
};
use crate::services::core::{CoreServiceError, EngineService, QidService};
use roaring::RoaringBitmap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ContentGapService;

impl ContentGapService {
	pub async fn get_content_gap(
		state: Arc<AppState>,
		category_qid: u32,
		category_label: &str,
		wikis: Vec<String>,
		depth: u32,
		include_articles: bool,
	) -> Result<ContentGapResult, CoreServiceError> {
		let mut wiki_bitmaps: Vec<(String, RoaringBitmap)> = Vec::new();
		let mut wiki_article_qids: HashMap<String, Vec<u32>> = HashMap::new();

		for wiki in &wikis {
			let graph = EngineService::get_or_build_graph_engine(Arc::clone(&state), wiki).await?;
			let article_qids = {
				let graph_lock = graph.read().map_err(|_| {
					CoreServiceError::InternalError("Failed to acquire graph lock".to_string())
				})?;
				graph_lock
					.get_articles_in_category(category_qid, depth)
					.map_err(|err| CoreServiceError::EngineError(err))?
			};
			let mut bitmap = RoaringBitmap::new();
			for qid in &article_qids {
				bitmap.insert(*qid);
			}
			wiki_article_qids.insert(wiki.clone(), article_qids);
			wiki_bitmaps.push((wiki.clone(), bitmap));
		}

		let overlap_bitmap = Self::compute_overlap(&wiki_bitmaps);
		let overlap_article_qids: Vec<u32> = overlap_bitmap.iter().collect();

		let reference_bitmap = wiki_bitmaps
			.iter()
			.find(|(wiki, _)| wiki == "enwiki")
			.map(|item| item.1.clone())
			.unwrap_or_else(RoaringBitmap::new);

		let mut missing_from: HashMap<String, ContentGapMissingResult> = HashMap::new();
		for (wiki, bitmap) in &wiki_bitmaps {
			if wiki == "enwiki" {
				continue;
			}
			let missing_bitmap = &reference_bitmap - bitmap;
			let missing_qids: Vec<u32> = missing_bitmap.iter().collect();
			missing_from.insert(
				wiki.clone(),
				ContentGapMissingResult {
					count: missing_qids.len(),
					article_qids: missing_qids,
				},
			);
		}

		let mut results: Vec<ContentGapWikiResult> = Vec::new();
		for wiki in &wikis {
			let article_qids = wiki_article_qids.get(wiki).cloned().unwrap_or_default();
			let articles = if include_articles {
				let titles_map =
					QidService::get_titles_by_qids(Arc::clone(&state), wiki, &article_qids)
						.await?;
				Some(
					article_qids
						.iter()
						.map(|qid| ArticleItem {
							qid: *qid,
							title: titles_map
								.get(qid)
								.cloned()
								.unwrap_or_else(|| format!("Q{}", qid)),
						})
						.collect::<Vec<ArticleItem>>(),
				)
			} else {
				None
			};

			results.push(ContentGapWikiResult {
				wiki: wiki.clone(),
				article_count: article_qids.len(),
				article_qids,
				articles,
			});
		}

		Ok(ContentGapResult {
			category: category_label.to_string(),
			category_qid,
			depth,
			wikis: results,
			overlap_count: overlap_article_qids.len(),
			overlap_article_qids,
			missing_from,
		})
	}

	fn compute_overlap(wiki_bitmaps: &[(String, RoaringBitmap)]) -> RoaringBitmap {
		let mut overlap = match wiki_bitmaps.first() {
			Some(item) => item.1.clone(),
			None => return RoaringBitmap::new(),
		};

		for (_, bitmap) in wiki_bitmaps.iter().skip(1) {
			overlap &= bitmap;
		}

		overlap
	}
}
