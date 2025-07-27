use axum::http::StatusCode;
use reqwest::{Client, Method};
use scraper::Html;
use crate::error::ApiError;
use crate::service::validate_channel_name;

#[derive(Debug, Clone)]
pub struct TelegramRequest {
    http_client: Client,
    base_url: String,
    timeout: std::time::Duration,
}

impl TelegramRequest {
    pub fn new(
        http_client: Client,
        base_url: impl Into<String>,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            http_client,
            base_url: base_url.into(),
            timeout,
        }
    }

    pub fn with_defaults(http_client: Client) -> Self {
        Self::new(
            http_client,
            "https://t.me/",
            std::time::Duration::from_secs(5),
        )
    }

    async fn fetch(
        &self,
        method: Method,
        endpoint: &str,
        body: Option<&[u8]>,
    ) -> Result<Html, ApiError> {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.http_client
            .request(method, &url)
            .timeout(self.timeout);

        if let Some(body_data) = body {
            request = request.body(body_data.to_vec());
        }

        let response = request
            .send()
            .await?;
        
        if response.status() != StatusCode::OK {
            return Err(ApiError::Custom(
                StatusCode::BAD_REQUEST,
                format!("Unexpected status code: {}", response.status())
            ));
        }

        let response_text = response.text().await?;

        if !response_text.trim_start().starts_with("<!DOCTYPE html>")
            && !response_text.trim_start().starts_with("<html") {
            return Err(ApiError::Custom(StatusCode::BAD_REQUEST, "Response is not HTML".to_string()));
        }

        Ok(Html::parse_document(&response_text))
    }

    pub async fn get_channel_page(&self, channel: &str) -> Result<Html, ApiError> {
        validate_channel_name(channel)?;

        self.fetch(Method::GET, channel, None).await
    }

    pub async fn get_channel_body_page(&self, channel: &str, position: &Option<u32>) -> Result<Html, ApiError> {
        validate_channel_name(channel)?;

        let path = match position {
            Some(pos) => format!("s/{channel}/{pos}"),
            None => format!("s/{channel}"),
        };

        self.fetch(Method::GET, &path, None).await
    }
}
