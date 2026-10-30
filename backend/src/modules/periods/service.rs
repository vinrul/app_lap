use chrono::{Datelike, NaiveDate};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    modules::periods::models::{CreatePeriodRequest, PeriodView, UpdatePeriodRequest},
    shared::{format_month_label, format_period_label},
};

pub async fn list(db: &SqlitePool, include_archived: bool) -> ApiResult<Vec<PeriodView>> {
    let rows = sqlx::query(
        "SELECT p.id, p.start_date, p.end_date, p.label, p.period_type, p.created_at,
                p.archived_at,
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

    Ok(rows
        .iter()
        .map(map_period)
        .filter(|period| include_archived || period.archived_at.is_none())
        .collect())
}

pub async fn get(db: &SqlitePool, period_id: &str) -> ApiResult<PeriodView> {
    let row = sqlx::query(
        "SELECT p.id, p.start_date, p.end_date, p.label, p.period_type, p.created_at,
                p.archived_at,
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
    let (start_date, end_date, period_type) = normalize_period(
        &input.start_date,
        &input.end_date,
        input.period_type.as_deref(),
        "range",
    )?;
    ensure_range_available(db, &start_date, &end_date, "").await?;

    let label = input
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| auto_label(&period_type, &start_date, &end_date));
    validate_label(&label)?;

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO report_periods(id,start_date,end_date,label,period_type,created_by)
         VALUES(?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&start_date)
    .bind(&end_date)
    .bind(&label)
    .bind(&period_type)
    .bind(created_by)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    get(db, &id).await
}

pub async fn update(
    db: &SqlitePool,
    period_id: &str,
    input: &UpdatePeriodRequest,
) -> ApiResult<PeriodView> {
    let row = sqlx::query(
        "SELECT start_date, end_date, label, period_type FROM report_periods WHERE id = ?",
    )
    .bind(period_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Periode laporan tidak ditemukan."))?;

    let current_start: String = row.get("start_date");
    let current_end: String = row.get("end_date");
    let current_label: String = row.get("label");
    let current_type: String = row.get("period_type");

    let (start_date, end_date, period_type) = normalize_period(
        input.start_date.as_deref().unwrap_or(&current_start),
        input.end_date.as_deref().unwrap_or(&current_end),
        input.period_type.as_deref(),
        &current_type,
    )?;
    ensure_range_available(db, &start_date, &end_date, period_id).await?;

    let requested = input.label.as_deref().map(str::trim);
    let unchanged = requested.map_or(true, |value| value == current_label);
    let label = if unchanged {
        // Nama otomatis ikut menyesuaikan tanggal baru; nama khusus dibiarkan.
        if current_label == auto_label(&current_type, &current_start, &current_end) {
            auto_label(&period_type, &start_date, &end_date)
        } else {
            current_label
        }
    } else {
        match requested.unwrap_or_default() {
            "" => auto_label(&period_type, &start_date, &end_date),
            value => value.to_string(),
        }
    };
    validate_label(&label)?;

    sqlx::query(
        "UPDATE report_periods
         SET start_date = ?, end_date = ?, label = ?, period_type = ?
         WHERE id = ?",
    )
    .bind(&start_date)
    .bind(&end_date)
    .bind(&label)
    .bind(&period_type)
    .bind(period_id)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    get(db, period_id).await
}

pub async fn set_archived(
    db: &SqlitePool,
    period_id: &str,
    archived: bool,
) -> ApiResult<PeriodView> {
    let result = if archived {
        sqlx::query(
            "UPDATE report_periods
             SET archived_at = COALESCE(archived_at, CURRENT_TIMESTAMP)
             WHERE id = ?",
        )
        .bind(period_id)
        .execute(db)
        .await
    } else {
        sqlx::query("UPDATE report_periods SET archived_at = NULL WHERE id = ?")
            .bind(period_id)
            .execute(db)
            .await
    }
    .map_err(ApiError::internal)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Periode laporan tidak ditemukan."));
    }

    get(db, period_id).await
}

fn normalize_period(
    start_date: &str,
    end_date: &str,
    period_type: Option<&str>,
    default_type: &str,
) -> ApiResult<(String, String, String)> {
    let start = NaiveDate::parse_from_str(start_date.trim(), "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Tanggal awal tidak valid."))?;
    let end = NaiveDate::parse_from_str(end_date.trim(), "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Tanggal akhir tidak valid."))?;

    if start > end {
        return Err(ApiError::bad_request(
            "Tanggal awal harus sebelum atau sama dengan tanggal akhir.",
        ));
    }
    if (end - start).num_days() > 366 {
        return Err(ApiError::bad_request("Rentang periode maksimal 366 hari."));
    }

    let period_type = match period_type
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("daily") | Some("harian") => "daily",
        Some("monthly") | Some("bulanan") => "monthly",
        Some("range") | Some("rentang") | Some("weekly") | Some("mingguan") => "range",
        _ => default_type,
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

    Ok((
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
        period_type.to_string(),
    ))
}

