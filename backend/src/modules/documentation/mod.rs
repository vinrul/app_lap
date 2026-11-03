//! Analisis dokumentasi: OCR + EXIF pada bukti dukung gambar.

mod media;

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, OnceLock},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use tokio::sync::Semaphore;

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    modules::{
        auth::AuthUser,
        imports::evidence_store,
        live::{Hub, JobHandle, JobRegistry, ServerMessage},
        periods,
    },
    state::AppState,
};

const MAX_DISTANCE_METERS: f64 = 500.0;
const DAY_TOLERANCE: i64 = 1;

/// Satu pemindaian berjalan pada satu waktu agar Tesseract tidak membanjiri CPU.
static SCAN_LOCK: OnceLock<Semaphore> = OnceLock::new();

fn scan_lock() -> &'static Semaphore {
    SCAN_LOCK.get_or_init(|| Semaphore::new(1))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/periods/{period_id}/documentation", get(list))
        .route(
            "/periods/{period_id}/documentation/rows/{row_id}",
            get(row_detail),
        )
        .route("/periods/{period_id}/documentation/scan", post(scan))
        .route("/periods/{period_id}/documentation/rescan", post(rescan))
        .route("/periods/{period_id}/documentation/cancel", post(cancel))
}

#[derive(Serialize)]
pub struct DocumentationRow {
    pub row_id: i64,
    pub tanggal: String,
    pub unit: String,
    pub status: String,
    pub reason: String,
    pub links: i64,
    pub images: i64,
    pub valid: i64,
    pub manual: i64,
}

#[derive(Serialize)]
pub struct DocumentationImage {
    pub evidence_id: String,
    pub filename: String,
    pub content_type: String,
    pub status: String,
    pub reason: String,
    pub ocr_text: String,
    pub analyzed_at: String,
}

#[derive(Serialize, Default)]
pub struct DocumentationSummary {
    pub total_rows: i64,
    pub valid: i64,
    pub manual: i64,
    pub mabes: i64,
    pub invalid: i64,
    pub pending: i64,
    pub images_total: i64,
    pub images_done: i64,
}

#[derive(Serialize)]
pub struct DocumentationResponse {
    pub summary: DocumentationSummary,
    pub rows: Vec<DocumentationRow>,
    pub scanning: bool,
}

fn job_id(period_id: &str) -> String {
    format!("docs:{period_id}")
}

#[derive(Default)]
struct EvidenceStats {
    images: i64,
    valid: i64,
    manual: i64,
    analyzed: i64,
    reason: String,
}

async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<DocumentationResponse>> {
    user.require_writer()?;
    let (start, end, _) = periods::service::dates(&state.db, &period_id).await?;

    let rows = period_rows(&state.db, &start, &end).await?;
    let mut stats: HashMap<i64, EvidenceStats> = HashMap::new();
    for row in evidence_rows(&state.db, &start, &end).await? {
        let row_id: i64 = row.get("row_id");
        let content_type: String = row.get("content_type");
        let evidence_status: String = row.get("evidence_status");
        let analysis: Option<String> = row.try_get("analysis_status").ok().flatten();
        let reason: Option<String> = row.try_get("reason").ok().flatten();
        let entry = stats.entry(row_id).or_default();
        if !content_type.starts_with("image/") || evidence_status != "ready" {
            continue;
        }
        entry.images += 1;
        match analysis.as_deref() {
            Some("valid") => {
                entry.valid += 1;
                entry.analyzed += 1;
            }
            Some(_) => {
                entry.manual += 1;
                entry.analyzed += 1;
                if entry.reason.is_empty() {
                    entry.reason = reason.unwrap_or_default();
                }
            }
            None => {}
        }
    }

    let mut summary = DocumentationSummary::default();
    let mut output = Vec::with_capacity(rows.len());
    for row in rows {
        let row_id: i64 = row.get("id");
        let tanggal: String = row.get("tanggal");
        let unit: String = row.get("unit");
        let parsed_json: String = row.get("parsed_json");
        let bukti: String = row.get("bukti_dukung");
        let links = row_links(&parsed_json, &bukti);
        let entry = stats.remove(&row_id).unwrap_or_default();
        let (status, reason) = row_status(&links, &entry);

        summary.total_rows += 1;
        summary.images_total += entry.images;
        summary.images_done += entry.analyzed;
        match status.as_str() {
            "valid" => summary.valid += 1,
            "manual" => summary.manual += 1,
            "mabes" => summary.mabes += 1,
            "invalid" => summary.invalid += 1,
            _ => summary.pending += 1,
        }
        output.push(DocumentationRow {
            row_id,
            tanggal,
            unit,
            status: status.to_string(),
            reason,
            links: links.len() as i64,
            images: entry.images,
            valid: entry.valid,
            manual: entry.manual,
        });
    }

    let scanning = is_scanning(&state.db, &period_id).await;
    Ok(Json(DocumentationResponse {
        summary,
        rows: output,
        scanning,
    }))
}

