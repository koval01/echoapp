pub mod notfound;
pub mod home;
mod middleware;
pub mod auth;

use chrono::{Local, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
// pub use middleware::auth_middleware;

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use axum_messages::Messages;
use tower_sessions::Session;

const FROM_PROTECTED_KEY: &str = "from_protected";
const TZONE_KEY: &str = "time_zone";

/// Set flag in session.
pub async fn set_flag_in_session(session: &Session, from_protected: bool) {
    session
        .insert(FROM_PROTECTED_KEY, from_protected)
        .await
        .unwrap();
}

/// Set tzone in session.
pub async fn set_tzone_in_session(session: &Session, tzone: String) {
    session.insert(TZONE_KEY, tzone).await.unwrap();
}

/// Format flash messages generated in redirects.
fn get_messages(messages: Messages) -> (String, String) {
    let mut messages = messages
        .into_iter()
        .map(|message| format!("{}: {}", message.level, message))
        .collect::<Vec<_>>()
        .join(", ");
    let mut messages_status = "".to_string();

    if messages.len() != 0 && messages.contains("Success") {
        messages_status = messages[..7].to_string();
        messages = messages[9..].to_string();
    } else if messages.len() != 0 && messages.contains("Error") {
        messages_status = messages[..5].to_string();
        messages = messages[7..].to_string();
    }

    (messages_status, messages)
}

/// convert_datetime converts the datetime format from the
/// database (UTC timestamp) to a string in RFC822Z format,
/// taking the client's timezone (&str) and a datetime (NaiveDateTime).
pub fn convert_datetime(tzone: &str, dt: NaiveDateTime) -> String {
    let tz = tzone.parse::<Tz>().unwrap();
    let converted = Local.from_utc_datetime(&dt);
    let dttz = converted.with_timezone(&tz).to_rfc2822();

    // conversion to RFC822Z format
    let chars = dttz.chars().collect::<Vec<_>>();
    let first_part = chars[5..22].iter().collect::<String>();
    let last_part = chars[25..].iter().collect::<String>();

    format!("{}{}", first_part, last_part)
}

/* --------------------------------------- */
/* ----------- enregion: Utils ----------- */
/* --------------------------------------- */

/* --------------------------------------- */
/* ------ region: Template Rendering ----- */
/* --------------------------------------- */

/// A wrapper type that we'll use to encapsulate HTML parsed
/// by askama into valid HTML for axum to serve.
pub struct HtmlTemplate<T>(pub T);

/// Allows us to convert Askama HTML templates into valid HTML
/// for axum to serve in the response.
impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        // Attempt to render the template with askama
        match self.0.render() {
            // If we're able to successfully parse and aggregate the template, serve it
            Ok(html) => Html(html).into_response(),
            // If we're not, return an error or some bit of fallback HTML
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

/// Home page template
#[derive(Default, Template)]
#[template(path = "auth/home.html")]
pub struct HomeTemplate {
    title: String,
    username: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Register page template
#[derive(Default, Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    title: String,
    username: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Login page template
#[derive(Default, Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    title: String,
    username: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Error 400 page template
#[derive(Default, Template)]
#[template(path = "error/error_400.html")]
pub struct Error400Template {
    title: String,
    username: String,
    reason: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Error 401 page template
#[derive(Default, Template)]
#[template(path = "error/error_401.html")]
pub struct Error401Template {
    title: String,
    username: String,
    reason: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Error 404 page template
#[derive(Default, Template)]
#[template(path = "error/error_404.html")]
pub struct Error404Template {
    title: String,
    username: String,
    reason: String,
    link: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Error 500 page template
#[derive(Default, Template)]
#[template(path = "error/error_500.html")]
pub struct Error500Template {
    title: String,
    username: String,
    reason: String,
    link: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}
