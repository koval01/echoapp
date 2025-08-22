use axum::{Json, Extension};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use std::sync::Arc;
use moka::future::Cache;
use sea_orm::DatabaseConnection;

use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use sha2::Sha256;
use std::collections::BTreeMap;
use time::{Duration, OffsetDateTime};
use axum::extract::State;
use serde_json::Value;
use tokio::sync::RwLock;
use entities::user::Model;
use crate::{error::ApiError, model::user::User, response::{ApiResponse}, extractor::InitData, cache_fetch, AppState};
use crate::service::get_user_by_telegram_id;
use crate::util::cache::{CacheBackend, CacheWrapper};

pub async fn auth_handler_get(
    InitData(user): InitData<User>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
    State(state): State<Arc<RwLock<AppState>>>,
    jar: CookieJar
) -> Result<(CookieJar, Json<ApiResponse<Model>>), ApiError> {
    let cache = CacheWrapper::<Model>::new(
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
    claims.insert("sub", Value::String(String::from(user.id)));
    let now = OffsetDateTime::now_utc();
    let iat = now.unix_timestamp();
    let exp = (now + Duration::minutes(15)).unix_timestamp();
    claims.insert("iat", Value::Number(iat.into()));
    claims.insert("exp", Value::Number(exp.into()));
    let token_str = claims.sign_with_key(&key)?;

    let mut c_auth = Cookie::new("auth_token", token_str);
    c_auth.set_http_only(true);
    c_auth.set_secure(true);
    c_auth.set_max_age(Duration::minutes(15));
    c_auth.set_path("/");
    let cookies = jar.add(c_auth);

    let response = ApiResponse::success(user);
    Ok((
        cookies,
        Json(response),
    ))
}