/// Rincian per gambar untuk satu baris laporan, dipakai panel cek manual agar
/// teks OCR dan alasan tiap gambar bisa ditelusuri.
async fn row_detail(
    State(state): State<AppState>,
    user: AuthUser,
    Path((period_id, row_id)): Path<(String, i64)>,
) -> ApiResult<Json<Vec<DocumentationImage>>> {
    user.require_writer()?;
    let (start, end, _) = periods::service::dates(&state.db, &period_id).await?;
    let rows = sqlx::query(
        "SELECT e.id, e.filename, e.content_type, e.status AS evidence_status,
                a.status AS analysis_status, a.reason, a.ocr_text, a.analyzed_at
         FROM import_evidence e
         LEFT JOIN evidence_analysis a ON a.evidence_id = e.id
         WHERE e.row_id = ?
           AND EXISTS (
               SELECT 1 FROM import_rows r
               WHERE r.id = e.row_id AND r.tanggal BETWEEN ? AND ?
           )
         ORDER BY e.id",
    )
    .bind(row_id)
    .bind(&start)
    .bind(&end)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?;

    let mut output = Vec::with_capacity(rows.len());
    for row in rows {
        let evidence_status: String = row.get("evidence_status");
        let analysis_status: Option<String> = row.try_get("analysis_status").ok().flatten();
        let status = match analysis_status {
            Some(status) => status,
            None if evidence_status == "ready" => "pending".to_string(),
            None => evidence_status,
        };
        let reason: Option<String> = row.try_get("reason").ok().flatten();
        let ocr_text: Option<String> = row.try_get("ocr_text").ok().flatten();
        let analyzed_at: Option<String> = row.try_get("analyzed_at").ok().flatten();
        output.push(DocumentationImage {
            evidence_id: row.get("id"),
            filename: row.get("filename"),
            content_type: row.get("content_type"),
            status,
            reason: reason.unwrap_or_default(),
            ocr_text: ocr_text.unwrap_or_default(),
            analyzed_at: analyzed_at.unwrap_or_default(),
        });
    }
    Ok(Json(output))
}

fn row_status(links: &[String], entry: &EvidenceStats) -> (String, String) {
    if entry.images > 0 {
        if entry.valid > 0 {
            return ("valid".to_string(), "Bukti dukung valid sesuai".to_string());
        }
        if entry.analyzed > 0 {
            let reason = if entry.reason.is_empty() {
                "Perlu cek manual".to_string()
            } else {
                entry.reason.clone()
            };
            return ("manual".to_string(), reason);
        }
        return ("pending".to_string(), "Belum dianalisis".to_string());
    }

    let has_http = links.iter().any(|link| link.starts_with("http"));
    let has_upload = links
        .iter()
        .any(|link| link.contains("/upload/") && !link.starts_with("http"));
    if has_upload {
        return (
            "mabes".to_string(),
            "Dokumentasi valid, cek ke mabes".to_string(),
        );
    }
    if links.is_empty() {
        return (
            "invalid".to_string(),
            "Tidak ada tautan bukti dukung".to_string(),
        );
    }
    if has_http {
        return (
            "invalid".to_string(),
            "Tautan tidak dapat diunduh atau bukan gambar".to_string(),
        );
    }
    (
        "invalid".to_string(),
        "Tidak ada gambar; bukti dukung harus berupa gambar".to_string(),
    )
}

