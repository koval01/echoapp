use std::env;
use std::env::VarError;

#[derive(Debug, Clone)]
pub struct Config {
    pub session_maxage: i64,
    pub cors_host: String,
    pub sentry_dsn: String,
    pub database_url: String,
    pub redis_url: Result<String, VarError>,
    pub server_bind_addr: String,
    pub bot_token: String,
    pub jwt_secret: String,
    pub test_pub_key: String,
}

impl Config {
    pub fn init() -> Self {
        let session_maxage = env::var("SESSION_MAXAGE").unwrap_or_else(|_| "14400".to_string());
        let cors_host = env::var("CORS_HOST").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let sentry_dsn = env::var("SENTRY_DSN").unwrap_or_else(|_| "".to_string());
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let redis_url = env::var("REDIS_URL");
        let server_bind_addr = env::var("SERVER_BIND").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
        let bot_token = env::var("BOT_TOKEN").expect("BOT_TOKEN must be set");
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let test_pub_key = env::var("TEST_PUBLIC_KEY").unwrap_or_else(|_| "".to_string());

        Self {
            session_maxage: session_maxage.parse::<i64>().unwrap(),
            cors_host,
            sentry_dsn,
            database_url,
            redis_url,
            server_bind_addr,
            bot_token,
            jwt_secret,
            test_pub_key
        }
    }
}
