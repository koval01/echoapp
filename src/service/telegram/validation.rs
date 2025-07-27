use regex::Regex;
use lazy_static::lazy_static;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid channel name format. Must match ^[a-zA-Z][a-zA-Z0-9_]{{3,31}}$")]
    InvalidChannelFormat,
}

pub fn validate_channel_name(channel: &str) -> Result<(), ValidationError> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]{3,31}$").unwrap();
    }

    if RE.is_match(channel) {
        Ok(())
    } else {
        Err(ValidationError::InvalidChannelFormat)
    }
}
