use std::sync::Arc;
use scraper::{Html, ElementRef};

use url::Url;

use super::{ParserError, BaseParser, EntitiesParser};
use crate::model::*;

pub struct ChannelBodyParser {
    base_parser: Arc<BaseParser>,
}

impl ChannelBodyParser {
    pub fn new() -> Self {
        let base_parser = BaseParser::new();
        Self {
            base_parser
        }
    }

    pub fn parse(&self, document: &Html) -> Result<Body, ParserError> {
        let root = document.root_element();
        self.validate_document(&root)?;

        Ok(Body {
            channel: self.parse_channel(&root)?,
            content: self.parse_content(&root)?,
            meta: self.parse_meta(&root)?,
        })
    }

    fn validate_document(&self, element_ref: &ElementRef) -> Result<(), ParserError> {
        // Check for the main message list container
        let messages_selector = self.base_parser.create_selector(".tgme_channel_history")?;
        if !self.base_parser.exists(element_ref, &messages_selector) {
            return Err(ParserError::ValidationFailed(
                "Not a valid Telegram channel page - missing message history container".to_string(),
            ));
        }

        // Check for at least one message
        let message_selector = self.base_parser.create_selector(".tgme_widget_message")?;
        if !self.base_parser.exists(element_ref, &message_selector) {
            return Err(ParserError::ValidationFailed(
                "No messages found on this channel page".to_string(),
            ));
        }

        // Check for the channel info section
        let channel_info_selector = self.base_parser.create_selector(".tgme_channel_info")?;
        if !self.base_parser.exists(element_ref, &channel_info_selector) {
            return Err(ParserError::ValidationFailed(
                "Not a valid Telegram channel page - missing channel info section".to_string(),
            ));
        }

        Ok(())
    }

    fn parse_channel(&self, element_ref: &ElementRef) -> Result<Channel, ParserError> {
        Ok(Channel {
            username: self.parse_username(element_ref)?,
            title: self.parse_channel_title(element_ref)?,
            description: self.parse_channel_description(element_ref),
            avatar: self.parse_channel_avatar(element_ref),
            counters: self.parse_counters(element_ref)?,
            labels: self.parse_labels(element_ref),
        })
    }

    fn parse_username(&self, element_ref: &ElementRef) -> Result<String, ParserError> {
        let selector = self.base_parser.create_selector(".tgme_channel_info_header_username")?;
        let username_text = self.base_parser.extract_text(element_ref, &selector, "Could not find channel username")?;
        Ok(username_text.trim_start_matches('@').to_string())
    }

    fn parse_channel_title(&self, element_ref: &ElementRef) -> Result<ParsedAndRaw, ParserError> {
        let selector = self.base_parser.create_selector(".tgme_channel_info_header_title")?;
        let element = self.base_parser.select_first(element_ref, &selector)
            .ok_or_else(|| ParserError::ElementNotFound("Channel title element not found".to_string()))?;
        Ok(self.base_parser.parse_parsed_and_raw(element))
    }

    fn parse_channel_description(&self, element_ref: &ElementRef) -> Option<ParsedAndRaw> {
        let selector = self.base_parser.create_selector(".tgme_channel_info_description").ok()?;
        let element = self.base_parser.select_first(element_ref, &selector)?;
        Some(self.base_parser.parse_parsed_and_raw(element))
    }

    fn parse_channel_avatar(&self, element_ref: &ElementRef) -> Option<Url> {
        let selector = self.base_parser.create_selector("i.tgme_page_photo_image>img").ok()?;
        self.base_parser.extract_url_attr_from_element(element_ref, &selector, "src", "")
            .ok()
    }

