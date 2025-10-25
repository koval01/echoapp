use serde::{Deserialize, Serialize};
use entities::user;

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: String,
    pub allows_write_to_pm: bool,
    pub photo_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicUser {
    pub id: uuid::Uuid,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: String,
    pub allows_write_to_pm: bool,
    pub photo_url: Option<String>,
    pub created_at: sea_orm::entity::prelude::DateTimeWithTimeZone,
    pub updated_at: sea_orm::entity::prelude::DateTimeWithTimeZone,
}

impl From<user::Model> for PublicUser {
    fn from(u: user::Model) -> Self {
        Self {
            id: u.id,
            first_name: u.first_name,
            last_name: u.last_name,
            username: u.username,
            language_code: u.language_code,
            allows_write_to_pm: u.allows_write_to_pm,
            photo_url: u.photo_url,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}
