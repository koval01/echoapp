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
use crate::service::{check_email_password, create_user};
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
    let result = create_user(
        form_data.email,
        form_data.password,
        form_data.username,
        &db,
    )
        .await;

    if let Err(err) = result {
        let err = format!("Something went wrong: {}", err);
        messages.error(err);

        return Redirect::to("/register");
    }

    messages.success("You have successfully registered!!");

    Redirect::to("/login")
}

/// Handler to serve the Login Page template.
pub async fn login_page_handler(session: Session, messages: Messages) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    let (messages_status, messages) = get_messages(messages);

    HtmlTemplate(LoginTemplate {
        title: "Login".to_string(),
        messages_status,
        messages,
        from_protected,
        ..Default::default()
    })
}

/// Handle the `POST` request of the user login form.
pub async fn login_user_handler(
    headers: HeaderMap,
    session: Session,
    messages: Messages,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    State(state): State<Arc<RwLock<AppState>>>,
    Form(form_data): Form<LoginUserSchema>,
) -> Response {
    let tzone = headers["x-timezone"].to_str().unwrap().to_string();
    set_tzone_in_session(&session, tzone).await;

    let result = check_email_password(
        form_data.email,
        form_data.password,
        &db
    )
        .await;

    if let Err(err) = result {
        let err = format!("Something went wrong: {}", err);
        messages.error(err);

        return Redirect::to("/login").into_response();
    }

    let user_id = result.unwrap().id;

    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
    let claims = TokenClaims {
        sub: String::from(user_id.clone()),
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&state.read().await.config.jwt_secret.as_ref()),
    )
        .unwrap();

    let cookie = Cookie::build(("token", token.to_owned()))
        .path("/")
        .max_age(Duration::hours(1))
        .same_site(SameSite::Lax)
        .http_only(true);

    let headers = AppendHeaders([(SET_COOKIE, cookie.to_string())]);

    messages.success("You have successfully logged in!!");

    (headers, Redirect::to("/todo/list")).into_response()
}

/// User Logout Handler.
pub async fn logout_handler(session: Session, messages: Messages) -> impl IntoResponse {
    set_flag_in_session(&session, false).await;

    let cookie = Cookie::build(("token", ""))
        .path("/")
        .max_age(Duration::hours(-1))
        .same_site(SameSite::Lax)
        .http_only(true);

    let headers = AppendHeaders([(SET_COOKIE, cookie.to_string())]);

    messages.success("You have successfully logged out!!");

    (headers, Redirect::to("/login"))
}