    fn parse_counters(&self, element_ref: &ElementRef) -> Result<Counter, ParserError> {
        let counter_selector = self.base_parser.create_selector(".tgme_channel_info_counter")?;
        let counters = self.base_parser.select_all(element_ref, &counter_selector);

        let mut counter_map = std::collections::HashMap::new();

        for counter in counters {
            let type_selector = self.base_parser.create_selector(".counter_type")?;
            let value_selector = self.base_parser.create_selector(".counter_value")?;

            if let (Some(type_element), Some(value_element)) = (
                self.base_parser.select_first(&counter, &type_selector),
                self.base_parser.select_first(&counter, &value_selector)
            ) {
                let counter_type = self.base_parser.element_to_text(&type_element).to_lowercase();
                let counter_value = self.base_parser.element_to_text(&value_element);
                counter_map.insert(counter_type, counter_value);
            }
        }

        // Handle both "subscribers" and "subscriber" keys
        let subscribers = if let Some(value) = counter_map.remove("subscribers") {
            value
        } else if let Some(value) = counter_map.remove("subscriber") {
            value
        } else {
            return Err(ParserError::ElementNotFound(
                "Neither 'subscribers' nor 'subscriber' counter found".to_string()
            ));
        };

        Ok(Counter {
            subscribers,
            photos: counter_map.remove("photos"),
            videos: counter_map.remove("videos"),
            files: counter_map.remove("files"),
            links: counter_map.remove("links"),
        })
    }

    fn parse_labels(&self, element_ref: &ElementRef) -> Option<Vec<String>> {
        let selector = self.base_parser.create_selector(".tgme_header_labels").ok()?;
        let elements = self.base_parser.select_all(element_ref, &selector);

        if elements.is_empty() {
            return None;
        }

        Some(elements.iter()
            .filter_map(|label_element| {
                // Get the first child element of the label
                label_element.first_child()?
                    .value()
                    .as_element()?
                    .attr("class")?
                    .split('-')
                    .next()
                    .map(|s| s.to_string())
            })
            .collect())
    }

    fn parse_content(&self, element_ref: &ElementRef) -> Result<Content, ParserError> {
        Ok(Content {
            posts: self.parse_posts(element_ref)?,
        })
    }

    fn parse_posts(&self, element_ref: &ElementRef) -> Result<Posts, ParserError> {
        let selector = self.base_parser.create_selector("div.tgme_widget_message_wrap")?;
        let post_elements = self.base_parser.select_all(element_ref, &selector);

        match post_elements.len() {
            0 => Err(ParserError::ElementNotFound("No posts found".to_string())),
            1 => {
                let post = self.parse_post(&post_elements[0])?;
                Ok(Posts::Single(post))
            }
            _ => {
                let mut posts = Vec::with_capacity(post_elements.len());
                for element in post_elements {
                    posts.push(self.parse_post(&element)?);
                }
                Ok(Posts::Multiple(posts))
            }
        }
    }

    fn parse_post(&self, post_element: &ElementRef) -> Result<Post, ParserError> {
        Ok(Post {
            id: self.parse_post_id(post_element)?,
            footer: self.parse_post_footer(post_element)?,
            forwarded: self.parse_forwarded(post_element),
            view: self.parse_view(post_element)?,
            content: self.parse_post_content(post_element)?
        })
    }

    fn parse_post_id(&self, post_element: &ElementRef) -> Result<i64, ParserError> {
        let message_selector = self.base_parser.create_selector(".tgme_widget_message")?;
        let message_element = self.base_parser.select_first(post_element, &message_selector)
            .ok_or_else(|| ParserError::ElementNotFound("Message element not found".to_string()))?;

        let post_value = self.base_parser.extract_attr(
            &message_element,
            "data-post",
            "data-post attribute not found"
        )?;

        let id_str = post_value.split('/')
            .last()
            .ok_or_else(|| ParserError::ValidationFailed("Invalid data-post format".to_string()))?;

        id_str.parse()
            .map_err(|_| ParserError::ValidationFailed(format!("Failed to parse post ID: {}", id_str)))
    }

    fn parse_post_content(&self, post_element: &ElementRef) -> Result<ContentPost, ParserError> {
        Ok(ContentPost {
            text: self.parse_post_text(post_element),
            media: self.parse_media(post_element),
            poll: self.parse_poll(post_element),
            inline: self.parse_inline_links(post_element),
            reply: self.parse_reply(post_element),
            preview_link: self.parse_preview_link(post_element),
            reacts: self.parse_reacts(post_element),
        })
    }

