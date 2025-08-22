use axum::{response::IntoResponse, http::StatusCode, Json, Extension};
use std::sync::Arc;
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use entities::user;

use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use sha2::Sha256;
use std::collections::BTreeMap;
use axum::extract::State;
use tokio::sync::RwLock;
use crate::{error::ApiError, model::user::User, response::{ApiResponse}, extractor::InitData, cache_fetch, AppState};
use crate::service::get_user_by_telegram_id;
use crate::util::cache::{CacheBackend, CacheWrapper};

pub async fn auth_handler_get(
    InitData(user): InitData<User>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
    State(state): State<Arc<RwLock<AppState>>>,
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

    let state = state.read().await;
    let key: Hmac<Sha256> = Hmac::new_from_slice(&state.config.jwt_secret.as_bytes())?;
    let mut claims = BTreeMap::new();
    claims.insert("sub", "someone");
    let token_str = claims.sign_with_key(&key)?;

    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}
