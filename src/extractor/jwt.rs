use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use base64::{engine::general_purpose, Engine as _};
use axum::http::header::AUTHORIZATION;
use serde::Deserialize;
use crate::api_error;
use crate::error::ApiError;

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
        let auth_header = parts.headers
            .get(AUTHORIZATION)
            .ok_or(api_error!(Unauthorized))?
            .to_str()
            .map_err(|_| api_error!(Unauthorized))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(api_error!(Unauthorized));
        }

        let token = auth_header[7..].to_string();

        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(api_error!(Unauthorized));
        }

        let payload = parts[1];

        let decoded_bytes = general_purpose::URL_SAFE_NO_PAD.decode(payload)
            .map_err(|_| api_error!(Unauthorized))?;

        let data: JwtPayload = serde_json::from_slice(&decoded_bytes)?;
        let user_id = data.sub.parse::<uuid::Uuid>().map_err(|_| api_error!(BadRequest))?;
        Ok(JWTExtractor(user_id))
    }
}
