use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    modules::periods::models::{CreatePeriodRequest, PeriodView},
    shared::format_period_label,
};

pub async fn list(db: &SqlitePool) -> ApiResult<Vec<PeriodView>> {
    let rows = sqlx::query(
        "SELECT p.id, p.start_date, p.end_date, p.label, p.created_at,
                COALESCE(u.username, 'Sistem') AS created_by,
                (SELECT COUNT(*) FROM imports i WHERE i.period_id = p.id) AS imports,
                (SELECT COUNT(*) FROM report_runs r WHERE r.period_id = p.id) AS reports
         FROM report_periods p
         LEFT JOIN users u ON u.id = p.created_by
         ORDER BY p.start_date DESC, p.created_at DESC",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows.into_iter().map(map_period).collect())
}

pub async fn get(db: &SqlitePool, period_id: &str) -> ApiResult<PeriodView> {
    let row = sqlx::query(
        "SELECT p.id, p.start_date, p.end_date, p.label, p.created_at,
                COALESCE(u.username, 'Sistem') AS created_by,
                (SELECT COUNT(*) FROM imports i WHERE i.period_id = p.id) AS imports,
                (SELECT COUNT(*) FROM report_runs r WHERE r.period_id = p.id) AS reports
         FROM report_periods p
         LEFT JOIN users u ON u.id = p.created_by
         WHERE p.id = ?",
    )
    .bind(period_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Periode laporan tidak ditemukan."))?;

    Ok(map_period(row))
}

pub async fn dates(db: &SqlitePool, period_id: &str) -> ApiResult<(String, String, String)> {
    let row = sqlx::query("SELECT start_date,end_date,label FROM report_periods WHERE id = ?")
        .bind(period_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Periode laporan tidak ditemukan."))?;
    Ok((row.get("start_date"), row.get("end_date"), row.get("label")))
}

pub async fn create(
    db: &SqlitePool,
    created_by: &str,
    input: &CreatePeriodRequest,
) -> ApiResult<PeriodView> {
    let start = NaiveDate::parse_from_str(input.start_date.trim(), "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Tanggal awal tidak valid."))?;
    let end = NaiveDate::parse_from_str(input.end_date.trim(), "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Tanggal akhir tidak valid."))?;

    if start > end {
        return Err(ApiError::bad_request(
            "Tanggal awal harus sebelum atau sama dengan tanggal akhir.",
        ));
    }
    if (end - start).num_days() > 366 {
        return Err(ApiError::bad_request("Rentang periode maksimal 366 hari."));
    }

    let start_date = start.format("%Y-%m-%d").to_string();
    let end_date = end.format("%Y-%m-%d").to_string();

    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM report_periods WHERE start_date = ? AND end_date = ?",
    )
    .bind(&start_date)
    .bind(&end_date)
    .fetch_one(db)
    .await
    .map_err(ApiError::internal)?;
    if duplicate > 0 {
        return Err(ApiError::conflict(
            "Periode dengan rentang tanggal tersebut sudah ada.",
        ));
    }

    let label = input
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format_period_label(&start_date, &end_date));

    if label.chars().count() > 120 {
        return Err(ApiError::bad_request("Nama periode maksimal 120 karakter."));
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO report_periods(id,start_date,end_date,label,created_by) VALUES(?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&start_date)
    .bind(&end_date)
    .bind(&label)
    .bind(created_by)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    get(db, &id).await
}

fn map_period(row: sqlx::sqlite::SqliteRow) -> PeriodView {
    PeriodView {
        id: row.get("id"),
        start_date: row.get("start_date"),
        end_date: row.get("end_date"),
        label: row.get("label"),
        created_at: row.get("created_at"),
        created_by: row.get("created_by"),
        imports: row.get("imports"),
        reports: row.get("reports"),
    }
}
