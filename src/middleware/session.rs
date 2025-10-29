use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request},
    middleware::Next,
    response::{IntoResponse},
};
use axum::extract::State;
use jwt::Error;
use rand::random;
use tokio::sync::RwLock;
use crate::{api_error, AppState};
use crate::error::{ApiError, RequestCtx};
use crate::service::{fetch_user_with_cache, JwtService};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthUser {
    pub user_id: uuid::Uuid,
    pub session_id: String,
}

pub async fn validate_jwt_middleware(
    State(state): State<Arc<RwLock<AppState>>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, ApiError> {
    let state = state.read().await;
    let ctx = req.extensions().get::<RequestCtx>().cloned().unwrap();
    let header_auth = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(api_error!(Unauthorized).with_ctx(ctx.clone()))?;
    
    let jwt_service = JwtService::new(&state.config.jwt_secret)
        .map_err(|_| api_error!(InternalServerError, "JWT service error").with_ctx(ctx.clone()))?;

    let token = extract_bearer_token(header_auth).ok_or(api_error!(Unauthorized).with_ctx(ctx.clone()))?;

    match jwt_service.validate_token(&token) {
        Ok(claims) => {
            let user_model = fetch_user_with_cache(
                claims.user_id, &state.shared_db, state.redis_backend.clone(), state.moka_cache.clone()
            ).await?;

            if user_model.is_banned {
                return Err(api_error!(Forbidden, "User is banned").with_ctx(ctx));
            }

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
            Err(api_error!(Unauthorized).with_ctx(ctx))
        }
        Err(_) => {
            Err(api_error!(Unauthorized).with_ctx(ctx))
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