fn row_links(parsed_json: &str, bukti: &str) -> Vec<String> {
    let mut links = serde_json::from_str::<serde_json::Value>(parsed_json)
        .ok()
        .and_then(|value| {
            value
                .get("links")
                .and_then(|links| links.as_array())
                .cloned()
        })
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| crate::shared::extract_links(bukti));
    links.sort();
    links.dedup();
    links
}

async fn scan(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    user.require_writer()?;
    let started = start_scan(
        &state.db,
        state.config.clone(),
        state.hub.clone(),
        state.jobs.clone(),
        &period_id,
        false,
    )
    .await?;
    Ok(Json(serde_json::json!({ "started": started })))
}

/// Menganalisis ulang seluruh gambar yang sudah terunduh, termasuk yang sudah
/// punya hasil analisis. Dipakai setelah aturan OCR diperbarui.
async fn rescan(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    user.require_writer()?;
    let started = start_scan(
        &state.db,
        state.config.clone(),
        state.hub.clone(),
        state.jobs.clone(),
        &period_id,
        true,
    )
    .await?;
    Ok(Json(serde_json::json!({ "started": started })))
}

async fn cancel(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    user.require_writer()?;
    let cancelled = state.jobs.cancel(&job_id(&period_id));
    Ok(Json(serde_json::json!({ "cancelled": cancelled })))
}

