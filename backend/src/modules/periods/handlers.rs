use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    error::ApiResult,
    modules::{
        auth::AuthUser,
        imports, periods,
        periods::models::{CreatePeriodRequest, PeriodDetail, PeriodView, UpdatePeriodRequest},
        reports,
    },
    shared::audit,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListPeriodsQuery {
    #[serde(default)]
    pub include_archived: bool,
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListPeriodsQuery>,
) -> ApiResult<Json<Vec<PeriodView>>> {
    Ok(Json(
        periods::service::list(&state.db, query.include_archived).await?,
    ))
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

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<UpdatePeriodRequest>,
) -> ApiResult<Json<PeriodView>> {
    let period = periods::service::update(&state.db, &period_id, &input).await?;
    audit::record(
        &state.db,
        Some(&user.id),
        "period.update",
        "period",
        &period.id,
        json!({
            "label": period.label,
            "start": period.start_date,
            "end": period.end_date,
            "type": period.period_type,
        }),
    )
    .await?;
    Ok(Json(period))
}

pub async fn archive(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<PeriodView>> {
    set_archived(state, user, period_id, true).await
}

pub async fn restore(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<PeriodView>> {
    set_archived(state, user, period_id, false).await
}

async fn set_archived(
    state: AppState,
    user: AuthUser,
    period_id: String,
    archived: bool,
) -> ApiResult<Json<PeriodView>> {
    let period = periods::service::set_archived(&state.db, &period_id, archived).await?;
    audit::record(
        &state.db,
        Some(&user.id),
        if archived {
            "period.archive"
        } else {
            "period.restore"
        },
        "period",
        &period.id,
        json!({ "label": period.label, "archived": archived }),
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
    let imports =
        imports::service::list_for_range(&state.db, &period.start_date, &period.end_date).await?;
    let reports = reports::service::list_for_period(&state.db, &period_id).await?;
    let draft = Some(reports::service::get_draft(&state.db, &period_id).await?);

    Ok(Json(PeriodDetail {
        period,
        imports,
        reports,
        draft,
    }))
}
