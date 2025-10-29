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

#[derive(Debug, Clone)]
pub struct RequestCtx {
    pub id: String,
    pub method: String,
    pub path: String,
    pub uri: String,
    pub instance: String,
}

#[derive(Debug)]
pub struct ApiError {
    pub inner: ApiErrorType,
    pub ctx: Option<RequestCtx>,
}

#[derive(Debug)]
pub enum ApiErrorType {
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
        match &self.inner {
            ApiErrorType::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiErrorType::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ApiErrorType::Forbidden { .. } => StatusCode::FORBIDDEN,
            ApiErrorType::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiErrorType::Conflict { .. } => StatusCode::CONFLICT,
            ApiErrorType::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::Redis { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::Reqwest { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::Serialization { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::Database { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::Anyhow { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::Cryptographic { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::JwtError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::Custom { status, .. } => *status,
        }
    }

    pub fn message(&self) -> String {
        match &self.inner {
            ApiErrorType::BadRequest { message, .. } => message.clone(),
            ApiErrorType::Unauthorized { message, .. } => message.clone(),
            ApiErrorType::Forbidden { message, .. } => message.clone(),
            ApiErrorType::NotFound { message, .. } => message.clone(),
            ApiErrorType::Conflict { message, .. } => message.clone(),
            ApiErrorType::InternalServerError { message, .. } => message.clone(),
            ApiErrorType::Redis { error, .. } => format!("redis error: {}", error),
            ApiErrorType::Reqwest { error, .. } => format!("HTTP request error: {}", error),
            ApiErrorType::Serialization { error, .. } => format!("JSON serialization error: {}", error),
            ApiErrorType::Database { error, .. } => format!("Database error: {}", error),
            ApiErrorType::Anyhow { error, .. } => format!("Internal error: {}", error),
            ApiErrorType::Cryptographic { message, .. } => format!("Cryptographic error: {}", message),
            ApiErrorType::JwtError { error, .. } => format!("JWT error: {}", error),
            ApiErrorType::Custom { message, .. } => message.clone(),
        }
    }

    pub fn location(&self) -> &'static Location<'static> {
        match &self.inner {
            ApiErrorType::BadRequest { location, .. } => location,
            ApiErrorType::Unauthorized { location, .. } => location,
            ApiErrorType::Forbidden { location, .. } => location,
            ApiErrorType::NotFound { location, .. } => location,
            ApiErrorType::Conflict { location, .. } => location,
            ApiErrorType::InternalServerError { location, .. } => location,
            ApiErrorType::Redis { location, .. } => location,
            ApiErrorType::Reqwest { location, .. } => location,
            ApiErrorType::Serialization { location, .. } => location,
            ApiErrorType::Database { location, .. } => location,
            ApiErrorType::Anyhow { location, .. } => location,
            ApiErrorType::Cryptographic { location, .. } => location,
            ApiErrorType::JwtError { location, .. } => location,
            ApiErrorType::Custom { location, .. } => location,
        }
    }

    pub fn module(&self) -> &str {
        match &self.inner {
            ApiErrorType::BadRequest { module, .. } => module,
            ApiErrorType::Unauthorized { module, .. } => module,
            ApiErrorType::Forbidden { module, .. } => module,
            ApiErrorType::NotFound { module, .. } => module,
            ApiErrorType::Conflict { module, .. } => module,
            ApiErrorType::InternalServerError { module, .. } => module,
            ApiErrorType::Redis { module, .. } => module,
            ApiErrorType::Reqwest { module, .. } => module,
            ApiErrorType::Serialization { module, .. } => module,
            ApiErrorType::Database { module, .. } => module,
            ApiErrorType::Anyhow { module, .. } => module,
            ApiErrorType::Cryptographic { module, .. } => module,
            ApiErrorType::JwtError { module, .. } => module,
            ApiErrorType::Custom { module, .. } => module,
        }
    }

    pub fn with_ctx(mut self, ctx: RequestCtx) -> Self {
        self.ctx = Some(ctx);
        self
    }

    fn log_error(&self) {
        let status = self.status_code();
        let message = self.message();
        let location = self.location();
        let module = self.module();

        // Log with request context if available
        if let Some(ctx) = &self.ctx {
            event!(
                Level::ERROR,
                status = status.as_u16(),
                error_type = ?std::any::type_name::<ApiErrorType>(),
                message = %message,
                module = %module,
                file = %location.file(),
                line = %location.line(),
                request_id = %ctx.id,
                method = %ctx.method,
                path = %ctx.path,
                uri = %ctx.uri,
                instance = %ctx.instance,
                "API Error occurred"
            );
        } else {
            event!(
                Level::ERROR,
                status = status.as_u16(),
                error_type = ?std::any::type_name::<ApiErrorType>(),
                message = %message,
                module = %module,
                file = %location.file(),
                line = %location.line(),
                "API Error occurred"
            );
        }
    }
}

#[macro_export]
macro_rules! api_error {
    ($error_type:ident) => {
        $crate::error::ApiError {
            inner: $crate::error::ApiErrorType::$error_type {
                message: stringify!($error_type).to_string(),
                location: std::panic::Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    };
    ($error_type:ident, $msg:expr) => {
        $crate::error::ApiError {
            inner: $crate::error::ApiErrorType::$error_type {
                message: $msg.to_string(),
                location: std::panic::Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    };
}

impl From<RedisError> for ApiError {
    #[track_caller]
    fn from(error: RedisError) -> Self {
        ApiError {
            inner: ApiErrorType::Redis {
                error: RunError::User(error),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<DbErr> for ApiError {
    #[track_caller]
    fn from(error: DbErr) -> Self {
        ApiError {
            inner: ApiErrorType::Database {
                error,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<AnyhowError> for ApiError {
    #[track_caller]
    fn from(error: AnyhowError) -> Self {
        ApiError {
            inner: ApiErrorType::Anyhow {
                error,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<InvalidLength> for ApiError {
    #[track_caller]
    fn from(error: InvalidLength) -> Self {
        ApiError {
            inner: ApiErrorType::Cryptographic {
                message: format!("Invalid length: {}", error),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<JwtError> for ApiError {
    #[track_caller]
    fn from(error: JwtError) -> Self {
        ApiError {
            inner: ApiErrorType::JwtError {
                error,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<ReqwestError> for ApiError {
    #[track_caller]
    fn from(error: ReqwestError) -> Self {
        ApiError {
            inner: ApiErrorType::Reqwest {
                error,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<SerdeJsonError> for ApiError {
    #[track_caller]
    fn from(error: SerdeJsonError) -> Self {
        ApiError {
            inner: ApiErrorType::Serialization {
                error,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<QueryRejection> for ApiError {
    #[track_caller]
    fn from(error: QueryRejection) -> Self {
        ApiError {
            inner: ApiErrorType::Custom {
                status: StatusCode::BAD_REQUEST,
                message: error.body_text(),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<PathRejection> for ApiError {
    #[track_caller]
    fn from(error: PathRejection) -> Self {
        ApiError {
            inner: ApiErrorType::Custom {
                status: StatusCode::BAD_REQUEST,
                message: error.body_text(),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            ctx: None,
        }
    }
}

impl From<CacheError> for ApiError {
    #[track_caller]
    fn from(err: CacheError) -> Self {
        let inner = match err {
            CacheError::Redis(e) => ApiErrorType::Redis {
                error: e,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            CacheError::Serialization(e) => ApiErrorType::Serialization {
                error: e,
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            CacheError::NotFound => ApiErrorType::NotFound {
                message: "Resource not found".to_string(),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            CacheError::FetchError(e) => ApiErrorType::Custom {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: e.to_string(),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
            CacheError::CachedError(c, e) => ApiErrorType::Custom {
                status: c,
                message: e.to_string(),
                location: Location::caller(),
                module: module_path!().to_string(),
            },
        };

        ApiError { inner, ctx: None }
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

impl std::error::Error for ApiError {}
