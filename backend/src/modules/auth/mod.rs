mod extractor;
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
        .route("/auth/login", post(handlers::login))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/password", post(handlers::change_password))
        .route("/auth/me", get(handlers::me))
}

pub use models::AuthUser;
