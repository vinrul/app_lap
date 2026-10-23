mod handlers;
pub mod layout;
pub mod metrics;
pub mod models;
pub mod render;
pub mod service;
pub mod sources;

use axum::{
    Router,
    routing::{delete, get, post},
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
        .route(
            "/periods/{period_id}/layout",
            get(handlers::get_layout).put(handlers::save_layout),
        )
        .route(
            "/periods/{period_id}/design/sources",
            get(handlers::design_sources),
        )
        .route(
            "/periods/{period_id}/design/templates/{template_id}/apply",
            post(handlers::apply_template),
        )
        .route(
            "/periods/{period_id}/layout/images",
            post(handlers::upload_layout_image),
        )
        .route(
            "/periods/{period_id}/layout/images/{image_id}",
            get(handlers::layout_image),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/design/templates",
            get(handlers::list_templates).post(handlers::save_template),
        )
        .route(
            "/design/templates/{template_id}",
            delete(handlers::delete_template),
        )
        .route("/reports/{report_id}/pdf", get(handlers::download))
        .route(
            "/reports/{report_id}/preview",
            get(handlers::preview_stored),
        )
}
