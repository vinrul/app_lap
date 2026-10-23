mod handlers;
mod service;

use axum::{Router, routing::get};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/dashboard", get(handlers::dashboard))
}
