use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use axum::http::header::AUTHORIZATION;
use crate::error::ApiError;

pub struct JWTExtractor(pub String);

impl<S> FromRequestParts<S> for JWTExtractor
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers
            .get(AUTHORIZATION)
            .ok_or(ApiError::Unauthorized)?
            .to_str()
            .map_err(|_| ApiError::Unauthorized)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::Unauthorized);
        }

        let token = auth_header[7..].to_string();

        Ok(JWTExtractor(token))
    }
}
