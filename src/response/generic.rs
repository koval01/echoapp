use axum::http::StatusCode;
use serde::Serialize;

#[derive(Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum ApiData<T> {
    Data(T)
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ApiData<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
}

#[allow(dead_code)]
impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: None,
            data: Some(ApiData::Data(data)),
            code: None,
        }
    }
}

impl<T> ApiResponse<T> {
    pub fn message_only(message: Option<&str>) -> Self {
        Self {
            status: "success".to_string(),
            message: message.map(|m| m.to_string()),
            data: None,
            code: None,
        }
    }

    pub fn error(message: Option<&str>, code: StatusCode) -> Self {
        Self {
            status: "error".to_string(),
            message: message.map(|m| m.to_string()),
            data: None,
            code: Some(code.as_u16()),
        }
    }
}
