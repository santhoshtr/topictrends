use polars::prelude::*;
use roaring::RoaringBitmap;
use std::collections::HashMap;
use std::collections::VecDeque;

/// The core high-performance graph structure.
/// All internal logic uses "Dense IDs" (0..N), not the raw Wikipedia Page IDs.
pub struct WikiGraph {
    pub(crate) children: Vec<Vec<u32>>,

    pub(crate) parents: Vec<Vec<u32>>,

    pub(crate) cat_articles: Vec<RoaringBitmap>,

    pub(crate) article_cats: Vec<Vec<u32>>,

    // --- Metadata / ID Translation ---
    // To convert results back to Strings/Original IDs for the user
    pub cat_dense_to_original: Vec<u32>, // Dense -> WikiID
    pub cat_original_to_dense: HashMap<u32, u32>, // WikiID -> Dense
    pub cat_names: Vec<String>,          // Dense -> Name

    pub art_dense_to_original: Vec<u32>,
    pub art_original_to_dense: HashMap<u32, u32>,
    pub art_names: Vec<String>,
}

impl WikiGraph {
    /// Find all articles in a category (and optionally subcategories to depth N)
    pub fn get_articles_in_category(&self, wiki_cat_id: u32, max_depth: u8) -> RoaringBitmap {
        // 1. Translate External ID -> Internal Dense ID
        let start_node = match self.cat_original_to_dense.get(&wiki_cat_id) {
            Some(&id) => id,
            None => return RoaringBitmap::new(), // Category not found
        };

        let mut result = RoaringBitmap::new();
        let mut visited = RoaringBitmap::new(); // To handle cycles
        let mut queue = VecDeque::new();

        // Queue stores (node_id, current_depth)
        queue.push_back((start_node, 0));
        visited.insert(start_node);

        while let Some((curr, depth)) = queue.pop_front() {
            // A. Collect articles from this category
            if let Some(articles) = self.cat_articles.get(curr as usize) {
                result |= articles;
            }

            // B. Traverse deeper if allowed
            if depth < max_depth
                && let Some(children) = self.children.get(curr as usize) {
                    for &child in children {
                        if !visited.contains(child) {
                            visited.insert(child);
                            queue.push_back((child, depth + 1));
                        }
                    }
                }
        }
        result
    }

    /// Get immediate subcategories (Depth 1)
    /// Returns a vector of tuples: (Original_Wiki_ID, Category_Name)
    pub fn get_child_categories(&self, wiki_cat_id: u32) -> Vec<(u32, String)> {
        // 1. Convert External ID -> Internal Dense ID
        let dense_id = match self.cat_original_to_dense.get(&wiki_cat_id) {
            Some(&id) => id,
            None => return Vec::new(), // Category not found
        };

        // 2. Lookup children in the Adjacency List
        if let Some(children_dense) = self.children.get(dense_id as usize) {
            // 3. Map back to (WikiID, Name)
            children_dense
                .iter()
                .map(|&child_dense| {
                    let idx = child_dense as usize;
                    (self.cat_dense_to_original[idx], self.cat_names[idx].clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }
    /// Get all subcategories up to a specific depth `n`.
    /// Returns a vector of tuples: (Original_Wiki_ID, Category_Name, Depth)
    pub fn get_descendant_categories(
        &self,
        wiki_cat_id: u32,
        max_depth: u8,
    ) -> Vec<(u32, String, u8)> {
        let start_node = match self.cat_original_to_dense.get(&wiki_cat_id) {
            Some(&id) => id,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        // Use a lightweight bitset for visited check to handle cycles
        let mut visited = RoaringBitmap::new();

        queue.push_back((start_node, 0));
        visited.insert(start_node);

        while let Some((curr, depth)) = queue.pop_front() {
            // If it's not the start node, add it to results
            if curr != start_node {
                let idx = curr as usize;
                results.push((
                    self.cat_dense_to_original[idx],
                    self.cat_names[idx].clone(),
                    depth,
                ));
            }

            // Stop if we reached max depth
            if depth >= max_depth {
                continue;
            }

            // Enqueue children
            if let Some(children) = self.children.get(curr as usize) {
                for &child in children {
                    if !visited.contains(child) {
                        visited.insert(child);
                        queue.push_back((child, depth + 1));
                    }
                }
            }
        }

        results
    }

    /// Find parent categories (Navigate Up)
    pub fn get_parent_categories(&self, wiki_cat_id: u32) -> Vec<u32> {
        let dense_id = match self.cat_original_to_dense.get(&wiki_cat_id) {
            Some(&id) => id,
            None => return Vec::new(),
        };

        if let Some(parents_dense) = self.parents.get(dense_id as usize) {
            // Convert back to Original IDs for the user
            parents_dense
                .iter()
                .map(|&p_dense| self.cat_dense_to_original[p_dense as usize])
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Helper to resolve Article Dense ID -> Name
    pub fn get_article_name(&self, dense_id: u32) -> Option<&String> {
        self.art_names.get(dense_id as usize)
    }

    /// Get all parent categories for a specific article.
    /// Returns a vector of tuples: (Category_Wiki_ID, Category_Name)
    pub fn get_categories_for_article(&self, wiki_article_id: u32) -> Vec<(u32, String)> {
        // 1. Convert Article External ID -> Article Internal Dense ID
        let dense_art_id = match self.art_original_to_dense.get(&wiki_article_id) {
            Some(&id) => id,
            None => return Vec::new(), // Article not found
        };

        // 2. Lookup the list of Category Dense IDs for this article
        if let Some(cat_dense_ids) = self.article_cats.get(dense_art_id as usize) {
            // 3. Map Category Dense IDs back to (WikiID, Name)
            cat_dense_ids
                .iter()
                .map(|&cat_dense| {
                    let idx = cat_dense as usize;
                    (self.cat_dense_to_original[idx], self.cat_names[idx].clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}
