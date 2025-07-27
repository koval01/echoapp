use std::sync::Arc;
use scraper::{ElementRef, Html};
use crate::{model::Preview, util::parse_subscriber_count};
use super::{ParserError, ChannelCommonParser, BaseParser};

pub struct ChannelPreviewParser {
    base_parser: Arc<BaseParser>,
    common_parser: ChannelCommonParser
}

impl ChannelPreviewParser {
    pub fn new() -> Self {
        let base_parser = BaseParser::new();
        Self {
            base_parser: base_parser.clone(),
            common_parser: ChannelCommonParser::new(base_parser),
        }
    }

    pub fn parse(&self, document: &Html) -> Result<Preview, ParserError> {
        let root = document.root_element();
        self.common_parser.validate_channel_page(&root)?;

        let title = self.common_parser.extract_title(&root)?;
        let description = self.common_parser.extract_description(&root);
        let subscribers = self.extract_subscribers(&root);
        let avatar = self.common_parser.extract_avatar(&root)?;
        let is_verified = self.common_parser.extract_verified_status(&root);

        let preview = Preview {
            title,
            description,
            subscribers,
            avatar,
            is_verified,
        };

        self.validate_preview(&preview)?;

        Ok(preview)
    }

    fn extract_subscribers(&self, element_ref: &ElementRef) -> Option<u64> {
        let parsed = self.base_parser.create_selector(".tgme_page_extra")
            .ok()
            .and_then(|selector| self.base_parser.extract_text_optional(element_ref, &selector))
            .unwrap_or_default();
        parse_subscriber_count(&parsed)
    }

    fn validate_preview(&self, preview: &Preview) -> Result<(), ParserError> {
        if preview.title.is_empty() {
            return Err(ParserError::ValidationFailed(
                "Failed to extract channel title".to_string(),
            ));
        }

        Ok(())
    }
}
