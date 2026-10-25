use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    modules::{
        imports::{self, models::ImportView},
        periods::{self, models::PeriodView},
        reports::{self, models::ReportMetrics, models::ReportView},
    },
};

#[derive(Debug, Serialize)]
pub struct DashboardCounts {
    pub periods: i64,
    pub imports: i64,
    pub reports: i64,
    pub rows: i64,
}

#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub polda_name: String,
    pub app_name: String,
    pub counts: DashboardCounts,
    pub latest_period: Option<PeriodView>,
    pub latest_import: Option<ImportView>,
    pub metrics: Option<ReportMetrics>,
    pub recent_periods: Vec<PeriodView>,
    pub recent_reports: Vec<ReportView>,
}

pub async fn load(db: &SqlitePool, config: &Config) -> ApiResult<DashboardResponse> {
    let counts = DashboardCounts {
        periods: scalar(db, "SELECT COUNT(*) FROM report_periods").await?,
        imports: scalar(db, "SELECT COUNT(*) FROM imports").await?,
        reports: scalar(db, "SELECT COUNT(*) FROM report_runs").await?,
        rows: scalar(db, "SELECT COUNT(*) FROM import_rows").await?,
    };

    let periods = periods::service::list(db).await?;
    let latest_period = periods.first().cloned();
    let latest_import = imports::service::list(db).await?.into_iter().next();

    let metrics = match latest_period.as_ref() {
        Some(period) => Some(reports::service::metrics_for(db, config, &period.id).await?),
        None => None,
    };

    let recent_periods = periods.into_iter().take(6).collect();
    let recent_reports = recent_reports(db).await?;

    Ok(DashboardResponse {
        polda_name: config.polda_name.clone(),
        app_name: config.app_name.clone(),
        counts,
        latest_period,
        latest_import,
        metrics,
        recent_periods,
        recent_reports,
    })
}

async fn scalar(db: &SqlitePool, query: &'static str) -> ApiResult<i64> {
    sqlx::query_scalar(query)
        .fetch_one(db)
        .await
        .map_err(ApiError::internal)
}

async fn recent_reports(db: &SqlitePool) -> ApiResult<Vec<ReportView>> {
    let rows = sqlx::query(
        "SELECT r.id, r.period_id, r.created_at,
                COALESCE(NULLIF(r.period_label, ''), p.label) AS period_label,
                r.source_filename
         FROM report_runs r
         JOIN report_periods p ON p.id = r.period_id
         ORDER BY r.created_at DESC
         LIMIT 6",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let id: String = row.get("id");
            ReportView {
                id: id.clone(),
                period_id: row.get("period_id"),
                created_at: row.get("created_at"),
                source_filename: row.get("source_filename"),
                period_label: row.get("period_label"),
                pdf_url: format!("/api/reports/{id}/pdf"),
                preview_url: format!("/api/reports/{id}/preview"),
            }
        })
        .collect())
}
