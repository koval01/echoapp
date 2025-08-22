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
use crate::handler::{user_by_id_handler_get, user_handler_get};
use crate::middleware::{sync_user_middleware, validate_middleware};

pub fn create_router(app_state: Arc<RwLock<AppState>>) -> Router {
    // Routes without auth middleware
    let public_routes = Router::new()
        .route("/healthz", get(health_checker_handler));

    /* TODO: protected_middlewares будет telegram_middleware, для обработки x-initdata,
        и создания access token + refresh token, то есть только для одного вызова, при запуске приложения.
        То есть будет route типа /v1/auth/init, который вернет в json access token и refresh token.
        А protected_middlewares станет middleware который проверяет access token, то есть
        поменяется только ожидаемый заголовок, вместо X-InitData нужно уже будет передавать cookies.
        Для refresh token можно выделить свой middleware, с route типа /v1/auth/refresh.
    */

    let protected_middlewares = ServiceBuilder::new()
        .layer(axum::middleware::from_fn_with_state(app_state.clone(), validate_middleware))
        .layer(axum::middleware::from_fn(sync_user_middleware))
        .into_inner();

    // Routes with telegram's auth middleware
    let protected_routes = Router::new()
        .route(
            "/v1/auth/init",
            get(user_handler_get)
        )
        .route(
            "/v1/auth/refresh",
            get(user_by_id_handler_get)
        )
        .layer(
            protected_middlewares
        );

    // Routes with auth middleware
    // let protected_routes = Router::new()
    //     .route(
    //         "/v1/user/me",
    //         get(user_handler_get)
    //     )
    //     .route(
    //         "/v1/user/{user_id}",
    //         get(user_by_id_handler_get)
    //     )
    //     .layer(
    //         protected_middlewares
    //     );

    // Merge routes and add shared state and fallback
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(app_state)
        .fallback(|| async { ApiError::NotFound("not found".to_string()).into_response() })
}
