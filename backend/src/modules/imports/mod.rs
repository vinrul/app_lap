pub mod analysis;
pub mod coverage;
pub mod drive;
pub mod duties;
pub mod evidence;
mod handlers;
pub mod models;
pub mod parser;
pub mod qw;
pub mod service;

use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

/// Import berdiri sendiri, tidak lagi menempel pada periode.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/imports", get(handlers::list).post(handlers::commit))
        .route("/imports/preview", post(handlers::preview))
        .route("/imports/{import_id}/records", get(handlers::records))
        .route(
            "/imports/{import_id}/evidence/retry",
            post(handlers::retry_evidence),
        )
        .route("/evidence/{evidence_id}", get(handlers::evidence_file))
}

/// Agregasi data periode dari seluruh import yang mencakup rentang tanggalnya.
pub fn period_routes() -> Router<AppState> {
    Router::new()
        .route("/periods/{period_id}/analysis", get(handlers::analysis))
        .route("/periods/{period_id}/coverage", get(handlers::coverage))
        .route(
            "/periods/{period_id}/unit-detail",
            get(handlers::unit_detail),
        )
        .route(
            "/periods/{period_id}/quick-wins/{code}",
            get(handlers::quick_win_detail),
        )
        .route(
            "/periods/{period_id}/records",
            get(handlers::period_records),
        )
        .route(
            "/periods/{period_id}/records/{record_id}",
            get(handlers::record_detail),
        )
}
