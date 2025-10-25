use axum::{response::IntoResponse, http::StatusCode, Json, Extension};
use std::sync::Arc;
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::{
    error::ApiError,
    response::ApiResponse,
    extractor::StrictUuid,
    cache_fetch,
    service::get_user_by_id,
    util::cache::{CacheBackend, CacheWrapper},
};
use crate::extractor::JWTExtractor;

async fn fetch_user<T, F, Fut>(
    user_id: T,
    db: Arc<DatabaseConnection>,
    redis_pool: CacheBackend,
    moka_cache: Cache<String, String>,
    redis_ttl: u64,
    moka_ttl: u64,
    cache_key_prefix: &str,
    fetch_fn: F,
) -> Result<impl IntoResponse, ApiError>
where
    T: ToString + Send + Sync + 'static,
    F: FnOnce(T, Arc<DatabaseConnection>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Option<Value>, ApiError>> + Send,
{
    let cache = CacheWrapper::<Value>::new(redis_pool, moka_cache, redis_ttl, moka_ttl);

    let cache_key = format!("{}{}", cache_key_prefix, user_id.to_string());

    let user = cache_fetch!(
        cache,
        &cache_key,
        async {
            let user = fetch_fn(user_id, db.clone()).await?;
            match user {
                Some(u) => Ok(Some(u)),
                None => Err(ApiError::NotFound("User not found".into())),
            }
        }
    )?;

    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}

pub async fn user_handler_get(
    JWTExtractor(user_id): JWTExtractor,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    fetch_user(
        user_id,
        db,
        redis_pool,
        moka_cache,
        60,
        30,
        "user_uuid:",
        |id, db| async move {
            get_user_by_id(id, &db, true).await.map_err(ApiError::from)
        },
    )
        .await
}

pub async fn user_by_id_handler_get(
    StrictUuid(user_id): StrictUuid,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    fetch_user(
        user_id,
        db,
        redis_pool,
        moka_cache,
        60,
        30,
        "user_uuid:",
        |id, db| async move {
            get_user_by_id(id, &db, false).await.map_err(ApiError::from)
        },
    )
        .await
}
