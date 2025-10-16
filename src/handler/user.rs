use axum::{response::IntoResponse, http::StatusCode, Json, Extension};
use std::sync::Arc;
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use entities::user;

use crate::{
    error::ApiError,
    model::user::User,
    response::ApiResponse,
    extractor::{InitData, StrictI64},
    cache_fetch,
    service::get_user_by_id,
    util::cache::{CacheBackend, CacheWrapper},
};
use crate::extractor::{JWTExtractor, JwtPayload};

async fn fetch_user(
    user_id: i64,
    db: Arc<DatabaseConnection>,
    redis_pool: CacheBackend,
    moka_cache: Cache<String, String>,
    redis_ttl: u64,
    moka_ttl: u64,
    cache_key_prefix: &str,
) -> Result<impl IntoResponse, ApiError> {
    let cache = CacheWrapper::<user::Model>::new(redis_pool, moka_cache, redis_ttl, moka_ttl);

    let user = cache_fetch!(
        cache,
        &format!("{}{}", cache_key_prefix, user_id),
        async {
            match get_user_by_id(user_id, &db).await {
                Ok(Some(user)) => Ok(Some(user)),
                Ok(None) => Err(ApiError::NotFound("User not found".to_string())),
                Err(e) => Err(ApiError::from(e)),
            }
        }
    )?;

    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}

pub async fn user_handler_get(
    JWTExtractor(jwt_token): JWTExtractor<JwtPayload>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = jwt_token.sub.parse::<i64>().map_err(|_| ApiError::BadRequest)?;
    fetch_user(
        user_id,
        db,
        redis_pool,
        moka_cache,
        10,
        10,
        "user:",
    )
        .await
}

pub async fn user_by_id_handler_get(
    StrictI64(user_id): StrictI64,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    fetch_user(
        user_id,
        db,
        redis_pool,
        moka_cache,
        120,
        30,
        "user_uuid:",
    )
        .await
}
