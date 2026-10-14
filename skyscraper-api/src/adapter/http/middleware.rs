use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

static REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

pub async fn request_id(mut req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map_or_else(|| uuid::Uuid::new_v4().to_string(), String::from);

    req.headers_mut().insert(
        &REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );

    let mut response = next.run(req).await;

    if let Ok(val) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(&REQUEST_ID_HEADER, val);
    }

    response
}
