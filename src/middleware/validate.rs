use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request},
    middleware::Next,
    response::{IntoResponse},
};
use axum::extract::State;
use axum::http::StatusCode;
use jwt::Error;
use rand::random;
use tokio::sync::RwLock;
use crate::AppState;
use crate::error::ApiError;
use crate::service::JwtService;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub session_id: String,
}

pub async fn validate_initdata_middleware(
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

pub async fn validate_jwt_middleware(
    State(state): State<Arc<RwLock<AppState>>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, ApiError> {
    let state = state.read().await;
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let jwt_service = JwtService::new(&state.config.jwt_secret)
        .map_err(|_| ApiError::Custom(StatusCode::INTERNAL_SERVER_ERROR, "JWT error".into()))?;

    match jwt_service.validate_token(&token) {
        Ok(claims) => {
            // Add user info to request extensions
            let auth_user = AuthUser {
                user_id: claims.user_id,
                session_id: generate_session_id(),
            };

            req.extensions_mut().insert(auth_user);
            req.extensions_mut().insert(claims);

            drop(state);
            Ok(next.run(req).await)
        }
        Err(Error::InvalidSignature) => {
            Err(ApiError::Unauthorized)
        }
        Err(_) => {
            Err(ApiError::BadRequest)
        }
    }
}

fn extract_bearer_token(header: &str) -> Option<String> {
    if header.starts_with("Bearer ") {
        Some(header[7..].trim().to_string())
    } else if header.starts_with("bearer ") {
        Some(header[7..].trim().to_string())
    } else {
        None
    }
}

fn generate_session_id() -> String {
    let random_bytes: [u8; 16] = random();
    hex::encode(random_bytes)
}
