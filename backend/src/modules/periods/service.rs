use chrono::{Datelike, NaiveDate};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    modules::periods::models::{CreatePeriodRequest, PeriodView},
    shared::{format_month_label, format_period_label},
};

pub async fn list(db: &SqlitePool) -> ApiResult<Vec<PeriodView>> {
    let rows = sqlx::query(
        "SELECT p.id, p.start_date, p.end_date, p.label, p.period_type, p.created_at,
                COALESCE(u.username, 'Sistem') AS created_by,
                (SELECT COUNT(*) FROM imports i
                 WHERE i.date_start <= p.end_date AND i.date_end >= p.start_date) AS imports,
                (SELECT COUNT(DISTINCT lower(trim(r.dedupe_key))) FROM import_rows r
                 WHERE r.tanggal BETWEEN p.start_date AND p.end_date) AS rows,
                (SELECT COUNT(*) FROM report_runs r WHERE r.period_id = p.id) AS reports
         FROM report_periods p
         LEFT JOIN users u ON u.id = p.created_by
         ORDER BY p.start_date DESC, p.created_at DESC",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows.iter().map(map_period).collect())
}

pub async fn get(db: &SqlitePool, period_id: &str) -> ApiResult<PeriodView> {
    let row = sqlx::query(
        "SELECT p.id, p.start_date, p.end_date, p.label, p.period_type, p.created_at,
                COALESCE(u.username, 'Sistem') AS created_by,
                (SELECT COUNT(*) FROM imports i
                 WHERE i.date_start <= p.end_date AND i.date_end >= p.start_date) AS imports,
                (SELECT COUNT(DISTINCT lower(trim(r.dedupe_key))) FROM import_rows r
                 WHERE r.tanggal BETWEEN p.start_date AND p.end_date) AS rows,
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

    Ok(map_period(&row))
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

    let period_type = match input
        .period_type
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("daily") | Some("harian") => "daily",
        Some("monthly") | Some("bulanan") => "monthly",
        _ => "range",
    };

    if period_type == "daily" && start != end {
        return Err(ApiError::bad_request(
            "Laporan harian harus memakai satu tanggal yang sama.",
        ));
    }
    if period_type == "monthly" {
        let first = NaiveDate::from_ymd_opt(start.year(), start.month(), 1)
            .ok_or_else(|| ApiError::bad_request("Bulan periode tidak valid."))?;
        let next_month = if start.month() == 12 {
            NaiveDate::from_ymd_opt(start.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1)
        }
        .ok_or_else(|| ApiError::bad_request("Bulan periode tidak valid."))?;
        let last = next_month
            .pred_opt()
            .ok_or_else(|| ApiError::bad_request("Bulan periode tidak valid."))?;
        if start != first || end != last {
            return Err(ApiError::bad_request(
                "Laporan bulanan harus mencakup satu bulan penuh.",
            ));
        }
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

    let auto_label = match period_type {
        "daily" => format!("Laporan Harian {}", format_period_label(&start_date, &end_date)),
        "monthly" => format!("Laporan Bulanan {}", format_month_label(&start_date)),
        _ => format!("Laporan {}", format_period_label(&start_date, &end_date)),
    };
    let label = input
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(auto_label);

    if label.chars().count() > 120 {
        return Err(ApiError::bad_request("Nama periode maksimal 120 karakter."));
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO report_periods(id,start_date,end_date,label,period_type,created_by)
         VALUES(?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&start_date)
    .bind(&end_date)
    .bind(&label)
    .bind(period_type)
    .bind(created_by)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    get(db, &id).await
}

fn map_period(row: &sqlx::sqlite::SqliteRow) -> PeriodView {
    PeriodView {
        id: row.get("id"),
        start_date: row.get("start_date"),
        end_date: row.get("end_date"),
        label: row.get("label"),
        period_type: row.get("period_type"),
        created_at: row.get("created_at"),
        created_by: row.get("created_by"),
        imports: row.get("imports"),
        rows: row.get("rows"),
        reports: row.get("reports"),
    }
}
