use super::CoreServiceError;
use crate::models::AppState;
use crate::wiki::get_or_create_db_pool;
use sqlx::Row;
use std::{collections::HashMap, sync::Arc};

pub struct QidService;

impl QidService {
    pub async fn get_qid_by_title(
        state: Arc<AppState>,
        wiki: &str,
        title: &str,
        namespace: i8,
    ) -> Result<u32, CoreServiceError> {
        let pool = get_or_create_db_pool(state, wiki).await?;

        let row = sqlx::query(include_str!("../../../../queries/get_qid_by_title.sql"))
            .bind(title)
            .bind(namespace)
            .fetch_optional(&pool)
            .await?;

        if let Some(row) = row {
            let qid: u32 = row.try_get("qid")?;
            return Ok(qid);
        }

        Err(CoreServiceError::NotFound)
    }

    pub async fn get_title_by_qid(
        state: Arc<AppState>,
        wiki: &str,
        qid: u32,
    ) -> Result<String, CoreServiceError> {
        let titles = Self::get_titles_by_qids(state, wiki, &vec![qid]).await?;
        titles.get(&qid).cloned().ok_or(CoreServiceError::NotFound)
    }

    pub async fn get_titles_by_qids(
        state: Arc<AppState>,
        wiki: &str,
        qids: &Vec<u32>,
    ) -> Result<HashMap<u32, String>, CoreServiceError> {
        let pool = get_or_create_db_pool(state, wiki).await?;

        if qids.is_empty() {
            return Ok(HashMap::new());
        }

        // Create placeholders for the IN clause
        let placeholders: Vec<String> = qids.iter().map(|_| "?".to_string()).collect();
        let placeholders_str = placeholders.join(",");

        // Build the query with the placeholders
        let query_template = include_str!("../../../../queries/get_titles_by_qids.sql");
        let query = query_template.replace("{}", &placeholders_str);

        let mut query_builder = sqlx::query(&query);

        // Bind each QID
        for qid in qids {
            query_builder = query_builder.bind(format!("Q{}", qid));
        }

        let rows = query_builder.fetch_all(&pool).await?;

        let mut result = HashMap::new();
        for row in rows {
            let qid: u32 = row.try_get("qid")?;
            // Get page_title as Vec<u8> and convert to String
            let title_bytes: Vec<u8> = row.try_get("page_title")?;
            let title = String::from_utf8_lossy(&title_bytes).to_string();
            result.insert(qid, title);
        }

        Ok(result)
    }

    pub async fn get_qids_by_titles(
        state: Arc<AppState>,
        wiki: &str,
        titles: Vec<String>,
        namespace: i8,
    ) -> Result<HashMap<String, u32>, CoreServiceError> {
        let pool = get_or_create_db_pool(state, wiki).await?;

        if titles.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = titles.iter().map(|_| "?".to_string()).collect();
        let placeholders_str = placeholders.join(",");

        let query = format!(
            "SELECT page_title, qid FROM page p 
             JOIN wb_items_per_site w ON p.page_id = w.ips_site_page 
             WHERE p.page_title IN ({}) AND p.page_namespace = ?",
            placeholders_str
        );

        let mut query_builder = sqlx::query(&query);

        // Bind each title
        for title in &titles {
            query_builder = query_builder.bind(title);
        }
        query_builder = query_builder.bind(namespace);

        let rows = query_builder.fetch_all(&pool).await?;

        let mut result = HashMap::new();
        for row in rows {
            let title_bytes: Vec<u8> = row.try_get("page_title")?;
            let title = String::from_utf8_lossy(&title_bytes).to_string();
            let qid: u32 = row.try_get("qid")?;
            result.insert(title, qid);
        }

        Ok(result)
    }
}
