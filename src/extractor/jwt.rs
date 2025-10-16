use std::sync::Arc;
use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    http::header::AUTHORIZATION,
};
use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use crate::AppState;
use crate::error::ApiError;
use crate::service::JwtService;

pub struct JWTExtractor<T>(pub T);

impl<T> FromRequestParts<AppState> for JWTExtractor<T>
where
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers
            .get(AUTHORIZATION)
            .ok_or_else(|| ApiError::Unauthorized)?
            .to_str()
            .map_err(|_| ApiError::Unauthorized)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::Unauthorized);
        }

        let token = &auth_header[7..].to_string();

        let jwt_service = JwtService::new(&state.config.jwt_secret)
            .map_err(|_| ApiError::Custom(StatusCode::INTERNAL_SERVER_ERROR, "JWT error".into()))?;

        let claims_value = jwt_service.validate_token_to_value(token)
            .map_err(|_| ApiError::Custom(StatusCode::INTERNAL_SERVER_ERROR, "JWT error".into()))?;

        let data: T = serde_json::from_value(claims_value)
            .map_err(|_| ApiError::Custom(StatusCode::INTERNAL_SERVER_ERROR, "Invalid token claims".into()))?;

        Ok(JWTExtractor(data))
    }
}