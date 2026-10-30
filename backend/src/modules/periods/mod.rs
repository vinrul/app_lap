mod handlers;
pub mod models;
pub mod service;

use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/periods", get(handlers::list).post(handlers::create))
        .route(
            "/periods/{period_id}",
            get(handlers::detail).patch(handlers::update),
        )
        .route("/periods/{period_id}/archive", post(handlers::archive))
        .route("/periods/{period_id}/restore", post(handlers::restore))
}
