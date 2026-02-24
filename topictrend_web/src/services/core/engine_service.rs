use super::CoreServiceError;
use crate::models::{AppState, MetricEngine, MetricType};
use std::sync::{Arc, RwLock};
use topictrend::graphbuilder::GraphBuilder;
use topictrend::pageedits_engine::PageEditsEngine;
use topictrend::pageview_engine::PageViewEngine;
use topictrend::wikigraph::WikiGraph;

pub struct EngineService;

impl EngineService {
    pub async fn get_or_build_graph_engine(
        state: Arc<AppState>,
        wiki: &str,
    ) -> Result<Arc<RwLock<WikiGraph>>, CoreServiceError> {
        let wiki = wiki.to_string();

        tokio::task::spawn_blocking(move || {
            let mut engines = state.engines.write().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
            })?;

            let key = (wiki.clone(), MetricType::Graph);

            if let Some(engine) = engines.get(&key) {
                return engine.as_graph().map(Arc::clone).ok_or_else(|| {
                    CoreServiceError::InternalError("Engine type mismatch".to_string())
                });
            }

            let graph_builder = GraphBuilder::new(&wiki);
            let graph = graph_builder
                .build()
                .map_err(|err| CoreServiceError::EngineError(err.to_string()))?;
            let new_engine = Arc::new(RwLock::new(graph));
            let new_metric_engine = MetricEngine::Graph(Arc::clone(&new_engine));
            engines.insert(key, new_metric_engine);
            Ok(new_engine)
        })
        .await
        .map_err(|_| CoreServiceError::InternalError("Failed to spawn blocking task".to_string()))?
    }
    pub async fn get_or_build_pageview_engine(
        state: Arc<AppState>,
        wiki: &str,
    ) -> Result<Arc<RwLock<PageViewEngine>>, CoreServiceError> {
        let wiki = wiki.to_string();

        tokio::task::spawn_blocking(move || {
            let mut engines = state.engines.write().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
            })?;

            let key = (wiki.clone(), MetricType::PageView);

            if let Some(engine) = engines.get(&key) {
                engine.as_pageview().map(Arc::clone).ok_or_else(|| {
                    CoreServiceError::InternalError("Engine type mismatch".to_string())
                })
            } else {
                let new_engine = Arc::new(RwLock::new(PageViewEngine::new(&wiki)));
                let new_metric_engine = MetricEngine::PageView(Arc::clone(&new_engine));
                engines.insert(key, new_metric_engine);
                Ok(new_engine)
            }
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
            let mut engines = state.engines.write().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
            })?;

            let key = (wiki.clone(), MetricType::PageEdit);

            if let Some(engine) = engines.get(&key) {
                engine.as_pageedit().map(Arc::clone).ok_or_else(|| {
                    CoreServiceError::InternalError("Engine type mismatch".to_string())
                })
            } else {
                let new_engine = Arc::new(RwLock::new(PageEditsEngine::new(&wiki)));
                let new_metric_engine = MetricEngine::PageEdit(Arc::clone(&new_engine));
                engines.insert(key, new_metric_engine);
                Ok(new_engine)
            }
        })
        .await
        .map_err(|_| CoreServiceError::InternalError("Failed to spawn blocking task".to_string()))?
    }
}
