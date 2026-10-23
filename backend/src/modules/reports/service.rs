use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
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
            layout::{
                DesignTemplateView, LayoutView, ReportLayout, UploadedImage, image_id_from_src,
            },
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

pub async fn get_layout(
    db: &SqlitePool,
    period_id: &str,
    import_id: &str,
) -> ApiResult<LayoutView> {
    imports::service::ensure_import(db, period_id, import_id).await?;
    let row = sqlx::query(
        "SELECT l.layout_json, l.updated_at, u.username AS updated_by
         FROM report_layouts l
         LEFT JOIN users u ON u.id = l.updated_by
         WHERE l.period_id = ? AND l.import_id = ?",
    )
    .bind(period_id)
    .bind(import_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(match row {
        Some(row) => {
            let layout = serde_json::from_str::<ReportLayout>(&row.get::<String, _>("layout_json"))
                .unwrap_or_default()
                .sanitize();
            LayoutView {
                layout,
                updated_at: Some(row.get("updated_at")),
                updated_by: row.try_get("updated_by").ok(),
            }
        }
        None => LayoutView {
            layout: ReportLayout::default().sanitize(),
            updated_at: None,
            updated_by: None,
        },
    })
}

pub async fn save_layout(
    db: &SqlitePool,
    user_id: &str,
    period_id: &str,
    import_id: &str,
    layout: ReportLayout,
) -> ApiResult<LayoutView> {
    imports::service::ensure_import(db, period_id, import_id).await?;
    let layout = layout.sanitize();
    let layout_json = serde_json::to_string(&layout).map_err(ApiError::internal)?;
    if layout_json.len() > 2 * 1024 * 1024 {
        return Err(ApiError::payload_too_large(
            "Layout desain terlalu besar. Kurangi jumlah elemen atau gambar.",
        ));
    }
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO report_layouts(period_id, import_id, layout_json, updated_by, updated_at)
         VALUES(?,?,?,?,?)
         ON CONFLICT(period_id, import_id) DO UPDATE SET
           layout_json = excluded.layout_json,
           updated_by = excluded.updated_by,
           updated_at = excluded.updated_at",
    )
    .bind(period_id)
    .bind(import_id)
    .bind(&layout_json)
    .bind(user_id)
    .bind(&now)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    audit::record(
        db,
        Some(user_id),
        "layout.update",
        "period",
        period_id,
        serde_json::json!({ "import_id": import_id }),
    )
    .await?;

    let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?;

    Ok(LayoutView {
        layout,
        updated_at: Some(now),
        updated_by: username,
    })
}

pub async fn list_templates(db: &SqlitePool) -> ApiResult<Vec<DesignTemplateView>> {
    let rows = sqlx::query(
        "SELECT t.id, t.name, t.layout_json, t.created_at, u.username AS created_by
         FROM design_templates t
         LEFT JOIN users u ON u.id = t.created_by
         ORDER BY t.created_at DESC, t.name COLLATE NOCASE",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows.into_iter().map(template_from_row).collect())
}

pub async fn save_template(
    db: &SqlitePool,
    user_id: &str,
    name: &str,
    layout: ReportLayout,
) -> ApiResult<DesignTemplateView> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("Nama template wajib diisi."));
    }
    if name.chars().count() > 80 {
        return Err(ApiError::bad_request("Nama template maksimal 80 karakter."));
    }

    let layout = layout.sanitize();
    let layout_json = serde_json::to_string(&layout).map_err(ApiError::internal)?;
    if layout_json.len() > 2 * 1024 * 1024 {
        return Err(ApiError::payload_too_large(
            "Layout template terlalu besar. Kurangi jumlah elemen atau gambar.",
        ));
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO design_templates(id, name, layout_json, created_by)
         VALUES(?,?,?,?)",
    )
    .bind(&id)
    .bind(name)
    .bind(&layout_json)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    audit::record(
        db,
        Some(user_id),
        "template.create",
        "design_template",
        &id,
        serde_json::json!({ "name": name }),
    )
    .await?;

    template_view(db, &id).await
}

