use crate::models::AppState;
use sqlx::{MySql, Pool};
use std::sync::Arc;

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