async fn is_scanning(db: &SqlitePool, period_id: &str) -> bool {
    sqlx::query_scalar::<_, String>("SELECT status FROM documentation_runs WHERE period_id = ?")
        .bind(period_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .is_some_and(|status| status == "running")
}

/// Memindai semua periode yang masih memiliki gambar terunduh namun belum dianalisis.
pub async fn resume(state: &AppState) {
    let started = std::time::Instant::now();
    match media::tesseract_version(&state.config).await {
        Ok(version) => tracing::info!(
            %version,
            path = %state.config.tesseract_path,
            "analisis dokumentasi: Tesseract siap"
        ),
        Err(error) => tracing::warn!(
            %error,
            path = %state.config.tesseract_path,
            "analisis dokumentasi: Tesseract tidak tersedia; OCR akan gagal dan hanya EXIF yang dipakai"
        ),
    }
    let periods: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT p.id
         FROM report_periods p
         JOIN import_rows r ON r.tanggal BETWEEN p.start_date AND p.end_date
         JOIN import_evidence e ON e.row_id = r.id
         LEFT JOIN evidence_analysis a ON a.evidence_id = e.id
         WHERE e.status = 'ready' AND e.content_type LIKE 'image/%'
           AND a.evidence_id IS NULL",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let total = periods.len();
    for period_id in periods {
        if let Err(error) = start_scan(
            &state.db,
            state.config.clone(),
            state.hub.clone(),
            state.jobs.clone(),
            &period_id,
            false,
        )
        .await
        {
            tracing::warn!(%error, %period_id, "gagal memulai analisis dokumentasi");
        }
    }
    tracing::info!(
        periods = total,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "analisis dokumentasi: pemeriksaan target awal selesai"
    );
}

/// Dipanggil setelah unduhan bukti dukung selesai: pindai periode yang mencakup
/// rentang tanggal impor tersebut.
pub async fn autoscan_import(
    db: &SqlitePool,
    config: Arc<Config>,
    hub: Arc<Hub>,
    jobs: Arc<JobRegistry>,
    import_id: &str,
) {
    let periods: Vec<String> = sqlx::query_scalar(
        "SELECT p.id FROM report_periods p
         JOIN imports i ON i.id = ?
         WHERE i.date_start <= p.end_date AND i.date_end >= p.start_date",
    )
    .bind(import_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for period_id in periods {
        if let Err(error) = start_scan(
            db,
            config.clone(),
            hub.clone(),
            jobs.clone(),
            &period_id,
            false,
        )
        .await
        {
            tracing::warn!(%error, %period_id, "gagal memulai analisis dokumentasi otomatis");
        }
    }
}

struct Target {
    evidence_id: String,
    import_id: String,
    row_id: i64,
    stored_path: String,
    tanggal: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

async fn start_scan(
    db: &SqlitePool,
    config: Arc<Config>,
    hub: Arc<Hub>,
    jobs: Arc<JobRegistry>,
    period_id: &str,
    force: bool,
) -> ApiResult<bool> {
    let job = job_id(period_id);
    if jobs.is_running(&job) {
        return Ok(false);
    }
    let (start, end, _) = periods::service::dates(db, period_id).await?;
    if scan_targets(db, &start, &end, force).await.is_empty() {
        return Ok(false);
    }
    let handle = jobs.start(&job);
    sqlx::query(
        "INSERT INTO documentation_runs(period_id, status, updated_at)
         VALUES(?, 'running', ?)
         ON CONFLICT(period_id) DO UPDATE SET status = 'running', updated_at = excluded.updated_at",
    )
    .bind(period_id)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    let task_db = db.clone();
    let task_config = config.clone();
    let task_hub = hub.clone();
    let task_jobs = jobs.clone();
    let task_job = job.clone();
    let task_period = period_id.to_string();
    let task_handle = handle.clone();
    let task = tokio::spawn(async move {
        run(
            task_db,
            task_config,
            task_hub,
            task_jobs,
            task_period,
            task_job,
            task_handle,
            start,
            end,
            force,
        )
        .await;
    });
    handle.attach_task(task);
    Ok(true)
}

async fn run(
    db: SqlitePool,
    config: Arc<Config>,
    hub: Arc<Hub>,
    jobs: Arc<JobRegistry>,
    period_id: String,
    job: String,
    handle: Arc<JobHandle>,
    start: String,
    end: String,
    force: bool,
) {
    let Ok(_permit) = scan_lock().acquire().await else {
        return;
    };
    let mut total = 0_i64;
    let mut done = 0_i64;
    let mut valid_count = 0_i64;
    let mut manual_count = 0_i64;
    let mut processed: HashSet<String> = HashSet::new();

    // Putaran berulang agar gambar yang selesai diunduh saat pemindaian berjalan
    // tetap ikut dianalisis. Pada pindai ulang, `processed` mencegah gambar yang
    // sama dianalisis berulang kali.
    loop {
        if handle.is_cancelled() {
            break;
        }
        let targets: Vec<Target> = scan_targets(&db, &start, &end, force)
            .await
            .into_iter()
            .filter(|target| !processed.contains(&target.evidence_id))
            .collect();
        if targets.is_empty() {
            break;
        }
        total += targets.len() as i64;
        update_run(&db, &period_id, "running", total, done).await;
        publish(&hub, &job, "scan", total, done).await;

        for target in targets {
            if handle.is_cancelled() {
                break;
            }
            let result = analyze(&config, &target).await;
            if result.status == "valid" {
                valid_count += 1;
            } else {
                manual_count += 1;
                tracing::warn!(
                    evidence_id = %target.evidence_id,
                    row_id = target.row_id,
                    file = %target.stored_path,
                    reason = %result.reason,
                    "analisis dokumentasi: bukti dukung perlu cek manual"
                );
            }
            store_analysis(&db, &target.evidence_id, target.row_id, &result).await;
            processed.insert(target.evidence_id.clone());
            done += 1;
            if done % 5 == 0 {
                update_run(&db, &period_id, "running", total, done).await;
                publish(&hub, &job, "scan", total, done).await;
            }
        }
    }

    let cancelled = handle.is_cancelled();
    let status = if cancelled { "cancelled" } else { "done" };
    tracing::info!(
        %period_id,
        total,
        done,
        valid = valid_count,
        manual = manual_count,
        cancelled,
        "analisis dokumentasi: pemindaian selesai"
    );
    update_run(&db, &period_id, status, total, done).await;
    jobs.finish(&job);
    hub.publish(
        &job,
        ServerMessage::JobFinished {
            job: job.clone(),
            status: status.to_string(),
            total,
            ready: done,
            failed: 0,
            skipped: 0,
            pending: (total - done).max(0),
        },
    )
    .await;
}

async fn publish(hub: &Hub, job: &str, phase: &str, total: i64, done: i64) {
    hub.publish(
        job,
        ServerMessage::JobState {
            job: job.to_string(),
            kind: "documentation".to_string(),
            phase: phase.to_string(),
            total,
            ready: done,
            failed: 0,
            skipped: 0,
            pending: (total - done).max(0),
        },
    )
    .await;
}

async fn update_run(db: &SqlitePool, period_id: &str, status: &str, total: i64, done: i64) {
    let _ = sqlx::query(
        "UPDATE documentation_runs SET status = ?, total = ?, done = ?, updated_at = ?
         WHERE period_id = ?",
    )
    .bind(status)
    .bind(total)
    .bind(done)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(period_id)
    .execute(db)
    .await;
}

struct AnalysisResult {
    status: String,
    reason: String,
    ocr_text: String,
    has_datetime: bool,
    has_address: bool,
    has_coords: bool,
    exif_datetime: String,
    exif_lat: Option<f64>,
    exif_lon: Option<f64>,
}

async fn analyze(config: &Config, target: &Target) -> AnalysisResult {
    let Some(path) =
        evidence_store::resolve_path(&config.data_dir, &target.stored_path, &target.import_id)
            .await
    else {
        tracing::warn!(
            evidence_id = %target.evidence_id,
            row_id = target.row_id,
            stored_path = %target.stored_path,
            "analisis dokumentasi: berkas bukti dukung tidak ditemukan"
        );
        return manual("Berkas bukti dukung tidak ditemukan");
    };
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(
                evidence_id = %target.evidence_id,
                file = %path.display(),
                %error,
                "analisis dokumentasi: berkas tidak dapat dibaca"
            );
            Vec::new()
        }
    };
    let exif = media::read_jpeg_exif(&bytes);
    let exif_present = !exif.datetime.is_empty() || exif.latitude.is_some();
    if !exif_present {
        tracing::debug!(
            evidence_id = %target.evidence_id,
            file = %path.display(),
            "analisis dokumentasi: EXIF tidak ditemukan"
        );
    }

    let ocr = media::ocr(config, &path).await;
    let ocr_error = ocr.as_ref().err().cloned();
    if let Some(error) = &ocr_error {
        tracing::warn!(
            evidence_id = %target.evidence_id,
            row_id = target.row_id,
            file = %path.display(),
            ocr_error = %error,
            exif_tersedia = exif_present,
            "analisis dokumentasi: OCR gagal"
        );
        if !exif_present {
            tracing::warn!(
                evidence_id = %target.evidence_id,
                row_id = target.row_id,
                "analisis dokumentasi: OCR dan EXIF sama-sama gagal; perlu cek manual"
            );
        }
    }
    let text = ocr.unwrap_or_default();
    if ocr_error.is_none() && !media::has_text(&text) {
        tracing::warn!(
            evidence_id = %target.evidence_id,
            row_id = target.row_id,
            file = %path.display(),
            bytes = bytes.len(),
            "analisis dokumentasi: OCR tidak menghasilkan teks"
        );
    } else if ocr_error.is_none() && !media::meaningful(&text) {
        tracing::warn!(
            evidence_id = %target.evidence_id,
            row_id = target.row_id,
            file = %path.display(),
            chars = text.chars().filter(|character| character.is_alphanumeric()).count(),
            "analisis dokumentasi: teks OCR sangat sedikit"
        );
    }

    let watermark_date = media::find_date(&text);
    let watermark_time = media::has_time(&text);
    let has_address = media::has_address(&text);
    let watermark_coords = media::find_coords(&text);
    let exif_date = media::exif_date(&exif.datetime);
    let coords = watermark_coords.or_else(|| match (exif.latitude, exif.longitude) {
        (Some(lat), Some(lon)) => Some((lat, lon)),
        _ => None,
    });
    let photo_date = watermark_date.or(exif_date);

    let mut result = AnalysisResult {
        status: "manual".to_string(),
        reason: String::new(),
        ocr_text: text.chars().take(2000).collect(),
        has_datetime: (watermark_date.is_some() && watermark_time) || exif_date.is_some(),
        has_address,
        has_coords: coords.is_some(),
        exif_datetime: exif.datetime.clone(),
        exif_lat: exif.latitude,
        exif_lon: exif.longitude,
    };

    if let Some(error) = ocr_error {
        let short: String = error.chars().take(160).collect();
        result.reason = if exif_present {
            format!("OCR gagal ({short}); EXIF tersedia, cek manual")
        } else {
            format!("OCR gagal ({short}); EXIF tidak tersedia, cek manual")
        };
        return result;
    }
    if !media::has_text(&text) {
        result.reason = if exif_present {
            "OCR tidak menghasilkan teks; EXIF tersedia, cek manual".to_string()
        } else {
            "OCR tidak menghasilkan teks; EXIF tidak tersedia, cek manual".to_string()
        };
        return result;
    }
    if watermark_date.is_none() || !watermark_time {
        result.reason = if watermark_date.is_some() {
            "Jam tidak terlihat pada watermark".to_string()
        } else {
            "Tanggal dan jam tidak terlihat pada watermark".to_string()
        };
        return result;
    }
    if !has_address {
        result.reason = "Alamat tidak terlihat pada watermark".to_string();
        return result;
    }
    if let Some(date) = photo_date {
        if !dates_close(&target.tanggal, date) {
            result.reason = format!(
                "Tanggal bukti ({}) tidak sesuai tanggal laporan ({})",
                date, target.tanggal
            );
            return result;
        }
    }
    if let (Some((lat, lon)), Some(row_lat), Some(row_lon)) =
        (coords, target.latitude, target.longitude)
    {
        let distance = media::haversine_meters((lat, lon), (row_lat, row_lon));
        if distance > MAX_DISTANCE_METERS {
            result.reason = format!(
                "Koordinat bukti berjarak {:.0} m dari lokasi laporan",
                distance
            );
            return result;
        }
    }

    result.status = "valid".to_string();
    result.reason = "Bukti dukung valid sesuai".to_string();
    result
}

fn manual(reason: &str) -> AnalysisResult {
    AnalysisResult {
        status: "manual".to_string(),
        reason: reason.to_string(),
        ocr_text: String::new(),
        has_datetime: false,
        has_address: false,
        has_coords: false,
        exif_datetime: String::new(),
        exif_lat: None,
        exif_lon: None,
    }
}

fn dates_close(raw: &str, date: NaiveDate) -> bool {
    let Ok(expected) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") else {
        return true;
    };
    (date - expected).num_days().abs() <= DAY_TOLERANCE
}

