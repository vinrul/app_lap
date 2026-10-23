use axum::{Json, extract::State};

use crate::{
    error::ApiResult,
    modules::{auth::AuthUser, dashboard::service},
    state::AppState,
};

pub async fn dashboard(
    State(state): State<AppState>,
    _user: AuthUser,
) -> ApiResult<Json<service::DashboardResponse>> {
    Ok(Json(service::load(&state.db, &state.config).await?))
}
