use axum::{
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};

pub async fn add_headers(request: Request, next: Next) -> Response {
    // Respons API tidak boleh dipakai ulang lintas akun, mis. metrik Polda
    // yang terbawa saat masuk sebagai akun polres.
    let is_api = request.uri().path().starts_with("/api");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("SAMEORIGIN"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    if is_api {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}
