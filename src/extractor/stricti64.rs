use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use axum::extract::Path;
use crate::api_error;
use crate::error::ApiError;

#[allow(dead_code)]
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
            .map_err(|e| api_error!(Conflict, e.to_string()))?;

        // Strict validation
        if let Ok(num) = s.parse::<i64>() {
            // Ensure the string representation matches exactly
            if num.to_string() == s {
                Ok(StrictI64(num))
            } else {
                Err(api_error!(BadRequest, "Invalid integer format"))
            }
        } else {
            Err(api_error!(BadRequest, "Invalid integer"))
        }
    }
}