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
    #[allow(dead_code)]
    pub user_id: uuid::Uuid,
    #[allow(dead_code)]
    pub session_id: String,
}

pub async fn validate_jwt_middleware(
    State(state): State<Arc<RwLock<AppState>>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, ApiError> {
    let state = state.read().await;
    let header_auth = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let jwt_service = JwtService::new(&state.config.jwt_secret)
        .map_err(|_| ApiError::Custom(StatusCode::INTERNAL_SERVER_ERROR, "JWT error".into()))?;

    let token = extract_bearer_token(header_auth).unwrap();

    match jwt_service.validate_token(&token) {
        Ok(claims) => {
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
