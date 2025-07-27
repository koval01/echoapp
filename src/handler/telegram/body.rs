use axum::{
    extract::{Path, Query, rejection::{PathRejection, QueryRejection}},
    response::IntoResponse,
    http::StatusCode,
    Json,
    Extension,
};

use serde::Deserialize;

use moka::future::Cache;

use crate::{
    cache_fetch,
    error::ApiError,
    model::Body,
    util::cache::{CacheWrapper, CacheBackend},
    service::{ChannelBodyParser, TelegramRequest, validate_channel_name},
    response::{ApiResponse, ChannelBodyResponseData}
};

#[derive(Debug, Deserialize)]
pub struct ChannelQueryParams {
    position: Option<u32>,
}

pub async fn channel_body_handler_get(
    channel: Result<Path<String>, PathRejection>,
    query_params: Result<Query<ChannelQueryParams>, QueryRejection>,
    Extension(telegram_client): Extension<TelegramRequest>,
    Extension(redis_pool): Extension<CacheBackend>,
    Extension(moka_cache): Extension<Cache<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let Path(channel) = channel.map_err(|e| ApiError::Conflict(e.to_string()))?;

    validate_channel_name(&channel)
        .map_err(|e| ApiError::BadRequestWithMessage(e.to_string()))?;

    let position = query_params.and_then(|q| Ok(q.position))?;

    let cache = CacheWrapper::<Body>::new(
        redis_pool,
        moka_cache,
        10,
        10
    );

    let cache_key = match position {
        Some(pos) => format!("channel_body:{}:{}", channel, pos),
        None => format!("channel_body:{}", channel),
    };

    let body = cache_fetch!(
        cache,
        &cache_key,
        async {
            let response = telegram_client.get_channel_body_page(&channel, &position).await?;
            let parser = ChannelBodyParser::new();
            match parser.parse(&response) {
                Ok(preview) => Ok(Some(preview)),
                Err(e) => Err(ApiError::from(e)),
            }
        }
    )?;

    let response_data = ChannelBodyResponseData {
        body,
    };

    Ok((StatusCode::OK, Json(ApiResponse::success(response_data))))
}
