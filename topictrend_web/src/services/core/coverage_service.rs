//! Loads the precomputed coverage matrix into the web layer and ranks
//! cross-wiki content gaps.
//!
//! For a target wiki `T` measured against a reference wiki `R`, the gap for a
//! category `C` is `qid_overlap(C, R) - qid_overlap(C, T)` — how many of `C`'s
//! globally-known articles `R` has that `T` lacks. Two bounded FIFO caches back
//! this (same shape as `pageview_engine::BoundedDailyViews`, values wrapped in
//! `Arc` so mid-request eviction is safe): per-wiki coverage snapshots, and
//! per-`(reference, target)` fully-sorted rankings. Filters and pagination are
//! applied at serve time over a cached ranking, so moving a slider never
//! triggers a rebuild.

use super::CoreServiceError;
use crate::models::AppState;
use chrono::NaiveDate;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use topictrend::coverage_parquet::{
    CoverageMatrix, latest_coverage_snapshot, load_coverage_parquet,
};

const COVERAGE_CACHE_ENV: &str = "TOPICTREND_COVERAGE_WIKIS";
const DEFAULT_COVERAGE_CACHE: usize = 8;
const RANKING_CACHE_ENV: &str = "TOPICTREND_GAP_RANKINGS";
const DEFAULT_RANKING_CACHE: usize = 4;

/// One wiki's coverage snapshot plus the date it was taken from.
#[derive(Debug)]
pub struct CoverageSnapshot {
    pub date: NaiveDate,
    pub matrix: CoverageMatrix,
}

/// One ranked gap row (pre-title-resolution; titles are added at the handler edge).
#[derive(Debug, Clone)]
pub struct GapRow {
    pub category_qid: u32,
    pub direct_target: u32,
    pub overlap_target: u32,
    pub overlap_reference: u32,
    pub gap: i64, // overlap_reference - overlap_target, always > 0 in a ranking
    pub has_category: bool, // target files >=1 article directly under C
    pub overlap_pageviews: u64, // reference-side windowed views of C's overlap articles
    pub weighted_score: u64, // overlap_pageviews × gap / overlap_reference (0 when unweighted)
}

/// Full sorted ranking for a `(reference, target)` pair. Sorted by
/// `weighted_score DESC, qid ASC` when weighted, else `gap DESC, qid ASC`.
#[derive(Debug)]
pub struct GapRanking {
    pub rows: Vec<GapRow>,
    pub reference_date: NaiveDate,
    pub target_date: NaiveDate,
    // True when weighting was requested AND the reference snapshot carries the
    // pageview column. False means the rows are the unweighted gap ranking.
    pub weighted_applied: bool,
}

/// A filtered, paginated slice of a `GapRanking` plus summary counts.
pub struct GapWindow {
    pub rows: Vec<GapRow>,
    pub filtered_total: usize, // rows passing the filters (before skip/limit)
    pub with_category: usize,  // of the filtered set, rows where has_category
    pub without_category: usize, // of the filtered set, rows where !has_category
}

impl GapRanking {
    /// Apply `min_ref` (a floor on the reference overlap, to drop tiny noise)
    /// and the `has_category` structure filter, then skip `offset` and take
    /// `limit`. Pure — no I/O.
    pub fn window(
        &self,
        min_ref: Option<u32>,
        has_category: Option<bool>,
        offset: usize,
        limit: usize,
    ) -> GapWindow {
        let mut filtered_total = 0usize;
        let mut with_category = 0usize;
        let mut without_category = 0usize;
        let mut rows = Vec::new();

        for r in &self.rows {
            if let Some(m) = min_ref
                && r.overlap_reference < m
            {
                continue;
            }
            if let Some(hc) = has_category
                && r.has_category != hc
            {
                continue;
            }

            if r.has_category {
                with_category += 1;
            } else {
                without_category += 1;
            }
            if filtered_total >= offset && rows.len() < limit {
                rows.push(r.clone());
            }
            filtered_total += 1;
        }

        GapWindow {
            rows,
            filtered_total,
            with_category,
            without_category,
        }
    }
}

/// Bounded FIFO cache (capacity 0 = unlimited). Mirrors
/// `pageview_engine::BoundedDailyViews`; values are `Arc` so a reader can keep a
/// snapshot alive across eviction.
#[derive(Debug)]
pub struct BoundedCache<K: Eq + Hash + Clone, V> {
    map: HashMap<K, Arc<V>>,
    order: VecDeque<K>,
    capacity: usize,
}

impl<K: Eq + Hash + Clone, V> BoundedCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    pub(crate) fn get(&self, key: &K) -> Option<Arc<V>> {
        self.map.get(key).cloned()
    }

    pub(crate) fn insert(&mut self, key: K, value: Arc<V>) {
        if self.map.contains_key(&key) {
            return;
        }
        if self.capacity > 0 {
            while self.map.len() >= self.capacity {
                match self.order.pop_front() {
                    Some(oldest) => {
                        self.map.remove(&oldest);
                    }
                    None => break,
                }
            }
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
    }
}

pub fn coverage_cache_capacity() -> usize {
    std::env::var(COVERAGE_CACHE_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_COVERAGE_CACHE)
}

pub fn ranking_cache_capacity() -> usize {
    std::env::var(RANKING_CACHE_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_RANKING_CACHE)
}

pub struct CoverageService;

