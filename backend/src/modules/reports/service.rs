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
                DesignTemplateView, LayoutView, MediaItem, ReportLayout, UploadedImage,
                image_id_from_src,
            },
            metrics,
            models::{
                DraftView, PublicShareView, ReportContent, ReportMetrics, ReportShareView,
                ReportView,
            },
            render,
        },
    },
    shared::{audit, truncate},
};

pub async fn list_for_period(db: &SqlitePool, period_id: &str) -> ApiResult<Vec<ReportView>> {
    periods::service::get(db, period_id).await?;
    let rows = sqlx::query(
        "SELECT r.id, r.period_id, r.created_at,
                COALESCE(NULLIF(r.period_label, ''), p.label) AS period_label,
                r.source_filename
         FROM report_runs r
         JOIN report_periods p ON p.id = r.period_id
         WHERE r.period_id = ?
         ORDER BY r.created_at DESC",
    )
    .bind(period_id)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows.into_iter().map(report_from_row).collect())
}

pub async fn get_draft(db: &SqlitePool, period_id: &str) -> ApiResult<DraftView> {
    periods::service::get(db, period_id).await?;
    let row = sqlx::query(
        "SELECT d.content_json, d.updated_at, u.username AS updated_by
         FROM report_drafts d
         LEFT JOIN users u ON u.id = d.updated_by
         WHERE d.period_id = ?",
    )
    .bind(period_id)
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
    content: &ReportContent,
) -> ApiResult<DraftView> {
    periods::service::get(db, period_id).await?;
    let content_json = serde_json::to_string(content).map_err(ApiError::internal)?;
    if content_json.len() > 512 * 1024 {
        return Err(ApiError::payload_too_large(
            "Isi editorial terlalu besar. Ringkas narasi laporan.",
        ));
    }

    sqlx::query(
        "INSERT INTO report_drafts(id, period_id, content_json, updated_by, updated_at)
         VALUES(?,?,?,?,CURRENT_TIMESTAMP)
         ON CONFLICT(period_id) DO UPDATE SET
           content_json = excluded.content_json,
           updated_by = excluded.updated_by,
           updated_at = CURRENT_TIMESTAMP",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(period_id)
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
        serde_json::json!({}),
    )
    .await?;

    get_draft(db, period_id).await
}