    fn parse_post_text(&self, post_element: &ElementRef) -> Option<Text> {
        let selector = self.base_parser.create_selector(".tgme_widget_message_text").ok()?;
        let text_element = self.base_parser.select_first(post_element, &selector)?;

        Some(self.parse_text_with_entities(&text_element))
    }

    fn parse_media(&self, post_element: &ElementRef) -> Option<Vec<MediaItem>> {
        let media_selectors = [
            ".link_preview_image",
            ".tgme_widget_message_photo_wrap",
            ".tgme_widget_message_video_player",
            ".tgme_widget_message_voice_player",
            ".tgme_widget_message_roundvideo_player",
            ".tgme_widget_message_sticker_wrap",
        ];

        let selector = match self.base_parser.create_selector(&media_selectors.join(", ")) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let media_elements = self.base_parser.select_all(post_element, &selector);
        if media_elements.is_empty() {
            return None;
        }

        let mut media_items = Vec::new();
        for element in media_elements {
            if let Some(media_item) = self.parse_media_item(&element) {
                media_items.push(media_item);
            }
        }

        if media_items.is_empty() {
            None
        } else {
            Some(media_items)
        }
    }

    fn parse_media_item(&self, media_element: &ElementRef) -> Option<MediaItem> {
        let class_name = media_element.value().attr("class")?.split_whitespace().next()?;

        match class_name {
            "link_preview_image" | "tgme_widget_message_photo_wrap" => self.parse_image(media_element),
            "tgme_widget_message_video_player" => self.parse_video(media_element),
            "tgme_widget_message_voice_player" => self.parse_voice(media_element),
            "tgme_widget_message_roundvideo_player" => self.parse_roundvideo(media_element),
            "tgme_widget_message_sticker_wrap" => self.parse_sticker(media_element),
            _ => None,
        }
    }

    fn parse_image(&self, element: &ElementRef) -> Option<MediaItem> {
        let url = self.base_parser.extract_url_from_style(element)?;

        Some(MediaItem {
            url: Some(url),
            thumb: None,
            duration: None,
            waves: None,
            media_type: MediaType::Image,
            available: None,
        })
    }

    fn parse_video(&self, element: &ElementRef) -> Option<MediaItem> {
        let thumb = self.base_parser.create_selector(".tgme_widget_message_video_thumb")
            .ok()
            .and_then(|s| self.base_parser.select_first(element, &s))
            .and_then(|e| self.base_parser.extract_url_from_style(&e));

        let duration = self.base_parser.create_selector("time.message_video_duration")
            .ok()
            .and_then(|s| self.base_parser.select_first(element, &s))
            .map(|e| self.parse_duration(&e));

        let video = self.base_parser.create_selector("video.tgme_widget_message_video")
            .ok()
            .and_then(|s| self.base_parser.select_first(element, &s));

        let video_available = video.and_then(|v| v.value().attr("src")).is_some();
        let url = video.and_then(|v| v.value().attr("src"))
            .and_then(|src| Url::parse(src).ok());

        let media_type = if duration.is_none() {
            MediaType::Gif
        } else {
            MediaType::Video
        };

        Some(MediaItem {
            url,
            thumb,
            duration,
            waves: None,
            media_type,
            available: if video_available { None } else { Some(false) },
        })
    }

    fn parse_voice(&self, element: &ElementRef) -> Option<MediaItem> {
        let audio = self.base_parser.create_selector(".tgme_widget_message_voice")
            .ok()
            .and_then(|s| self.base_parser.select_first(element, &s))?;

        let duration_node = self.base_parser.create_selector("time.tgme_widget_message_voice_duration")
            .ok()
            .and_then(|s| self.base_parser.select_first(element, &s));

        let duration_text = duration_node.map(|e| self.base_parser.element_to_text(&e));
        let duration = duration_text.as_ref().map(|t| self.parse_duration_text(t));

        let url = audio.value().attr("src")
            .and_then(|src| Url::parse(src).ok());

        let waves = audio.value().attr("data-waveform").map(|s| s.to_string());

        Some(MediaItem {
            url,
            thumb: None,
            duration,
            waves,
            media_type: MediaType::Voice,
            available: None,
        })
    }

