use std::sync::Arc;
use sea_orm::{EntityTrait, DatabaseConnection, ActiveModelTrait, DbErr};
use sea_orm::{entity::*, query::*};

use anyhow::{anyhow, bail, Result};
use entities::user;
use crate::model::user::User;

#[allow(dead_code)]
pub async fn get_user_by_id(
    user_id: i64,
    db: &Arc<DatabaseConnection>,
) -> Result<Option<user::Model>, DbErr> {
    user::Entity::find()
        .filter(user::Column::TelegramId.eq(user_id))
        .one(db.as_ref())
        .await
}

pub async fn create_user(
    user: User,
    db: &Arc<DatabaseConnection>,
) -> Result<user::Model> {
    let user_exists = get_user_by_id(user.id, db)
        .await?;

    if user_exists.is_some() {
        bail!("user is already exists");
    }

    let new_user = user::ActiveModel {
        telegram_id: Set(user.id),
        first_name: Set(user.first_name),
        last_name: Set(user.last_name),
        username: Set(user.username),
        language_code: Set(user.language_code),
        allows_write_to_pm: Set(user.allows_write_to_pm),
        photo_url: Set(user.photo_url),
        ..Default::default()
    };

    let user = new_user.insert(db.as_ref())
        .await
        .map_err(|e| anyhow!("database error: {}", e))?;

    Ok(user)
}
