use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request},
    middleware::Next,
    response::{IntoResponse},
};
use axum::extract::State;
use tokio::sync::RwLock;
use crate::AppState;
use crate::error::ApiError;

pub async fn validate_middleware(
    State(state): State<Arc<RwLock<AppState>>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, ApiError> {
    let state = state.read().await;
    let init_data = req
        .headers()
        .get("X-InitData")
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let decoded_init_data = urlencoding::decode(init_data)
        .map_err(|_| ApiError::Unauthorized)?
        .into_owned();

    match crate::util::validator::validate_init_data(&decoded_init_data, &state.config.bot_token) {
        Ok(true) => {
            req.extensions_mut().insert(decoded_init_data);
            Ok(next.run(req).await)
        },
        Ok(false) => Err(ApiError::Unauthorized),
        Err(_) => Err(ApiError::BadRequest),
    }
}
