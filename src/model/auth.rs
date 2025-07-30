use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RegisterUserSchema {
    pub email: String,
    pub password: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginUserSchema {
    pub email: String,
    pub password: String,
}
