use std::str::FromStr;

pub fn parse_subscriber_count(s: &str) -> Option<u64> {
    // Find the numeric part before "subscriber"
    let numeric_part = s.split_whitespace()
        .take_while(|part| part.chars().all(|c| c.is_ascii_digit() || c == ' '))
        .collect::<Vec<&str>>()
        .join("");

    // Remove all whitespace from the numeric part
    let clean_numeric = numeric_part.replace(' ', "");

    // Parse to u64
    u64::from_str(&clean_numeric).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subscriber_count() {
        assert_eq!(parse_subscriber_count("1 subscriber"), Some(1));
        assert_eq!(parse_subscriber_count("22 subscribers"), Some(22));
        assert_eq!(parse_subscriber_count("62 015 subscribers"), Some(62015));
        assert_eq!(parse_subscriber_count("314 551 subscribers"), Some(314551));
        assert_eq!(parse_subscriber_count("11 432 132 subscribers"), Some(11432132));
        assert_eq!(parse_subscriber_count("1 234 567 890 subscribers"), Some(1234567890));
        assert_eq!(parse_subscriber_count("123"), None); // No "subscriber" text
        assert_eq!(parse_subscriber_count("subscribers"), None); // No number
        assert_eq!(parse_subscriber_count("abc 123 subscribers"), None); // Invalid prefix
    }
}
