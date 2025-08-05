use axum::{
    extract::Path,
    response::IntoResponse,
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{
    error::ApiError,
    model::user::User,
    response::{ApiResponse},
    extractor::InitData,
};

/// Handles GET requests for the authenticated user's profile
pub async fn user_handler_get(
    InitData(user): InitData<User>,
) -> Result<impl IntoResponse, ApiError> {

    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}
