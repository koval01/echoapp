use std::sync::Arc;
use axum::{
    routing::{get},
    Router,
    response::IntoResponse,
};
use tokio::sync::RwLock;
use tower::ServiceBuilder;

use crate::{handler::{
    health_checker_handler,
}, error::ApiError, AppState};
use crate::middleware::validate_middleware;

pub fn create_router(app_state: Arc<RwLock<AppState>>) -> Router {
    // Routes without auth middleware
    let public_routes = Router::new()
        .route("/healthz", get(health_checker_handler));

    let protected_middlewares = ServiceBuilder::new()
        .layer(axum::middleware::from_fn(validate_middleware))
        .into_inner();

    // Routes with auth middleware
    let protected_routes = Router::new()
        // .route(
        //     "/v1/preview/{channel}",
        //     get(channel_preview_handler_get)
        // )
        // .route(
        //     "/v1/body/{channel}",
        //     get(channel_body_handler_get)
        // )
        .layer(
            protected_middlewares
        );

    // Merge routes and add shared state and fallback
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(app_state)
        .fallback(|| async { ApiError::NotFound("not found".to_string()).into_response() })
}
