use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RegisterUserSchema {
    pub email: String,
    pub password: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LoginUserSchema {
    pub email: String,
    pub password: String,
}
