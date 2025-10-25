use axum::{
    middleware::Next,
    response::Response,
    http::{Request, HeaderValue},
    body::Body,
};
use hostname::get;

pub async fn instance_name_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let hostname = get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    let response = next.run(request).await;

    let mut response = response;
    response.headers_mut().insert(
        "x-instance",
        HeaderValue::from_str(&hostname).unwrap(),
    );

    response
}