impl CoverageService {
    /// Get (and lazily build/cache) the full sorted gap ranking for a
    /// `(reference, target)` pair.
    pub async fn get_or_build_ranking(
        state: Arc<AppState>,
        reference: &str,
        target: &str,
        weighted: bool,
    ) -> Result<Arc<GapRanking>, CoreServiceError> {
        let reference = reference.to_string();
        let target = target.to_string();
        tokio::task::spawn_blocking(move || {
            Self::get_or_build_ranking_blocking(&state, &reference, &target, weighted)
        })
        .await
        .map_err(|_| CoreServiceError::InternalError("Failed to spawn blocking task".to_string()))?
    }

    fn get_or_build_ranking_blocking(
        state: &AppState,
        reference: &str,
        target: &str,
        weighted: bool,
    ) -> Result<Arc<GapRanking>, CoreServiceError> {
        let key = (reference.to_string(), target.to_string(), weighted);

        // Fast path: ranking already cached.
        {
            let cache = state.gap_rankings.read().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire gap_rankings lock".to_string())
            })?;
            if let Some(ranking) = cache.get(&key) {
                return Ok(ranking);
            }
        }

        // Slow path: load both snapshots and merge.
        let ref_snap = Self::get_or_load_snapshot_blocking(state, reference)?;
        let tgt_snap = Self::get_or_load_snapshot_blocking(state, target)?;

        // Weighting needs the reference snapshot's pageview column; degrade to an
        // unweighted ranking (and report it) if this snapshot predates it.
        let weighted_applied = weighted && ref_snap.matrix.has_pageviews();

        let mut rows: Vec<GapRow> = Vec::new();
        for (category_qid, _direct_ref, overlap_reference, overlap_pageviews) in
            ref_snap.matrix.iter()
        {
            // Excluded categories are filtered out of topology at ETL.
            let (direct_target, overlap_target, _pv_target) = tgt_snap.matrix.get(category_qid);
            let gap = overlap_reference as i64 - overlap_target as i64;
            if gap > 0 {
                // overlap_reference ≥ gap > 0, so the division is always defined.
                let weighted_score = if weighted_applied {
                    (overlap_pageviews as u128 * gap as u128 / overlap_reference as u128) as u64
                } else {
                    0
                };
                rows.push(GapRow {
                    category_qid,
                    direct_target,
                    overlap_target,
                    overlap_reference,
                    gap,
                    has_category: direct_target > 0,
                    overlap_pageviews,
                    weighted_score,
                });
            }
        }
        if weighted_applied {
            rows.sort_unstable_by(|a, b| {
                b.weighted_score
                    .cmp(&a.weighted_score)
                    .then(a.category_qid.cmp(&b.category_qid))
            });
        } else {
            rows.sort_unstable_by(|a, b| {
                b.gap.cmp(&a.gap).then(a.category_qid.cmp(&b.category_qid))
            });
        }

        let ranking = Arc::new(GapRanking {
            rows,
            reference_date: ref_snap.date,
            target_date: tgt_snap.date,
            weighted_applied,
        });

        let mut cache = state.gap_rankings.write().map_err(|_| {
            CoreServiceError::InternalError("Failed to acquire gap_rankings lock".to_string())
        })?;
        if let Some(existing) = cache.get(&key) {
            return Ok(existing);
        }
        cache.insert(key, Arc::clone(&ranking));
        Ok(ranking)
    }

    /// Get (and lazily load/cache) the latest coverage snapshot for one wiki.
    pub async fn get_or_load_snapshot(
        state: Arc<AppState>,
        wiki: &str,
    ) -> Result<Arc<CoverageSnapshot>, CoreServiceError> {
        let wiki = wiki.to_string();
        tokio::task::spawn_blocking(move || Self::get_or_load_snapshot_blocking(&state, &wiki))
            .await
            .map_err(|_| {
                CoreServiceError::InternalError("Failed to spawn blocking task".to_string())
            })?
    }

    fn get_or_load_snapshot_blocking(
        state: &AppState,
        wiki: &str,
    ) -> Result<Arc<CoverageSnapshot>, CoreServiceError> {
        // Fast path.
        {
            let cache = state.coverage_snapshots.read().map_err(|_| {
                CoreServiceError::InternalError(
                    "Failed to acquire coverage_snapshots lock".to_string(),
                )
            })?;
            if let Some(snap) = cache.get(&wiki.to_string()) {
                return Ok(snap);
            }
        }

        // Slow path: discover the newest snapshot and load it.
        let (date, path) = latest_coverage_snapshot(wiki).ok_or_else(|| {
            let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
            CoreServiceError::EngineError(format!(
                "no coverage snapshot for wiki '{}' under {}/{}/coverage/",
                wiki, data_dir, wiki
            ))
        })?;
        let matrix = load_coverage_parquet(&path)
            .map_err(|e| CoreServiceError::EngineError(e.to_string()))?;
        let snap = Arc::new(CoverageSnapshot { date, matrix });

        let mut cache = state.coverage_snapshots.write().map_err(|_| {
            CoreServiceError::InternalError("Failed to acquire coverage_snapshots lock".to_string())
        })?;
        if let Some(existing) = cache.get(&wiki.to_string()) {
            return Ok(existing);
        }
        cache.insert(wiki.to_string(), Arc::clone(&snap));
        Ok(snap)
    }
}