pub async fn get_layout(db: &SqlitePool, period_id: &str) -> ApiResult<LayoutView> {
    periods::service::get(db, period_id).await?;
    let row = sqlx::query(
        "SELECT l.layout_json, l.updated_at, u.username AS updated_by
         FROM report_layouts l
         LEFT JOIN users u ON u.id = l.updated_by
         WHERE l.period_id = ?",
    )
    .bind(period_id)
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
    layout: ReportLayout,
) -> ApiResult<LayoutView> {
    periods::service::get(db, period_id).await?;
    let layout = layout.sanitize();
    let layout_json = serde_json::to_string(&layout).map_err(ApiError::internal)?;
    if layout_json.len() > 2 * 1024 * 1024 {
        return Err(ApiError::payload_too_large(
            "Layout desain terlalu besar. Kurangi jumlah elemen atau gambar.",
        ));
    }
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO report_layouts(period_id, layout_json, updated_by, updated_at)
         VALUES(?,?,?,?)
         ON CONFLICT(period_id) DO UPDATE SET
           layout_json = excluded.layout_json,
           updated_by = excluded.updated_by,
           updated_at = excluded.updated_at",
    )
    .bind(period_id)
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
        serde_json::json!({}),
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
    template_id: &str,
) -> ApiResult<LayoutView> {
    periods::service::get(db, period_id).await?;
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

    save_layout(db, user_id, period_id, layout).await
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

pub async fn list_media(db: &SqlitePool, period_id: &str) -> ApiResult<Vec<MediaItem>> {
    periods::service::get(db, period_id).await?;
    let (start, end, _) = periods::service::dates(db, period_id).await?;
    let mut items = Vec::new();

    let design_rows = sqlx::query(
        "SELECT d.id, d.filename, d.content_type, d.size_bytes, d.created_at,
                d.period_id, COALESCE(p.label, '') AS period_label
         FROM design_images d
         LEFT JOIN report_periods p ON p.id = d.period_id
         ORDER BY d.created_at DESC, d.id DESC
         LIMIT 200",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;
    for row in design_rows {
        let id: String = row.get("id");
        let origin_period: String = row.get("period_id");
        let period_label: String = row.get("period_label");
        items.push(MediaItem {
            source: "design".to_string(),
            preview_url: format!("/api/periods/{origin_period}/layout/images/{id}"),
            id,
            filename: row.get("filename"),
            content_type: row.get("content_type"),
            size_bytes: row.get("size_bytes"),
            created_at: row.get("created_at"),
            period_id: origin_period,
            period_label: period_label.clone(),
            import_filename: String::new(),
            context: if period_label.is_empty() {
                "Gambar desain".to_string()
            } else {
                format!("Desain · {period_label}")
            },
        });
    }

    let evidence_rows = sqlx::query(
        "SELECT e.id, e.filename, e.content_type, e.size_bytes, e.created_at,
                e.stored_path, i.filename AS import_filename, r.unit, r.polres, r.kegiatan
         FROM import_evidence e
         JOIN import_rows r ON r.id = e.row_id
         JOIN imports i ON i.id = e.import_id
         WHERE e.status = 'ready'
           AND e.content_type IN ('image/png', 'image/jpeg')
           AND r.tanggal BETWEEN ? AND ?
         ORDER BY e.created_at DESC, e.id DESC
         LIMIT 400",
    )
    .bind(&start)
    .bind(&end)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;
    let mut seen_paths = HashSet::new();
    for row in evidence_rows {
        let stored_path: String = row.get("stored_path");
        if !seen_paths.insert(stored_path) {
            continue;
        }
        let id: String = row.get("id");
        let unit: String = row.get("unit");
        let polres: String = row.get("polres");
        let kegiatan: String = row.get("kegiatan");
        let location = if !unit.trim().is_empty() {
            unit
        } else {
            polres
        };
        let context = if kegiatan.trim().is_empty() {
            location
        } else {
            format!("{location} · {}", truncate(&kegiatan, 80))
        };
        items.push(MediaItem {
            source: "evidence".to_string(),
            preview_url: format!("/api/evidence/{id}"),
            id,
            filename: row.get("filename"),
            content_type: row.get("content_type"),
            size_bytes: row.get("size_bytes"),
            created_at: row.get("created_at"),
            period_id: String::new(),
            period_label: String::new(),
            import_filename: row.get("import_filename"),
            context,
        });
    }

    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    items.truncate(300);
    Ok(items)
}

pub async fn import_media_image(
    db: &SqlitePool,
    config: &Config,
    user_id: &str,
    period_id: &str,
    source: &str,
    media_id: &str,
) -> ApiResult<UploadedImage> {
    periods::service::get(db, period_id).await?;
    match source {
        "design" => {
            let row = sqlx::query(
                "SELECT period_id, filename, stored_path FROM design_images WHERE id = ?",
            )
            .bind(media_id)
            .fetch_optional(db)
            .await
            .map_err(ApiError::internal)?
            .ok_or_else(|| ApiError::not_found("Gambar media tidak ditemukan."))?;
            let origin_period: String = row.get("period_id");
            if origin_period == period_id {
                return Ok(UploadedImage {
                    id: media_id.to_string(),
                    url: format!("/api/periods/{period_id}/layout/images/{media_id}"),
                });
            }
            let filename: String = row.get("filename");
            let stored_path: String = row.get("stored_path");
            let bytes = tokio::fs::read(&stored_path)
                .await
                .map_err(|_| ApiError::not_found("Berkas gambar media tidak ditemukan."))?;
            save_layout_image(db, config, user_id, period_id, &filename, &bytes).await
        }
        "evidence" => {
            let (start, end, _) = periods::service::dates(db, period_id).await?;
            let row = sqlx::query(
                "SELECT e.filename, e.stored_path, e.content_type, e.status
                 FROM import_evidence e
                 JOIN import_rows r ON r.id = e.row_id
                 WHERE e.id = ? AND r.tanggal BETWEEN ? AND ?",
            )
            .bind(media_id)
            .bind(&start)
            .bind(&end)
            .fetch_optional(db)
            .await
            .map_err(ApiError::internal)?
            .ok_or_else(|| ApiError::not_found("Bukti dukung tidak ditemukan pada periode ini."))?;
            let status: String = row.get("status");
            if status != "ready" {
                return Err(ApiError::bad_request(
                    "Bukti dukung belum berhasil diunduh.",
                ));
            }
            let content_type: String = row.get("content_type");
            if !matches!(content_type.as_str(), "image/png" | "image/jpeg") {
                return Err(ApiError::bad_request(
                    "Hanya bukti dukung PNG/JPEG yang dapat dipakai di desain.",
                ));
            }
            let filename: String = row.get("filename");
            let stored_path: String = row.get("stored_path");
            let bytes = tokio::fs::read(&stored_path)
                .await
                .map_err(|_| ApiError::not_found("Berkas bukti dukung tidak ditemukan."))?;
            save_layout_image(db, config, user_id, period_id, &filename, &bytes).await
        }
        _ => Err(ApiError::bad_request("Sumber media tidak dikenali.")),
    }
}

pub async fn load_design(
    db: &SqlitePool,
    period_id: &str,
) -> ApiResult<(ReportLayout, HashMap<String, String>)> {
    let layout = get_layout(db, period_id).await?.layout;
    let images = load_images(db, period_id, &layout).await?;
    Ok((layout, images))
}

async fn load_images(
    db: &SqlitePool,
    period_id: &str,
    layout: &ReportLayout,
) -> ApiResult<HashMap<String, String>> {
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

    Ok(images)
}

async fn source_summary(db: &SqlitePool, start: &str, end: &str) -> ApiResult<String> {
    let imports = imports::service::list_for_range(db, start, end).await?;
    Ok(match imports.as_slice() {
        [] => "Tidak ada impor".to_string(),
        [single] => single.filename.clone(),
        [first, rest @ ..] => format!("{} + {} berkas lainnya", first.filename, rest.len()),
    })
}

pub async fn metrics_for(
    db: &SqlitePool,
    config: &Config,
    period_id: &str,
) -> ApiResult<ReportMetrics> {
    let (start, end, label) = periods::service::dates(db, period_id).await?;
    let filename = source_summary(db, &start, &end).await?;
    metrics::load_period(db, &filename, &label, &start, &end, &config.polda_name).await
}

pub async fn preview_html(
    db: &SqlitePool,
    config: &Config,
    period_id: &str,
    content_override: Option<&ReportContent>,
    layout_override: Option<&ReportLayout>,
) -> ApiResult<String> {
    let metrics = metrics_for(db, config, period_id).await?;
    let content = match content_override {
        Some(content) => content.clone(),
        None => get_draft(db, period_id).await?.content,
    };
    let (layout, images) = match layout_override {
        Some(layout) => {
            let layout = layout.clone().sanitize();
            let images = load_images(db, period_id, &layout).await?;
            (layout, images)
        }
        None => load_design(db, period_id).await?,
    };
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
) -> ApiResult<ReportView> {
    let (start_date, end_date, label) = periods::service::dates(db, period_id).await?;
    let source_filename = source_summary(db, &start_date, &end_date).await?;
    let content = get_draft(db, period_id).await?.content;
    let metrics = metrics_for(db, config, period_id).await?;
    let (layout, images) = load_design(db, period_id).await?;
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
            id, period_id, content_json, metrics_json, layout_json, pdf_path, html_path,
            polda_name, source_filename, period_label, created_by
         ) VALUES(?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&report_id)
    .bind(period_id)
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
            "start_date": start_date,
            "end_date": end_date,
        }),
    )
    .await?;

    Ok(ReportView {
        id: report_id.clone(),
        period_id: period_id.to_string(),
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
            "Pratinjau dokumen tidak tersedia. Buat ulang PDF.",
        ));
    }
    tokio::fs::read_to_string(&html_path)
        .await
        .map_err(ApiError::internal)
}