    fn parse_roundvideo(&self, element: &ElementRef) -> Option<MediaItem> {
        let video = self.base_parser.create_selector("video.tgme_widget_message_roundvideo")
            .ok()
            .and_then(|s| self.base_parser.select_first(element, &s))?;

        let duration_node = self.base_parser.create_selector("time.tgme_widget_message_roundvideo_duration")
            .ok()
            .and_then(|s| self.base_parser.select_first(element, &s));

        let duration_text = duration_node.map(|e| self.base_parser.element_to_text(&e));
        let duration = duration_text.as_ref().map(|t| self.parse_duration_text(t));

        let thumb = self.base_parser.create_selector(".tgme_widget_message_roundvideo_thumb")
            .ok()
            .and_then(|s| self.base_parser.select_first(element, &s))
            .and_then(|e| self.base_parser.extract_url_from_style(&e));

        let url = video.value().attr("src")
            .and_then(|src| Url::parse(src).ok());

        Some(MediaItem {
            url,
            thumb,
            duration,
            waves: None,
            media_type: MediaType::RoundVideo,
            available: None,
        })
    }

    fn parse_sticker(&self, element: &ElementRef) -> Option<MediaItem> {
        let sticker_classes = [
            "picture.tgme_widget_message_tgsticker",
            "i.tgme_widget_message_sticker",
            "div.tgme_widget_message_videosticker",
        ];

        let mut sticker = None;
        let mut key_idx = None;

        for (i, class_selector) in sticker_classes.iter().enumerate() {
            if let Ok(selector) = self.base_parser.create_selector(class_selector) {
                if let Some(s) = self.base_parser.select_first(element, &selector) {
                    sticker = Some(s);
                    key_idx = Some(i);
                    break;
                }
            }
        }

        let sticker = sticker?;
        let key_idx = key_idx?;

        let key_map = [
            ("source", "srcset"),
            ("i.tgme_widget_message_sticker", "data-webp"),
            ("video.js-videosticker_video", "src"),
        ];

        let (selector, attr) = key_map[key_idx];
        let source_selector = match self.base_parser.create_selector(selector) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let source = self.base_parser.select_first(&sticker, &source_selector)?;
        let url = source.value().attr(attr)
            .and_then(|src| Url::parse(src).ok())?;

        let thumb = if let Ok(img_selector) = self.base_parser.create_selector("img") {
            self.base_parser.select_first(&sticker, &img_selector)
                .and_then(|img| img.value().attr("src"))
                .and_then(|src| Url::parse(src).ok())
        } else {
            None
        };

        Some(MediaItem {
            url: Some(url),
            thumb,
            duration: None,
            waves: None,
            media_type: MediaType::Sticker,
            available: None,
        })
    }

    fn parse_duration(&self, element: &ElementRef) -> Duration {
        let text = self.base_parser.element_to_text(element);
        self.parse_duration_text(&text)
    }

    fn parse_duration_text(&self, text: &str) -> Duration {
        let raw = text.split(':')
            .map(|s| s.parse::<u32>().unwrap_or(0))
            .fold(0, |acc, x| acc * 60 + x);

        Duration {
            formatted: text.to_string(),
            raw: Some(raw),
        }
    }

    fn parse_poll(&self, post_element: &ElementRef) -> Option<Poll> {
        let poll_selector = self.base_parser.create_selector(".tgme_widget_message_poll").ok()?;
        let poll_element = self.base_parser.select_first(post_element, &poll_selector)?;

        let question_selector = self.base_parser.create_selector(".tgme_widget_message_poll_question").ok()?;
        let question = self.base_parser.extract_text(&poll_element, &question_selector, "").ok()?;

        let type_selector = self.base_parser.create_selector(".tgme_widget_message_poll_type").ok()?;
        let poll_type = self.base_parser.extract_text_optional(&poll_element, &type_selector);

        let votes_selector = self.base_parser.create_selector(".tgme_widget_message_voters").ok()?;
        let votes = self.base_parser.extract_text_optional(&poll_element, &votes_selector);

        let option_selector = self.base_parser.create_selector(".tgme_widget_message_poll_option").ok()?;
        let option_elements = self.base_parser.select_all(&poll_element, &option_selector);

        let mut options = Vec::with_capacity(option_elements.len());
        for option_element in option_elements {
            let percent_selector = self.base_parser.create_selector(".tgme_widget_message_poll_option_percent").ok()?;
            let percent_text = self.base_parser.extract_text(&option_element, &percent_selector, "").ok()?;
            let percent = percent_text.trim_end_matches('%').parse().ok()?;

            let text_selector = self.base_parser.create_selector(".tgme_widget_message_poll_option_text").ok()?;
            let name = self.base_parser.extract_text(&option_element, &text_selector, "").ok()?;

            options.push(PollOption {
                name,
                percent,
            });
        }

        Some(Poll {
            question,
            poll_type,
            votes: votes.unwrap_or_else(|| "0".to_string()),
            options,
        })
    }

