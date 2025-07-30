mod route;
mod middleware;
mod error;
mod handler;
mod model;
mod response;
mod util;
mod entities;
mod database;
mod config;

use std::env;
use std::sync::Arc;
use std::time::Duration;

use bb8_redis::RedisConnectionManager;
use bb8_redis::bb8;

use redis::AsyncCommands;
use moka::future::Cache;
use reqwest::ClientBuilder;

use axum::{
    http::{header::{ACCEPT, CONTENT_TYPE}, HeaderName, HeaderValue, Method},
    extract::Extension,
};
use route::create_router;

use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::{CompressionLayer, DefaultPredicate}
};

use sentry::{ClientOptions, IntoDsn};
use sentry_tower::NewSentryLayer;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[allow(warnings, unused)]
use crate::middleware::{request_id_middleware, process_time_middleware};
use crate::util::cache::CacheBackend;

use migration::{Migrator, MigratorTrait};
use crate::config::Config;

pub struct AppState {
    pub config: Config,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .parse("panel=info,tower_http=info")
                .unwrap()
        )
        .with_span_events(fmt::format::FmtSpan::CLOSE)
        .with_ansi(false)
        .init();

    let _dsn = env::var("SENTRY_DSN").unwrap_or_else(|_| "".to_string());
    let _guard = sentry::init((
        _dsn.into_dsn().unwrap(),
        ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 0.2,
            ..Default::default()
        },
    ));

    let cors_host = env::var("CORS_HOST").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let cors = CorsLayer::new()
        .allow_origin(cors_host.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([
            ACCEPT,
            CONTENT_TYPE,
            HeaderName::from_static("x-timestamp"),
        ]);

    let predicate = DefaultPredicate::new();
    let compression_layer: CompressionLayer = CompressionLayer::new()
        .br(true)
        .deflate(true)
        .gzip(true)
        .zstd(true)
        .compress_when(predicate);

    let moka_cache: Cache<String, String> = Cache::builder()
        .time_to_live(Duration::from_secs(10))
        .max_capacity(16_000)
        .build();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    let db = database::establish_connection(&database_url)
        .await
        .expect("Failed to connect to database");
    let shared_db = Arc::new(db);
    Migrator::up(&connection, None).await?;

    let redis_backend = if let Ok(redis_url) = env::var("REDIS_URL") {
        let redis_manager = RedisConnectionManager::new(redis_url).unwrap();
        let redis_pool = bb8::Pool::builder()
            .max_size((num_cpus::get() * 10) as u32)
            .min_idle((num_cpus::get() * 2 + 1) as u32)
            .max_lifetime(None)
            .connection_timeout(Duration::from_millis(2000))
            .idle_timeout(Some(Duration::from_secs(60)))
            .build(redis_manager)
            .await
            .unwrap();

        // Perform health check
        {
            let mut conn = redis_pool.get().await.unwrap();
            let _: () = conn.set("health_check", "ok").await.unwrap();
        }

        CacheBackend::Redis(redis_pool)
    } else {
        CacheBackend::Disabled
    };

    let http_client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(60))
        .user_agent(format!("{}/{} (https://github.com/koval01/{}; yaroslav@koval.page)", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME")))
        .gzip(true)
        .build()
        .expect("Failed to create HTTP client");

    let middleware_stack = ServiceBuilder::new()
        .layer(NewSentryLayer::new_from_top())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(tower::limit::ConcurrencyLimitLayer::new(1000));

    let middleware_stack = middleware_stack
        .layer(axum::middleware::from_fn(process_time_middleware))
        .layer(axum::middleware::from_fn(request_id_middleware));

    let app = create_router().await
        .layer(middleware_stack)
        .layer(compression_layer)
        .layer(Extension(shared_db))
        .layer(Extension(redis_backend))
        .layer(Extension(moka_cache))
        .layer(Extension(http_client));

    let _bind = env::var("SERVER_BIND").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = tokio::net::TcpListener::bind(&_bind)
        .await
        .unwrap();

    info!("🚀 Server started successfully on {}", _bind);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>()
    )
        .await
        .unwrap();
}
