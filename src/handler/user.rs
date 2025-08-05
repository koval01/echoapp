use axum::{response::IntoResponse, http::StatusCode, Json, Extension};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::{
    error::ApiError,
    model::user::User,
    response::{ApiResponse},
    extractor::InitData,
};
use crate::extractor::StrictI64;
use crate::service::get_user_by_id;

pub async fn user_handler_get(
    InitData(user): InitData<User>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<impl IntoResponse, ApiError> {
    let user = get_user_by_id(user.id, &db)
        .await
        .map_err(|e| ApiError::from(e))?;

    let user = user.ok_or(ApiError::NotFound("User not found".to_string()))?;

    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}

pub async fn user_by_id_handler_get(
    StrictI64(user_id): StrictI64,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<impl IntoResponse, ApiError> {
    let user = get_user_by_id(user_id, &db)
        .await
        .map_err(|e| ApiError::from(e))?;

    let user = user.ok_or(ApiError::NotFound("User not found".to_string()))?;

    let response = ApiResponse::success(user);
    Ok((StatusCode::OK, Json(response)))
}
