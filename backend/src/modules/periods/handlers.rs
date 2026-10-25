use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;

use crate::{
    error::ApiResult,
    modules::{
        auth::AuthUser,
        imports, periods,
        periods::models::{CreatePeriodRequest, PeriodDetail, PeriodView},
        reports,
    },
    shared::audit,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> ApiResult<Json<Vec<PeriodView>>> {
    Ok(Json(periods::service::list(&state.db).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreatePeriodRequest>,
) -> ApiResult<Json<PeriodView>> {
    let period = periods::service::create(&state.db, &user.id, &input).await?;
    audit::record(
        &state.db,
        Some(&user.id),
        "period.create",
        "period",
        &period.id,
        json!({
            "start": period.start_date,
            "end": period.end_date,
            "type": period.period_type,
        }),
    )
    .await?;
    Ok(Json(period))
}

pub async fn detail(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<PeriodDetail>> {
    let period = periods::service::get(&state.db, &period_id).await?;
    let imports = imports::service::list_for_range(
        &state.db,
        &period.start_date,
        &period.end_date,
    )
    .await?;
    let reports = reports::service::list_for_period(&state.db, &period_id).await?;
    let draft = Some(reports::service::get_draft(&state.db, &period_id).await?);

    Ok(Json(PeriodDetail {
        period,
        imports,
        reports,
        draft,
    }))
}
