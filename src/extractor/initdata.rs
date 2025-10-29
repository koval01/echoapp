use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};

use serde::de::DeserializeOwned;
use url::form_urlencoded;
use crate::api_error;
use crate::error::ApiError;
use crate::extractor::get_error;

pub struct InitData<T>(pub T);

impl<T, S> FromRequestParts<S> for InitData<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let decoded_init_data = parts
            .extensions
            .get::<String>()
            .ok_or_else(|| get_error(parts, api_error!(BadRequest, "Missing init data")))?;

        let mut query_pairs = form_urlencoded::parse(decoded_init_data.as_bytes());
        let user_query = query_pairs
            .find(|(key, _)| key == "user")
            .ok_or_else(|| get_error(parts, api_error!(BadRequest, "Missing user data in init data")))?
            .1
            .to_string();

        let data: T = serde_json::from_str(&user_query).map_err(|e| {
            get_error(parts, api_error!(BadRequest, &format!("Failed to parse user data: {}", e)))
        })?;

        Ok(InitData(data))
    }
}
