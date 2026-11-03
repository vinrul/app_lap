mod handlers;
pub mod models;
pub mod service;

use axum::{
    Router,
    routing::{delete, get, patch},
};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(handlers::list).post(handlers::create))
        .route("/admin/users/options", get(handlers::options))
        .route("/admin/users/{user_id}", patch(handlers::update))
        .route("/admin/sessions", get(handlers::sessions))
        .route(
            "/admin/sessions/{session_id}",
            delete(handlers::revoke_session),
        )
        .route(
            "/admin/users/{user_id}/sessions",
            delete(handlers::revoke_user_sessions),
        )
}
