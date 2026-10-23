mod handlers;
pub mod models;
pub mod service;

use axum::{Router, routing::get};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/periods", get(handlers::list).post(handlers::create))
        .route("/periods/{period_id}", get(handlers::detail))
}
