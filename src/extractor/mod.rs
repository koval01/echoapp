mod initdata;
mod strictuuid;
mod jwt;

use axum::http::request::Parts;
pub use initdata::*;
pub use jwt::*;
pub use strictuuid::*;
use crate::error::{ApiError, RequestCtx};

fn get_error(parts: &Parts, error: ApiError) -> ApiError {
    if let Some(ctx) = parts.extensions.get::<RequestCtx>() {
        error.with_ctx(ctx.clone())
    } else {
        error
    }
}
