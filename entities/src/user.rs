use sea_orm::entity::prelude::*;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(indexed, unique)]
    pub telegram_id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    #[sea_orm(indexed)]
    pub username: Option<String>,
    #[sea_orm(default_value = "en")]
    pub language_code: String,
    #[sea_orm(default_value = "true")]
    pub allows_write_to_pm: bool,
    #[sea_orm(default_value = "false")]
    pub is_admin: bool,
    pub photo_url: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let mut active_model = <Self as ActiveModelTrait>::default();
        active_model.id = Set(Uuid::new_v4());
        active_model.allows_write_to_pm = Set(true);
        active_model.language_code = Set("en".to_owned());
        active_model.is_admin = Set(false);
        active_model
    }
}