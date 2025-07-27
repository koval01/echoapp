use axum::{
    extract::{Path, rejection::PathRejection},
    response::IntoResponse,
    http::StatusCode,
    Json,
    Extension,
};

use moka::future::Cache;

use crate::{
    cache_fetch, 
    error::ApiError,
    model::Preview,
    util::cache::{CacheWrapper, CacheBackend},
    service::{ChannelPreviewParser, TelegramRequest, validate_channel_name},
    response::{ApiResponse, ChannelPreviewResponseData}
};

pub async fn channel_preview_handler_get(
    channel: Result<Path<String>, PathRejection>,
    Extension(telegram_client): Extension<TelegramRequest>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let Path(channel) = channel.map_err(|e| ApiError::Conflict(e.to_string()))?;

    validate_channel_name(&channel)
        .map_err(|e| ApiError::BadRequestWithMessage(e.to_string()))?;

    let cache = CacheWrapper::<Preview>::new(
        redis_pool,
        moka_cache,
        10,
        10
    );

    let preview = cache_fetch!(
        cache,
        &format!("preview_channel:{}", channel),
        async {
            let response = telegram_client.get_channel_page(&channel).await?;
            let parser = ChannelPreviewParser::new();
            match parser.parse(&response) {
                Ok(preview) => Ok(Some(preview)),
                Err(e) => Err(ApiError::from(e)),
            }
        }
    )?;

    let response_data = ChannelPreviewResponseData {
        channel: preview,
    };

    Ok((StatusCode::OK, Json(ApiResponse::success(response_data))))
}
