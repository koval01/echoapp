use std::sync::Arc;
use sea_orm::{
    EntityTrait,
    QueryFilter,
    ColumnTrait,
    DatabaseConnection,
};

use anyhow::{anyhow, bail, Result};
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
};
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