use std::time::Instant;

use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct RequestId(pub String);

pub async fn attach_request_id(mut request: Request<Body>, next: Next) -> Response {
    let started_at = Instant::now();
    let request_id = Uuid::now_v7().to_string();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).expect("UUID request IDs are valid header values"),
    );

    info!(
        request_id,
        %method,
        %path,
        status = response.status().as_u16(),
        duration_ms = started_at.elapsed().as_millis(),
        "request completed"
    );
    response
}