    fn parse_inline_links(&self, post_element: &ElementRef) -> Option<Vec<Inline>> {
        let row_selector = self.base_parser.create_selector(".tgme_widget_message_inline_row").ok()?;
        let rows = self.base_parser.select_all(&post_element, &row_selector);

        if rows.is_empty() {
            return None;
        }

        let button_selector = self.base_parser.create_selector(".tgme_widget_message_inline_button").ok()?;
        let button_text_selector = self.base_parser.create_selector(".tgme_widget_message_inline_button_text").ok()?;

        let mut inlines = Vec::new();

        for row in rows {
            for button in row.select(&button_selector) {
                let title = button.select(&button_text_selector)
                    .next()
                    .map(|e| self.base_parser.element_to_text(&e))
                    .unwrap_or_default();

                let url = button.value()
                    .attr("href")
                    .and_then(|href| Url::parse(href).ok());

                if let Some(url) = url {
                    inlines.push(Inline {
                        title,
                        url,
                    });
                }
            }
        }

        if inlines.is_empty() {
            None
        } else {
            Some(inlines)
        }
    }

    fn parse_reply(&self, post_element: &ElementRef) -> Option<Reply> {
        let selector = self.base_parser.create_selector(".tgme_widget_message_reply").ok()?;
        let reply_element = self.base_parser.select_first(post_element, &selector)?;

        let cover = self.base_parser.create_selector(".tgme_widget_message_reply_thumb").ok()
            .and_then(|sel| self.base_parser.select_first(&reply_element, &sel))
            .and_then(|thumb_element| self.base_parser.extract_url_from_style(&thumb_element));

        let author_selector = self.base_parser.create_selector(".tgme_widget_message_author_name").ok()?;
        let author_element = self.base_parser.select_first(&reply_element, &author_selector)?;
        let name = self.base_parser.parse_parsed_and_raw(author_element);

        let text_selector = self.base_parser.create_selector(".js-message_reply_text").ok()?;
        let text_element = self.base_parser.select_first(&reply_element, &text_selector)?;
        let text = self.base_parser.parse_parsed_and_raw(text_element);

        let url = self.base_parser.extract_url_attr(&reply_element, "href", "").ok()?;
        let to_message = url.path_segments()?.last()?.parse().ok()?;

        Some(Reply {
            cover,
            name,
            text,
            to_message,
        })
    }

    fn parse_preview_link(&self, post_element: &ElementRef) -> Option<PreviewLink> {
        let selector = self.base_parser.create_selector(".tgme_widget_message_link_preview").ok()?;
        let preview_element = self.base_parser.select_first(post_element, &selector)?;

        let url = self.base_parser.extract_url_attr(&preview_element, "href", "").ok()?;

        let site_name_selector = self.base_parser.create_selector(".link_preview_site_name").ok()?;
        let site_name = self.base_parser.extract_text_optional(&preview_element, &site_name_selector);

        let title_selector = self.base_parser.create_selector(".link_preview_title").ok()?;
        let title = self.base_parser.extract_text_optional(&preview_element, &title_selector);

        let description_selector = self.base_parser.create_selector(".link_preview_description").ok()?;
        let description = self.base_parser.select_first(&preview_element, &description_selector)
            .map(|e| self.base_parser.parse_parsed_and_raw(e));

        let thumb = self.base_parser.create_selector(".link_preview_right_image").ok()
            .and_then(|sel| self.base_parser.select_first(&preview_element, &sel))
            .and_then(|image_element| self.base_parser.extract_url_from_style(&image_element));

        Some(PreviewLink {
            title,
            url,
            site_name,
            description,
            thumb,
        })
    }

