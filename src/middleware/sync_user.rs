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
use crate::service::{create_user, get_user_by_telegram_id, needs_update, update_user};
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
            match get_user_by_telegram_id(user.id, &db).await {
                Ok(Some(db_user)) => {
                    if needs_update(&user, &db_user) {
                        match update_user(&user, &db_user, &db).await {
                            Ok(updated_user) => Ok(Some(updated_user)),
                            Err(e) => Err(ApiError::from(e)),
                        }
                    } else {
                        Ok(Some(db_user))
                    }
                },
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
