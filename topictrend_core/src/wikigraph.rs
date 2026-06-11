use crate::{csr_adjacency::CsrAdjacency, direct_map::DirectMap};
use roaring::RoaringBitmap;
use std::collections::{HashMap, VecDeque};

/// The core high-performance graph structure.
/// All internal logic uses "Dense IDs" (0..N), not the raw Wikipedia Page QIDs.
#[derive(Debug)]
pub struct WikiGraph {
    pub children: CsrAdjacency,
    pub parents: CsrAdjacency,
    pub cat_articles: Vec<RoaringBitmap>,
    pub article_cats: CsrAdjacency,
    /// Per-edge cross-wiki agreement counts, parallel to `article_cats`'
    /// flattened edge array (slice via `article_cats.edge_range`). Empty when
    /// the graph was built from the local relation, which carries no weights.
    pub article_cat_weights: Vec<u16>,
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
        max_depth: u32,
    ) -> Result<Vec<u32>, String> {
        let articles_dense = self
            .get_articles_in_category_as_dense(category_qid, max_depth)
            .unwrap();
        // Map back to QID
        Ok(articles_dense
            .iter()
            .map(|dense_id| self.art_dense_to_original[dense_id as usize])
            .collect())
    }

    pub fn get_articles_in_category_as_dense(
        &self,
        category_qid: u32,
        max_depth: u32,
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
                let children = self.children.get(curr);
                for &child in children {
                    if !visited.contains(child) {
                        visited.insert(child);
                        queue.push_back((child, depth + 1));
                    }
                }
            }
        }
        Ok(articles_dense)
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

    /// Categories for an article ranked by cross-wiki agreement, highest
    /// first (ties broken by category QID for stable output). Returns
    /// `(category_qid, weight)` pairs. Weight is 1 for every edge when the
    /// graph was built from the local relation (no weights loaded).
    pub fn get_categories_for_article_ranked(&self, article_qid: u32) -> Vec<(u32, u16)> {
        let dense = match self.art_original_to_dense.get(article_qid) {
            Some(id) => id,
            None => return Vec::new(),
        };

        let cats = self.article_cats.get(dense);
        let range = self.article_cats.edge_range(dense);
        let mut ranked: Vec<(u32, u16)> = cats
            .iter()
            .enumerate()
            .map(|(i, &cat_dense)| {
                let weight = if self.article_cat_weights.is_empty() {
                    1
                } else {
                    self.article_cat_weights[range.start + i]
                };
                (self.cat_dense_to_original[cat_dense as usize], weight)
            })
            .collect();
        ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        ranked
    }

    /// Find articles most related to `article_qid` by shared-category overlap.
    ///
    /// Reverse scatter: for each category the input article belongs to, tally
    /// every other article in that category. Each article's tally is the number
    /// of categories it shares with the input. Returns up to `top`
    /// `(article_qid, shared_category_count)` pairs, highest overlap first
    /// (ties broken by dense id for stable output), excluding the input article.
    /// Empty if the article is not in the graph.
    pub fn related_by_categories(&self, article_qid: u32, top: usize) -> Vec<(u32, u32)> {
        let input_dense = match self.art_original_to_dense.get(article_qid) {
            Some(id) => id,
            None => return Vec::new(),
        };

        let mut counts: HashMap<u32, u32> = HashMap::new();
        for &cat_dense in self.article_cats.get(input_dense) {
            if let Some(articles) = self.cat_articles.get(cat_dense as usize) {
                for art_dense in articles {
                    if art_dense != input_dense {
                        *counts.entry(art_dense).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut ranked: Vec<(u32, u32)> = counts.into_iter().collect();
        ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        ranked.truncate(top);

        ranked
            .into_iter()
            .map(|(dense, score)| (self.art_dense_to_original[dense as usize], score))
            .collect()
    }

    /// Calculates the depth of every category starting from a specific Root.
    /// Returns:
    /// 1. Max Depth found
    /// 2. Average Depth
    /// 3. Histogram: Map<Depth, Count_of_Categories>
    /// 4. Unreachable Count (Islands)
    pub fn analyze_depth_from_root(&self, root_qid: u32) -> (u32, f64, HashMap<u32, u32>, u32) {
        println!("Analyzing graph depth starting from '{}'...", root_qid);
        // 1. Find Root Dense ID
        // Note: In some dumps the namespace "Category:" is part of the title, in others it is not.
        // Adjust the string lookup based on your specific dump format.
        let root_id = self.cat_original_to_dense.get(root_qid);

        let root_id = match root_id {
            Some(id) => id,
            None => {
                println!("Error: Root category '{}' not found!", root_qid);
                return (0, 0.0, HashMap::new(), 0);
            }
        };

        //  BFS State
        let num_cats = self.cat_dense_to_original.len();

        // Store depth for every node. u32::MAX represents "Unvisited/Unreachable".
        let mut depths = vec![u32::MAX; num_cats];
        let mut queue = VecDeque::new();

        // 3. Initialize BFS
        depths[root_id as usize] = 0;
        queue.push_back(root_id);

        let mut max_depth = 0;
        let mut visited_count = 0;
        let mut total_depth_sum: u64 = 0;

        // 4. Run BFS
        while let Some(curr) = queue.pop_front() {
            let curr_depth = depths[curr as usize];

            if curr_depth > max_depth {
                max_depth = curr_depth;
            }

            visited_count += 1;
            total_depth_sum += curr_depth as u64;

            // Iterate Children (using CSR)
            let children = self.children.get(curr);

            for &child in children {
                // Only visit if we haven't found a shorter path to this node yet
                if depths[child as usize] == u32::MAX {
                    depths[child as usize] = curr_depth + 1;
                    queue.push_back(child);
                }
            }
        }

        // 5. Build Histogram
        let mut histogram = HashMap::new();
        for &d in &depths {
            if d != u32::MAX {
                *histogram.entry(d).or_insert(0) += 1;
            }
        }

        let avg_depth = if visited_count > 0 {
            total_depth_sum as f64 / visited_count as f64
        } else {
            0.0
        };

        let unreachable = num_cats as u32 - visited_count;

        (max_depth, avg_depth, histogram, unreachable)
    }
}
