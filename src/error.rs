use std::fmt;
use std::panic::Location;

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
use hmac::digest::InvalidLength;
use jwt::error::Error as JwtError;
use tracing::{event, Level};

use crate::{
    response::ApiResponse,
    util::cache::CacheError
};

#[derive(Debug)]
#[allow(dead_code)]
pub enum ApiError {
    BadRequest {
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
    Unauthorized {
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
    Forbidden {
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
    NotFound {
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
    Conflict {
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
    InternalServerError {
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
    Redis {
        error: RunError<RedisError>,
        location: &'static Location<'static>,
        module: String,
    },
    Reqwest {
        error: ReqwestError,
        location: &'static Location<'static>,
        module: String,
    },
    Serialization {
        error: SerdeJsonError,
        location: &'static Location<'static>,
        module: String,
    },
    SelectorParseError {
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
    Database {
        error: DbErr,
        location: &'static Location<'static>,
        module: String,
    },
    Anyhow {
        error: AnyhowError,
        location: &'static Location<'static>,
        module: String,
    },
    Cryptographic {
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
    JwtError {
        error: JwtError,
        location: &'static Location<'static>,
        module: String,
    },
    Custom {
        status: StatusCode,
        message: String,
        location: &'static Location<'static>,
        module: String,
    },
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Redis { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Reqwest { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Serialization { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::SelectorParseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Database { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Anyhow { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Cryptographic { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::JwtError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Custom { status, .. } => *status,
        }
    }

    pub fn message(&self) -> String {
        match self {
            ApiError::BadRequest { message, .. } => message.clone(),
            ApiError::Unauthorized { message, .. } => message.clone(),
            ApiError::Forbidden { message, .. } => message.clone(),
            ApiError::NotFound { message, .. } => message.clone(),
            ApiError::Conflict { message, .. } => message.clone(),
            ApiError::InternalServerError { message, .. } => message.clone(),
            ApiError::Redis { error, .. } => format!("redis error: {}", error),
            ApiError::Reqwest { error, .. } => format!("HTTP request error: {}", error),
            ApiError::Serialization { error, .. } => format!("JSON serialization error: {}", error),
            ApiError::SelectorParseError { message, .. } => format!("Selector parse error: {}", message),
            ApiError::Database { error, .. } => format!("Database error: {}", error),
            ApiError::Anyhow { error, .. } => format!("Internal error: {}", error),
            ApiError::Cryptographic { message, .. } => format!("Cryptographic error: {}", message),
            ApiError::JwtError { error, .. } => format!("JWT error: {}", error),
            ApiError::Custom { message, .. } => message.clone(),
        }
    }

    pub fn location(&self) -> &'static Location<'static> {
        match self {
            ApiError::BadRequest { location, .. } => location,
            ApiError::Unauthorized { location, .. } => location,
            ApiError::Forbidden { location, .. } => location,
            ApiError::NotFound { location, .. } => location,
            ApiError::Conflict { location, .. } => location,
            ApiError::InternalServerError { location, .. } => location,
            ApiError::Redis { location, .. } => location,
            ApiError::Reqwest { location, .. } => location,
            ApiError::Serialization { location, .. } => location,
            ApiError::SelectorParseError { location, .. } => location,
            ApiError::Database { location, .. } => location,
            ApiError::Anyhow { location, .. } => location,
            ApiError::Cryptographic { location, .. } => location,
            ApiError::JwtError { location, .. } => location,
            ApiError::Custom { location, .. } => location,
        }
    }

    pub fn module(&self) -> &str {
        match self {
            ApiError::BadRequest { module, .. } => module,
            ApiError::Unauthorized { module, .. } => module,
            ApiError::Forbidden { module, .. } => module,
            ApiError::NotFound { module, .. } => module,
            ApiError::Conflict { module, .. } => module,
            ApiError::InternalServerError { module, .. } => module,
            ApiError::Redis { module, .. } => module,
            ApiError::Reqwest { module, .. } => module,
            ApiError::Serialization { module, .. } => module,
            ApiError::SelectorParseError { module, .. } => module,
            ApiError::Database { module, .. } => module,
            ApiError::Anyhow { module, .. } => module,
            ApiError::Cryptographic { module, .. } => module,
            ApiError::JwtError { module, .. } => module,
            ApiError::Custom { module, .. } => module,
        }
    }

    fn log_error(&self) {
        let status = self.status_code();
        let message = self.message();
        let location = self.location();
        let module = self.module();

        event!(
            Level::ERROR,
            status = status.as_u16(),
            error_type = ?std::any::type_name::<Self>(),
            message = %message,
            module = %module,
            file = %location.file(),
            line = %location.line(),
            "API Error occurred"
        );
    }
}

// Макрос для удобного создания ошибок
#[macro_export]
macro_rules! api_error {
    ($error_type:ident) => {
        $crate::error::ApiError::$error_type {
            message: stringify!($error_type).to_string(),
            location: std::panic::Location::caller(),
            module: module_path!().to_string(),
        }
    };
    ($error_type:ident, $msg:expr) => {
        $crate::error::ApiError::$error_type {
            message: $msg.to_string(),
            location: std::panic::Location::caller(),
            module: module_path!().to_string(),
        }
    };
}

#[allow(dead_code)]
impl ApiError {
    #[track_caller]
    pub fn bad_request() -> Self {
        ApiError::BadRequest {
            message: "bad request".to_string(),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }

    #[track_caller]
    pub fn unauthorized() -> Self {
        ApiError::Unauthorized {
            message: "unauthorized".to_string(),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }

    #[track_caller]
    pub fn forbidden() -> Self {
        ApiError::Forbidden {
            message: "forbidden".to_string(),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }

    #[track_caller]
    pub fn not_found(message: &str) -> Self {
        ApiError::NotFound {
            message: message.to_string(),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }

    #[track_caller]
    pub fn conflict(message: &str) -> Self {
        ApiError::Conflict {
            message: message.to_string(),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }

    #[track_caller]
    pub fn internal_error(message: &str) -> Self {
        ApiError::InternalServerError {
            message: message.to_string(),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

// Реализации From с сохранением location

impl From<RedisError> for ApiError {
    #[track_caller]
    fn from(error: RedisError) -> Self {
        ApiError::Redis {
            error: RunError::User(error),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<DbErr> for ApiError {
    #[track_caller]
    fn from(error: DbErr) -> Self {
        ApiError::Database {
            error,
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<AnyhowError> for ApiError {
    #[track_caller]
    fn from(error: AnyhowError) -> Self {
        ApiError::Anyhow {
            error,
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<InvalidLength> for ApiError {
    #[track_caller]
    fn from(error: InvalidLength) -> Self {
        ApiError::Cryptographic {
            message: format!("Invalid length: {}", error),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<JwtError> for ApiError {
    #[track_caller]
    fn from(error: JwtError) -> Self {
        ApiError::JwtError {
            error,
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<ReqwestError> for ApiError {
    #[track_caller]
    fn from(error: ReqwestError) -> Self {
        ApiError::Reqwest {
            error,
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<SerdeJsonError> for ApiError {
    #[track_caller]
    fn from(error: SerdeJsonError) -> Self {
        ApiError::Serialization {
            error,
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<QueryRejection> for ApiError {
    #[track_caller]
    fn from(error: QueryRejection) -> Self {
        ApiError::Custom {
            status: StatusCode::BAD_REQUEST,
            message: error.body_text(),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<PathRejection> for ApiError {
    #[track_caller]
    fn from(error: PathRejection) -> Self {
        ApiError::Custom {
            status: StatusCode::BAD_REQUEST,
            message: error.body_text(),
            location: Location::caller(),
            module: module_path!().to_string(),
        }
    }
}

impl From<CacheError> for ApiError {
    #[track_caller]
    fn from(err: CacheError) -> Self {
        match err {
            CacheError::Redis(e) => ApiError::Redis {
                error: e,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            CacheError::Serialization(e) => ApiError::Serialization {
                error: e,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            CacheError::NotFound => ApiError::NotFound {
                message: "Resource not found".to_string(),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            CacheError::FetchError(e) => ApiError::Custom {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: e.to_string(),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            CacheError::CachedError(c, e) => ApiError::Custom {
                status: c,
                message: e.to_string(),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        self.log_error();

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
