use hmac::{Hmac, Mac};
use jwt::{Error, SignWithKey, VerifyWithKey};
use serde_json::Value;
use sha2::Sha256;
use std::collections::BTreeMap;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct JwtClaims {
    pub user_id: Uuid,
    pub issued_at: i64,
    pub expiration: i64,
}

pub struct JwtService {
    key: Hmac<Sha256>,
}

impl JwtService {
    pub fn new(secret: &str) -> Result<Self, Error> {
        let key = Hmac::new_from_slice(secret.as_bytes())?;
        Ok(Self { key })
    }

    pub fn generate_token(&self, user_id: Uuid, expiration_seconds: i64) -> Result<String, Error> {
        let mut claims = BTreeMap::new();

        claims.insert("sub", Value::String(user_id.to_string()));
        claims.insert("iat", Value::Number(OffsetDateTime::now_utc().unix_timestamp().into()));
        claims.insert("exp", Value::Number((OffsetDateTime::now_utc() + Duration::seconds(expiration_seconds)).unix_timestamp().into()));
        claims.insert("iss", Value::String("echoapp".to_string()));
        claims.insert("aud", Value::String("users".to_string()));

        let token = claims.sign_with_key(&self.key)?;
        Ok(token)
    }

    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, Error> {
        let claims: BTreeMap<String, Value> = token.verify_with_key(&self.key)?;

        // Validate expiration
        let exp = claims.get("exp")
            .and_then(|v| v.as_i64())
            .ok_or(Error::InvalidSignature)?;

        let current_time = OffsetDateTime::now_utc().unix_timestamp();
        if exp < current_time {
            return Err(Error::InvalidSignature);
        }

        // Validate issuer and audience (optional but recommended)
        if let Some(iss) = claims.get("iss") {
            if iss != "echoapp" {
                return Err(Error::InvalidSignature);
            }
        }

        // Extract user_id
        let user_id = claims
            .get("sub")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(Error::InvalidSignature)?;

        let issued_at = claims.get("iat")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(JwtClaims {
            user_id,
            issued_at,
            expiration: exp,
        })
    }
}
