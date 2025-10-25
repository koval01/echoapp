use axum::{
    middleware::Next,
    response::Response,
    http::{Request, HeaderValue},
    body::Body,
};
use std::time::Instant;

pub async fn process_time_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let start_time = Instant::now();

    let response = next.run(request).await;

    let duration = start_time.elapsed();
    let process_time_ms = duration.as_micros() as f64 / 1000.0;

    let process_time_header = if process_time_ms < 10.0 {
        format!("{:.1} ms", process_time_ms)
    } else {
        format!("{:.0} ms", process_time_ms)
    };

    let mut response = response;
    response.headers_mut().insert(
        "x-process-time",
        HeaderValue::from_str(&process_time_header).unwrap(),
    );

    response
}
