use anyhow::Result;
use polars::prelude::*;
use roaring::RoaringBitmap;
use std::path::Path;
use std::time::Instant;

use crate::csr_adjacency::CsrAdjacency;
use crate::direct_map::DirectMap;
use crate::wikigraph::WikiGraph;

pub struct GraphBuilder {
    pub wiki: String,
}

impl GraphBuilder {
    pub fn new(wiki: &str) -> Self {
        Self {
            wiki: wiki.to_string(),
        }
    }

    pub fn build(&self) -> Result<WikiGraph> {
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());

        // TOPICTREND_TOPOLOGY=canonical switches the article->category relation
        // (and the category node universe) to the cross-wiki canonical
        // projection produced by `make canonical`. Articles and the category
        // hierarchy stay local in both modes.
        let canonical =
            std::env::var("TOPICTREND_TOPOLOGY").is_ok_and(|v| v == "canonical");

        println!(
            "Starting Graph Build for {} ({} topology)...",
            self.wiki,
            if canonical { "canonical" } else { "local" }
        );
        let start = Instant::now();

        // A. Load Categories & Create Mapping
        print!("  Loading Categories...");
        let categories_file = if canonical {
            "categories_canonical.parquet"
        } else {
            "categories.parquet"
        };
        let (cat_dense_to_original, cat_original_to_dense) =
            Self::load_nodes(format!("{}/{}/{}", data_dir, self.wiki, categories_file))?;

        let num_cats = cat_dense_to_original.len();
        println!("\r  Loaded {} categories.", num_cats);

        // B. Load Articles & Create Mapping
        print!("  Loading Articles...");
        let (art_dense_to_original, art_original_to_dense) =
            Self::load_nodes(format!("{}/{}/articles.parquet", data_dir, self.wiki))?;

        let num_arts: usize = art_dense_to_original.len();
        println!("\r  Loaded {} articles.", num_arts);

        // D. Load Relations: Category Parent -> Child
        // Note: User provided 'cat_parents.parquet' (parent, child)
        print!("  Loading Category Hierarchy...");
        let path: PlRefPath = PlRefPath::try_from_path(Path::new(
            format!("{}/{}/category_graph.parquet", data_dir, self.wiki).as_str(),
        ))?;
        let df_rel: DataFrame = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

        let p_col = df_rel.column("parent_qid")?.u32()?;
        let c_col = df_rel.column("child_qid")?.u32()?;
        //  Create a temporary vector of pairs (Parent_Dense -> Child_Dense)
        // We estimate capacity to avoid reallocations
        let mut forward_edges: Vec<(u32, u32)> = Vec::with_capacity(p_col.len());
        let mut backward_edges: Vec<(u32, u32)> = Vec::with_capacity(p_col.len());
        // Iterate and populate adjacency lists
        // We use the HashMaps to convert Raw ID -> Dense ID on the fly
        for (opt_p, opt_c) in p_col.into_iter().zip(c_col) {
            if let (Some(p_raw), Some(c_raw)) = (opt_p, opt_c)
                && let (Some(p_dense), Some(c_dense)) = (
                    cat_original_to_dense.get(p_raw),
                    cat_original_to_dense.get(c_raw),
                )
            {
                // Forward: Parent -> Child
                forward_edges.push((p_dense, c_dense));
                // Backward: Child -> Parent (for the parents CSR)
                backward_edges.push((c_dense, p_dense));
            }
        }
        // Build the optimized CSR Structures
        // This moves the data into the compact format and drops the temp vectors
        let children = CsrAdjacency::from_pairs(num_cats, &forward_edges);
        let parents = CsrAdjacency::from_pairs(num_cats, &backward_edges);
        // Drop temp vectors explicitly (optional, Rust does this automatically)
        drop(forward_edges);
        drop(backward_edges);

        println!("\r  Loaded Category Hierarchy");
        // Load Article -> Category
        print!("  Loading Article-Category definitions...");
        let relation_file = if canonical {
            "article_category_canonical.parquet"
        } else {
            "article_category.parquet"
        };
        let path: PlRefPath = PlRefPath::try_from_path(Path::new(
            format!("{}/{}/{}", data_dir, self.wiki, relation_file).as_str(),
        ))?;
        let mut cat_articles = vec![RoaringBitmap::new(); num_cats];
        let mut article_cats_vec: Vec<(u32, u32)> = Vec::with_capacity(num_arts);
        let mut weights_vec: Vec<u16> = Vec::new();

        let df_art_cat = LazyFrame::scan_parquet(path, Default::default())?.collect()?;
        let a_col = df_art_cat.column("article_qid")?.u32()?;
        let c_col_ac = df_art_cat.column("category_qid")?.u32()?;
        // The canonical relation carries a per-edge cross-wiki agreement
        // count; collected parallel to the row order so filtering below keeps
        // pairs and weights aligned.
        let w_by_row: Option<Vec<u16>> = if canonical {
            Some(
                df_art_cat
                    .column("wiki_count")?
                    .u32()?
                    .into_iter()
                    .map(|w| w.unwrap_or(1).min(u16::MAX as u32) as u16)
                    .collect(),
            )
        } else {
            None
        };

        for (row, (opt_a, opt_c)) in a_col.into_iter().zip(c_col_ac).enumerate() {
            if let (Some(a_raw), Some(c_raw)) = (opt_a, opt_c)
                && let (Some(a_dense), Some(c_dense)) = (
                    art_original_to_dense.get(a_raw),
                    cat_original_to_dense.get(c_raw),
                )
            {
                // Populate RoaringBitmap for Category
                cat_articles[c_dense as usize].insert(a_dense);
                article_cats_vec.push((a_dense, c_dense));
                if let Some(w) = &w_by_row {
                    weights_vec.push(w[row]);
                }
            }
        }
        let (article_cats, article_cat_weights) = if canonical {
            CsrAdjacency::from_pairs_with_weights(num_arts, &article_cats_vec, &weights_vec)
        } else {
            (
                CsrAdjacency::from_pairs(num_arts, &article_cats_vec),
                Vec::new(),
            )
        };

        println!("\r  Loaded Article-Category definitions");
        println!(
            "Graph build completed for {} in {:.2?}s",
            self.wiki,
            start.elapsed()
        );

        Ok(WikiGraph {
            children,
            parents,
            cat_articles,
            article_cats,
            article_cat_weights,
            cat_dense_to_original,
            cat_original_to_dense,
            art_dense_to_original,
            art_original_to_dense,
        })
    }

    // Helper to load node definitions and create ID mappings
    fn load_nodes(path: String) -> Result<(Vec<u32>, DirectMap)> {
        let path: PlRefPath = PlRefPath::try_from_path(Path::new(&path))?;
        let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;
        let ids = df.column("qid")?.u32()?;

        let max_length = ids.len();
        let mut dense_to_original = Vec::with_capacity(ids.len());
        let mut mapper = DirectMap::new(max_length);

        let mut dense_counter = 0;

        for id in ids.into_iter().flatten() {
            dense_to_original.push(id);
            mapper.insert(id, dense_counter);
            dense_counter += 1;
        }

        Ok((dense_to_original, mapper))
    }
}
