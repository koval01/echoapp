use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use base64::{engine::general_purpose, Engine as _};
use axum::http::header::AUTHORIZATION;
use serde::Deserialize;
use crate::api_error;
use crate::error::{ApiError, RequestCtx};

#[derive(Deserialize, Debug)]
struct JwtPayload {
    pub sub: String,
}

pub struct JWTExtractor(pub uuid::Uuid);

impl<S> FromRequestParts<S> for JWTExtractor
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        macro_rules! ctx_err {
            ($error:expr) => {
                if let Some(ctx) = parts.extensions.get::<RequestCtx>() {
                    $error.with_ctx(ctx.clone())
                } else {
                    $error
                }
            };
        }

        let auth_header = parts.headers
            .get(AUTHORIZATION)
            .ok_or_else(|| ctx_err!(api_error!(Unauthorized)))?
            .to_str()
            .map_err(|_| ctx_err!(api_error!(Unauthorized)))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ctx_err!(api_error!(Unauthorized)));
        }

        let token = auth_header[7..].to_string();

        let token_parts: Vec<&str> = token.split('.').collect();
        if token_parts.len() != 3 {
            return Err(ctx_err!(api_error!(Unauthorized)));
        }

        let payload = token_parts[1];

        let decoded_bytes = general_purpose::URL_SAFE_NO_PAD.decode(payload)
            .map_err(|_| ctx_err!(api_error!(Unauthorized)))?;

        let data: JwtPayload = serde_json::from_slice(&decoded_bytes).map_err(|e| {
            ctx_err!(api_error!(Unauthorized, &format!("Invalid JWT payload: {}", e)))
        })?;

        let user_id = data.sub.parse::<uuid::Uuid>().map_err(|_| {
            ctx_err!(api_error!(BadRequest, "Invalid user ID format in JWT"))
        })?;

        Ok(JWTExtractor(user_id))
    }
}
