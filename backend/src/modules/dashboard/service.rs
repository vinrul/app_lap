use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    modules::{
        imports::{
            self,
            models::{CoverageResponse, ImportView},
        },
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
    /// Cakupan periode terbaru, dipakai agar ringkasan dashboard sama dengan
    /// halaman Cakupan periode tersebut.
    pub coverage: Option<CoverageResponse>,
    pub recent_periods: Vec<PeriodView>,
    pub recent_reports: Vec<ReportView>,
}

pub async fn load(db: &SqlitePool, config: &Config) -> ApiResult<DashboardResponse> {
    let counts = DashboardCounts {
        periods: scalar(
            db,
            "SELECT COUNT(*) FROM report_periods WHERE archived_at IS NULL",
        )
        .await?,
        imports: scalar(db, "SELECT COUNT(*) FROM imports").await?,
        reports: scalar(
            db,
            "SELECT COUNT(*) FROM report_runs r
             JOIN report_periods p ON p.id = r.period_id
             WHERE p.archived_at IS NULL",
        )
        .await?,
        rows: scalar(db, "SELECT COUNT(*) FROM import_rows").await?,
    };

    let periods = periods::service::list(db, false).await?;
    let latest_period = periods.first().cloned();
    let latest_import = imports::service::list(db).await?.into_iter().next();

    let metrics = match latest_period.as_ref() {
        Some(period) => Some(reports::service::metrics_for(db, config, &period.id).await?),
        None => None,
    };
    let coverage = metrics
        .as_ref()
        .and_then(|metrics| metrics.coverage.clone());

    let recent_periods = periods.into_iter().take(6).collect();
    let recent_reports = recent_reports(db).await?;

    Ok(DashboardResponse {
        polda_name: config.polda_name.clone(),
        app_name: config.app_name.clone(),
        counts,
        latest_period,
        latest_import,
        metrics,
        coverage,
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
         WHERE p.archived_at IS NULL
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn exposes_coverage_for_latest_period() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::test_config();
        config.data_dir = dir.path().to_path_buf();
        let db = crate::db::connect(&config).await.unwrap();

        sqlx::query(
            "INSERT INTO users(id,username,password_hash,role) VALUES('u1','tester','x','admin')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO report_periods(id,start_date,end_date,label,created_by)
             VALUES('p1','2026-10-26','2026-11-01','Laporan Uji','u1')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO imports(
                id,filename,file_hash,stored_path,parser_version,row_count,skipped_rows,
                duplicate_rows,date_start,date_end,warnings_json,imported_by
             ) VALUES('imp1','x.xlsx','hash','path','4',1,0,0,'2026-10-26','2026-11-01','[]','u1')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO import_rows(import_id,source_row,tanggal,dedupe_key,raw_json)
             VALUES('imp1',1,'2026-10-26','kunci-1','{}')",
        )
        .execute(&db)
        .await
        .unwrap();

        let dashboard = load(&db, &config).await.unwrap();
        let coverage = dashboard
            .coverage
            .expect("cakupan periode terbaru tersedia");
        assert_eq!(coverage.total_rows, 1);
        assert_eq!(coverage.date_start, "2026-10-26");
        assert_eq!(coverage.date_end, "2026-11-01");
        assert_eq!(
            dashboard.metrics.as_ref().map(|metrics| metrics.total_rows),
            Some(1)
        );
    }
}