fn report_from_row(row: sqlx::sqlite::SqliteRow) -> ReportView {
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
}

fn file_url(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.starts_with('/') {
        format!("file://{normalized}")
    } else {
        format!("file:///{normalized}")
    }
}

pub async fn report_paths(
    db: &SqlitePool,
    report_id: &str,
) -> ApiResult<(PathBuf, String, String)> {
    let row = sqlx::query("SELECT pdf_path, html_path, period_label FROM report_runs WHERE id = ?")
        .bind(report_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Dokumen laporan tidak ditemukan."))?;
    Ok((
        PathBuf::from(row.get::<String, _>("pdf_path")),
        row.get::<String, _>("html_path"),
        row.get("period_label"),
    ))
}

fn report_filename(label: &str) -> String {
    let sanitized: String = label
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { '-' })
        .collect();
    format!("laporan-{}.pdf", sanitized.trim_matches('-'))
}

pub async fn download_path(
    db: &SqlitePool,
    _config: &Config,
    report_id: &str,
) -> ApiResult<(PathBuf, String)> {
    let (path, _html, label) = report_paths(db, report_id).await?;
    if !path.exists() {
        return Err(ApiError::not_found("Berkas PDF tidak ditemukan."));
    }
    Ok((path, report_filename(&label)))
}

