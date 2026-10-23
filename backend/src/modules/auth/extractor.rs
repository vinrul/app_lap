use axum::{extract::FromRequestParts, http::request::Parts};
use tower_cookies::Cookies;

use crate::{error::ApiError, state::AppState};

use super::models::AuthUser;

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookies = Cookies::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::internal("cookie layer tidak tersedia"))?;
        let token = cookies
            .get(crate::modules::auth::service::COOKIE_NAME)
            .ok_or_else(ApiError::unauthorized)?
            .value()
            .to_string();

        let row = sqlx::query(
            "SELECT users.id, users.username, users.role
             FROM sessions
             JOIN users ON users.id = sessions.user_id
             WHERE sessions.token = ?
               AND sessions.expires_at > ?
               AND users.active = 1",
        )
        .bind(token)
        .bind(chrono::Utc::now().to_rfc3339())
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(ApiError::unauthorized)?;

        use sqlx::Row;
        Ok(AuthUser {
            id: row.get("id"),
            username: row.get("username"),
            role: row.get("role"),
        })
    }
}
