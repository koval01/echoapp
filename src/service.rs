use std::sync::Arc;
use sea_orm::{EntityTrait, DatabaseConnection, ActiveValue, ActiveModelTrait};

use anyhow::{anyhow, bail, Result};
use entities::user;
use crate::model::user::User;

pub async fn get_user_by_id(
    user_id: i64,
    db: &Arc<DatabaseConnection>,
) -> Result<Option<user::Model>, String> {
    user::Entity::find_by_id(user_id)
        .one(db.as_ref())
        .await
        .map_err(|e| format!("error fetching user from database: {}", e))
}

pub async fn create_user(
    user: User,
    db: &Arc<DatabaseConnection>,
) -> Result<user::Model> {
    let user_exists = get_user_by_id(user.id, db)
        .await
        .map_err(|e| anyhow!("database error: {}", e))?;;

    if user_exists.is_some() {
        bail!("the email is already in use.");
    }

    let new_user = user::ActiveModel {
        id: ActiveValue::Set(user.id),
        first_name: ActiveValue::Set(user.first_name),
        last_name: ActiveValue::Set(user.last_name),
        username: ActiveValue::Set(user.username),
        language_code: ActiveValue::Set(user.language_code),
        allows_write_to_pm: ActiveValue::Set(user.allows_write_to_pm),
        photo_url: ActiveValue::Set(user.photo_url),
        ..Default::default()
    };

    let user = new_user.insert(db.as_ref())
        .await
        .map_err(|e| anyhow!("database error: {}", e))?;

    Ok(user)
}