    fn parse_reacts(&self, post_element: &ElementRef) -> Option<Vec<Reaction>> {
        let selector = match self.base_parser.create_selector(".tgme_widget_message_reactions") {
            Ok(s) => s,
            Err(_) => {
                return None;
            }
        };

        let reactions_node = match self.base_parser.select_first(post_element, &selector) {
            Some(node) => node,
            None => {
                return None;
            }
        };

        let reaction_selector = match self.base_parser.create_selector(".tgme_reaction") {
            Ok(s) => s,
            Err(_) => {
                return None;
            }
        };

        let reaction_nodes = self.base_parser.select_all(&reactions_node, &reaction_selector);

        let mut reactions = Vec::new();
        for (_, reaction_node) in reaction_nodes.iter().enumerate() {
            if let Some(reaction) = self.parse_single_reaction(reaction_node) {
                reactions.push(reaction);
            }
        }

        if reactions.is_empty() {
            None
        } else {
            Some(reactions)
        }
    }

    fn parse_single_reaction(&self, reaction_node: &ElementRef) -> Option<Reaction> {
        let count = reaction_node.text().collect::<String>().trim().to_string();
        if count.is_empty() {
            return None;
        }

        if let Some(class_attr) = reaction_node.value().attr("class") {
            if class_attr.contains("tgme_reaction_paid") {
                return Some(Reaction {
                    r#type: ReactionType::TelegramStars,
                    count,
                    emoji: Some("⭐".to_string()),
                    emoji_image: None,
                    emoji_id: None,
                });
            }
        }

        if let Ok(emoji_selector) = self.base_parser.create_selector("i.emoji") {
            if let Some(emoji_node) = self.base_parser.select_first(reaction_node, &emoji_selector) {
                let emoji = emoji_node.select(&self.base_parser.create_selector("b").ok()?)
                    .next()
                    .map(|b| self.base_parser.element_to_text(&b))
                    .unwrap_or_default();

                let emoji_image = self.extract_emoji_from_style(&emoji_node);

                return Some(Reaction {
                    r#type: ReactionType::Emoji,
                    count,
                    emoji: Some(emoji),
                    emoji_image,
                    emoji_id: None,
                });
            }
        }

        if let Ok(custom_emoji_selector) = self.base_parser.create_selector("tg-emoji") {
            if let Some(emoji_node) = self.base_parser.select_first(reaction_node, &custom_emoji_selector) {
                let emoji_id = emoji_node.value()
                    .attr("emoji-id")
                    .map(|s| s.to_string());

                return Some(Reaction {
                    r#type: ReactionType::CustomEmoji,
                    count,
                    emoji: None,
                    emoji_image: None,
                    emoji_id,
                });
            }
        }

        None
    }

    fn extract_emoji_from_style(&self, emoji_node: &ElementRef) -> Option<Url> {
        let style = emoji_node.value().attr("style")?;
        let url_start = style.find("url('")? + 5;
        let url_end = style[url_start..].find('\'')? + url_start;
        let url_str = &style[url_start..url_end];

        if url_str.starts_with("//") {
            Url::parse(&format!("https:{}", url_str)).ok()
        } else {
            Url::parse(url_str).ok()
        }
    }

    fn parse_post_footer(&self, post_element: &ElementRef) -> Result<Footer, ParserError> {
        Ok(Footer {
            views: self.parse_views(post_element),
            edited: self.parse_edited_status(post_element),
            author: self.parse_author(post_element),
            date: self.parse_date(post_element)?,
        })
    }

    fn parse_views(&self, post_element: &ElementRef) -> Option<String> {
        let selector = self.base_parser.create_selector(".tgme_widget_message_views").ok()?;
        post_element.select(&selector).next()
            .map(|e| self.base_parser.element_to_text(&e))
    }

