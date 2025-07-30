use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password: String,
    pub username: String,
}
