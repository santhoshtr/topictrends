use super::CoreServiceError;
use crate::models::{AppState, MetricEngine, MetricType};
use std::sync::{Arc, RwLock};
use topictrend::google_search_engine::GoogleSearchEngine;
use topictrend::graphbuilder::GraphBuilder;
use topictrend::pageedits_engine::PageEditsEngine;
use topictrend::pageview_engine::PageViewEngine;
use topictrend::wikigraph::WikiGraph;

pub struct EngineService;

impl EngineService {
    /// Acquire the shared `Arc<WikiGraph>` for a wiki, building and caching
    /// it on first request. All metric engines for the same wiki share this
    /// graph so the topology (CSR adjacency, DirectMaps over the QID space,
    /// per-category RoaringBitmaps) is paid for once, not once per engine
    /// type. The graph is immutable once built — no lock needed.
    pub async fn get_or_build_graph_engine(
        state: Arc<AppState>,
        wiki: &str,
    ) -> Result<Arc<WikiGraph>, CoreServiceError> {
        let wiki = wiki.to_string();

        tokio::task::spawn_blocking(move || Self::get_or_build_graph_blocking(&state, &wiki))
            .await
            .map_err(|_| CoreServiceError::InternalError("Failed to spawn blocking task".to_string()))?
    }

    /// Synchronous core of [`get_or_build_graph_engine`], usable from inside
    /// other `spawn_blocking` closures so a downstream engine builder
    /// doesn't need to nest tasks.
    fn get_or_build_graph_blocking(
        state: &AppState,
        wiki: &str,
    ) -> Result<Arc<WikiGraph>, CoreServiceError> {
        // Fast path: graph already cached — read lock only.
        {
            let engines = state.engines.read().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
            })?;
            if let Some(engine) = engines.get(&(wiki.to_string(), MetricType::Graph)) {
                return engine
                    .as_graph()
                    .map(Arc::clone)
                    .ok_or_else(|| CoreServiceError::InternalError("Engine type mismatch".to_string()));
            }
        }

        // Slow path: build the graph (potentially expensive), then insert under
        // the write lock — but only if another thread hasn't beaten us to it.
        let new_graph = Arc::new(
            GraphBuilder::new(wiki)
                .build()
                .map_err(|err| CoreServiceError::EngineError(err.to_string()))?,
        );

        let mut engines = state.engines.write().map_err(|_| {
            CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
        })?;
        let key = (wiki.to_string(), MetricType::Graph);
        if let Some(engine) = engines.get(&key) {
            // Concurrent builder won — discard our build.
            return engine
                .as_graph()
                .map(Arc::clone)
                .ok_or_else(|| CoreServiceError::InternalError("Engine type mismatch".to_string()));
        }
        engines.insert(key, MetricEngine::Graph(Arc::clone(&new_graph)));
        Ok(new_graph)
    }
    pub async fn get_or_build_pageview_engine(
        state: Arc<AppState>,
        wiki: &str,
    ) -> Result<Arc<RwLock<PageViewEngine>>, CoreServiceError> {
        let wiki = wiki.to_string();

        tokio::task::spawn_blocking(move || {
            // Fast path: pageview engine already cached.
            {
                let engines = state.engines.read().map_err(|_| {
                    CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
                })?;
                if let Some(engine) = engines.get(&(wiki.clone(), MetricType::PageView)) {
                    return engine.as_pageview().map(Arc::clone).ok_or_else(|| {
                        CoreServiceError::InternalError("Engine type mismatch".to_string())
                    });
                }
            }

            // Slow path: build over the shared graph.
            let graph = Self::get_or_build_graph_blocking(&state, &wiki)?;
            let new_engine = Arc::new(RwLock::new(PageViewEngine::with_graph(&wiki, graph)));

            let mut engines = state.engines.write().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
            })?;
            let key = (wiki.clone(), MetricType::PageView);
            if let Some(engine) = engines.get(&key) {
                return engine.as_pageview().map(Arc::clone).ok_or_else(|| {
                    CoreServiceError::InternalError("Engine type mismatch".to_string())
                });
            }
            engines.insert(key, MetricEngine::PageView(Arc::clone(&new_engine)));
            Ok(new_engine)
        })
        .await
        .map_err(|_| CoreServiceError::InternalError("Failed to spawn blocking task".to_string()))?
    }

    pub async fn get_or_build_pageedit_engine(
        state: Arc<AppState>,
        wiki: &str,
    ) -> Result<Arc<RwLock<PageEditsEngine>>, CoreServiceError> {
        let wiki = wiki.to_string();

        tokio::task::spawn_blocking(move || {
            {
                let engines = state.engines.read().map_err(|_| {
                    CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
                })?;
                if let Some(engine) = engines.get(&(wiki.clone(), MetricType::PageEdit)) {
                    return engine.as_pageedit().map(Arc::clone).ok_or_else(|| {
                        CoreServiceError::InternalError("Engine type mismatch".to_string())
                    });
                }
            }

            let graph = Self::get_or_build_graph_blocking(&state, &wiki)?;
            let new_engine = Arc::new(RwLock::new(PageEditsEngine::with_graph(&wiki, graph)));

            let mut engines = state.engines.write().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
            })?;
            let key = (wiki.clone(), MetricType::PageEdit);
            if let Some(engine) = engines.get(&key) {
                return engine.as_pageedit().map(Arc::clone).ok_or_else(|| {
                    CoreServiceError::InternalError("Engine type mismatch".to_string())
                });
            }
            engines.insert(key, MetricEngine::PageEdit(Arc::clone(&new_engine)));
            Ok(new_engine)
        })
        .await
        .map_err(|_| CoreServiceError::InternalError("Failed to spawn blocking task".to_string()))?
    }

    pub async fn get_or_build_google_search_engine(
        state: Arc<AppState>,
        wiki: &str,
    ) -> Result<Arc<RwLock<GoogleSearchEngine>>, CoreServiceError> {
        let wiki = wiki.to_string();

        tokio::task::spawn_blocking(move || {
            {
                let engines = state.engines.read().map_err(|_| {
                    CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
                })?;
                if let Some(engine) = engines.get(&(wiki.clone(), MetricType::GoogleSearch)) {
                    return engine.as_google_search().map(Arc::clone).ok_or_else(|| {
                        CoreServiceError::InternalError("Engine type mismatch".to_string())
                    });
                }
            }

            let graph = Self::get_or_build_graph_blocking(&state, &wiki)?;
            let new_engine = Arc::new(RwLock::new(GoogleSearchEngine::with_graph(&wiki, graph)));

            let mut engines = state.engines.write().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
            })?;
            let key = (wiki.clone(), MetricType::GoogleSearch);
            if let Some(engine) = engines.get(&key) {
                return engine.as_google_search().map(Arc::clone).ok_or_else(|| {
                    CoreServiceError::InternalError("Engine type mismatch".to_string())
                });
            }
            engines.insert(key, MetricEngine::GoogleSearch(Arc::clone(&new_engine)));
            Ok(new_engine)
        })
        .await
        .map_err(|_| CoreServiceError::InternalError("Failed to spawn blocking task".to_string()))?
    }
}
