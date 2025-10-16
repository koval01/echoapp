use axum::{Extension, Json};
use axum::extract::State;
use axum_extra::extract::cookie::CookieJar;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    error::ApiError,
    extractor::InitData,
    model::user::User,
    response::ApiResponse,
    util::cache::{CacheBackend, CacheWrapper},
    AppState, cache_fetch,
};

use entities::user::Model;
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use crate::service::{CookieService, JwtService};

pub async fn auth_handler_get(
    InitData(user): InitData<User>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
    State(state): State<Arc<RwLock<AppState>>>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<ApiResponse<Model>>), ApiError> {
    let user_model = fetch_user_with_cache(
        user.id, &db, redis_pool, moka_cache
    ).await?;

    let token = generate_auth_token(
        &state, user_model.telegram_id
    ).await?;

    let updated_jar = CookieService::add_auth_cookie(
        jar, &token, 8
    );

    let response = ApiResponse::success(user_model);
    Ok((updated_jar, Json(response)))
}

async fn fetch_user_with_cache(
    user_id: i64,
    db: &Arc<DatabaseConnection>,
    redis_pool: CacheBackend,
    moka_cache: Cache<String, String>,
) -> Result<Model, ApiError> {
    let cache = CacheWrapper::<Model>::new(redis_pool, moka_cache, 10, 10);
    let cache_key = format!("user:{}", user_id);

    let user_option = cache_fetch!(
        cache,
        &cache_key,
        async {
            crate::service::get_user_by_id(user_id, db)
                .await
        }
    );
    user_option.map_err(|_| ApiError::NotFound("User not found".to_string()))
}

async fn generate_auth_token(
    state: &Arc<RwLock<AppState>>,
    user_id: i64,
) -> Result<String, ApiError> {
    let state_guard = state.read().await;
    let jwt_service = JwtService::new(&state_guard.config.jwt_secret)?;

    jwt_service
        .generate_token(user_id, state_guard.config.session_maxage)
        .map_err(ApiError::from)
}
