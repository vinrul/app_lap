use std::path::{Path, PathBuf};

use serde_json::Value;
use sqlx::{Row, SqlitePool};
use tokio::{process::Command, sync::Mutex};
use uuid::Uuid;

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    modules::{
        imports, periods,
        reports::{
            metrics,
            models::{DraftView, ReportContent, ReportMetrics, ReportView},
            render,
        },
    },
    shared::audit,
};

pub async fn list_for_period(db: &SqlitePool, period_id: &str) -> ApiResult<Vec<ReportView>> {
    periods::service::get(db, period_id).await?;
    let rows = sqlx::query(
        "SELECT r.id, r.period_id, r.import_id, r.created_at,
                COALESCE(NULLIF(r.period_label, ''), p.label) AS period_label,
                i.filename AS source_filename
         FROM report_runs r
         JOIN imports i ON i.id = r.import_id
         JOIN report_periods p ON p.id = r.period_id
         WHERE r.period_id = ?
         ORDER BY r.created_at DESC",
    )
    .bind(period_id)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| ReportView {
            id: row.get("id"),
            period_id: row.get("period_id"),
            import_id: row.get("import_id"),
            created_at: row.get("created_at"),
            source_filename: row.get("source_filename"),
            period_label: row.get("period_label"),
            pdf_url: format!("/api/reports/{}/pdf", row.get::<String, _>("id")),
            preview_url: format!("/api/reports/{}/preview", row.get::<String, _>("id")),
        })
        .collect())
}

pub async fn get_draft(db: &SqlitePool, period_id: &str, import_id: &str) -> ApiResult<DraftView> {
    imports::service::ensure_import(db, period_id, import_id).await?;
    let row = sqlx::query(
        "SELECT d.content_json, d.updated_at, u.username AS updated_by
         FROM report_drafts d
         LEFT JOIN users u ON u.id = d.updated_by
         WHERE d.period_id = ? AND d.import_id = ?",
    )
    .bind(period_id)
    .bind(import_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(match row {
        Some(row) => {
            let raw: String = row.get("content_json");
            let value: Value =
                serde_json::from_str(&raw).unwrap_or(Value::Object(Default::default()));
            DraftView {
                content: ReportContent::from_value(&value),
                updated_at: Some(row.get("updated_at")),
                updated_by: row.try_get("updated_by").ok(),
            }
        }
        None => DraftView {
            content: ReportContent::default(),
            updated_at: None,
            updated_by: None,
        },
    })
}

pub async fn save_draft(
    db: &SqlitePool,
    user_id: &str,
    period_id: &str,
    import_id: &str,
    content: &ReportContent,
) -> ApiResult<DraftView> {
    imports::service::ensure_import(db, period_id, import_id).await?;
    let content_json = serde_json::to_string(content).map_err(ApiError::internal)?;
    if content_json.len() > 512 * 1024 {
        return Err(ApiError::payload_too_large(
            "Isi editorial terlalu besar. Ringkas narasi laporan.",
        ));
    }

    sqlx::query(
        "INSERT INTO report_drafts(id, period_id, import_id, content_json, updated_by, updated_at)
         VALUES(?,?,?,?,?,CURRENT_TIMESTAMP)
         ON CONFLICT(period_id, import_id) DO UPDATE SET
           content_json = excluded.content_json,
           updated_by = excluded.updated_by,
           updated_at = CURRENT_TIMESTAMP",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(period_id)
    .bind(import_id)
    .bind(&content_json)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    audit::record(
        db,
        Some(user_id),
        "draft.update",
        "period",
        period_id,
        serde_json::json!({ "import_id": import_id }),
    )
    .await?;

    get_draft(db, period_id, import_id).await
}

pub async fn metrics_for(
    db: &SqlitePool,
    config: &Config,
    period_id: &str,
    import_id: Option<&str>,
) -> ApiResult<ReportMetrics> {
    let (start, end, label) = periods::service::dates(db, period_id).await?;
    let import_id = match import_id {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => imports::service::latest_import_id(db, period_id).await?,
    };
    let (filename, _, _) = imports::service::ensure_import(db, period_id, &import_id).await?;
    metrics::load(
        db,
        &import_id,
        &filename,
        &label,
        &start,
        &end,
        &config.polda_name,
    )
    .await
}

pub async fn preview_html(
    db: &SqlitePool,
    config: &Config,
    period_id: &str,
    import_id: &str,
    content_override: Option<&ReportContent>,
) -> ApiResult<String> {
    let metrics = metrics_for(db, config, period_id, Some(import_id)).await?;
    let content = match content_override {
        Some(content) => content.clone(),
        None => get_draft(db, period_id, import_id).await?.content,
    };
    Ok(render::render(&metrics, &content, config))
}

pub async fn generate(
    db: &SqlitePool,
    config: &Config,
    pdf_lock: &Mutex<()>,
    user_id: &str,
    period_id: &str,
    import_id: &str,
) -> ApiResult<ReportView> {
    let (start_date, end_date, label) = periods::service::dates(db, period_id).await?;
    let (source_filename, _, _) = imports::service::ensure_import(db, period_id, import_id).await?;
    let content = get_draft(db, period_id, import_id).await?.content;
    let metrics = metrics_for(db, config, period_id, Some(import_id)).await?;
    let html = render::render(&metrics, &content, config);

    let report_id = Uuid::new_v4().to_string();
    let report_dir = config.data_dir.join("reports").join(period_id);
    tokio::fs::create_dir_all(&report_dir)
        .await
        .map_err(ApiError::internal)?;

    let html_path = report_dir.join(format!("{report_id}.html"));
    let pdf_path = report_dir.join(format!("{report_id}.pdf"));
    tokio::fs::write(&html_path, &html)
        .await
        .map_err(ApiError::internal)?;

    if pdf_path.exists() {
        let _ = tokio::fs::remove_file(&pdf_path).await;
    }

    let chrome_dir = report_dir.join(format!("chrome-{report_id}"));
    let result = {
        let _guard = pdf_lock.lock().await;
        let output = Command::new(&config.chrome_path)
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--disable-extensions")
            .arg("--hide-scrollbars")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg(format!("--user-data-dir={}", chrome_dir.display()))
            .arg("--no-pdf-header-footer")
            .arg("--run-all-compositor-stages-before-draw")
            .arg("--virtual-time-budget=2000")
            .arg(format!("--print-to-pdf={}", pdf_path.display()))
            .arg(file_url(&html_path))
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(ApiError::internal(format!(
                    "PDF gagal dibuat oleh Chrome/Edge: {}",
                    stderr.trim()
                )))
            }
            Err(error) => Err(ApiError::bad_request(format!(
                "Chrome/Edge tidak dapat dijalankan ({error}). Atur LAPORAN_CHROME ke path chrome.exe atau msedge.exe."
            ))),
        }
    };

    let _ = tokio::fs::remove_dir_all(&chrome_dir).await;
    result?;

    let meta = tokio::fs::metadata(&pdf_path)
        .await
        .map_err(|_| ApiError::internal("Chrome tidak menghasilkan berkas PDF."))?;
    if meta.len() == 0 {
        return Err(ApiError::internal("Berkas PDF yang dihasilkan kosong."));
    }

    sqlx::query(
        "INSERT INTO report_runs(
            id, period_id, import_id, content_json, metrics_json, pdf_path, html_path,
            polda_name, source_filename, period_label, created_by
         ) VALUES(?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&report_id)
    .bind(period_id)
    .bind(import_id)
    .bind(serde_json::to_string(&content).map_err(ApiError::internal)?)
    .bind(serde_json::to_string(&metrics).map_err(ApiError::internal)?)
    .bind(pdf_path.to_string_lossy().to_string())
    .bind(html_path.to_string_lossy().to_string())
    .bind(&config.polda_name)
    .bind(&source_filename)
    .bind(&label)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    audit::record(
        db,
        Some(user_id),
        "report.generate",
        "report",
        &report_id,
        serde_json::json!({
            "import_id": import_id,
            "start_date": start_date,
            "end_date": end_date,
        }),
    )
    .await?;

    Ok(ReportView {
        id: report_id.clone(),
        period_id: period_id.to_string(),
        import_id: import_id.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        source_filename,
        period_label: label,
        pdf_url: format!("/api/reports/{report_id}/pdf"),
        preview_url: format!("/api/reports/{report_id}/preview"),
    })
}

