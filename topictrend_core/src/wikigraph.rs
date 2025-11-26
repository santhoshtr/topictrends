use roaring::RoaringBitmap;
use std::collections::HashMap;
use std::collections::VecDeque;
/// The core high-performance graph structure.
/// All internal logic uses "Dense IDs" (0..N), not the raw Wikipedia Page IDs.
#[derive(Clone, Debug)]
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
    pub fn get_articles_in_category(
        &self,
        category_title: &String,
        max_depth: u8,
    ) -> Result<RoaringBitmap, String> {
        let category_id = self.get_category_id(category_title)?;
        // Translate External ID -> Internal Dense ID
        let start_node = match self.cat_original_to_dense.get(&category_id) {
            Some(&id) => id,
            None => {
                return Ok(RoaringBitmap::new());
            } // Dense ID not found
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
                && let Some(children) = self.children.get(curr as usize)
            {
                for &child in children {
                    if !visited.contains(child) {
                        visited.insert(child);
                        queue.push_back((child, depth + 1));
                    }
                }
            }
        }
        Ok(result)
    }

    /// Get immediate subcategories (Depth 1)
    /// Returns a vector of tuples: (Original_Wiki_ID, Category_Name)
    pub fn get_child_categories(
        &self,
        category_title: &String,
    ) -> Result<Vec<(u32, String)>, String> {
        let category_id = self.get_category_id(category_title)?;

        // 1. Convert External ID -> Internal Dense ID
        let dense_id = match self.cat_original_to_dense.get(&category_id) {
            Some(&id) => id,
            None => return Ok(Vec::new()), // Category not found
        };

        // 2. Lookup children in the Adjacency List
        if let Some(children_dense) = self.children.get(dense_id as usize) {
            // 3. Map back to (WikiID, Name)
            Ok(children_dense
                .iter()
                .map(|&child_dense| {
                    let idx = child_dense as usize;
                    (self.cat_dense_to_original[idx], self.cat_names[idx].clone())
                })
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
    /// Get all subcategories up to a specific depth `n`.
    /// Returns a vector of tuples: (Original_Wiki_ID, Category_Name, Depth)
    pub fn get_descendant_categories(
        &self,
        category_title: &String,
        max_depth: u8,
    ) -> Result<Vec<(u32, String, u8)>, String> {
        let category_id = self.get_category_id(category_title)?;

        let start_node = match self.cat_original_to_dense.get(&category_id) {
            Some(&id) => id,
            None => return Ok(Vec::new()),
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

        Ok(results)
    }

    /// Find parent categories (Navigate Up)
    pub fn get_parent_categories(&self, category_title: &String) -> Result<Vec<u32>, String> {
        let wiki_cat_id = self.get_category_id(category_title)?;
        let dense_id = match self.cat_original_to_dense.get(&wiki_cat_id) {
            Some(&id) => id,
            None => return Ok(Vec::new()),
        };

        if let Some(parents_dense) = self.parents.get(dense_id as usize) {
            // Convert back to Original IDs for the user
            Ok(parents_dense
                .iter()
                .map(|&p_dense| self.cat_dense_to_original[p_dense as usize])
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Helper to resolve Article Dense ID -> Name
    pub fn get_article_name(&self, dense_id: u32) -> Option<&String> {
        self.art_names.get(dense_id as usize)
    }

    /// Helper to get category_id for the given category_title
    pub fn get_category_id(&self, category_title: &String) -> Result<u32, String> {
        // Normalize both sides when searching
        match self
            .cat_names
            .iter()
            .position(|name| name == category_title)
        {
            Some(dense_id) => Ok(self.cat_dense_to_original[dense_id]),
            None => Err(format!("Category title '{}' not found", category_title)),
        }
    }

    /// Helper to get article_id for the given article_title
    pub fn get_article_id(&self, article_title: &String) -> Result<u32, String> {
        match self.art_names.iter().position(|name| name == article_title) {
            Some(dense_id) => Ok(self.art_dense_to_original[dense_id]),
            None => Err(format!("Article title '{}' not found", article_title)),
        }
    }

    /// Get all parent categories for a specific article.
    /// Returns a vector of tuples: (Category_Wiki_ID, Category_Name)
    pub fn get_categories_for_article(
        &self,
        wiki_article_id: u32,
    ) -> Result<Vec<(u32, String)>, String> {
        // 1. Convert Article External ID -> Article Internal Dense ID
        let dense_art_id = match self.art_original_to_dense.get(&wiki_article_id) {
            Some(&id) => id,
            None => return Ok(Vec::new()), // Article not found
        };

        // 2. Lookup the list of Category Dense IDs for this article
        if let Some(cat_dense_ids) = self.article_cats.get(dense_art_id as usize) {
            // 3. Map Category Dense IDs back to (WikiID, Name)
            Ok(cat_dense_ids
                .iter()
                .map(|&cat_dense| {
                    let idx = cat_dense as usize;
                    (self.cat_dense_to_original[idx], self.cat_names[idx].clone())
                })
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
}
