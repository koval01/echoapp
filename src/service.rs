use std::sync::Arc;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, DatabaseConnection, ActiveValue, ActiveModelTrait};

use anyhow::{anyhow, bail, Result};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use uuid::Uuid;
use entities::user;

pub async fn check_email_password(
    email: String,
    password: String,
    db: &Arc<DatabaseConnection>,
) -> Result<user::Model> {
    let email = email.to_ascii_lowercase();

    // Correct query construction
    let user = user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db.as_ref())
        .await
        .map_err(|e| anyhow!("database error: {}.", e))?
        .ok_or_else(|| anyhow!("invalid email or password."))?;

    let is_valid = match PasswordHash::new(&user.password) {
        Ok(parsed_hash) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_or(false, |_| true),
        Err(_err) => false,
    };

    if !is_valid {
        bail!("invalid email or password.");
    }

    Ok(user)
}

pub async fn create_user(
    email: String,
    password: String,
    username: String,
    db: &Arc<DatabaseConnection>,
) -> Result<user::Model> {
    let email = email.to_ascii_lowercase();
    let user_exists = user::Entity::find()
        .filter(user::Column::Email.eq(&email))
        .one(db.as_ref())
        .await
        .map_err(|e| anyhow!("database error: {}", e))?;

    if user_exists.is_some() {
        bail!("the email is already in use.");
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("failed to hash password: {}", e))
        .map(|hash| hash.to_string())?;

    // Create new user
    let uuid = Uuid::new_v4().to_string();
    let new_user = user::ActiveModel {
        id: ActiveValue::Set(uuid.parse()?),
        email: ActiveValue::Set(email),
        password: ActiveValue::Set(hashed_password),
        username: ActiveValue::Set(username),
        ..Default::default()
    };

    let user = new_user.insert(db.as_ref())
        .await
        .map_err(|e| anyhow!("database error: {}", e))?;

    Ok(user)
}