pub async fn stored_preview(
    db: &SqlitePool,
    config: &Config,
    report_id: &str,
) -> ApiResult<String> {
    let row = sqlx::query("SELECT content_json, metrics_json FROM report_runs WHERE id = ?")
        .bind(report_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Dokumen laporan tidak ditemukan."))?;

    let content: ReportContent =
        serde_json::from_str::<Value>(&row.get::<String, _>("content_json"))
            .map(|value| ReportContent::from_value(&value))
            .unwrap_or_default();
    let metrics: ReportMetrics = serde_json::from_str(&row.get::<String, _>("metrics_json"))
        .map_err(|error| ApiError::internal(format!("snapshot metrik rusak: {error}")))?;

    Ok(render::render(&metrics, &content, config))
}

pub async fn download_path(
    db: &SqlitePool,
    config: &Config,
    report_id: &str,
) -> ApiResult<(PathBuf, String)> {
    let row = sqlx::query(
        "SELECT r.pdf_path, r.polda_name, p.start_date
         FROM report_runs r JOIN report_periods p ON p.id = r.period_id
         WHERE r.id = ?",
    )
    .bind(report_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Dokumen laporan tidak ditemukan."))?;

    let path: String = row.get("pdf_path");
    if !Path::new(&path).exists() {
        return Err(ApiError::not_found(
            "Berkas PDF tidak ditemukan pada penyimpanan.",
        ));
    }

    let stored_polda: String = row.get("polda_name");
    let polda = if stored_polda.trim().is_empty() {
        config.polda_name.clone()
    } else {
        stored_polda
    };
    let start: String = row.get("start_date");
    let filename = format!(
        "Laporan-Mingguan-{}-{}.pdf",
        sanitize_filename(&polda),
        start
    );
    Ok((PathBuf::from(path), filename))
}

fn file_url(path: &Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}

fn sanitize_filename(value: &str) -> String {
    let mut output: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    while output.contains("--") {
        output = output.replace("--", "-");
    }
    output.trim_matches('-').to_uppercase()
}