async fn ensure_range_available(
    db: &SqlitePool,
    start_date: &str,
    end_date: &str,
    exclude_id: &str,
) -> ApiResult<()> {
    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM report_periods
         WHERE start_date = ? AND end_date = ? AND id <> ?",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(exclude_id)
    .fetch_one(db)
    .await
    .map_err(ApiError::internal)?;

    if duplicate > 0 {
        return Err(ApiError::conflict(
            "Periode dengan rentang tanggal tersebut sudah ada.",
        ));
    }
    Ok(())
}

fn auto_label(period_type: &str, start_date: &str, end_date: &str) -> String {
    match period_type {
        "daily" => format!(
            "Laporan Harian {}",
            format_period_label(start_date, end_date)
        ),
        "monthly" => format!("Laporan Bulanan {}", format_month_label(start_date)),
        _ => format!("Laporan {}", format_period_label(start_date, end_date)),
    }
}

fn validate_label(label: &str) -> ApiResult<()> {
    if label.chars().count() > 120 {
        return Err(ApiError::bad_request("Nama periode maksimal 120 karakter."));
    }
    Ok(())
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
        archived_at: row.get("archived_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    async fn setup() -> (tempfile::TempDir, SqlitePool) {
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
        (dir, db)
    }

    fn create_request(start: &str, end: &str) -> CreatePeriodRequest {
        CreatePeriodRequest {
            start_date: start.to_string(),
            end_date: end.to_string(),
            label: None,
            period_type: Some("range".to_string()),
        }
    }

    #[tokio::test]
    async fn updates_title_dates_and_auto_label() {
        let (_dir, db) = setup().await;
        let period = create(&db, "u1", &create_request("2026-10-19", "2026-10-25"))
            .await
            .unwrap();
        assert_eq!(period.label, "Laporan 19–25 Oktober 2026");

        let updated = update(
            &db,
            &period.id,
            &UpdatePeriodRequest {
                start_date: Some("2026-10-20".to_string()),
                end_date: Some("2026-10-26".to_string()),
                label: None,
                period_type: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.start_date, "2026-10-20");
        assert_eq!(updated.end_date, "2026-10-26");
        assert_eq!(updated.label, "Laporan 20–26 Oktober 2026");

        let renamed = update(
            &db,
            &period.id,
            &UpdatePeriodRequest {
                start_date: Some("2026-10-20".to_string()),
                end_date: Some("2026-10-26".to_string()),
                label: Some("Laporan Khusus Gubernur".to_string()),
                period_type: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(renamed.label, "Laporan Khusus Gubernur");

        let moved = update(
            &db,
            &period.id,
            &UpdatePeriodRequest {
                start_date: Some("2026-10-21".to_string()),
                end_date: Some("2026-10-27".to_string()),
                label: Some("Laporan Khusus Gubernur".to_string()),
                period_type: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(moved.label, "Laporan Khusus Gubernur");
    }

    #[tokio::test]
    async fn update_rejects_overlapping_range() {
        let (_dir, db) = setup().await;
        create(&db, "u1", &create_request("2026-10-19", "2026-10-25"))
            .await
            .unwrap();
        let second = create(&db, "u1", &create_request("2026-10-26", "2026-11-01"))
            .await
            .unwrap();

        let error = update(
            &db,
            &second.id,
            &UpdatePeriodRequest {
                start_date: Some("2026-10-19".to_string()),
                end_date: Some("2026-10-25".to_string()),
                label: None,
                period_type: None,
            },
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Periode dengan rentang tanggal tersebut sudah ada. (conflict)"
        );
    }

    #[tokio::test]
    async fn archives_and_restores_periods() {
        let (_dir, db) = setup().await;
        let period = create(&db, "u1", &create_request("2026-10-19", "2026-10-25"))
            .await
            .unwrap();

        let archived = set_archived(&db, &period.id, true).await.unwrap();
        assert!(archived.archived_at.is_some());
        assert!(list(&db, false).await.unwrap().is_empty());
        assert_eq!(list(&db, true).await.unwrap().len(), 1);

        let restored = set_archived(&db, &period.id, false).await.unwrap();
        assert!(restored.archived_at.is_none());
        assert_eq!(list(&db, false).await.unwrap().len(), 1);
    }
}