async fn store_analysis(db: &SqlitePool, evidence_id: &str, row_id: i64, result: &AnalysisResult) {
    let _ = sqlx::query(
        "INSERT INTO evidence_analysis(
            evidence_id, row_id, status, reason, ocr_text, has_datetime, has_address,
            has_coords, exif_datetime, exif_lat, exif_lon, analyzed_at
         ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)
         ON CONFLICT(evidence_id) DO UPDATE SET
            status = excluded.status,
            reason = excluded.reason,
            ocr_text = excluded.ocr_text,
            has_datetime = excluded.has_datetime,
            has_address = excluded.has_address,
            has_coords = excluded.has_coords,
            exif_datetime = excluded.exif_datetime,
            exif_lat = excluded.exif_lat,
            exif_lon = excluded.exif_lon,
            analyzed_at = excluded.analyzed_at",
    )
    .bind(evidence_id)
    .bind(row_id)
    .bind(&result.status)
    .bind(&result.reason)
    .bind(&result.ocr_text)
    .bind(result.has_datetime as i64)
    .bind(result.has_address as i64)
    .bind(result.has_coords as i64)
    .bind(&result.exif_datetime)
    .bind(result.exif_lat)
    .bind(result.exif_lon)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db)
    .await;
}

fn ranked_rows() -> &'static str {
    "WITH ranked AS (
        SELECT r.id, r.tanggal, r.unit, r.polres, r.polsek, r.satker,
               r.parsed_json, r.bukti_dukung, r.latitude, r.longitude,
               ROW_NUMBER() OVER (
                   PARTITION BY lower(trim(r.dedupe_key))
                   ORDER BY i.created_at DESC, i.id DESC, r.source_row DESC
               ) AS rn
        FROM import_rows r
        JOIN imports i ON i.id = r.import_id
        WHERE r.tanggal BETWEEN ? AND ?
    )"
}

