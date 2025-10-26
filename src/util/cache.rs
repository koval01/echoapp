use bb8_redis::{
    bb8::{Pool, RunError},
    RedisConnectionManager,
    redis::{RedisError, AsyncCommands},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{to_string, from_str};

use moka::future::Cache;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::future::Future;
use axum::http::StatusCode;
use crate::error::ApiError;

#[derive(Clone, Serialize, Deserialize)]
struct CachedValue {
    value: String,
    expires_at: u64,
}

#[derive(Debug)]
pub enum CacheError {
    Redis(RunError<RedisError>),      // Error related to Redis connection or operations
    Serialization(serde_json::Error), // Error related to JSON serialization/deserialization
    NotFound,                         // Error indicating that the data was not found
    FetchError(String),               // Generic error for fetch operations
    CachedError(StatusCode, String),  // Cached errors
}

// Implement conversion from Redis errors to CacheError
impl From<RunError<RedisError>> for CacheError {
    fn from(err: RunError<RedisError>) -> Self {
        CacheError::Redis(err)
    }
}

// Implement conversion from JSON errors to CacheError
impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::Serialization(err)
    }
}

impl From<RedisError> for CacheError {
    fn from(err: RedisError) -> Self {
        CacheError::Redis(RunError::User(err))
    }
}

#[derive(Debug, Clone)]
pub enum CacheBackend {
    Redis(Pool<RedisConnectionManager>),
    Disabled,
}

pub struct CacheWrapper<T> {
    backend: CacheBackend,             // Redis connection pool or disabled
    moka_cache: Cache<String, String>, // Moka in-memory cache
    cache_ttl: Duration,               // Time-to-live for Success cache
    error_ttl: Duration,               // Time-to-life for Error cache
    _phantom: std::marker::PhantomData<T>, // Marker for generic type T
}

#[allow(dead_code)]
impl<T> CacheWrapper<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    pub fn new(
        backend: CacheBackend,
        moka_cache: Cache<String, String>,
        cache_ttl_secs: u64,
        error_ttl_secs: u64,
    ) -> Self {
        Self {
            backend,
            moka_cache,
            cache_ttl: Duration::from_secs(cache_ttl_secs),
            error_ttl: Duration::from_secs(error_ttl_secs),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn get_or_fetch<F, Fut, E>(
        &self,
        key: &str,
        fetch_fn: F,
    ) -> Result<T, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Option<T>, E>> + Send,
        E: std::fmt::Display + Into<ApiError>,
    {
        // Check caches first
        if let Some(cached_value) = self.get_from_caches(key).await? {
            return self.handle_cached_value(cached_value).await;
        }

        // Fetch from provided function if not found in caches
        match fetch_fn().await {
            Ok(Some(data)) => {
                let serialized = to_string(&data)?;
                self.cache_value(key, serialized, self.cache_ttl).await?;
                Ok(data)
            }
            Ok(None) => {
                self.cache_not_found(key).await?;
                Err(CacheError::NotFound)
            }
            Err(err) => {
                let api_error: ApiError = err.into();
                self.cache_error(key, &api_error).await?;
                Err(CacheError::CachedError(api_error.status_code(), api_error.message()))
            }
        }
    }

    async fn get_from_caches(&self, key: &str) -> Result<Option<String>, CacheError> {
        if let Some(cached_raw) = self.moka_cache.get(key).await {
            match from_str::<CachedValue>(&cached_raw) {
                Ok(cached) => {
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                    if cached.expires_at > now {
                        return Ok(Some(cached.value));
                    } else {
                        self.moka_cache.invalidate(key).await;
                    }
                }
                Err(_) => {
                    self.moka_cache.invalidate(key).await;
                }
            }
        }

        if let CacheBackend::Redis(pool) = &self.backend {
            let mut conn = pool.get().await?;
            if let Ok(Some(cached_data)) = conn.get::<_, Option<String>>(key).await {
                let ttl_secs = match conn.ttl::<&str, i64>(key).await {
                    Ok(secs) if secs > 0 => secs as u64,
                    _ => self.cache_ttl.as_secs(),
                };

                let expires_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    + Duration::from_secs(ttl_secs.max(2));

                let cached_value = CachedValue {
                    value: cached_data.clone(),
                    expires_at: expires_at.as_secs(),
                };

                if let Ok(serialized) = to_string(&cached_value) {
                    self.moka_cache.insert(key.to_string(), serialized).await;
                }

                return Ok(Some(cached_data));
            }
        }

        Ok(None)
    }

    async fn handle_cached_value(&self, value: String) -> Result<T, CacheError> {
        if value.starts_with("__error__") {
            let stripped = value.trim_start_matches("__error__");
            let parts: Vec<&str> = stripped.splitn(2, "__").collect();

            if parts.len() == 2 {
                let status = parts[0].parse().unwrap_or(500);
                let message = parts[1].to_string();
                return Err(CacheError::CachedError(
                    StatusCode::from_u16(status)
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    message
                ));
            }
            return Err(CacheError::FetchError("Invalid error format".to_string()));
        } else if value == "__not_found__" {
            return Err(CacheError::NotFound);
        }

        from_str(&value).map_err(Into::into)
    }

    async fn cache_value(&self, key: &str, value: String, ttl: Duration) -> Result<(), CacheError> {
        self.moka_cache.insert(key.to_string(), value.clone()).await;
        if let CacheBackend::Redis(pool) = &self.backend {
            let mut conn = pool.get().await?;
            conn.set_ex::<_, _, ()>(key, value, ttl.as_secs()).await?;
        }
        Ok(())
    }

    async fn cache_error(&self, key: &str, error: &ApiError) -> Result<(), CacheError> {
        let error_value = format!("__error__{}__{}",
                                  error.status_code().as_u16(),
                                  error.message());
        self.cache_value(key, error_value, self.error_ttl).await
    }

    pub async fn cache_not_found(&self, key: &str) -> Result<(), CacheError> {
        self.cache_value(key, "__not_found__".to_string(), self.cache_ttl).await
    }

    #[allow(unused)]
    pub async fn set(&self, key: &str, data: &T) -> Result<(), CacheError> {
        let serialized = to_string(data)?;

        if let Some(cached_value) = self.moka_cache.get(key).await {
            if cached_value == serialized {
                return Ok(());
            }
        }

        self.cache_value(key, serialized, self.cache_ttl).await
    }

    #[allow(unused)]
    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        self.moka_cache.invalidate(key).await;
        if let CacheBackend::Redis(pool) = &self.backend {
            let mut conn = pool.get().await?;
            conn.del::<_, ()>(key).await?;
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! cache_fetch {
    ($cache:expr, $key:expr, $fetch_fn:expr) => {
        $cache.get_or_fetch($key, || async {
            $fetch_fn.await
        }).await
    };

    ($cache:expr, $key:expr, $fetch_fn:expr, $error_handler:expr) => {
        $cache.get_or_fetch($key, || async {
            $fetch_fn.await
        }).await.map_err($error_handler)
    };
}
