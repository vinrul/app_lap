use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::state::AppState;

/// Menolak Host header yang tidak diizinkan. Tujuannya mengurangi risiko DNS
/// rebinding saat server berjalan di localhost, dan membatasi akses saat bind
/// diarahkan ke 0.0.0.0 melalui `LAPORAN_ALLOWED_HOSTS`.
pub async fn require_local_host(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if let Some(host) = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
    {
        let host = normalize_host(host);
        let allowed = is_loopback_host(&host)
            || state
                .config
                .allowed_hosts
                .iter()
                .any(|allowed| allowed == "*" || allowed.eq_ignore_ascii_case(&host))
            || (state.config.allowed_hosts.is_empty() && !state.config.bind_addr.is_loopback());
        if !allowed {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "Host tidak diizinkan. Tambahkan host ini ke LAPORAN_ALLOWED_HOSTS.",
                    "code": "forbidden",
                })),
            )
                .into_response();
        }
    }
    next.run(request).await
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn normalize_host(raw: &str) -> String {
    let raw = raw.trim();
    if raw.starts_with('[') {
        if let Some(end) = raw.find(']') {
            return raw[1..end].to_ascii_lowercase();
        }
    }
    if raw.matches(':').count() > 1 {
        return raw.to_ascii_lowercase();
    }
    let host = raw
        .rsplit_once(':')
        .filter(|(_, port)| !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()))
        .map(|(name, _)| name)
        .unwrap_or(raw);
    host.to_ascii_lowercase()
}
