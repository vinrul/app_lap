use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;
use time::Duration as TimeDuration;
use tower_cookies::{Cookie, Cookies};

use crate::{
    error::{ApiError, ApiResult},
    modules::auth::{
        AuthUser,
        models::{LoginRequest, UserView},
        service::{self, COOKIE_NAME},
    },
    shared::audit,
    state::AppState,
};

pub async fn login(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(input): Json<LoginRequest>,
) -> ApiResult<Json<UserView>> {
    if input.username.trim().is_empty() || input.password.is_empty() {
        return Err(ApiError::bad_request(
            "Nama pengguna dan kata sandi wajib diisi.",
        ));
    }

    let user = service::authenticate(&state.db, &input.username, &input.password).await?;
    let (token, _expires) =
        service::create_session(&state.db, &user.id, state.config.session_hours).await?;

    let mut cookie = Cookie::new(COOKIE_NAME, token);
    cookie.set_http_only(true);
    cookie.set_same_site(tower_cookies::cookie::SameSite::Strict);
    cookie.set_path("/");
    cookie.set_max_age(TimeDuration::hours(state.config.session_hours));
    if state.config.secure_cookie {
        cookie.set_secure(true);
    }
    cookies.add(cookie);

    audit::record(
        &state.db,
        Some(&user.id),
        "auth.login",
        "user",
        &user.id,
        json!({ "username": user.username }),
    )
    .await?;

    Ok(Json(user.into()))
}

pub async fn logout(State(state): State<AppState>, cookies: Cookies) -> ApiResult<StatusCode> {
    if let Some(cookie) = cookies.get(COOKIE_NAME) {
        service::delete_session(&state.db, cookie.value()).await?;
    }
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_max_age(TimeDuration::seconds(0));
    cookies.add(cookie);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(user: AuthUser) -> Json<UserView> {
    Json(user.into())
}