pub async fn delete_report(
    db: &SqlitePool,
    config: &Config,
    user_id: &str,
    report_id: &str,
) -> ApiResult<()> {
    let mut tx = db.begin().await.map_err(ApiError::internal)?;
    let row = sqlx::query("SELECT pdf_path, html_path FROM report_runs WHERE id = ?")
        .bind(report_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Dokumen laporan tidak ditemukan."))?;
    let pdf_path: String = row.get("pdf_path");
    let html_path: String = row.get("html_path");

    sqlx::query("DELETE FROM report_shares WHERE report_id = ?")
        .bind(report_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::internal)?;
    sqlx::query("DELETE FROM report_runs WHERE id = ?")
        .bind(report_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::internal)?;

    audit::record_tx(
        &mut tx,
        Some(user_id),
        "report.delete",
        "report",
        report_id,
        serde_json::json!({ "pdf_path": &pdf_path, "html_path": &html_path }),
    )
    .await?;
    tx.commit().await.map_err(ApiError::internal)?;

    for raw_path in [pdf_path, html_path] {
        if raw_path.trim().is_empty() {
            continue;
        }
        let path = PathBuf::from(&raw_path);
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            continue;
        }
        if !managed_report_file(config, &path) {
            tracing::warn!(path = %path.display(), "melewati berkas laporan di luar direktori reports");
            continue;
        }
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "gagal menghapus berkas laporan");
            }
        }
    }
    Ok(())
}

fn managed_report_file(config: &Config, path: &Path) -> bool {
    let Ok(base) = config.data_dir.join("reports").canonicalize() else {
        return false;
    };
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    path.starts_with(base)
}

