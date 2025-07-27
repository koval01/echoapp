mod base;
mod channel_preview;
mod channel_body;
mod channel_common;
mod entities;

use axum::http::StatusCode;

pub use base::BaseParser;
pub use channel_common::ChannelCommonParser;

pub use channel_preview::ChannelPreviewParser;
pub use channel_body::ChannelBodyParser;

pub use entities::EntitiesParser;

use crate::error::ApiError;

#[derive(thiserror::Error, Debug)]
pub enum ParserError {
    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Selector parse error: {0}")]
    SelectorParseError(String),
}

impl From<scraper::error::SelectorErrorKind<'static>> for ParserError {
    fn from(err: scraper::error::SelectorErrorKind<'static>) -> Self {
        ParserError::SelectorParseError(err.to_string())
    }
}

impl From<ParserError> for ApiError {
    fn from(err: ParserError) -> Self {
        match err {
            ParserError::ElementNotFound(e) => ApiError::NotFound(e),
            ParserError::ValidationFailed(e) => ApiError::Custom(StatusCode::BAD_REQUEST, e),
            ParserError::SelectorParseError(e) => ApiError::SelectorParseError(e),
        }
    }
}
