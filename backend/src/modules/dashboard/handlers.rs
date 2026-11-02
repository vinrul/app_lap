use axum::{Json, extract::State};

use crate::{
    error::ApiResult,
    modules::{auth::AuthUser, dashboard::service},
    state::AppState,
};

pub async fn dashboard(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<service::DashboardResponse>> {
    Ok(Json(
        service::load(&state.db, &state.config, &user.data_scope()).await?,
    ))
}