pub async fn create_share(
    db: &SqlitePool,
    user_id: &str,
    report_id: &str,
) -> ApiResult<ReportShareView> {
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM report_runs WHERE id = ?")
        .bind(report_id)
        .fetch_one(db)
        .await
        .map_err(ApiError::internal)?;
    if exists == 0 {
        return Err(ApiError::not_found("Dokumen laporan tidak ditemukan."));
    }

    if let Some(row) = sqlx::query(
        "SELECT token, created_at FROM report_shares
         WHERE report_id = ? AND revoked_at IS NULL
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(report_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    {
        return Ok(ReportShareView {
            token: row.get("token"),
            created_at: row.get("created_at"),
        });
    }

    let token = Uuid::new_v4().simple().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT OR IGNORE INTO report_shares(id, report_id, token, created_by, created_at)
         VALUES(?,?,?,?,?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(report_id)
    .bind(&token)
    .bind(user_id)
    .bind(&created_at)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    let row = sqlx::query(
        "SELECT token, created_at FROM report_shares
         WHERE report_id = ? AND revoked_at IS NULL
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(report_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::internal("Tautan publik gagal dibuat."))?;

    if result.rows_affected() > 0 {
        audit::record(
            db,
            Some(user_id),
            "report.share",
            "report",
            report_id,
            serde_json::json!({ "token": token }),
        )
        .await?;
    }

    Ok(ReportShareView {
        token: row.get("token"),
        created_at: row.get("created_at"),
    })
}

async fn public_share_row(db: &SqlitePool, token: &str) -> ApiResult<sqlx::sqlite::SqliteRow> {
    sqlx::query(
        "SELECT r.pdf_path, r.html_path,
                COALESCE(NULLIF(r.period_label, ''), p.label) AS period_label,
                s.created_at AS shared_at
         FROM report_shares s
         JOIN report_runs r ON r.id = s.report_id
         JOIN report_periods p ON p.id = r.period_id
         WHERE s.token = ? AND s.revoked_at IS NULL",
    )
    .bind(token)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Tautan laporan tidak ditemukan atau sudah tidak aktif."))
}

pub async fn public_share(db: &SqlitePool, token: &str) -> ApiResult<PublicShareView> {
    let row = public_share_row(db, token).await?;
    let period_label: String = row.get("period_label");
    Ok(PublicShareView {
        token: token.to_string(),
        title: period_label.clone(),
        period_label,
        created_at: row.get("shared_at"),
    })
}

pub async fn public_preview(db: &SqlitePool, token: &str) -> ApiResult<String> {
    let row = public_share_row(db, token).await?;
    let html_path: String = row.get("html_path");
    if html_path.trim().is_empty() || !Path::new(&html_path).exists() {
        return Err(ApiError::not_found(
            "Pratinjau presentasi tidak tersedia. Buat ulang PDF.",
        ));
    }
    tokio::fs::read_to_string(&html_path)
        .await
        .map_err(ApiError::internal)
}

pub async fn public_download_path(db: &SqlitePool, token: &str) -> ApiResult<(PathBuf, String)> {
    let row = public_share_row(db, token).await?;
    let path = PathBuf::from(row.get::<String, _>("pdf_path"));
    if !path.exists() {
        return Err(ApiError::not_found("Berkas PDF tidak ditemukan."));
    }
    let label: String = row.get("period_label");
    Ok((path, report_filename(&label)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn media_library_lists_and_copies_images() {
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
             VALUES('p1','2026-10-19','2026-10-25','19–25 Oktober 2026','u1'),
                   ('p2','2026-10-26','2026-11-01','26 Oktober–1 November 2026','u1')",
        )
        .execute(&db)
        .await
        .unwrap();

        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let uploaded = super::save_layout_image(&db, &config, "u1", "p1", "desain.png", &png)
            .await
            .unwrap();

        let evidence_dir = config.data_dir.join("evidence").join("imp1");
        tokio::fs::create_dir_all(&evidence_dir).await.unwrap();
        let evidence_path = evidence_dir.join("ev1.png");
        tokio::fs::write(&evidence_path, png).await.unwrap();

        sqlx::query(
            "INSERT INTO imports(id,filename,file_hash,stored_path,parser_version,row_count,
                skipped_rows,duplicate_rows,date_start,date_end,warnings_json,imported_by)
             VALUES('imp1','bukti.xlsx','hash','berkas.xlsx','3',1,0,0,
                '2026-10-19','2026-10-25','[]','u1')",
        )
        .execute(&db)
        .await
        .unwrap();
        let row_id = sqlx::query(
            "INSERT INTO import_rows(import_id,source_row,tanggal,unit,polres,kegiatan,raw_json)
             VALUES('imp1',2,'2026-10-20','Polsek Kuta','Polresta Denpasar','Patroli','{}')",
        )
        .execute(&db)
        .await
        .unwrap()
        .last_insert_rowid();
        sqlx::query(
            "INSERT INTO import_evidence(id,import_id,row_id,source_url,filename,content_type,
                stored_path,size_bytes,status)
             VALUES('ev1','imp1',?,'https://contoh/foto.png','foto.png','image/png',?,?,'ready')",
        )
        .bind(row_id)
        .bind(evidence_path.to_string_lossy().to_string())
        .bind(png.len() as i64)
        .execute(&db)
        .await
        .unwrap();

        let media = super::list_media(&db, "p1").await.unwrap();
        assert_eq!(media.len(), 2);
        assert!(media.iter().any(|item| item.source == "design"));
        let evidence = media.iter().find(|item| item.source == "evidence").unwrap();
        assert_eq!(evidence.id, "ev1");
        assert_eq!(evidence.preview_url, "/api/evidence/ev1");
        assert_eq!(evidence.context, "Polsek Kuta · Patroli");

        let same = super::import_media_image(&db, &config, "u1", "p1", "design", &uploaded.id)
            .await
            .unwrap();
        assert_eq!(same.id, uploaded.id);

        let copied = super::import_media_image(&db, &config, "u1", "p1", "evidence", "ev1")
            .await
            .unwrap();
        assert_ne!(copied.id, uploaded.id);
        assert!(
            tokio::fs::try_exists(dir.path().join("design/p1"))
                .await
                .unwrap()
        );

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM design_images WHERE period_id = 'p1'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(count, 2);

        let cross = super::import_media_image(&db, &config, "u1", "p2", "design", &uploaded.id)
            .await
            .unwrap();
        assert_ne!(cross.id, uploaded.id);
    }

    #[tokio::test]
    async fn shares_and_deletes_report_with_public_links() {
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
             VALUES('p1','2026-10-19','2026-10-25','19–25 Oktober 2026','u1')",
        )
        .execute(&db)
        .await
        .unwrap();

        let report_dir = dir.path().join("reports").join("p1");
        tokio::fs::create_dir_all(&report_dir).await.unwrap();
        let html_path = report_dir.join("r1.html");
        let pdf_path = report_dir.join("r1.pdf");
        tokio::fs::write(
            &html_path,
            "<html><body><section class=\"page\">Halaman presentasi</section></body></html>",
        )
        .await
        .unwrap();
        tokio::fs::write(&pdf_path, b"%PDF-1.4").await.unwrap();

        sqlx::query(
            "INSERT INTO report_runs(
                id, period_id, content_json, metrics_json, layout_json, pdf_path, html_path,
                polda_name, source_filename, period_label, created_by
             ) VALUES('r1','p1','{}','{}','{}',?,?,'POLDA BALI','data.xlsx','19–25 Oktober 2026','u1')",
        )
        .bind(pdf_path.to_string_lossy().to_string())
        .bind(html_path.to_string_lossy().to_string())
        .execute(&db)
        .await
        .unwrap();

        let share = super::create_share(&db, "u1", "r1").await.unwrap();
        assert_eq!(share.token.len(), 32);
        let again = super::create_share(&db, "u1", "r1").await.unwrap();
        assert_eq!(again.token, share.token);

        let metadata = super::public_share(&db, &share.token).await.unwrap();
        assert_eq!(metadata.period_label, "19–25 Oktober 2026");
        assert_eq!(metadata.title, "19–25 Oktober 2026");

        let preview = super::public_preview(&db, &share.token).await.unwrap();
        assert!(preview.contains("Halaman presentasi"));

        let (path, filename) = super::public_download_path(&db, &share.token)
            .await
            .unwrap();
        assert!(path.exists());
        assert_eq!(filename, "laporan-19-25-Oktober-2026.pdf");
        assert!(super::public_share(&db, "tidak-ada").await.is_err());

        super::delete_report(&db, &config, "u1", "r1")
            .await
            .unwrap();
        assert!(!html_path.exists());
        assert!(!pdf_path.exists());
        let shares: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM report_shares WHERE report_id = 'r1'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(shares, 0);
        assert!(super::public_share(&db, &share.token).await.is_err());
        assert!(super::delete_report(&db, &config, "u1", "r1").await.is_err());
    }
}
