use axum::{response::IntoResponse, Json};

use crate::{
    response::ApiResponse
};

pub async fn health_checker_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Hello from duolang core!";
    let response: ApiResponse<()> = ApiResponse::message_only(Some(MESSAGE));
    Json(response)
}
