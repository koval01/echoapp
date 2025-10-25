use axum::{response::IntoResponse, http::StatusCode, Json, Extension};
use std::sync::Arc;
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use entities::user::Model;
use uuid::Uuid;
use crate::{
    error::ApiError,
    response::ApiResponse,
    extractor::{StrictUuid, JWTExtractor},
    cache_fetch,
    service::get_user_by_id,
    util::cache::{CacheBackend, CacheWrapper, CacheError},
};
use crate::model::user::PublicUser;

async fn fetch_user<T, F, Fut>(
    user_id: T,
    db: Arc<DatabaseConnection>,
    redis_pool: CacheBackend,
    moka_cache: Cache<String, String>,
    redis_ttl: u64,
    moka_ttl: u64,
    cache_key_prefix: &str,
    fetch_fn: F,
) -> Result<Model, CacheError>
where
    T: Into<Uuid> + ToString + Send + Sync + 'static,
    F: FnOnce(Uuid, Arc<DatabaseConnection>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Option<Model>, ApiError>> + Send,
{
    let cache = CacheWrapper::<Model>::new(redis_pool, moka_cache, redis_ttl, moka_ttl);
    let cache_key = format!("{}{}", cache_key_prefix, user_id.to_string());

    cache_fetch!(
        cache,
        &cache_key,
        async {
            let uuid_id = user_id.into();
            let user = fetch_fn(uuid_id, db.clone()).await?;
            match user {
                Some(u) => Ok(Some(u)),
                None => Err(ApiError::NotFound("User not found".into())),
            }
        }
    )
}

// Common function to fetch user with standard caching configuration
async fn fetch_user_with_default_cache<T>(
    user_id: T,
    db: Arc<DatabaseConnection>,
    redis_pool: CacheBackend,
    moka_cache: Cache<String, String>,
) -> Result<Model, ApiError>
where
    T: Into<Uuid> + ToString + Send + Sync + 'static,
{
    fetch_user(
        user_id,
        db,
        redis_pool,
        moka_cache,
        60,
        30,
        "user_uuid:",
        |id, db| async move {
            get_user_by_id(id, &db).await.map_err(ApiError::from)
        },
    )
        .await
        .map_err(ApiError::from)
}

pub async fn user_handler_get(
    JWTExtractor(user_id): JWTExtractor,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let user = fetch_user_with_default_cache(user_id, db, redis_pool, moka_cache).await?;
    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}

pub async fn user_by_id_handler_get(
    StrictUuid(user_id): StrictUuid,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let user = fetch_user_with_default_cache(user_id, db, redis_pool, moka_cache).await?;
    let response = ApiResponse::success(PublicUser::from(user));
    Ok((StatusCode::OK, Json(response)))
}
