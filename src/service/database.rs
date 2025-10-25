use std::sync::Arc;
use sea_orm::{EntityTrait, DatabaseConnection, ActiveModelTrait, DbErr, ActiveValue::Set};
use sea_orm::{entity::*, query::*};

use anyhow::{anyhow, bail, Result};
use moka::future::Cache;
use sea_orm::sqlx::types::chrono::Utc;
use uuid::Uuid;
use entities::user;
use entities::user::Model;
use crate::cache_fetch;
use crate::error::ApiError;
use crate::model::user::{PublicUser, User};
use crate::util::cache::{CacheBackend, CacheWrapper};

#[allow(dead_code)]
pub async fn get_user_by_id(
    user_id: Uuid,
    db: &Arc<DatabaseConnection>,
    display_full: bool,
) -> Result<Option<serde_json::Value>, DbErr> {
    let user = user::Entity::find_by_id(user_id)
        .one(db.as_ref())
        .await?;

    let result = match user {
        Some(u) => {
            if display_full {
                Some(serde_json::to_value(u).unwrap())
            } else {
                Some(serde_json::to_value(PublicUser::from(u)).unwrap())
            }
        }
        None => None,
    };

    Ok(result)
}

pub async fn get_user_by_telegram_id(
    telegram_id: i64,
    db: &Arc<DatabaseConnection>,
) -> Result<Option<Model>, DbErr> {
    user::Entity::find()
        .filter(user::Column::TelegramId.eq(telegram_id))
        .one(db.as_ref())
        .await
}

pub async fn fetch_user_with_cache(
    user_id: Uuid,
    db: &Arc<DatabaseConnection>,
    redis_pool: CacheBackend,
    moka_cache: Cache<String, String>,
) -> Result<Model, ApiError> {
    let cache = CacheWrapper::<Model>::new(redis_pool, moka_cache, 60, 10);
    let cache_key = format!("user_uuid_full:{}", user_id);

    let user: Model = cache_fetch!(
        cache,
        &cache_key,
        async {
            user::Entity::find_by_id(user_id)
                .one(db.as_ref())
                .await
                .map_err(ApiError::from)
        }
    )?;

    Ok(user)
}

pub async fn create_user(
    user: User,
    db: &Arc<DatabaseConnection>,
) -> Result<Model> {
    let user_exists = get_user_by_telegram_id(user.id, db)
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

pub async fn update_user(
    init_user: &User,
    db_user: &Model,
    db: &Arc<DatabaseConnection>,
) -> Result<Model> {
    let mut user_to_update: user::ActiveModel = db_user.clone().into();

    let mut has_changes = false;

    // Update only the fields that have changed
    if init_user.first_name != db_user.first_name {
        user_to_update.first_name = Set(init_user.first_name.clone());
        has_changes = true;
    }
    if init_user.last_name != db_user.last_name {
        user_to_update.last_name = Set(init_user.last_name.clone());
        has_changes = true;
    }
    if init_user.username != db_user.username {
        user_to_update.username = Set(init_user.username.clone());
        has_changes = true;
    }
    if init_user.language_code != db_user.language_code {
        user_to_update.language_code = Set(init_user.language_code.clone());
        has_changes = true;
    }
    if init_user.allows_write_to_pm != db_user.allows_write_to_pm {
        user_to_update.allows_write_to_pm = Set(init_user.allows_write_to_pm);
        has_changes = true;
    }
    if init_user.photo_url != db_user.photo_url {
        user_to_update.photo_url = Set(init_user.photo_url.clone());
        has_changes = true;
    }

    // Only update if there are actual changes
    if has_changes {
        // Update the updated_at timestamp with the correct SeaORM type
        user_to_update.updated_at = Set(sea_orm::prelude::DateTimeWithTimeZone::from(Utc::now()));

        let updated_user = user_to_update.update(db.as_ref())
            .await
            .map_err(|e| anyhow!("database error: {}", e))?;

        Ok(updated_user)
    } else {
        // No changes needed, return the original user
        Ok(db_user.clone())
    }
}

// Make this public so it can be used by both modules
pub fn needs_update(init_user: &User, db_user: &Model) -> bool {
    init_user.first_name != db_user.first_name
        || init_user.last_name != db_user.last_name
        || init_user.username != db_user.username
        || init_user.language_code != db_user.language_code
        || init_user.allows_write_to_pm != db_user.allows_write_to_pm
        || init_user.photo_url != db_user.photo_url
}
