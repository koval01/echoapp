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

// Telegram's Ed25519 public key
const TELEGRAM_PUBLIC_KEY: &str = "e7bf03a2fa4602af4580703d88dda5bb59f32ed8b02a56c187fe7d34caed242d";

pub fn validate_init_data(init_data: &str, bot_token: &str, test_pub_key: &str) -> Result<bool, &'static str> {
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
            let value = &value[1..]; // Remove the '=' character

            match key {
                "hash" => received_hash = Some(value),
                "signature" => received_signature = Some(value),
                _ => {
                    params.insert(key, value);
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

    if current_time > auth_date + 50 {
        return Err("auth_date expired");
    }

    // For HMAC validation, we need all parameters except hash (but including signature)
    let hmac_valid = if let Some(hash) = received_hash {
        validate_with_hmac(&params, received_signature, hash, bot_token)?
    } else {
        return Err("Missing 'hash' parameter for HMAC validation");
    };

    // For Ed25519 validation, we need all parameters except signature
    let signature_valid = if let Some(signature) = received_signature {
        validate_with_ed25519(&params, signature, bot_token, test_pub_key)?
    } else {
        return Err("Missing 'signature' parameter for Ed25519 validation");
    };

    Ok(hmac_valid && signature_valid)
}

fn validate_with_hmac(
    params: &AHashMap<&str, &str>,
    signature: Option<&str>,
    received_hash: &str,
    bot_token: &str
) -> Result<bool, &'static str> {
    // Compute secret key from bot_token
    // Telegram requirement: secret_key = HMAC_SHA256("WebAppData", bot_token)
    let mut mac = HmacSha256::new_from_slice(b"WebAppData")
        .expect("Failed to create HMAC instance");
    mac.update(bot_token.as_bytes());
    let secret_key = mac.finalize().into_bytes();

    // Create a new map that includes all original params plus signature (if present)
    // but excludes hash
    let mut hmac_params = params.clone();
    if let Some(sig) = signature {
        hmac_params.insert("signature", sig);
    }

    let mut pairs: Vec<(String, String)> = Vec::with_capacity(hmac_params.len());
    for (k, v) in &hmac_params {
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

fn validate_with_ed25519(
    params: &AHashMap<&str, &str>,
    signature: &str,
    bot_token: &str,
    test_pub_key: &str
) -> Result<bool, &'static str> {
    let bot_id = bot_token.split(':').next().ok_or("Invalid bot token format")?;

    // For Ed25519 validation, we use all parameters except signature
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
    let public_key_hex = if !test_pub_key.is_empty() {
        test_pub_key
    } else {
        TELEGRAM_PUBLIC_KEY
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
