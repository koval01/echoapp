use std::sync::Arc;

use axum::{extract::State, http::{header::SET_COOKIE, HeaderMap}, response::{AppendHeaders, IntoResponse, Redirect, Response}, Extension, Form};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_messages::Messages;
use jsonwebtoken::{encode, EncodingKey, Header};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use time::Duration;
use tokio::sync::RwLock;
use tower_sessions::Session;
use uuid::Uuid;
use entities::user::{ActiveModel, Entity as User};

use crate::{
    handler::view::set_tzone_in_session,
    model::auth::{LoginUserSchema, RegisterUserSchema},
    model::jwt::TokenClaims,
    AppState,
};

use super::{
    get_messages, set_flag_in_session, Error404Template, Error500Template, HomeTemplate,
    HtmlTemplate, LoginTemplate, RegisterTemplate, FROM_PROTECTED_KEY,
};

pub async fn register_page_handler(session: Session, messages: Messages) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    let (messages_status, messages) = get_messages(messages);

    HtmlTemplate(RegisterTemplate {
        title: "Register".to_string(),
        messages_status,
        messages,
        from_protected,
        ..Default::default()
    })
}

pub async fn register_user_handler(
    messages: Messages,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Form(form_data): Form<RegisterUserSchema>,
) -> impl IntoResponse {
    let user = ActiveModel {
        id: Set(Uuid::new_v4()),
        username: Set(form_data.username),
        email: Set(form_data.email),
        password: Set(form_data.password),
        ..Default::default()
    };

    let result = user.insert(&*db)
        .await;

    if let Err(err) = result {
        let err = format!("Something went wrong: {}", err);
        messages.error(err);

        return Redirect::to("/register");
    }

    messages.success("You have successfully registered!!");

    Redirect::to("/login")
}
