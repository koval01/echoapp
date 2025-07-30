use std::env;

use axum::{
    routing::{get},
    Router,
};
use axum_messages::MessagesManagerLayer;

use sea_orm::sqlx::PgPool;
use tower_http::services::ServeDir;
use tower_sessions::{CachingSessionStore, SessionManagerLayer};
use tower_sessions_moka_store::MokaStore;
use tower_sessions_sqlx_store::PostgresStore;

use crate::{
    handler::{
        health_checker_handler,
        view::{
            notfound::handler_404,
            home::home_handler
        },
    },
};

pub async fn create_router() -> Router {
    let assets_path = env::current_dir().unwrap();

    let public_routes = Router::new()
        .route("/healthz", get(health_checker_handler));

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost".to_string());
    let pool = PgPool::connect(&database_url).await.unwrap();
    let session_store = PostgresStore::new(pool);
    let moka_store = MokaStore::new(Some(2_000));
    let session_store = CachingSessionStore::new(moka_store, session_store);
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let pages_router = Router::new()
        .route("/", get(home_handler))
        // .route(
        //     "/register",
        //     get(register_page_handler).post(register_user_handler),
        // )
        // .route("/login", get(login_page_handler).post(login_user_handler))
        // .route(
        //     "/todo/list",
        //     get(todo_list_handler)
        //         .route_layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        // )
        // .route(
        //     "/logout",
        //     post(logout_handler)
        //         .route_layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        // )
        // .route(
        //     "/create",
        //     get(todo_create_handler)
        //         .post(todo_add_handler)
        //         .route_layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        // )
        // .route(
        //     "/edit",
        //     get(todo_edit_handler)
        //         .patch(todo_patch_handler)
        //         .route_layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        // )
        // .route("/delete", delete(todo_delete_handler))
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", assets_path.to_str().unwrap())),
        )
        .fallback(handler_404)
        .layer(MessagesManagerLayer)
        .layer(session_layer);

    // Merge routes and add shared state and fallback
    Router::new()
        .merge(public_routes)
        .merge(pages_router)
}
