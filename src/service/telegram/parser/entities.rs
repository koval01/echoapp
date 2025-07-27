use regex::Regex;
use crate::model::{EntityType, TextEntity};

#[derive(Debug)]
pub struct EntitiesParser {
    html_text: String,
    pub(crate) text_only: String,
    patterns: Vec<(Regex, EntityPattern)>,
}

#[derive(Debug)]
struct EntityPattern {
    entity_type: &'static str,
    content_group: usize,
    url_group: Option<usize>,
    #[allow(dead_code)] // This field is part of the struct's contract even if not used internally
    is_url_entity: bool,
}

impl EntitiesParser {
    pub fn new(html_body: &str) -> Self {
        let html_text = html_body
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n");
        let text_only = Self::strip_tags(&html_text);
        let patterns = Self::build_patterns().unwrap_or_else(|e| {
            log::error!("Failed to build regex patterns: {}", e);
            Vec::new()
        });

        Self { html_text, text_only, patterns }
    }

    fn strip_tags(html: &str) -> String {
        let re = Regex::new(r"<br\s*/?>").unwrap();
        let html_with_newlines = re.replace_all(html, "\n");
        let tag_re = Regex::new(r"<[^>]+>").unwrap();
        tag_re.replace_all(&html_with_newlines, "").into_owned()
    }

    fn build_patterns() -> Result<Vec<(Regex, EntityPattern)>, regex::Error> {
        Ok(vec![
            (Regex::new(r"<s>(.+?)</s>")?, EntityPattern {
                entity_type: "strikethrough",
                content_group: 1,
                url_group: None,
                is_url_entity: false,
            }),
            (Regex::new(r"<code>(.+?)</code>")?, EntityPattern {
                entity_type: "code",
                content_group: 1,
                url_group: None,
                is_url_entity: false,
            }),
            (Regex::new(r"<u>(.+?)</u>")?, EntityPattern {
                entity_type: "underline",
                content_group: 1,
                url_group: None,
                is_url_entity: false,
            }),
            (Regex::new(r"<i>(.+?)</i>")?, EntityPattern {
                entity_type: "italic",
                content_group: 1,
                url_group: None,
                is_url_entity: false,
            }),
            (Regex::new(r"<b>(.+?)</b>")?, EntityPattern {
                entity_type: "bold",
                content_group: 1,
                url_group: None,
                is_url_entity: false,
            }),
            (Regex::new(r"<tg-spoiler>(.+?)</tg-spoiler>")?, EntityPattern {
                entity_type: "spoiler",
                content_group: 1,
                url_group: None,
                is_url_entity: false,
            }),
            (Regex::new(r#"<a\s+(?:[^>]*?\s+)?href="([^"]*)"[^>]*\s+onclick="[^"]*"[^>]*>(.*?)</a>"#)?, EntityPattern {
                entity_type: "text_link",
                content_group: 2,
                url_group: Some(1),
                is_url_entity: true,
            }),
            (Regex::new(r#"<a\s+href="(https?://[^"]*)"[^>]*>(.*?)</a>"#)?, EntityPattern {
                entity_type: "url",
                content_group: 2,
                url_group: Some(1),
                is_url_entity: false,
            }),
            (Regex::new(r"<pre>(.*?)</pre>")?, EntityPattern {
                entity_type: "pre",
                content_group: 1,
                url_group: None,
                is_url_entity: false,
            })
        ])
    }

    pub fn parse_entities(&self) -> Vec<TextEntity> {
        let mut entities = Vec::new();
        let _text_offset = 0;

        for (re, pattern) in &self.patterns {
            for cap in re.captures_iter(&self.html_text) {
                if let Some(content_match) = cap.get(pattern.content_group) {
                    let content = content_match.as_str();
                    let html_start = content_match.start();
                    let html_end = content_match.end();

                    if let Some(text_start) = self.find_text_position(html_start, html_end) {
                        let length = content.len();

                        if entities.iter().any(|e: &TextEntity| {
                            (text_start >= e.offset && text_start < e.offset + e.length) ||
                                (text_start + length > e.offset && text_start + length <= e.offset + e.length)
                        }) {
                            continue;
                        }

                        let entity = match pattern.entity_type {
                            "strikethrough" => TextEntity {
                                offset: text_start,
                                length,
                                entity_type: EntityType::Strikethrough,
                            },
                            "code" => TextEntity {
                                offset: text_start,
                                length,
                                entity_type: EntityType::Code,
                            },
                            "underline" => TextEntity {
                                offset: text_start,
                                length,
                                entity_type: EntityType::Underline,
                            },
                            "italic" => TextEntity {
                                offset: text_start,
                                length,
                                entity_type: EntityType::Italic,
                            },
                            "bold" => TextEntity {
                                offset: text_start,
                                length,
                                entity_type: EntityType::Bold,
                            },
                            "spoiler" => TextEntity {
                                offset: text_start,
                                length,
                                entity_type: EntityType::Spoiler,
                            },
                            "text_link" => {
                                let url = pattern.url_group.and_then(|g| cap.get(g))
                                    .map(|m| m.as_str().to_string())
                                    .unwrap_or_default();
                                TextEntity {
                                    offset: text_start,
                                    length,
                                    entity_type: EntityType::TextLink { url },
                                }
                            },
                            "url" => TextEntity {
                                offset: text_start,
                                length,
                                entity_type: EntityType::Url,
                            },
                            "pre" => TextEntity {
                                offset: text_start,
                                length,
                                entity_type: EntityType::Pre { language: None },
                            },
                            _ => continue,
                        };

                        entities.push(entity);
                    }
                }
            }
        }

        entities.sort_by_key(|e| e.offset);
        entities
    }

    fn find_text_position(&self, html_start: usize, html_end: usize) -> Option<usize> {
        let html_before = &self.html_text[..html_start];
        let text_before = Self::strip_tags(html_before);
        let text_start = text_before.len();
        let html_match = &self.html_text[html_start..html_end];
        let text_match = Self::strip_tags(html_match);

        if self.text_only[text_start..].starts_with(&text_match) {
            Some(text_start)
        } else {
            self.text_only[text_start..].find(&text_match).map(|pos| text_start + pos)
        }
    }
}
