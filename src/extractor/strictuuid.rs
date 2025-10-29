use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use axum::extract::Path;

use uuid::Uuid;
use crate::api_error;
use crate::error::ApiError;

pub struct StrictUuid(pub Uuid);

impl<S> FromRequestParts<S> for StrictUuid
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

        // Parse as UUID and validate the string representation matches exactly
        Uuid::parse_str(&s)
            .map(StrictUuid)
            .map_err(|_| api_error!(BadRequest, "Invalid UUID format"))
    }
}
