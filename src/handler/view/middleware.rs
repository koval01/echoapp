use std::sync::Arc;
use uuid::Uuid;

use axum::{
    extract::{Request, Extension, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::{DatabaseConnection, EntityTrait};
use tower_sessions::Session;
use tokio::sync::RwLock;
use crate::entities::user::Entity as UserModel;

use crate::handler::view::{set_flag_in_session, Error401Template, HtmlTemplate};
use crate::model::jwt::TokenClaims;
use crate::AppState;

/// Middleware to manage authorization.
pub async fn auth_middleware(
    cookie_jar: CookieJar,
    session: Session,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    State(state): State<Arc<RwLock<AppState>>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let token_option = cookie_jar
        .get("token")
        .map(|cookie| cookie.value().to_string())
        .or_else(|| {
            req.headers()
                .get(header::AUTHORIZATION)
                .and_then(|auth_header| auth_header.to_str().ok())
                .and_then(|auth_value| {
                    if auth_value.starts_with("Bearer ") {
                        Some(auth_value[7..].to_owned())
                    } else {
                        None
                    }
                })
        });

    let token = if let Some(tk) = token_option {
        tk
    } else {
        set_flag_in_session(&session, false).await;
        return Err(HtmlTemplate(Error401Template {
            title: "Error 401".to_string(),
            reason: "You are not logged in, please provide token".to_string(),
            is_error: true,
            ..Default::default()
        })
            .into_response());
    };

    let claims = match decode::<TokenClaims>(
        &token,
        &DecodingKey::from_secret(&state.read().await.config.jwt_secret.as_ref()),
        &Validation::default(),
    ) {
        Ok(token_data) => token_data.claims,
        Err(_) => {
            set_flag_in_session(&session, false).await;
            return Err(HtmlTemplate(Error401Template {
                title: "Error 401".to_string(),
                reason: "Invalid token".to_string(),
                is_error: true,
                ..Default::default()
            })
                .into_response());
        }
    };

    // Convert the string to Uuid
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            set_flag_in_session(&session, false).await;
            return Err(HtmlTemplate(Error401Template {
                title: "Error 401".to_string(),
                reason: "Invalid user ID format".to_string(),
                is_error: true,
                ..Default::default()
            })
                .into_response());
        }
    };

    // Fetch user from a database
    let user = match UserModel::find_by_id(user_id).one(&*db).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            set_flag_in_session(&session, false).await;
            return Err(HtmlTemplate(Error401Template {
                title: "Error 401".to_string(),
                reason: "The user belonging to this token no longer exists".to_string(),
                is_error: true,
                ..Default::default()
            })
                .into_response());
        }
        Err(e) => {
            set_flag_in_session(&session, false).await;
            return Err(HtmlTemplate(Error401Template {
                title: "Error 401".to_string(),
                reason: format!("Database error: {}", e),
                is_error: true,
                ..Default::default()
            })
                .into_response());
        }
    };

    set_flag_in_session(&session, true).await;
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}
