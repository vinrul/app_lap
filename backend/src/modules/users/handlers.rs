use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    error::ApiResult,
    modules::{
        auth::{AuthUser, service as auth_service},
        live::ServerMessage,
        notifications,
        users::{
            models::{AdminUserView, CreateUserRequest, UpdateUserRequest, UserOptions},
            service,
        },
    },
    shared::audit,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<AdminUserView>>> {
    user.require_admin()?;
    Ok(Json(service::list(&state.db).await?))
}

pub async fn options(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<UserOptions>> {
    user.require_admin()?;
    Ok(Json(
        service::options(&state.db, &state.config.polda_name).await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateUserRequest>,
) -> ApiResult<Json<AdminUserView>> {
    user.require_admin()?;
    let created = service::create(&state.db, &input).await?;
    audit::record(
        &state.db,
        Some(&user.id),
        "user.create",
        "user",
        &created.id,
        serde_json::json!({ "role": created.role, "polres": created.polres, "satker": created.satker }),
    )
    .await?;
    Ok(Json(created))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(user_id): Path<String>,
    Json(input): Json<UpdateUserRequest>,
) -> ApiResult<Json<AdminUserView>> {
    user.require_admin()?;
    let password_reset = input
        .password
        .as_deref()
        .is_some_and(|value| !value.is_empty());
    let updated = service::update(&state.db, &user.id, &user_id, &input).await?;
    audit::record(
        &state.db,
        Some(&user.id),
        "user.update",
        "user",
        &updated.id,
        serde_json::json!({
            "role": updated.role,
            "polres": updated.polres,
            "satker": updated.satker,
            "active": updated.active,
            "password_reset": password_reset
        }),
    )
    .await?;
    if password_reset {
        notifications::notify(
            &state.db,
            &state.hub,
            &updated.id,
            "account",
            "warn",
            "Kata sandi direset",
            "Administrator mereset kata sandi akun Anda. Masuk kembali dengan kata sandi baru.",
            "/#/dashboard",
        )
        .await;
    }
    Ok(Json(updated))
}

pub async fn sessions(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<auth_service::SessionView>>> {
    user.require_admin()?;
    Ok(Json(auth_service::list_sessions(&state.db).await?))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    user: AuthUser,
    Path(session_id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    user.require_admin()?;
    match auth_service::revoke_session(&state.db, session_id).await? {
        Some((target, token)) => {
            state
                .hub
                .publish(
                    &format!("user:{target}"),
                    ServerMessage::SessionRevoked {
                        token: auth_service::token_hash(&token),
                    },
                )
                .await;
            Ok(Json(serde_json::json!({ "revoked": true })))
        }
        None => Ok(Json(serde_json::json!({ "revoked": false }))),
    }
}

pub async fn revoke_user_sessions(
    State(state): State<AppState>,
    user: AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    user.require_admin()?;
    let tokens = auth_service::revoke_user_sessions(&state.db, &user_id).await?;
    for token in &tokens {
        state
            .hub
            .publish(
                &format!("user:{user_id}"),
                ServerMessage::SessionRevoked {
                    token: auth_service::token_hash(token),
                },
            )
            .await;
    }
    Ok(Json(serde_json::json!({ "revoked": tokens.len() })))
}
