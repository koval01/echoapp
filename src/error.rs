use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    extract::rejection::QueryRejection,
    Json,
};
use axum::extract::rejection::PathRejection;

use bb8::RunError;
use redis::RedisError;

use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use sea_orm::DbErr;
use anyhow::Error as AnyhowError;

use tracing::debug;

use crate::{
    response::ApiResponse, 
    util::cache::CacheError
};

#[derive(Debug)]
#[allow(dead_code)]
pub enum ApiError {
    BadRequest,
    BadRequestWithMessage(String),
    Unauthorized,
    Forbidden,
    NotFound(String),
    Conflict(String),
    InternalServerError(String),
    Redis(RunError<RedisError>),
    Reqwest(ReqwestError),
    Serialization(SerdeJsonError),
    SelectorParseError(String),
    Database(DbErr),
    Anyhow(AnyhowError),
    Custom(StatusCode, String),
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest => StatusCode::BAD_REQUEST,
            ApiError::BadRequestWithMessage(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Reqwest(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::SelectorParseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Custom(code, _) => *code,
        }
    }

    pub fn message(&self) -> String {
        match self {
            ApiError::BadRequest => "bad request".to_string(),
            ApiError::BadRequestWithMessage(msg) => msg.clone(),
            ApiError::Unauthorized => "unauthorized".to_string(),
            ApiError::Forbidden => "forbidden".to_string(),
            ApiError::NotFound(error) => if error.is_empty() { "not found".to_string() } else { error.clone() },
            ApiError::Conflict(error) => if error.is_empty() { "conflict".to_string() } else { error.clone() },
            ApiError::InternalServerError(error) => if error.is_empty() { "internal error".to_string() } else { error.clone() },
            ApiError::Redis(error) => format!("redis error: {}", error),
            ApiError::Reqwest(error) => format!("HTTP request error: {}", error),
            ApiError::Serialization(error) => format!("JSON serialization error: {}", error),
            ApiError::SelectorParseError(error) => format!("Selector parse error: {}", error),
            ApiError::Database(error) => format!("Database error: {}", error),
            ApiError::Anyhow(error) => format!("Internal error: {}", error),
            ApiError::Custom(_, message) => message.clone(),
        }
    }
}

impl From<fn() -> ApiError> for ApiError {
    fn from(_: fn() -> ApiError) -> Self {
        ApiError::BadRequest
    }
}

impl From<RedisError> for ApiError {
    fn from(error: RedisError) -> Self {
        debug!("{:#?}", error);
        ApiError::Redis(RunError::User(error))
    }
}

impl From<DbErr> for ApiError {
    fn from(error: DbErr) -> Self {
        debug!("Database error: {:#?}", error);
        ApiError::Database(error)
    }
}

impl From<AnyhowError> for ApiError {
    fn from(error: AnyhowError) -> Self {
        debug!("Anyhow error: {:#?}", error);
        ApiError::Anyhow(error)
    }
}

impl From<ReqwestError> for ApiError {
    fn from(error: ReqwestError) -> Self {
        debug!("Reqwest error: {:#?}", error);
        ApiError::Reqwest(error)
    }
}

impl From<SerdeJsonError> for ApiError {
    fn from(error: SerdeJsonError) -> Self {
        debug!("Serialization error: {:#?}", error);
        ApiError::Serialization(error)
    }
}

impl From<QueryRejection> for ApiError {
    fn from(error: QueryRejection) -> Self {
        debug!("{:#?}", error);
        ApiError::Custom(StatusCode::BAD_REQUEST, error.body_text()) 
    }
}

impl From<PathRejection> for ApiError {
    fn from(error: PathRejection) -> Self {
        debug!("{:#?}", error);
        ApiError::Custom(StatusCode::BAD_REQUEST, error.body_text())
    }
}

impl From<CacheError> for ApiError {
    fn from(err: CacheError) -> Self {
        match err {
            CacheError::Redis(e) => ApiError::Redis(e),
            CacheError::Serialization(e) => ApiError::Serialization(e),
            CacheError::NotFound => ApiError::NotFound("Resource not found".to_string()),
            CacheError::FetchError(e) => ApiError::Custom(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            CacheError::CachedError(c, e) => ApiError::Custom(c, e.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = self.message();
        let response = ApiResponse::<()>::error(Some(&message), status);
        (status, Json(response)).into_response()
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
