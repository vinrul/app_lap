use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

pub async fn add_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("SAMEORIGIN"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}
