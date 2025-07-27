use std::sync::Arc;
use scraper::ElementRef;
use url::Url;
use super::{ParserError, BaseParser};

pub struct ChannelCommonParser {
    base_parser: Arc<BaseParser>,
}

impl ChannelCommonParser {
    pub fn new(base_parser: Arc<BaseParser>) -> Self {
        Self { base_parser }
    }

    pub fn extract_title(&self, element_ref: &ElementRef) -> Result<String, ParserError> {
        let selector = self.base_parser.create_selector(".tgme_page_title>span")?;
        self.base_parser.extract_text(element_ref, &selector, "Could not find channel title")
    }

    pub fn extract_description(&self, element_ref: &ElementRef) -> Option<String> {
        let selector = self.base_parser.create_selector(".tgme_page_description").ok()?;
        self.base_parser.extract_text_with_newlines_optional(element_ref, &selector)
    }

    pub fn extract_avatar(&self, element_ref: &ElementRef) -> Result<Url, ParserError> {
        let selector = self.base_parser.create_selector(".tgme_page_photo_image")?;
        let url_str = self.base_parser.extract_attr_from_element(
            element_ref,
            &selector,
            "src",
            "Could not find channel avatar"
        )?;
        Url::parse(&url_str)
            .map_err(|e| ParserError::ValidationFailed(format!("Invalid avatar URL: {}", e)))
    }

    pub fn extract_verified_status(&self, element_ref: &ElementRef) -> bool {
        self.base_parser.create_selector(".verified-icon")
            .ok()
            .map(|selector| self.base_parser.exists(element_ref, &selector))
            .unwrap_or(false)
    }

    pub fn validate_channel_page(&self, element_ref: &ElementRef) -> Result<(), ParserError> {
        let selector = self.base_parser.create_selector(
            ".tgme_page_context_link_wrap>a.tgme_page_context_link"
        )?;

        let preview_button = self.base_parser.extract_text(
            element_ref,
            &selector,
            "Could not find preview button"
        )?;

        if preview_button != "Preview channel" {
            return Err(ParserError::ValidationFailed(
                "The username probably doesn't match any Telegram channel.".to_string()
            ));
        }

        Ok(())
    }
}
