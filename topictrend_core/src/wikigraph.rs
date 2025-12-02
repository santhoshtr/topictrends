use crate::{csr_adjacency::CsrAdjacency, direct_map::DirectMap};
use roaring::RoaringBitmap;
use std::collections::VecDeque;

/// The core high-performance graph structure.
/// All internal logic uses "Dense IDs" (0..N), not the raw Wikipedia Page QIDs.
#[derive(Debug)]
pub struct WikiGraph {
    pub children: CsrAdjacency,
    pub parents: CsrAdjacency,
    pub cat_articles: Vec<RoaringBitmap>,
    pub article_cats: CsrAdjacency,
    pub cat_dense_to_original: Vec<u32>,  // Dense -> QID
    pub cat_original_to_dense: DirectMap, // QID -> Dense
    pub art_dense_to_original: Vec<u32>,
    pub art_original_to_dense: DirectMap,
}

impl WikiGraph {
    /// Find all articles in a category (and optionally subcategories to depth N)
    pub fn get_articles_in_category(
        &self,
        category_qid: u32,
        max_depth: u8,
    ) -> Result<RoaringBitmap, String> {
        // Translate External ID -> Internal Dense ID
        let start_node = match self.cat_original_to_dense.get(category_qid) {
            Some(id) => id,
            None => {
                return Ok(RoaringBitmap::new());
            } // Dense ID not found
        };

        let mut articles_dense = RoaringBitmap::new();
        let mut visited = RoaringBitmap::new(); // To handle cycles
        let mut queue = VecDeque::new();

        // Queue stores (node_id, current_depth)
        queue.push_back((start_node, 0));
        visited.insert(start_node);

        while let Some((curr, depth)) = queue.pop_front() {
            // A. Collect articles from this category
            if let Some(articles) = self.cat_articles.get(curr as usize) {
                articles_dense |= articles;
            }

            // B. Traverse deeper if allowed
            if depth < max_depth {
                let children = self.children.get(curr as u32);
                for &child in children {
                    if !visited.contains(child) {
                        visited.insert(child);
                        queue.push_back((child, depth + 1));
                    }
                }
            }
        }
        // Map back to QID
        Ok(articles_dense
            .iter()
            .map(|article_dense| {
                let idx = article_dense as usize;
                self.art_dense_to_original[idx]
            })
            .collect())
    }

    /// Get immediate subcategories (Depth 1)
    /// Returns a vector of category_qids: Original_Wiki_ID
    pub fn get_child_categories(&self, category_qid: u32) -> Result<Vec<u32>, String> {
        // Convert External ID -> Internal Dense ID
        let dense_id = match self.cat_original_to_dense.get(category_qid) {
            Some(id) => id,
            None => return Ok(Vec::new()), // Category not found
        };

        // Lookup children in the Adjacency List
        let children_dense = self.children.get(dense_id);
        // Map back to QID
        Ok(children_dense
            .iter()
            .map(|&child_dense| {
                let idx = child_dense as usize;
                self.cat_dense_to_original[idx]
            })
            .collect())
    }

    /// Get all subcategories up to a specific depth `n`.
    /// Returns a vector of tuples: (Original_QID, Depth)
    pub fn get_descendant_categories(
        &self,
        category_qid: u32,
        max_depth: u8,
    ) -> Result<Vec<(u32, u8)>, String> {
        let start_node = match self.cat_original_to_dense.get(category_qid) {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        let mut results: Vec<(u32, u8)> = Vec::new();
        let mut queue = VecDeque::new();
        // Use a lightweight bitset for visited check to handle cycles
        let mut visited = RoaringBitmap::new();

        queue.push_back((start_node, 0));
        visited.insert(start_node);

        while let Some((curr, depth)) = queue.pop_front() {
            // If it's not the start node, add it to results
            if curr != start_node {
                let idx = curr as usize;
                results.push((self.cat_dense_to_original[idx], depth));
            }

            // Stop if we reached max depth
            if depth >= max_depth {
                continue;
            }

            // Enqueue children
            let children = self.children.get(curr);
            for &child in children {
                if !visited.contains(child) {
                    visited.insert(child);
                    queue.push_back((child, depth + 1));
                }
            }
        }

        Ok(results)
    }

    /// Find parent categories (Navigate Up)
    pub fn get_parent_categories(&self, category_qid: u32) -> Result<Vec<u32>, String> {
        let dense_id = match self.cat_original_to_dense.get(category_qid) {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        let parents_dense = self.parents.get(dense_id);
        // Convert back to Original IDs for the user
        Ok(parents_dense
            .iter()
            .map(|&p_dense| self.cat_dense_to_original[p_dense as usize])
            .collect())
    }

    /// Get all parent categories for a specific article.
    /// Returns a vector of Category_QID
    pub fn get_categories_for_article(&self, wiki_article_qid: u32) -> Result<Vec<u32>, String> {
        //  Convert Article External ID -> Article Internal Dense ID
        let dense_article_qid = match self.art_original_to_dense.get(wiki_article_qid) {
            Some(id) => id,
            None => return Ok(Vec::new()), // Article not found
        };

        //  Lookup the list of Category Dense IDs for this article
        let category_dense_ids = self.article_cats.get(dense_article_qid);
        //  Map Category Dense IDs back to QID
        Ok(category_dense_ids
            .iter()
            .map(|&cat_dense| {
                let idx = cat_dense as usize;
                self.cat_dense_to_original[idx]
            })
            .collect())
    }
}
