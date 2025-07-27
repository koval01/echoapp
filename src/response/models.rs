use serde::Serialize;
use crate::model::{Body, Preview};

#[derive(Serialize)]
pub struct ChannelPreviewResponseData {
    pub channel: Preview,
}

#[derive(Serialize)]
pub struct ChannelBodyResponseData {
    pub body: Body,
}
