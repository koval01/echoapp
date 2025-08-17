use hmac::{Hmac, Mac};
use subtle::ConstantTimeEq;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use ahash::AHashMap;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use hex::FromHex;
use base64::Engine as _;
use base64::engine::general_purpose;

type HmacSha256 = Hmac<Sha256>;

// Telegram's Ed25519 public keys
const TELEGRAM_TEST_PUBLIC_KEY: &str = "40055058a4ee38156a06562e52eece92a771bcd8346a8c4615cb7376eddf72ec";
const TELEGRAM_PRODUCTION_PUBLIC_KEY: &str = "e7bf03a2fa4602af4580703d88dda5bb59f32ed8b02a56c187fe7d34caed242d";

pub fn validate_init_data(init_data: &str, bot_token: &str) -> Result<bool, &'static str> {
    if init_data.len() > 1024 {
        return Err("Input data too long");
    }

    if !init_data.chars().all(|c| (c.is_ascii() && !c.is_control()) || c == '&' || c == '=') {
        return Err("Invalid characters in input");
    }

    let mut params = AHashMap::with_capacity(10);
    let mut received_hash = None;
    let mut received_signature = None;

    for pair in init_data.split('&') {
        if let Some(sep_idx) = pair.find('=') {
            let (key, value) = pair.split_at(sep_idx);
            match key {
                "hash" => received_hash = Some(&value[1..]),
                "signature" => received_signature = Some(&value[1..]),
                _ => {
                    params.insert(key, &value[1..]);
                }
            }
        }
    }

    // Validate auth_date
    let auth_date = params
        .get("auth_date")
        .ok_or("Missing 'auth_date' parameter")?
        .parse::<u64>()
        .map_err(|_| "Invalid 'auth_date' value")?;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "System time is before UNIX epoch")?
        .as_secs();

    if current_time > auth_date + 14400 {
        return Err("auth_date expired");
    }

    // First try to validate with Ed25519 signature if present
    if let Some(signature) = received_signature {
        if validate_with_ed25519(&params, signature, bot_token)? {
            return Ok(true);
        }
    }

    // Fall back to HMAC validation if signature validation fails or isn't present
    if let Some(hash) = received_hash {
        return validate_with_hmac(&params, hash, bot_token);
    }

    Err("Neither hash nor signature provided for validation")
}

fn validate_with_hmac(params: &AHashMap<&str, &str>, received_hash: &str, bot_token: &str) -> Result<bool, &'static str> {
    // Compute secret key from bot_token
    // Telegram requirement: secret_key = HMAC_SHA256("WebAppData", bot_token)
    let mut mac = HmacSha256::new_from_slice(b"WebAppData")
        .expect("Failed to create HMAC instance");
    mac.update(bot_token.as_bytes());
    let secret_key = mac.finalize().into_bytes();

    let mut pairs: Vec<(String, String)> = Vec::with_capacity(params.len());
    for (k, v) in params {
        pairs.push((k.to_string(), v.to_string()));
    }
    pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    let data_check_string = pairs.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    let mut mac = HmacSha256::new_from_slice(&secret_key)
        .map_err(|_| "Failed to create HMAC instance")?;
    mac.update(data_check_string.as_bytes());
    let hash = mac.finalize().into_bytes();

    let mut hex_buf = [0u8; 64];
    hex::encode_to_slice(&hash, &mut hex_buf)
        .map_err(|_| "Failed to encode hash")?;

    let computed_hash = std::str::from_utf8(&hex_buf)
        .map_err(|_| "Invalid UTF-8 in hash")?;

    let ok = computed_hash.as_bytes().ct_eq(received_hash.as_bytes()).unwrap_u8() == 1;
    Ok(ok)
}

fn validate_with_ed25519(params: &AHashMap<&str, &str>, signature: &str, bot_token: &str) -> Result<bool, &'static str> {
    let bot_id = bot_token.split(':').next().ok_or("Invalid bot token format")?;

    let mut pairs: Vec<(String, String)> = Vec::with_capacity(params.len());
    for (k, v) in params {
        pairs.push((k.to_string(), v.to_string()));
    }
    pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    // Construct the data-check-string
    let data_check_string = format!("{}:WebAppData\n{}",
                                    bot_id,
                                    pairs.iter()
                                        .map(|(k, v)| format!("{}={}", k, v))
                                        .collect::<Vec<_>>()
                                        .join("\n")
    );

    // Get the appropriate public key
    let public_key_hex = if cfg!(test) {
        TELEGRAM_TEST_PUBLIC_KEY
    } else {
        TELEGRAM_PRODUCTION_PUBLIC_KEY
    };

    // Convert hex public key to bytes
    let public_key_bytes: [u8; 32] = Vec::from_hex(public_key_hex)
        .map_err(|_| "Invalid Telegram public key hex")?
        .try_into()
        .map_err(|_| "Invalid public key length")?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| "Invalid Telegram public key bytes")?;

    // Decode base64url signature
    let signature_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| "Invalid base64url signature")?;

    // Convert to fixed-size array
    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| "Invalid signature length")?;

    let sig = Signature::from_slice(&signature_array)
        .map_err(|_| "Invalid signature bytes")?;

    // Verify the signature
    Ok(verifying_key.verify(data_check_string.as_bytes(), &sig).is_ok())
}
