use axum::{response::IntoResponse, http::StatusCode, Json, Extension};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::{
    error::ApiError,
    model::user::User,
    response::{ApiResponse},
    extractor::InitData,
};
use crate::service::get_user_by_id;

/// Handles GET requests for the authenticated user's profile
pub async fn user_handler_get(
    InitData(user): InitData<User>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<impl IntoResponse, ApiError> {
    let user = get_user_by_id(user.id, &db).await;

    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}
