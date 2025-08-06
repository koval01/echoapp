use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use axum::extract::Path;

use crate::error::ApiError;

pub struct StrictI64(pub i64);

impl<S> FromRequestParts<S> for StrictI64
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(s) = Path::<String>::from_request_parts(parts, &())
            .await
            .map_err(|e| ApiError::Conflict(e.to_string()))?;

        // Strict validation
        if let Ok(num) = s.parse::<i64>() {
            // Ensure the string representation matches exactly
            if num.to_string() == s {
                Ok(StrictI64(num))
            } else {
                Err(ApiError::BadRequestWithMessage("Invalid integer format".to_string()))
            }
        } else {
            Err(ApiError::BadRequestWithMessage("Invalid integer".to_string()))
        }
    }
}