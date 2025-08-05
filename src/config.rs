use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub session_maxage: i32,
}

impl Config {
    pub fn init() -> Self {
        let session_maxage = env::var("SESSION_MAXAGE").expect("SESSION_MAXAGE must be set");

        Self {
            session_maxage: session_maxage.parse::<i32>().unwrap(),
        }
    }
}
