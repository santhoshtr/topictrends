use anyhow::Result;
use polars::prelude::*;
use roaring::RoaringBitmap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::time::Instant;

/// The core high-performance graph structure.
/// All internal logic uses "Dense IDs" (0..N), not the raw Wikipedia Page IDs.
pub struct WikiGraph {
    // --- Graph Topology ---
    // Index = Parent Dense ID. Value = List of Child Dense IDs.
    children: Vec<Vec<u32>>,

    // Index = Child Dense ID. Value = List of Parent Dense IDs.
    parents: Vec<Vec<u32>>,

    // --- Content Mapping ---
    // Index = Category Dense ID. Value = Set of Article Dense IDs.
    cat_articles: Vec<RoaringBitmap>,

    // Index = Article Dense ID. Value = List of Category Dense IDs.
    article_cats: Vec<Vec<u32>>,

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
            if depth < max_depth {
                if let Some(children) = self.children.get(curr as usize) {
                    for &child in children {
                        if !visited.contains(child) {
                            visited.insert(child);
                            queue.push_back((child, depth + 1));
                        }
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

pub struct GraphBuilder;

impl GraphBuilder {
    pub fn build(data_dir: &str) -> Result<WikiGraph> {
        println!("Starting Graph Build...");
        let start = Instant::now();

        // A. Load Categories & Create Mapping
        println!("Loading Categories...");
        let (cat_dense_to_original, cat_names, cat_original_to_dense) =
            Self::load_nodes(format!("{}/categories.parquet", data_dir))?;

        let num_cats = cat_dense_to_original.len();
        println!("Loaded {} categories.", num_cats);

        // B. Load Articles & Create Mapping
        println!("Loading Articles...");
        let (art_dense_to_original, art_names, art_original_to_dense) =
            Self::load_nodes(format!("{}/articles.parquet", data_dir))?;

        let num_arts: usize = art_dense_to_original.len();
        println!("Loaded {} articles.", num_arts);

        // C. Initialize Structure Containers
        let mut children = vec![Vec::new(); num_cats];
        let mut parents = vec![Vec::new(); num_cats];
        let mut cat_articles = vec![RoaringBitmap::new(); num_cats];
        let mut article_cats = vec![Vec::new(); num_arts];

        // D. Load Relations: Category Parent -> Child
        // Note: User provided 'cat_parents.parquet' (parent, child)
        println!("Loading Category Hierarchy...");
        let path: PlPath = PlPath::Local(Arc::from(Path::new(
            format!("{}/cat_parents.parquet", data_dir).as_str(),
        )));
        let df_rel: DataFrame = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

        let p_col = df_rel.column("parent")?.u32()?;
        let c_col = df_rel.column("child")?.u32()?;

        // Iterate and populate adjacency lists
        // We use the HashMaps to convert Raw ID -> Dense ID on the fly
        for (opt_p, opt_c) in p_col.into_iter().zip(c_col.into_iter()) {
            if let (Some(p_raw), Some(c_raw)) = (opt_p, opt_c) {
                if let (Some(&p_dense), Some(&c_dense)) = (
                    cat_original_to_dense.get(&p_raw),
                    cat_original_to_dense.get(&c_raw),
                ) {
                    children[p_dense as usize].push(c_dense);
                    // If 'cat_children.parquet' didn't exist, we could populate 'parents' here too:
                    // parents[c_dense as usize].push(p_dense);
                }
            }
        }

        // E. Load Relations: Category Child -> Parent (Reverse Graph)
        // Note: User provided 'cat_children.parquet' (child, parent)
        println!("Loading Reverse Hierarchy...");
        let path: PlPath = PlPath::Local(Arc::from(Path::new(
            format!("{}/cat_children.parquet", data_dir).as_str(),
        )));
        let df_rev: DataFrame = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

        let c_col_rev: &ChunkedArray<UInt32Type> = df_rev.column("child")?.u32()?;
        let p_col_rev: &ChunkedArray<UInt32Type> = df_rev.column("parent")?.u32()?;

        for (opt_c, opt_p) in c_col_rev.into_iter().zip(p_col_rev.into_iter()) {
            if let (Some(c_raw), Some(p_raw)) = (opt_c, opt_p) {
                if let (Some(&c_dense), Some(&p_dense)) = (
                    cat_original_to_dense.get(&c_raw),
                    cat_original_to_dense.get(&p_raw),
                ) {
                    parents[c_dense as usize].push(p_dense);
                }
            }
        }

        // F. Load Article -> Category
        println!("Loading Article-Category definitions...");
        let path: PlPath = PlPath::Local(Arc::from(Path::new(
            format!("{}/article_category.parquet", data_dir).as_str(),
        )));
        let df_art_cat = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

        let a_col = df_art_cat.column("article_id")?.u32()?;
        let c_col_ac = df_art_cat.column("category_id")?.u32()?;

        for (opt_a, opt_c) in a_col.into_iter().zip(c_col_ac.into_iter()) {
            if let (Some(a_raw), Some(c_raw)) = (opt_a, opt_c) {
                if let (Some(&a_dense), Some(&c_dense)) = (
                    art_original_to_dense.get(&a_raw),
                    cat_original_to_dense.get(&c_raw),
                ) {
                    // Populate RoaringBitmap for Category
                    cat_articles[c_dense as usize].insert(a_dense);

                    // Populate Article metadata
                    article_cats[a_dense as usize].push(c_dense);
                }
            }
        }

        println!("Graph build completed in {:.2?}s", start.elapsed());

        Ok(WikiGraph {
            children,
            parents,
            cat_articles,
            article_cats,
            cat_dense_to_original,
            cat_original_to_dense,
            cat_names,
            art_dense_to_original,
            art_original_to_dense,
            art_names,
        })
    }

    // Helper to load node definitions and create ID mappings
    fn load_nodes(path: String) -> Result<(Vec<u32>, Vec<String>, HashMap<u32, u32>)> {
        let path: PlPath = PlPath::Local(Arc::from(Path::new(&path)));
        let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

        let ids = df.column("page_id")?.u32()?;
        let titles = df.column("page_title")?.str()?;

        let mut dense_to_original = Vec::with_capacity(ids.len());
        let mut names = Vec::with_capacity(ids.len());
        let mut original_to_dense = HashMap::with_capacity(ids.len());

        let mut dense_counter = 0;

        for (opt_id, opt_title) in ids.into_iter().zip(titles.into_iter()) {
            if let (Some(id), Some(title)) = (opt_id, opt_title) {
                dense_to_original.push(id);
                names.push(title.to_string());
                original_to_dense.insert(id, dense_counter);
                dense_counter += 1;
            }
        }

        Ok((dense_to_original, names, original_to_dense))
    }
}