    fn parse_edited_status(&self, post_element: &ElementRef) -> Option<bool> {
        let selector = self.base_parser.create_selector(".tgme_widget_message_edited").ok()?;
        Some(post_element.select(&selector).next().is_some())
    }

    fn parse_author(&self, post_element: &ElementRef) -> Option<ParsedAndRaw> {
        let selector = self.base_parser.create_selector(".tgme_widget_message_from_author").ok()?;
        post_element.select(&selector).next()
            .map(|e| self.base_parser.parse_parsed_and_raw(e))
    }

    fn parse_date(&self, post_element: &ElementRef) -> Result<Date, ParserError> {
        let time_selector = self.base_parser.create_selector(".tgme_widget_message_date time[datetime]")?;
        let time_element = self.base_parser.select_first(post_element, &time_selector)
            .ok_or_else(|| ParserError::ElementNotFound("Time element not found".to_string()))?;

        let datetime = self.base_parser.extract_attr(&time_element, "datetime", "datetime attribute not found")?;

        let dt = chrono::DateTime::parse_from_rfc3339(&datetime)
            .map_err(|e| ParserError::ValidationFailed(format!("Failed to parse datetime: {}", e)))?;
        let unix = dt.timestamp();

        Ok(Date {
            formatted: datetime,
            unix,
        })
    }

    fn parse_forwarded(&self, post_element: &ElementRef) -> Option<Forwarded> {
        let forwarded_selector = self.base_parser.create_selector(".tgme_widget_message_forwarded_from").ok()?;
        let forwarded_header = post_element.select(&forwarded_selector).next()?;

        let name_selector = self.base_parser.create_selector(".tgme_widget_message_forwarded_from_name").ok();
        let name_element = if let Some(selector) = name_selector {
            forwarded_header.select(&selector).next()
        } else {
            forwarded_header.first_child().and_then(ElementRef::wrap)
        }?;

        let name = self.base_parser.parse_parsed_and_raw(name_element);
        let url = name_element.value()
            .attr("href")
            .and_then(|href| Url::parse(href).ok());

        Some(Forwarded {
            name,
            url,
        })
    }

    fn parse_view(&self, post_element: &ElementRef) -> Result<String, ParserError> {
        let message_selector = self.base_parser.create_selector(".tgme_widget_message")?;
        let message_element = self.base_parser.select_first(post_element, &message_selector)
            .ok_or_else(|| ParserError::ElementNotFound("Message element not found".to_string()))?;

        self.base_parser.extract_attr(&message_element, "data-view", "data-view attribute not found")
    }

    fn parse_meta(&self, element_ref: &ElementRef) -> Result<Meta, ParserError> {
        Ok(Meta {
            offset: self.parse_offset(element_ref)?,
        })
    }

    fn parse_offset(&self, element_ref: &ElementRef) -> Result<OffsetItem, ParserError> {
        Ok(OffsetItem {
            before: self.parse_offset_before(element_ref),
            after: self.parse_offset_after(element_ref),
        })
    }

    fn parse_offset_before(&self, element_ref: &ElementRef) -> Option<i64> {
        let selector = self.base_parser.create_selector("a.tme_messages_more[data-before]").ok()?;
        let element = self.base_parser.select_first(element_ref, &selector)?;

        element.value()
            .attr("data-before")
            .and_then(|s| s.parse().ok())
    }

    fn parse_offset_after(&self, element_ref: &ElementRef) -> Option<i64> {
        let selector = self.base_parser.create_selector("a.tme_messages_more[data-after]").ok()?;
        let element = self.base_parser.select_first(element_ref, &selector)?;

        element.value()
            .attr("data-after")
            .and_then(|s| s.parse().ok())
    }

    fn parse_text_with_entities(&self, element_ref: &ElementRef) -> Text {
        let html = self.base_parser.element_to_html(element_ref);
        let parser = EntitiesParser::new(&html);
        let entities = parser.parse_entities();
        let text_only = parser.text_only;

        Text {
            plain: text_only.clone(),
            html,
            entities: if entities.is_empty() { None } else { Some(entities) },
        }
    }
}
