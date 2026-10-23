use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    error::ApiResult,
    modules::{
        auth::AuthUser,
        users::{
            models::{AdminUserView, CreateUserRequest, UpdateUserRequest},
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
        serde_json::json!({ "role": created.role }),
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
    let updated = service::update(&state.db, &user.id, &user_id, &input).await?;
    audit::record(
        &state.db,
        Some(&user.id),
        "user.update",
        "user",
        &updated.id,
        serde_json::json!({ "role": updated.role, "active": updated.active }),
    )
    .await?;
    Ok(Json(updated))
}
