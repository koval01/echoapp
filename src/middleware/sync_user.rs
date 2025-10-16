use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
    Extension
};

use crate::{cache_fetch, error::ApiError, extractor::InitData, util::cache::CacheWrapper};

use moka::future::Cache;

use std::sync::Arc;
use sea_orm::DatabaseConnection;
use entities::user;
use crate::model::user::User;
use crate::service::{create_user, get_user_by_id};
use crate::util::cache::CacheBackend;

pub async fn sync_user_middleware(
    InitData(user): InitData<User>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let cache = CacheWrapper::<user::Model>::new(
        redis_pool,
        moka_cache,
        10,
        10
    );

    let _ = cache_fetch!(
        cache,
        &format!("user:{}", &user.id),
        async {
            match get_user_by_id(user.id, &db).await {
                Ok(Some(user)) => Ok(Some(user)),
                Ok(None) => {
                    match create_user(user, &db).await {
                        Ok(user) => Ok(Some(user)),
                        Err(e) => Err(ApiError::from(e)),
                    }
                },
                Err(e) => Err(ApiError::from(e)),
            }
        }
    )?;

    Ok(next.run(request).await)
}

#[allow(dead_code)]
#[inline(always)]
fn needs_update(init_user: &User, db_user: &user::Model) -> bool {
    init_user.first_name != db_user.first_name
        || init_user.last_name != db_user.last_name
        || init_user.username != db_user.username
        || init_user.language_code != db_user.language_code
        || init_user.allows_write_to_pm != db_user.allows_write_to_pm
        || init_user.photo_url != db_user.photo_url
}