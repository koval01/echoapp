mod route;
mod middleware;
mod error;
mod handler;
mod model;
mod response;
mod util;
mod database;
mod config;
mod service;
mod extractor;

use std::env;
use std::sync::Arc;
use std::time::Duration;

use bb8_redis::RedisConnectionManager;
use bb8_redis::bb8;

use redis::AsyncCommands;
use moka::future::Cache;
use reqwest::ClientBuilder;

use axum::http::{header::{ACCEPT, CONTENT_TYPE}, HeaderName, HeaderValue, Method, Request};
use axum::extract::Extension;
use axum::body::Body;
use sea_orm::DatabaseConnection;
use route::create_router;

use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    classify::ServerErrorsFailureClass,
    compression::{CompressionLayer, DefaultPredicate},
};

use sentry::{ClientOptions, IntoDsn};
use sentry_tower::NewSentryLayer;

use tokio::sync::RwLock;

use hostname::get;

use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use migration::{Migrator, MigratorTrait};

use crate::{
    config::Config,
    util::cache::CacheBackend,
    middleware::{request_id_middleware, process_time_middleware}
};
use crate::util::telegram_logging::init_telegram_logging;

pub struct AppState {
    pub config: Config,
    pub shared_db: Arc<DatabaseConnection>,
    pub redis_backend: CacheBackend,
    pub moka_cache: Cache<String, String>
}

#[tokio::main]
async fn main() {
    let config = Config::init();

    let telegram_layer = init_telegram_logging(
        config.bot_token.clone(),
        config.telegram_chat_id.clone()
    );

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_span_events(fmt::format::FmtSpan::FULL)
        .with_ansi(false)
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);

    let filter = EnvFilter::builder()
        .with_default_directive(tracing::Level::WARN.into())
        .parse("echoapp=info,echoapp::error=off,echoapp::middleware=info,sqlx::query=warn,tower_http=warn")
        .unwrap();

    match telegram_layer {
        Some(tg_layer) => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .with(tg_layer)
                .init();
        }
        None => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .init();
        }
    }

    let _guard = sentry::init((
        config.sentry_dsn.clone().into_dsn().unwrap(),
        ClientOptions {
            release: sentry::release_name!(),
            send_default_pii: true,
            ..Default::default()
        },
    ));

    let cors = CorsLayer::new()
        .allow_origin(config.cors_host.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_credentials(true)
        .allow_headers([
            ACCEPT,
            CONTENT_TYPE,
            HeaderName::from_static("x-initdata")
        ]);

    let predicate = DefaultPredicate::new();
    let compression_layer: CompressionLayer = CompressionLayer::new()
        .br(true)
        .deflate(true)
        .gzip(true)
        .zstd(true)
        .compress_when(predicate);

    let moka_cache: Cache<String, String> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(24_000)
        .build();

    let db = database::establish_connection(&config.database_url)
        .await
        .expect("Failed to connect to database");
    let _ = Migrator::up(&db, None).await.unwrap();
    let shared_db = Arc::new(db);

    let redis_backend = if let Ok(redis_url) = config.redis_url.clone() {
        let redis_manager = RedisConnectionManager::new(redis_url).unwrap();
        let redis_pool = bb8::Pool::builder()
            .max_size((num_cpus::get() * 10) as u32)
            .min_idle((num_cpus::get() * 2 + 1) as u32)
            .max_lifetime(None)
            .connection_timeout(Duration::from_millis(3000))
            .idle_timeout(Some(Duration::from_secs(120)))
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
        .pool_max_idle_per_host(num_cpus::get() * 10)
        .pool_idle_timeout(Duration::from_secs(60))
        .user_agent(format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
        .gzip(true)
        .build()
        .expect("Failed to create HTTP client");

    let trace_layer = TraceLayer::new_for_http()
        .on_failure(
            |error: ServerErrorsFailureClass, latency: Duration, _span: &tracing::Span| {
                tracing::error!(
                "Error request processing (latency: {:?}): {:?}",
                latency,
                error
            );
            },
        );

    let middleware_stack = ServiceBuilder::new()
        .layer(NewSentryLayer::<Request<Body>>::new_from_top())
        .layer(trace_layer)
        .layer(cors)
        .layer(tower::limit::ConcurrencyLimitLayer::new(1024));

    let middleware_stack = middleware_stack
        .layer(axum::middleware::from_fn(process_time_middleware))
        .layer(axum::middleware::from_fn(request_id_middleware));

    let _bind = config.server_bind_addr.clone();
    let app_state = Arc::new(RwLock::new(AppState {
        config,
        shared_db: shared_db.clone(),
        redis_backend: redis_backend.clone(),
        moka_cache: moka_cache.clone()
    }));

    let app = create_router(app_state)
        .layer(middleware_stack)
        .layer(compression_layer)
        .layer(Extension(shared_db))
        .layer(Extension(redis_backend))
        .layer(Extension(moka_cache))
        .layer(Extension(http_client));

    let listener = tokio::net::TcpListener::bind(&_bind)
        .await
        .unwrap();

    let instance = get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    info!("Server started on instance {} successfully on {}", instance, _bind);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>()
    )
        .await
        .unwrap();
}