async fn period_rows(
    db: &SqlitePool,
    start: &str,
    end: &str,
) -> ApiResult<Vec<sqlx::sqlite::SqliteRow>> {
    let sql = format!(
        "{} SELECT id, tanggal,
                COALESCE(NULLIF(TRIM(unit), ''), NULLIF(TRIM(polsek), ''),
                         NULLIF(TRIM(polres), ''), TRIM(satker)) AS unit,
                parsed_json, bukti_dukung
         FROM ranked WHERE rn = 1 ORDER BY tanggal ASC, unit ASC, id ASC",
        ranked_rows()
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(start)
        .bind(end)
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)
}

async fn evidence_rows(
    db: &SqlitePool,
    start: &str,
    end: &str,
) -> ApiResult<Vec<sqlx::sqlite::SqliteRow>> {
    let sql = format!(
        "{} SELECT e.row_id, e.content_type, e.status AS evidence_status,
                a.status AS analysis_status, a.reason
         FROM import_evidence e
         JOIN ranked r ON r.id = e.row_id AND r.rn = 1
         LEFT JOIN evidence_analysis a ON a.evidence_id = e.id",
        ranked_rows()
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(start)
        .bind(end)
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)
}

async fn scan_targets(db: &SqlitePool, start: &str, end: &str, force: bool) -> Vec<Target> {
    let filter = if force {
        ""
    } else {
        "AND a.evidence_id IS NULL"
    };
    let sql = format!(
        "{} SELECT e.id, e.import_id, e.row_id, e.stored_path,
                r.tanggal, r.latitude, r.longitude
         FROM import_evidence e
         JOIN ranked r ON r.id = e.row_id AND r.rn = 1
         LEFT JOIN evidence_analysis a ON a.evidence_id = e.id
         WHERE e.status = 'ready' AND e.content_type LIKE 'image/%'
           {filter}
         ORDER BY e.row_id, e.id",
        ranked_rows()
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(start)
        .bind(end)
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| Target {
            evidence_id: row.get("id"),
            import_id: row.get("import_id"),
            row_id: row.get("row_id"),
            stored_path: row.get("stored_path"),
            tanggal: row.get("tanggal"),
            latitude: row.try_get("latitude").ok().flatten(),
            longitude: row.try_get("longitude").ok().flatten(),
        })
        .collect()
}
