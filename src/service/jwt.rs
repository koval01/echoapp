use hmac::{Hmac, Mac};
use jwt::{Error, SignWithKey};
use serde_json::Value;
use sha2::Sha256;
use std::collections::BTreeMap;
use time::{Duration, OffsetDateTime};

pub struct JwtService {
    key: Hmac<Sha256>,
}

impl JwtService {
    pub fn new(secret: &str) -> Result<Self, Error> {
        let key = Hmac::new_from_slice(secret.as_bytes())?;
        Ok(Self { key })
    }

    pub fn generate_token(&self, user_id: uuid::Uuid, expiration_hours: i64) -> Result<String, Error> {
        let mut claims = BTreeMap::new();

        claims.insert("sub", Value::String(user_id.to_string()));
        claims.insert("iat", Value::Number(OffsetDateTime::now_utc().unix_timestamp().into()));
        claims.insert("exp", Value::Number((OffsetDateTime::now_utc() + Duration::hours(expiration_hours)).unix_timestamp().into()));

        let token = claims.sign_with_key(&self.key)?;
        Ok(token)
    }
}
