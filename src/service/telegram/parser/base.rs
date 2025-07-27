use scraper::{Selector, ElementRef};
use scraper::node::Node;

use regex::Regex;
use url::Url;

use std::sync::Arc;
use std::collections::HashMap;
use once_cell::sync::Lazy;

use crate::model::ParsedAndRaw;
use super::ParserError;

static HTML_TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^<[^>]+>(.*)</[^>]+>$").unwrap()
});

#[derive(Clone)]
pub struct BaseParser {
    selector_cache: Arc<HashMap<String, Selector>>,
}

impl BaseParser {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            selector_cache: Arc::new(HashMap::with_capacity(16)),
        })
    }

    /// Creates a selector from a CSS pattern with caching
    pub fn create_selector(&self, pattern: &str) -> Result<Selector, ParserError> {
        if let Some(selector) = self.selector_cache.get(pattern) {
            return Ok(selector.clone());
        }

        let selector = Selector::parse(pattern)
            .map_err(|e| ParserError::SelectorParseError(e.to_string()))?;

        let mut cache = Arc::clone(&self.selector_cache);
        let cache = Arc::make_mut(&mut cache);
        cache.insert(pattern.to_string(), selector.clone());

        Ok(selector)
    }

    /// Extracts text from the first element matching the selector
    pub fn extract_text(
        &self,
        element_ref: &ElementRef,
        selector: &Selector,
        error_msg: &str,
    ) -> Result<String, ParserError> {
        self.select_first(element_ref, selector)
            .map(|e| self.element_to_text(&e))
            .ok_or_else(|| ParserError::ElementNotFound(error_msg.to_string()))
    }

    /// Extracts optional text from the first element matching the selector
    pub fn extract_text_optional(
        &self,
        element_ref: &ElementRef,
        selector: &Selector,
    ) -> Option<String> {
        self.select_first(element_ref, selector)
            .map(|e| self.element_to_text(&e))
    }

    /// Extracts optional text with HTML line breaks converted to newlines
    pub fn extract_text_with_newlines_optional(
        &self,
        element_ref: &ElementRef,
        selector: &Selector,
    ) -> Option<String> {
        self.select_first(element_ref, selector)
            .map(|e| self.element_to_text_with_newlines(e))
    }

    /// Extracts an attribute from the first element matching the selector
    pub fn extract_attr_from_element(
        &self,
        element_ref: &ElementRef,
        selector: &Selector,
        attr: &str,
        error_msg: &str,
    ) -> Result<String, ParserError> {
        element_ref.select(selector)
            .next()
            .and_then(|e| e.value().attr(attr))
            .map(|s| s.to_string())
            .ok_or_else(|| ParserError::ElementNotFound(error_msg.to_string()))
    }

    /// Extracts an attribute from an element if it exists
    pub fn extract_attr_optional(
        &self,
        element: &ElementRef,
        attr: &str,
    ) -> Option<String> {
        element.value().attr(attr).map(|s| s.to_string())
    }

    /// Extracts an attribute from an element or returns an error
    pub fn extract_attr(
        &self,
        element: &ElementRef,
        attr: &str,
        error_msg: &str,
    ) -> Result<String, ParserError> {
        self.extract_attr_optional(element, attr)
            .ok_or_else(|| ParserError::ElementNotFound(error_msg.to_string()))
    }

    /// Extracts an attribute and parses it as a URL
    pub fn extract_url_attr_from_element(
        &self,
        element_ref: &ElementRef,
        selector: &Selector,
        attr: &str,
        error_msg: &str,
    ) -> Result<Url, ParserError> {
        let url_str = self.extract_attr_from_element(element_ref, selector, attr, error_msg)?;
        Url::parse(&url_str)
            .map_err(|e| ParserError::ValidationFailed(format!("Invalid URL: {}", e)))
    }

    /// Extracts a URL attribute from an element
    pub fn extract_url_attr(
        &self,
        element: &ElementRef,
        attr: &str,
        error_msg: &str,
    ) -> Result<Url, ParserError> {
        let url_str = self.extract_attr(element, attr, error_msg)?;
        Url::parse(&url_str)
            .map_err(|e| ParserError::ValidationFailed(format!("Invalid URL: {}", e)))
    }

    /// Extracts a URL from a style attribute (e.g., background-image: url('...'))
    pub fn extract_url_from_style(
        &self,
        element: &ElementRef,
    ) -> Option<Url> {
        element.value()
            .attr("style")
            .and_then(|style| {
                style.find("url('")
                    .map(|start| &style[start + 5..])
                    .and_then(|remaining| remaining.find("')").map(|end| &remaining[..end]))
            })
            .and_then(|url_str| Url::parse(url_str).ok())
    }

    /// Checks if an element matching the selector exists
    pub fn exists(
        &self,
        element_ref: &ElementRef,
        selector: &Selector,
    ) -> bool {
        element_ref.select(selector).next().is_some()
    }

    /// Helper: Selects the first matching element
    pub fn select_first<'a>(
        &self,
        element_ref: &ElementRef<'a>,
        selector: &Selector,
    ) -> Option<ElementRef<'a>> {
        element_ref.select(selector).next()
    }

    /// Selects all matching elements
    pub fn select_all<'a>(
        &self,
        element_ref: &ElementRef<'a>,
        selector: &Selector,
    ) -> Vec<ElementRef<'a>> {
        element_ref.select(selector).collect()
    }

    /// Converts an element to plain text (ignores all HTML tags)
    pub fn element_to_text(&self, element_ref: &ElementRef) -> String {
        let mut text = String::with_capacity(128);
        self.traverse_nodes(element_ref, &mut text);
        text
    }

    /// Text structure analysis and nested data extraction
    fn traverse_nodes(&self, node: &ElementRef, output: &mut String) {
        for child in node.children() {
            match child.value() {
                Node::Text(text_node) => {
                    let trimmed = text_node.trim();
                    if !trimmed.is_empty() {
                        output.push_str(trimmed);
                    }
                }
                Node::Element(element) => {
                    match element.name() {
                        "br" => output.push('\n'),
                        "a" => {
                            let child_ref = ElementRef::wrap(child).unwrap();
                            self.traverse_nodes(&child_ref, output);
                        }
                        "tg-emoji" => {
                            if let Some(b_tag) = child.first_child().and_then(ElementRef::wrap) {
                                if let Some(emoji_text) = b_tag.text().next() {
                                    output.push_str(emoji_text.trim());
                                }
                            }
                        }
                        _ => {
                            let child_ref = ElementRef::wrap(child).unwrap();
                            self.traverse_nodes(&child_ref, output);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Converts an element to text with <br> tags replaced by newlines
    pub fn element_to_text_with_newlines(&self, element_ref: ElementRef) -> String {
        let mut result = String::with_capacity(256);
        self.process_node(element_ref, &mut result, false);
        result.trim().to_string()
    }

    /// Converts an element to its HTML string representation
    pub fn element_to_html(&self, element_ref: &ElementRef) -> String {
        element_ref.html()
    }

    /// Parses an element into ParsedAndRaw structure
    pub fn parse_parsed_and_raw(&self, element_ref: ElementRef) -> ParsedAndRaw {
        let html_content = element_ref.html();

        let processed_html = HTML_TAG_REGEX.captures(&html_content)
            .and_then(|captures| captures.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or(html_content);

        ParsedAndRaw {
            plain: self.element_to_text(&element_ref),
            html: processed_html,
        }
    }

    /// Recursively processes nodes for text conversion
    fn process_node(&self, node: ElementRef, output: &mut String, is_preformatted: bool) {
        for node in node.children() {
            match node.value() {
                Node::Text(text) => {
                    let text = text.trim();
                    if !text.is_empty() {
                        if !output.is_empty() && !output.ends_with('\n') && !is_preformatted {
                            output.push(' ');
                        }
                        output.push_str(text);
                    }
                }
                Node::Element(element) => {
                    if let Some(child) = ElementRef::wrap(node) {
                        match element.name() {
                            "br" => output.push('\n'),
                            "p" | "div" => {
                                if !output.is_empty() && !output.ends_with('\n') {
                                    output.push('\n');
                                }
                                self.process_node(child, output, false);
                                if !output.ends_with('\n') {
                                    output.push('\n');
                                }
                            }
                            "pre" => {
                                if !output.is_empty() {
                                    output.push('\n');
                                }
                                self.process_node(child, output, true);
                                if !output.ends_with('\n') {
                                    output.push('\n');
                                }
                            }
                            _ => {
                                self.process_node(child, output, is_preformatted);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
