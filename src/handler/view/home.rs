use axum::response::IntoResponse;
use tower_sessions::Session;

use super::{
    HomeTemplate, HtmlTemplate, FROM_PROTECTED_KEY,
};

pub async fn home_handler(session: Session) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    HtmlTemplate(HomeTemplate {
        title: "Home".to_string(),
        from_protected,
        ..Default::default()
    })
}
