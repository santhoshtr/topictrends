use crate::{csr_adjacency::CsrAdjacency, direct_map::DirectMap};
use roaring::RoaringBitmap;
use std::collections::{HashMap, VecDeque};

/// One topic produced by [`WikiGraph::cluster_articles`]: a category and the
/// input articles assigned to it. `size == article_qids.len()`.
#[derive(Debug)]
pub struct ArticleCluster {
    pub category_qid: u32,
    pub article_qids: Vec<u32>,
    pub size: u32,
}

/// Result of [`WikiGraph::cluster_articles`].
#[derive(Debug)]
pub struct ClusterOutcome {
    pub clusters: Vec<ArticleCluster>,
    /// Input articles placed in no cluster: QIDs absent from this wiki's graph,
    /// articles whose only categories are non-local, and (when `max_clusters`
    /// caps the picks) any left uncovered.
    pub unclustered_qids: Vec<u32>,
}

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
    /// Category dense IDs that exist as a local page in this wiki (i.e. have a
    /// local label). Ranking surfaces restrict their candidate set to these so
    /// "top categories in <wiki>" never lists a category that only exists in
    /// another edition. Under the local topology the node universe already is
    /// the local categories, so every node is set.
    pub local_categories: RoaringBitmap,
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

    /// Union of several categories' article sets, as dense IDs. An article
    /// filed under more than one of the input categories appears once, so
    /// aggregations over the result never double-count. Categories absent
    /// from this wiki's graph contribute nothing — callers pass QID lists
    /// from cross-wiki sources (e.g. topic search) where partial coverage is
    /// normal.
    pub fn get_articles_in_categories_as_dense(
        &self,
        category_qids: &[u32],
        max_depth: u32,
    ) -> RoaringBitmap {
        let mut union = RoaringBitmap::new();
        for &qid in category_qids {
            if let Ok(mask) = self.get_articles_in_category_as_dense(qid, max_depth) {
                union |= mask;
            }
        }
        union
    }

    /// Number of articles filed directly under `category_qid` in this wiki's
    /// (canonical) relation. `None` if the category is absent from the graph.
    pub fn category_member_count(&self, category_qid: u32) -> Option<u64> {
        let dense = self.cat_original_to_dense.get(category_qid)?;
        Some(self.cat_articles.get(dense as usize).map_or(0, |b| b.len()))
    }

    /// Total number of articles in this wiki's graph.
    pub fn article_count(&self) -> usize {
        self.art_dense_to_original.len()
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

    /// Articles in a category, keeping only direct members whose membership
    /// edge has at least `min_agreement` wikis asserting it. Articles reached
    /// through depth>0 subcategory traversal are independent membership paths
    /// and pass through unfiltered. Graphs built from the local relation carry
    /// no weights — every edge counts as 1 — so `min_agreement > 1` drops all
    /// direct members there.
    pub fn get_articles_in_category_filtered(
        &self,
        category_qid: u32,
        max_depth: u32,
        min_agreement: u16,
    ) -> Result<Vec<u32>, String> {
        if min_agreement <= 1 {
            return self.get_articles_in_category(category_qid, max_depth);
        }
        let start_node = match self.cat_original_to_dense.get(category_qid) {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        // BFS as in get_articles_in_category_as_dense, but keep the start
        // node's direct members separate from the subtree's.
        let mut subtree = RoaringBitmap::new();
        let mut visited = RoaringBitmap::new();
        let mut queue = VecDeque::new();
        queue.push_back((start_node, 0u32));
        visited.insert(start_node);
        while let Some((curr, depth)) = queue.pop_front() {
            if curr != start_node
                && let Some(articles) = self.cat_articles.get(curr as usize)
            {
                subtree |= articles;
            }
            if depth < max_depth {
                for &child in self.children.get(curr) {
                    if !visited.contains(child) {
                        visited.insert(child);
                        queue.push_back((child, depth + 1));
                    }
                }
            }
        }

        let mut result = subtree;
        if let Some(direct) = self.cat_articles.get(start_node as usize) {
            for art_dense in direct {
                if self.article_cat_weight(art_dense, start_node) >= min_agreement {
                    result.insert(art_dense);
                }
            }
        }

        Ok(result
            .iter()
            .map(|dense| self.art_dense_to_original[dense as usize])
            .collect())
    }

    /// Cross-wiki agreement count of the (article, category) edge, in dense
    /// IDs. 1 when the graph carries no weights or the edge does not exist.
    fn article_cat_weight(&self, art_dense: u32, cat_dense: u32) -> u16 {
        if self.article_cat_weights.is_empty() {
            return 1;
        }
        let range = self.article_cats.edge_range(art_dense);
        for (i, &c) in self.article_cats.get(art_dense).iter().enumerate() {
            if c == cat_dense {
                return self.article_cat_weights[range.start + i];
            }
        }
        1
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

    /// Group `article_qids` into category-topics by the same reverse-scatter +
    /// greedy-coverage method the trending top-categories ranking uses, but with
    /// each article weighted equally (1) rather than by pageviews. Each article
    /// is assigned to at most one cluster — its single broadest-coverage topic —
    /// so near-duplicate categories collapse to one representative. `max_clusters`
    /// bounds the number of clusters returned; `None` keeps selecting until every
    /// coverable article is assigned.
    ///
    /// Candidate categories are restricted to `local_categories`, so every
    /// returned category has a local page (hence a resolvable label), matching
    /// the trending surfaces. Summed cross-wiki agreement breaks ties between
    /// equally-covering categories, preferring a broad canonical category over a
    /// wiki-local maintenance bucket. Runs entirely on the in-memory graph — no
    /// Parquet, no metric data.
    pub fn cluster_articles(
        &self,
        article_qids: &[u32],
        max_clusters: Option<usize>,
    ) -> ClusterOutcome {
        let has_weights = !self.article_cat_weights.is_empty();

        // Dedupe input QIDs, preserving first-seen order for stable output.
        let mut seen = RoaringBitmap::new();
        let mut input: Vec<u32> = Vec::with_capacity(article_qids.len());
        for &qid in article_qids {
            if seen.insert(qid) {
                input.push(qid);
            }
        }

        // Scatter: per local category, collect the input articles filed under it
        // (dense IDs) and the summed cross-wiki agreement of those membership
        // edges. Sparse by construction — only categories the input touches
        // appear — so a HashMap, not a dense per-category vector over the whole
        // wiki as the metric-wide trending path uses.
        let mut cats: HashMap<u32, (u64, Vec<u32>)> = HashMap::new();
        for &qid in &input {
            let Some(art_dense) = self.art_original_to_dense.get(qid) else {
                continue;
            };
            let edges = self.article_cats.get(art_dense);
            let edge_start = self.article_cats.edge_range(art_dense).start;
            for (i, &cat_dense) in edges.iter().enumerate() {
                if !self.local_categories.contains(cat_dense) {
                    continue;
                }
                let weight = if has_weights {
                    self.article_cat_weights[edge_start + i] as u64
                } else {
                    1
                };
                let entry = cats.entry(cat_dense).or_insert_with(|| (0, Vec::new()));
                entry.0 += weight;
                entry.1.push(art_dense);
            }
        }

        // Candidate pool sorted by raw coverage (article count) descending, ties
        // by QID for determinism. Pool order is the final tie-break in selection,
        // so a broader category wins among true duplicates.
        let mut pool: Vec<(u32, u64, Vec<u32>)> = cats
            .into_iter()
            .map(|(cat_dense, (weight_sum, arts))| (cat_dense, weight_sum, arts))
            .collect();
        pool.sort_by(|a, b| b.2.len().cmp(&a.2.len()).then(a.0.cmp(&b.0)));

        // Greedy maximum coverage: repeatedly take the category explaining the
        // most not-yet-assigned articles; among those covering equally many,
        // prefer higher mean cross-wiki agreement, then pool order. (The
        // trending path fuzzes "equal" to within 95% because it ranks by noisy
        // pageview magnitudes; with unit-weight article counts, a tie is exact
        // equality — anything looser lets a narrower, higher-agreement category
        // outrank a broader one and emit empty clusters.)
        let max_picks = max_clusters.unwrap_or(pool.len()).min(pool.len());
        let mut covered = RoaringBitmap::new();
        let mut taken = vec![false; pool.len()];
        let mut clusters = Vec::with_capacity(max_picks);

        for _ in 0..max_picks {
            let mut marginals = vec![0u64; pool.len()];
            let mut best_marginal = 0u64;
            for (pi, (_, _, arts)) in pool.iter().enumerate() {
                if taken[pi] {
                    continue;
                }
                let m = arts.iter().filter(|&&a| !covered.contains(a)).count() as u64;
                marginals[pi] = m;
                best_marginal = best_marginal.max(m);
            }
            if best_marginal == 0 {
                break;
            }

            let mut best_pi = usize::MAX;
            let mut best_key = (0u64, 0u64);
            for (pi, (_, weight_sum, arts)) in pool.iter().enumerate() {
                if taken[pi] || marginals[pi] < best_marginal {
                    continue;
                }
                let count = arts.len().max(1) as u64;
                let key = (*weight_sum / count, marginals[pi]);
                if best_pi == usize::MAX || key > best_key {
                    best_key = key;
                    best_pi = pi;
                }
            }

            taken[best_pi] = true;
            let (cat_dense, _, arts) = &pool[best_pi];
            let newly: Vec<u32> = arts
                .iter()
                .copied()
                .filter(|&a| !covered.contains(a))
                .collect();
            for &a in &newly {
                covered.insert(a);
            }
            clusters.push(ArticleCluster {
                category_qid: self.cat_dense_to_original[*cat_dense as usize],
                article_qids: newly
                    .iter()
                    .map(|&d| self.art_dense_to_original[d as usize])
                    .collect(),
                size: newly.len() as u32,
            });
        }

        // Anything left uncovered: absent from the graph, only non-local
        // categories, or left over when max_clusters capped the picks.
        let unclustered_qids: Vec<u32> = input
            .into_iter()
            .filter(|&qid| match self.art_original_to_dense.get(qid) {
                Some(dense) => !covered.contains(dense),
                None => true,
            })
            .collect();

        ClusterOutcome {
            clusters,
            unclustered_qids,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csr_adjacency::CsrAdjacency;

    /// Two categories C0 (QID 100) and C1 (QID 101, child of C0); three
    /// articles A0..A2 (QIDs 10..12). Direct members of C0: A0 (weight 3),
    /// A1 (weight 1). Member of C1: A1 (weight 5). A2 is uncategorized.
    fn weighted_graph() -> WikiGraph {
        let children = CsrAdjacency::from_pairs(2, &[(0, 1)]);
        let parents = CsrAdjacency::from_pairs(2, &[(1, 0)]);
        let mut c0 = RoaringBitmap::new();
        c0.insert(0);
        c0.insert(1);
        let mut c1 = RoaringBitmap::new();
        c1.insert(1);
        let (article_cats, article_cat_weights) =
            CsrAdjacency::from_pairs_with_weights(3, &[(0, 0), (1, 0), (1, 1)], &[3, 1, 5]);
        WikiGraph {
            children,
            parents,
            cat_articles: vec![c0, c1],
            article_cats,
            article_cat_weights,
            cat_dense_to_original: vec![100, 101],
            cat_original_to_dense: [(100, 0), (101, 1)].into_iter().collect(),
            art_dense_to_original: vec![10, 11, 12],
            art_original_to_dense: [(10, 0), (11, 1), (12, 2)].into_iter().collect(),
            local_categories: [0u32, 1].into_iter().collect(),
        }
    }

    #[test]
    fn filtered_membership_drops_weak_direct_edges() {
        let g = weighted_graph();
        // depth 0: A1's direct edge to C0 has weight 1 < 2 -> dropped.
        let mut got = g.get_articles_in_category_filtered(100, 0, 2).unwrap();
        got.sort_unstable();
        assert_eq!(got, vec![10]);
        // min_agreement=1 keeps everything (delegates to unfiltered path).
        let mut got = g.get_articles_in_category_filtered(100, 0, 1).unwrap();
        got.sort_unstable();
        assert_eq!(got, vec![10, 11]);
    }

    #[test]
    fn filtered_membership_keeps_subtree_paths() {
        let g = weighted_graph();
        // depth 1: A1 is reachable via C1, so it survives even though its
        // direct edge to C0 is below the threshold.
        let mut got = g.get_articles_in_category_filtered(100, 1, 2).unwrap();
        got.sort_unstable();
        assert_eq!(got, vec![10, 11]);
    }

    #[test]
    fn ranked_categories_order_and_weights() {
        let g = weighted_graph();
        assert_eq!(g.get_categories_for_article_ranked(11), vec![(101, 5), (100, 1)]);
        assert_eq!(g.get_categories_for_article_ranked(12), vec![]);
    }

    /// Five categories, five articles. "Actors" (C0, QID 100) and "Actors by
    /// alpha" (C4, QID 104) both contain articles 0,1,2; "American" (C1) and
    /// "Film" (C2) are redundant subsets; "Cricket" (C3, QID 103) holds 3,4.
    /// C0's membership edges carry agreement 5, C4's only 1.
    fn cluster_graph() -> WikiGraph {
        let pairs = [
            (0u32, 0u32), (0, 1), (0, 2), (0, 4),
            (1, 0), (1, 1), (1, 4),
            (2, 0), (2, 2), (2, 4),
            (3, 3),
            (4, 3),
        ];
        let weights = [5u16, 2, 2, 1, 5, 2, 1, 5, 2, 1, 2, 2];
        let (article_cats, article_cat_weights) =
            CsrAdjacency::from_pairs_with_weights(5, &pairs, &weights);
        let bitmap = |arts: &[u32]| arts.iter().copied().collect::<RoaringBitmap>();
        WikiGraph {
            children: CsrAdjacency::from_pairs(5, &[]),
            parents: CsrAdjacency::from_pairs(5, &[]),
            cat_articles: vec![
                bitmap(&[0, 1, 2]),
                bitmap(&[0, 1]),
                bitmap(&[0, 2]),
                bitmap(&[3, 4]),
                bitmap(&[0, 1, 2]),
            ],
            article_cats,
            article_cat_weights,
            cat_dense_to_original: vec![100, 101, 102, 103, 104],
            cat_original_to_dense: [(100, 0), (101, 1), (102, 2), (103, 3), (104, 4)]
                .into_iter()
                .collect(),
            art_dense_to_original: vec![10, 11, 12, 20, 21],
            art_original_to_dense: [(10, 0), (11, 1), (12, 2), (20, 3), (21, 4)]
                .into_iter()
                .collect(),
            local_categories: [0u32, 1, 2, 3, 4].into_iter().collect(),
        }
    }

    #[test]
    fn cluster_collapses_duplicates_and_partitions() {
        let g = cluster_graph();
        // 999 is not in the graph; the rest split cleanly into actors + cricket.
        let out = g.cluster_articles(&[10, 11, 12, 20, 21, 999], None);

        let got: Vec<(u32, Vec<u32>)> = out
            .clusters
            .iter()
            .map(|c| (c.category_qid, c.article_qids.clone()))
            .collect();
        // C0 ("Actors") absorbs all three actor articles, so the redundant C1,
        // C2 and the lower-agreement duplicate C4 contribute zero marginal
        // coverage and drop. C3 ("Cricket") follows.
        assert_eq!(got, vec![(100, vec![10, 11, 12]), (103, vec![20, 21])]);
        assert_eq!(out.unclustered_qids, vec![999]);
    }

    #[test]
    fn cluster_prefers_coverage_over_agreement_and_emits_no_empty_clusters() {
        // C_A (q100) covers articles 0,1; C_B (q101) covers only article 2 with
        // low agreement; C_C (q102) covers only article 0 (already taken by C_A)
        // with very high agreement. C_A must win round 1 on coverage despite
        // lower agreement, and round 2 must pick C_B for article 2 — C_C, with
        // zero remaining coverage, must never be selected.
        let pairs = [(0u32, 0u32), (0, 2), (1, 0), (2, 1)];
        let weights = [3u16, 9, 3, 1];
        let (article_cats, article_cat_weights) =
            CsrAdjacency::from_pairs_with_weights(3, &pairs, &weights);
        let bitmap = |arts: &[u32]| arts.iter().copied().collect::<RoaringBitmap>();
        let g = WikiGraph {
            children: CsrAdjacency::from_pairs(3, &[]),
            parents: CsrAdjacency::from_pairs(3, &[]),
            cat_articles: vec![bitmap(&[0, 1]), bitmap(&[2]), bitmap(&[0])],
            article_cats,
            article_cat_weights,
            cat_dense_to_original: vec![100, 101, 102],
            cat_original_to_dense: [(100, 0), (101, 1), (102, 2)].into_iter().collect(),
            art_dense_to_original: vec![10, 11, 12],
            art_original_to_dense: [(10, 0), (11, 1), (12, 2)].into_iter().collect(),
            local_categories: [0u32, 1, 2].into_iter().collect(),
        };

        let out = g.cluster_articles(&[10, 11, 12], None);
        let got: Vec<(u32, Vec<u32>)> = out
            .clusters
            .iter()
            .map(|c| (c.category_qid, c.article_qids.clone()))
            .collect();
        assert_eq!(got, vec![(100, vec![10, 11]), (101, vec![12])]);
        assert!(out.clusters.iter().all(|c| c.size > 0));
        assert!(out.unclustered_qids.is_empty());
    }

    #[test]
    fn cluster_respects_max_clusters() {
        let g = cluster_graph();
        let out = g.cluster_articles(&[10, 11, 12, 20, 21], Some(1));
        assert_eq!(out.clusters.len(), 1);
        assert_eq!(out.clusters[0].category_qid, 100);
        // The capped-out cricket articles fall through to unclustered.
        assert_eq!(out.unclustered_qids, vec![20, 21]);
    }
}
