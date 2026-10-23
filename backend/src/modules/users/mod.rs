mod handlers;
pub mod models;
pub mod service;

use axum::{
    Router,
    routing::{get, patch},
};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(handlers::list).post(handlers::create))
        .route("/admin/users/{user_id}", patch(handlers::update))
}
