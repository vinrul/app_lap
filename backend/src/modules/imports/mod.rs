mod handlers;
pub mod models;
pub mod parser;
pub mod service;

use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

/// Route yang menempel pada resource periode.
pub fn period_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/periods/{period_id}/imports",
            get(handlers::list).post(handlers::commit),
        )
        .route(
            "/periods/{period_id}/imports/preview",
            post(handlers::preview),
        )
        .route("/periods/{period_id}/records", get(handlers::records))
}
