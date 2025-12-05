use super::CoreServiceError;
use crate::models::AppState;
use std::sync::{Arc, RwLock};
use topictrend::pageview_engine::PageViewEngine;

pub struct EngineService;

impl EngineService {
    pub async fn get_or_build_engine(
        state: Arc<AppState>,
        wiki: &str,
    ) -> Result<Arc<RwLock<PageViewEngine>>, CoreServiceError> {
        let wiki = wiki.to_string();

        tokio::task::spawn_blocking(move || {
            let mut engines = state.engines.write().map_err(|_| {
                CoreServiceError::InternalError("Failed to acquire engines lock".to_string())
            })?;

            if let Some(engine) = engines.get(&wiki) {
                Ok(Arc::clone(engine))
            } else {
                let new_engine = Arc::new(RwLock::new(PageViewEngine::new(&wiki)));
                engines.insert(wiki.clone(), Arc::clone(&new_engine));
                Ok(new_engine)
            }
        })
        .await
        .map_err(|_| CoreServiceError::InternalError("Failed to spawn blocking task".to_string()))?
    }
}
