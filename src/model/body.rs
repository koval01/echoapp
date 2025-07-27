use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Body {
    pub channel: Channel,
    pub content: Content,
    pub meta: Meta,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Channel {
    pub username: String,
    pub title: ParsedAndRaw,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<ParsedAndRaw>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Url>,
    pub counters: Counter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParsedAndRaw {
    #[serde(rename = "string")]
    pub plain: String,
    pub html: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Counter {
    pub subscribers: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub videos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Content {
    pub posts: Posts,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Posts {
    Single(Post),
    Multiple(Vec<Post>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Post {
    pub id: i64,
    pub content: ContentPost,
    pub footer: Footer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forwarded: Option<Forwarded>,
    pub view: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContentPost {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<Vec<MediaItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<Poll>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline: Option<Vec<Inline>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<Reply>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_link: Option<PreviewLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reacts: Option<Vec<Reaction>>
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Text {
    #[serde(rename = "string")]
    pub plain: String,
    pub html: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<TextEntity>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntityType {
    Hashtag,
    Bold,
    Italic,
    Underline,
    Code,
    Strikethrough,
    Spoiler,
    Emoji {
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    TextLink {
        url: String,
    },
    Url,
    Animoji {
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    Pre {
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEntity {
    pub offset: usize,
    pub length: usize,
    #[serde(flatten)]
    pub entity_type: EntityType,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waves: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<Duration>,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Video,
    Voice,
    RoundVideo,
    Sticker,
    Gif,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Duration {
    pub formatted: String,
    pub raw: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Poll {
    pub question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub poll_type: Option<String>,
    pub votes: String,
    pub options: Vec<PollOption>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PollOption {
    pub name: String,
    pub percent: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Inline {
    pub title: String,
    pub url: Url,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reply {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Url>,
    pub name: ParsedAndRaw,
    pub text: ParsedAndRaw,
    pub to_message: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PreviewLink {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<ParsedAndRaw>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<Url>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionType {
    TelegramStars,
    Emoji,
    CustomEmoji,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reaction {
    pub count: String,
    #[serde(rename = "type")]
    pub r#type: ReactionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_image: Option<Url>
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Footer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub views: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<ParsedAndRaw>,
    pub date: Date,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Date {
    #[serde(rename = "string")]
    pub formatted: String,
    pub unix: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Forwarded {
    pub name: ParsedAndRaw,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Meta {
    pub offset: OffsetItem,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OffsetItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<i64>,
}
