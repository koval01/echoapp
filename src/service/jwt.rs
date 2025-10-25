use hmac::{Hmac, Mac};
use jwt::{Error, SignWithKey, VerifyWithKey};
use serde_json::Value;
use sha2::Sha256;
use std::collections::BTreeMap;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct JwtClaims {
    pub user_id: Uuid,
    #[allow(dead_code)]
    pub issued_at: i64,
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn validate_token_to_value(&self, token: &str) -> Result<Value, Error> {
        let claims: BTreeMap<String, Value> = token.verify_with_key(&self.key)?;

        // Validate expiration
        let exp = claims.get("exp")
            .and_then(|v| v.as_i64())
            .ok_or(Error::InvalidSignature)?;

        let current_time = OffsetDateTime::now_utc().unix_timestamp();
        if exp < current_time {
            return Err(Error::InvalidSignature);
        }

        // Validate issuer and audience
        if let Some(iss) = claims.get("iss") {
            if iss != "echoapp" {
                return Err(Error::InvalidSignature);
            }
        }

        // Convert BTreeMap to Value
        Ok(Value::Object(
            claims.into_iter().map(|(k, v)| (k, v)).collect()
        ))
    }

    #[allow(dead_code)]
    pub fn validate_token_to_map(&self, token: &str) -> Result<BTreeMap<String, Value>, Error> {
        let claims: BTreeMap<String, Value> = token.verify_with_key(&self.key)?;

        // Validate expiration
        let exp = claims.get("exp")
            .and_then(|v| v.as_i64())
            .ok_or(Error::InvalidSignature)?;

        let current_time = OffsetDateTime::now_utc().unix_timestamp();
        if exp < current_time {
            return Err(Error::InvalidSignature);
        }

        // Validate issuer and audience
        if let Some(iss) = claims.get("iss") {
            if iss != "echoapp" {
                return Err(Error::InvalidSignature);
            }
        }

        Ok(claims)
    }

    #[allow(dead_code)]
    pub fn validate_and_refresh(&self, token: &str, refresh_threshold_hours: i64) -> Result<(JwtClaims, Option<String>), Error> {
        let claims = self.validate_token(token)?;

        // Check if token needs refresh
        let refresh_time = claims.expiration - (refresh_threshold_hours * 3600);
        let current_time = OffsetDateTime::now_utc().unix_timestamp();

        let new_token = if current_time >= refresh_time {
            // Generate new token with same expiration duration
            let hours_remaining = (claims.expiration - current_time) / 3600;
            let new_expiration = hours_remaining.max(1); // At least 1 hour
            Some(self.generate_token(claims.user_id, new_expiration)?)
        } else {
            None
        };

        Ok((claims, new_token))
    }
}
