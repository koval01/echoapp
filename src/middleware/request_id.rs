use axum::{
    middleware::Next,
    response::Response,
    http::{Request, HeaderValue},
    body::Body,
};
use sentry::Scope;
use tracing::{debug_span, Instrument};
use uuid::Uuid;
use hostname::get;

use crate::error::RequestCtx;

pub async fn request_id_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let path = request.uri().to_string();

    let instance = get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    request.extensions_mut().insert(RequestCtx {
        id: request_id.clone(),
        method: method.clone().to_string(),
        path: path.clone(),
        instance: instance.clone(),
    });

    sentry::configure_scope(|scope: &mut Scope| {
        scope.set_tag("request_id", &request_id);
        scope.set_tag("http.method", method.as_str());
        scope.set_tag("http.url", path.to_string());
        scope.set_tag("instance", &instance);
    });

    let span = debug_span!(
        "request",
        request_id = %request_id,
        method = %method,
        path = %path,
        instance = %instance
    );

    let response = next.run(request).instrument(span).await;

    let mut response = response;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap(),
    );
    response.headers_mut().insert(
        "x-instance",
        HeaderValue::from_str(&instance).unwrap(),
    );

    response
}
