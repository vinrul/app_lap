mod handlers;
pub mod metrics;
pub mod models;
pub mod render;
pub mod service;

use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

/// Route yang menempel pada resource periode.
pub fn period_routes() -> Router<AppState> {
    Router::new()
        .route("/periods/{period_id}/metrics", get(handlers::metrics))
        .route(
            "/periods/{period_id}/drafts/{import_id}",
            get(handlers::get_draft).put(handlers::save_draft),
        )
        .route(
            "/periods/{period_id}/reports",
            get(handlers::list).post(handlers::generate),
        )
        .route(
            "/periods/{period_id}/reports/preview",
            post(handlers::preview_render),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/reports/{report_id}/pdf", get(handlers::download))
        .route(
            "/reports/{report_id}/preview",
            get(handlers::preview_stored),
        )
}
