use axum::{response::IntoResponse, http::StatusCode, Json, Extension};
use std::sync::Arc;
use axum::extract::Path;
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use entities::user;
use crate::{error::ApiError, model::user::User, response::{ApiResponse}, extractor::InitData, cache_fetch};
use crate::extractor::StrictUuid;
use crate::service::{get_user_by_id, get_user_by_telegram_id};
use crate::util::cache::{CacheBackend, CacheWrapper};

pub async fn user_handler_get(
    InitData(user): InitData<User>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let cache = CacheWrapper::<user::Model>::new(
        redis_pool,
        moka_cache,
        10,
        10
    );

    let user = cache_fetch!(
        cache,
        &format!("user:{}", &user.id),
        async {
            match get_user_by_telegram_id(user.id, &db).await {
                Ok(Some(user)) => Ok(Some(user)),
                Ok(None) => Err(ApiError::NotFound("User not found".to_string())),
                Err(e) => Err(ApiError::from(e)),
            }
        }
    )?;

    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}

pub async fn user_by_id_handler_get(
    StrictUuid(user_id): StrictUuid,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let cache = CacheWrapper::<user::Model>::new(
        redis_pool,
        moka_cache,
        10,
        10
    );

    let user = cache_fetch!(
        cache,
        &format!("user:{}", &user_id),
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
