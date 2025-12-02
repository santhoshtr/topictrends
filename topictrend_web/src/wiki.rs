use crate::models::AppState;
use sqlx::{MySql, Pool, Row};
use std::{collections::HashMap, sync::Arc};

pub async fn get_or_create_db_pool(
    state: Arc<AppState>,
    wiki: &str,
) -> Result<Pool<MySql>, sqlx::Error> {
    // Check if pool already exists
    {
        let pools = state.db_pools.read().unwrap();
        if let Some(pool) = pools.get(wiki) {
            return Ok(pool.clone());
        }
    }

    // Create new connection pool
    let database_url = format!(
        "mysql://{}:{}@{}.web.db.svc.wikimedia.cloud:3306/{}_p",
        state.db_username, state.db_password, wiki, wiki
    );

    let pool = sqlx::MySqlPool::connect(&database_url).await?;

    // Store the pool
    {
        let mut pools = state.db_pools.write().unwrap();
        pools.insert(wiki.to_string(), pool.clone());
    }

    Ok(pool)
}

pub async fn get_id_by_title(
    state: Arc<AppState>,
    wiki: &str,
    title: &str,
    namespace: &i8,
) -> Result<u32, sqlx::Error> {
    let pool = get_or_create_db_pool(state, wiki).await?;

    let row = sqlx::query(include_str!("../../queries/get_id_by_title.sql"))
        .bind(title)
        .bind(namespace)
        .fetch_optional(&pool)
        .await?;

    if let Some(row) = row {
        let page_id: u32 = row.try_get("page_id")?;
        return Ok(page_id);
    }

    Err(sqlx::Error::RowNotFound)
}

pub async fn get_qid_by_title(
    state: Arc<AppState>,
    wiki: &str,
    title: &str,
    namespace: &i8,
) -> Result<u32, sqlx::Error> {
    let pool = get_or_create_db_pool(state, wiki).await?;

    let row = sqlx::query(include_str!("../../queries/get_qid_by_title.sql"))
        .bind(title)
        .bind(namespace)
        .fetch_optional(&pool)
        .await?;

    if let Some(row) = row {
        let qid: u32 = row.try_get("qid")?;
        return Ok(qid);
    }

    Err(sqlx::Error::RowNotFound)
}

pub async fn get_title_by_qid(
    state: Arc<AppState>,
    wiki: &str,
    qid: &u32,
) -> Result<u32, sqlx::Error> {
    let pool = get_or_create_db_pool(state, wiki).await?;

    let row = sqlx::query(include_str!("../../queries/get_qid_by_title.sql"))
        .bind(qid)
        .fetch_optional(&pool)
        .await?;

    if let Some(row) = row {
        let qid: u32 = row.try_get("qid")?;
        return Ok(qid);
    }

    Err(sqlx::Error::RowNotFound)
}

pub async fn get_titles_by_qids(
    state: Arc<AppState>,
    wiki: &str,
    qids: Vec<u32>,
) -> Result<HashMap<u32, String>, sqlx::Error> {
    let pool = get_or_create_db_pool(state, wiki).await?;

    if qids.is_empty() {
        return Ok(HashMap::new());
    }

    // Create placeholders for the IN clause
    let placeholders: Vec<String> = qids.iter().map(|_| "?".to_string()).collect();
    let placeholders_str = placeholders.join(",");

    // Build the query with the placeholders
    let query_template = include_str!("../../queries/get_titles_by_qids.sql");
    let query = query_template.replace("{}", &placeholders_str);

    let mut query_builder = sqlx::query(&query).bind(wiki);

    // Bind each QID
    for qid in &qids {
        query_builder = query_builder.bind(format!("Q{}", qid));
    }

    let rows = query_builder.fetch_all(&pool).await?;

    let mut result = HashMap::new();
    for row in rows {
        let qid: u32 = row.try_get("qid")?;
        let title: String = row.try_get("page_title")?;
        result.insert(qid, title);
    }

    Ok(result)
}
