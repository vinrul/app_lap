use axum::{Json, Router, extract::DefaultBodyLimit, middleware, routing::get};
use serde::Serialize;
use serde_json::json;
use tower_cookies::CookieManagerLayer;
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::{middleware::local_only, modules, state::AppState};

#[derive(Serialize)]
struct MetaResponse {
    polda_name: String,
    app_name: String,
    max_upload_bytes: usize,
    timezone_label: String,
}

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/meta", get(meta))
        .merge(modules::auth::routes())
        .merge(modules::dashboard::routes())
        .merge(modules::periods::routes())
        .merge(modules::imports::routes())
        .merge(modules::imports::period_routes())
        .merge(modules::map::routes())
        .merge(modules::reports::period_routes())
        .merge(modules::reports::routes())
        .merge(modules::users::routes())
        .with_state(state.clone());

    let frontend =
        ServeDir::new(state.config.frontend_dir.clone()).append_index_html_on_directories(true);

    Router::new()
        .nest("/api", api)
        .fallback_service(frontend)
        .layer(middleware::from_fn(
            crate::middleware::security_headers::add_headers,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            local_only::require_local_host,
        ))
        .layer(DefaultBodyLimit::max(
            state.config.max_upload_bytes + 1024 * 1024,
        ))
        .layer(CookieManagerLayer::new())
        .layer(TraceLayer::new_for_http())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn meta(axum::extract::State(state): axum::extract::State<AppState>) -> Json<MetaResponse> {
    Json(MetaResponse {
        polda_name: state.config.polda_name.clone(),
        app_name: state.config.app_name.clone(),
        max_upload_bytes: state.config.max_upload_bytes,
        timezone_label: state.config.timezone_label.clone(),
    })
}
