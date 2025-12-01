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

        println!("Starting Graph Build for {}...", self.wiki);
        let start = Instant::now();

        // A. Load Categories & Create Mapping
        print!("  Loading Categories...");
        let (cat_dense_to_original, cat_original_to_dense) =
            Self::load_nodes(format!("{}/{}/categories.parquet", data_dir, self.wiki))?;

        let num_cats = cat_dense_to_original.len();
        println!("\r  Loaded {} categories.", num_cats);

        // B. Load Articles & Create Mapping
        print!("  Loading Articles...");
        let (art_dense_to_original, art_original_to_dense) =
            Self::load_nodes(format!("{}/{}/articles.parquet", data_dir, self.wiki))?;

        let num_arts: usize = art_dense_to_original.len();
        println!("\r  Loaded {} articles.", num_arts);

        // C. Initialize Structure Containers
        let mut cat_articles = vec![RoaringBitmap::new(); num_cats];
        let article_cats = vec![Vec::new(); num_arts];

        // D. Load Relations: Category Parent -> Child
        // Note: User provided 'cat_parents.parquet' (parent, child)
        print!("  Loading Category Hierarchy...");
        let path: PlPath = PlPath::Local(Arc::from(Path::new(
            format!("{}/{}/category_graph.parquet", data_dir, self.wiki).as_str(),
        )));
        let df_rel: DataFrame = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

        let p_col = df_rel.column("parent")?.u32()?;
        let c_col = df_rel.column("child")?.u32()?;
        //  Create a temporary vector of pairs (Parent_Dense -> Child_Dense)
        // We estimate capacity to avoid reallocations
        let mut forward_edges: Vec<(u32, u32)> = Vec::with_capacity(p_col.len());
        let mut backward_edges: Vec<(u32, u32)> = Vec::with_capacity(p_col.len());
        // Iterate and populate adjacency lists
        // We use the HashMaps to convert Raw ID -> Dense ID on the fly
        for (opt_p, opt_c) in p_col.into_iter().zip(c_col.into_iter()) {
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
        let path: PlPath = PlPath::Local(Arc::from(Path::new(
            format!("{}/{}/article_category.parquet", data_dir, self.wiki).as_str(),
        )));

        let df_art_cat = LazyFrame::scan_parquet(path, Default::default())?
            .select([col("article_id"), col("category_id")])
            .with_new_streaming(true)
            .collect()?;

        let a_col = df_art_cat.column("article_id")?.u32()?;
        let c_col_ac = df_art_cat.column("category_id")?.u32()?;

        for (opt_a, opt_c) in a_col.into_iter().zip(c_col_ac.into_iter()) {
            if let (Some(a_raw), Some(c_raw)) = (opt_a, opt_c)
                && let (Some(a_dense), Some(c_dense)) = (
                    art_original_to_dense.get(a_raw),
                    cat_original_to_dense.get(c_raw),
                )
            {
                // Populate RoaringBitmap for Category
                cat_articles[c_dense as usize].insert(a_dense);

                // Populate Article metadata
                // FIXME
                // article_cats[a_dense as usize].push(c_dense);
            }
        }

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
            cat_dense_to_original,
            cat_original_to_dense,
            art_dense_to_original,
            art_original_to_dense,
        })
    }

    // Helper to load node definitions and create ID mappings
    fn load_nodes(path: String) -> Result<(Vec<u32>, DirectMap)> {
        let path: PlPath = PlPath::Local(Arc::from(Path::new(&path)));
        let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;
        let ids = df.column("page_id")?.u32()?;

        let max_length = ids.len();
        let mut dense_to_original = Vec::with_capacity(ids.len());
        let mut mapper = DirectMap::new(max_length as usize);

        let mut dense_counter = 0;

        for opt_id in ids.into_iter() {
            if let Some(id) = opt_id {
                dense_to_original.push(id);
                //                names.push(title.to_string());
                mapper.insert(id, dense_counter);
                dense_counter += 1;
            }
        }

        Ok((dense_to_original, mapper))
    }
}
