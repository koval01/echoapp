use axum::{
    routing::{get},
    Router,
    response::IntoResponse,
};

use crate::{
    handler::{
        health_checker_handler,
    },
    error::ApiError,
};

pub fn create_router() -> Router {
    // Routes without middleware
    let public_routes = Router::new()
        .route("/healthz", get(health_checker_handler));

    // Merge routes and add shared state and fallback
    Router::new()
        .merge(public_routes)
        .fallback(|| async { ApiError::NotFound("not found".to_string()).into_response() })
}
