use axum::response::IntoResponse;
use tower_sessions::Session;

use super::{
    Error404Template, HtmlTemplate, FROM_PROTECTED_KEY,
};


pub async fn handler_404(session: Session) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    let link = if from_protected {
        "/todo/list".to_string()
    } else {
        "/".to_string()
    };

    HtmlTemplate(Error404Template {
        title: "Error 404".to_string(),
        reason: "Nothing to see here".to_string(),
        link,
        is_error: true,
        ..Default::default()
    })
}