pub async fn delete_template(db: &SqlitePool, user_id: &str, template_id: &str) -> ApiResult<()> {
    let result = sqlx::query("DELETE FROM design_templates WHERE id = ?")
        .bind(template_id)
        .execute(db)
        .await
        .map_err(ApiError::internal)?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Template desain tidak ditemukan."));
    }

    audit::record(
        db,
        Some(user_id),
        "template.delete",
        "design_template",
        template_id,
        serde_json::json!({}),
    )
    .await?;
    Ok(())
}

pub async fn apply_template(
    db: &SqlitePool,
    config: &Config,
    user_id: &str,
    period_id: &str,
    import_id: &str,
    template_id: &str,
) -> ApiResult<LayoutView> {
    imports::service::ensure_import(db, period_id, import_id).await?;
    let row = sqlx::query("SELECT layout_json FROM design_templates WHERE id = ?")
        .bind(template_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Template desain tidak ditemukan."))?;

    let mut layout = serde_json::from_str::<ReportLayout>(&row.get::<String, _>("layout_json"))
        .unwrap_or_default()
        .sanitize();
    let mut cloned = HashMap::new();

    for page in &mut layout.pages {
        for element in &mut page.elements {
            if element.kind != "image" {
                continue;
            }
            let Some(old_id) = image_id_from_src(&element.src).map(str::to_string) else {
                continue;
            };
            if let Some(new_id) = cloned.get(&old_id) {
                element.src = format!("upload:{new_id}");
                continue;
            }

            let image = sqlx::query("SELECT filename, stored_path FROM design_images WHERE id = ?")
                .bind(&old_id)
                .fetch_optional(db)
                .await
                .map_err(ApiError::internal)?;
            let Some(image) = image else {
                element.src.clear();
                continue;
            };
            let path: String = image.get("stored_path");
            let filename: String = image.get("filename");
            let bytes = match tokio::fs::read(&path).await {
                Ok(bytes) => bytes,
                Err(error) => {
                    tracing::warn!(error = %error, image_id = %old_id, "gambar template tidak dapat dibaca");
                    element.src.clear();
                    continue;
                }
            };
            match save_layout_image(db, config, user_id, period_id, &filename, &bytes).await {
                Ok(uploaded) => {
                    cloned.insert(old_id, uploaded.id.clone());
                    element.src = format!("upload:{}", uploaded.id);
                }
                Err(error) => {
                    tracing::warn!(error = %error, image_id = %old_id, "gambar template tidak dapat disalin");
                    element.src.clear();
                }
            }
        }
    }

    save_layout(db, user_id, period_id, import_id, layout).await
}

fn template_from_row(row: sqlx::sqlite::SqliteRow) -> DesignTemplateView {
    let layout = serde_json::from_str::<ReportLayout>(&row.get::<String, _>("layout_json"))
        .unwrap_or_default()
        .sanitize();
    DesignTemplateView {
        id: row.get("id"),
        name: row.get("name"),
        layout,
        created_at: row.get("created_at"),
        created_by: row.try_get("created_by").ok(),
    }
}

async fn template_view(db: &SqlitePool, template_id: &str) -> ApiResult<DesignTemplateView> {
    let row = sqlx::query(
        "SELECT t.id, t.name, t.layout_json, t.created_at, u.username AS created_by
         FROM design_templates t
         LEFT JOIN users u ON u.id = t.created_by
         WHERE t.id = ?",
    )
    .bind(template_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Template desain tidak ditemukan."))?;
    Ok(template_from_row(row))
}

pub async fn save_layout_image(
    db: &SqlitePool,
    config: &Config,
    user_id: &str,
    period_id: &str,
    filename: &str,
    bytes: &[u8],
) -> ApiResult<UploadedImage> {
    periods::service::get(db, period_id).await?;
    if bytes.is_empty() {
        return Err(ApiError::bad_request("Berkas gambar kosong."));
    }
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(ApiError::payload_too_large("Ukuran gambar maksimal 8 MB."));
    }

    let (extension, content_type) = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        ("png", "image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ("jpg", "image/jpeg")
    } else {
        return Err(ApiError::bad_request(
            "Gambar harus berformat PNG atau JPEG.",
        ));
    };

    let id = Uuid::new_v4().to_string();
    let directory = config.data_dir.join("design").join(period_id);
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(ApiError::internal)?;
    let stored_path = directory.join(format!("{id}.{extension}"));
    tokio::fs::write(&stored_path, bytes)
        .await
        .map_err(ApiError::internal)?;

    let original_name = Path::new(filename)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("gambar")
        .chars()
        .take(200)
        .collect::<String>();

    let result = sqlx::query(
        "INSERT INTO design_images(id, period_id, filename, content_type, stored_path, size_bytes, created_by)
         VALUES(?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(period_id)
    .bind(&original_name)
    .bind(content_type)
    .bind(stored_path.to_string_lossy().to_string())
    .bind(bytes.len() as i64)
    .bind(user_id)
    .execute(db)
    .await;

    if let Err(error) = result {
        let _ = tokio::fs::remove_file(&stored_path).await;
        return Err(ApiError::internal(error));
    }

    audit::record(
        db,
        Some(user_id),
        "layout.image.upload",
        "period",
        period_id,
        serde_json::json!({ "image_id": id, "bytes": bytes.len() }),
    )
    .await?;

    Ok(UploadedImage {
        url: format!("/api/periods/{period_id}/layout/images/{id}"),
        id,
    })
}

pub async fn layout_image(
    db: &SqlitePool,
    period_id: &str,
    image_id: &str,
) -> ApiResult<(PathBuf, String, String)> {
    let row = sqlx::query(
        "SELECT stored_path, content_type, filename FROM design_images
         WHERE id = ? AND period_id = ?",
    )
    .bind(image_id)
    .bind(period_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Gambar desain tidak ditemukan."))?;

    Ok((
        PathBuf::from(row.get::<String, _>("stored_path")),
        row.get("content_type"),
        row.get("filename"),
    ))
}

pub async fn load_design(
    db: &SqlitePool,
    period_id: &str,
    import_id: &str,
) -> ApiResult<(ReportLayout, HashMap<String, String>)> {
    let layout = get_layout(db, period_id, import_id).await?.layout;
    let mut ids = HashSet::new();
    for page in &layout.pages {
        for element in &page.elements {
            if element.hidden || element.kind != "image" {
                continue;
            }
            if let Some(id) = image_id_from_src(&element.src) {
                ids.insert(id.to_string());
            }
        }
    }

    let mut images = HashMap::new();
    let mut total_bytes = 0usize;
    for id in ids {
        let row = sqlx::query(
            "SELECT stored_path, content_type FROM design_images
             WHERE id = ? AND period_id = ?",
        )
        .bind(&id)
        .bind(period_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?;
        let Some(row) = row else { continue };
        let path: String = row.get("stored_path");
        let Ok(bytes) = tokio::fs::read(&path).await else {
            continue;
        };
        total_bytes += bytes.len();
        if total_bytes > 40 * 1024 * 1024 {
            break;
        }
        let content_type: String = row.get("content_type");
        images.insert(
            id,
            format!("data:{content_type};base64,{}", STANDARD.encode(bytes)),
        );
    }

    Ok((layout, images))
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
    let (layout, images) = load_design(db, period_id, import_id).await?;
    Ok(render::render_with_layout(
        &metrics, &content, config, &layout, &images,
    ))
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
    let (layout, images) = load_design(db, period_id, import_id).await?;
    let html = render::render_with_layout(&metrics, &content, config, &layout, &images);

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
            id, period_id, import_id, content_json, metrics_json, layout_json, pdf_path, html_path,
            polda_name, source_filename, period_label, created_by
         ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&report_id)
    .bind(period_id)
    .bind(import_id)
    .bind(serde_json::to_string(&content).map_err(ApiError::internal)?)
    .bind(serde_json::to_string(&metrics).map_err(ApiError::internal)?)
    .bind(serde_json::to_string(&layout).map_err(ApiError::internal)?)
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
    _config: &Config,
    report_id: &str,
) -> ApiResult<String> {
    let row = sqlx::query("SELECT html_path FROM report_runs WHERE id = ?")
        .bind(report_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Dokumen laporan tidak ditemukan."))?;
    let html_path: String = row.get("html_path");
    if html_path.trim().is_empty() || !Path::new(&html_path).exists() {
        return Err(ApiError::not_found(
            "Snapshot HTML laporan tidak ditemukan. Buat ulang PDF.",
        ));
    }
    tokio::fs::read_to_string(&html_path)
        .await
        .map_err(ApiError::internal)
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
