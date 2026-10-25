//! Unduhan bukti dukung dari tautan pada workbook.
//!
//! Unduhan berjalan di latar belakang setelah impor disimpan. Hanya tautan
//! yang benar-benar menunjuk berkas yang dicoba: gambar/PDF langsung, berkas
//! Google Drive, serta ekspor Google Docs. Folder Drive dan tautan media
//! sosial ditandai `skipped` agar tidak membanjiri jaringan dengan permintaan
//! yang tidak akan pernah menghasilkan berkas.

use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

use reqwest::{
    Client, Url,
    header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
};

const MAX_EVIDENCE_BYTES: usize = 8 * 1024 * 1024;
const MAX_CONCURRENT_DOWNLOADS: usize = 4;
const REQUEST_TIMEOUT_SECS: u64 = 25;

pub fn spawn_import_downloads(db: SqlitePool, config: Arc<Config>, import_id: String) {
    tokio::spawn(async move {
        if let Err(error) = process_import(&db, &config, &import_id).await {
            tracing::warn!(%error, %import_id, "unduhan bukti dukung gagal diproses");
        }
    });
}

pub async fn retry_import(
    db: &SqlitePool,
    config: Arc<Config>,
    import_id: &str,
) -> ApiResult<u64> {
    ensure_evidence_rows(db, import_id).await?;

    sqlx::query(
        "UPDATE import_evidence
         SET status = 'pending', error = ''
         WHERE import_id = ? AND status IN ('failed', 'skipped')",
    )
    .bind(import_id)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    let queued: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM import_evidence WHERE import_id = ? AND status = 'pending'",
    )
    .bind(import_id)
    .fetch_one(db)
    .await
    .map_err(ApiError::internal)?;

    if queued > 0 {
        spawn_import_downloads(db.clone(), config, import_id.to_string());
    }
    Ok(queued.max(0) as u64)
}

pub async fn open(db: &SqlitePool, evidence_id: &str) -> ApiResult<(Vec<u8>, String, String)> {
    let row = sqlx::query(
        "SELECT stored_path, filename, content_type, status
         FROM import_evidence WHERE id = ?",
    )
    .bind(evidence_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Bukti dukung tidak ditemukan."))?;

    let status: String = row.get("status");
    if status != "ready" {
        return Err(ApiError::not_found(
            "Bukti dukung belum berhasil diunduh. Coba unduh ulang dari halaman import.",
        ));
    }
    let stored_path: String = row.get("stored_path");
    if stored_path.is_empty() {
        return Err(ApiError::not_found("Berkas bukti dukung tidak tersedia."));
    }
    let bytes = tokio::fs::read(&stored_path)
        .await
        .map_err(|_| ApiError::not_found("Berkas bukti dukung tidak ditemukan di penyimpanan."))?;

    let filename = row.get::<String, _>("filename");
    let content_type = row.get::<String, _>("content_type");
    Ok((bytes, content_type, filename))
}

/// Impor lama (sebelum pemisahan modul) belum memiliki baris bukti dukung.
/// Saat pengguna meminta unduh ulang, tautan dari `parsed_json`/`bukti_dukung`
/// disalin dulu agar unduhan dapat dijadwalkan.
async fn ensure_evidence_rows(db: &SqlitePool, import_id: &str) -> ApiResult<()> {
    let rows =
        sqlx::query("SELECT id, parsed_json, bukti_dukung FROM import_rows WHERE import_id = ?")
            .bind(import_id)
            .fetch_all(db)
            .await
            .map_err(ApiError::internal)?;

    let mut tx = db.begin().await.map_err(ApiError::internal)?;
    for row in rows {
        let row_id: i64 = row.get("id");
        let parsed_json: String = row.get("parsed_json");
        let bukti_dukung: String = row.get("bukti_dukung");
        let mut links = serde_json::from_str::<serde_json::Value>(&parsed_json)
            .ok()
            .and_then(|value| {
                value
                    .get("links")
                    .and_then(|links| links.as_array())
                    .map(|links| {
                        links
                            .iter()
                            .filter_map(|link| link.as_str())
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default();
        if links.is_empty() {
            links = crate::shared::extract_links(&bukti_dukung);
        }
        links.sort();
        links.dedup();
        for link in links {
            if !(link.starts_with("http://") || link.starts_with("https://")) {
                continue;
            }
            sqlx::query(
                "INSERT INTO import_evidence(id, import_id, row_id, source_url)
                 SELECT ?, ?, ?, ?
                 WHERE NOT EXISTS (
                     SELECT 1 FROM import_evidence WHERE row_id = ? AND source_url = ?
                 )",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(import_id)
            .bind(row_id)
            .bind(&link)
            .bind(row_id)
            .bind(&link)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::internal)?;
        }
    }
    tx.commit().await.map_err(ApiError::internal)?;
    Ok(())
}

async fn process_import(db: &SqlitePool, config: &Config, import_id: &str) -> ApiResult<()> {
    let rows = sqlx::query(
        "SELECT id, source_url FROM import_evidence
         WHERE import_id = ? AND status = 'pending'
         ORDER BY source_url ASC, created_at ASC",
    )
    .bind(import_id)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;
    if rows.is_empty() {
        return Ok(());
    }

    let mut groups = HashMap::<String, (Url, Vec<String>)>::new();
    let mut skipped = HashMap::<String, Vec<String>>::new();
    for row in rows {
        let id: String = row.get("id");
        let source_url: String = row.get("source_url");
        match normalize_target(&source_url) {
            Ok(target) => {
                groups
                    .entry(target.as_str().to_string())
                    .or_insert_with(|| (target, Vec::new()))
                    .1
                    .push(id);
            }
            Err(reason) => skipped.entry(reason).or_default().push(id),
        }
    }

    for (reason, ids) in skipped {
        mark_many(db, &ids, "skipped", &reason).await;
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent("LaporanMingguanPBM/1.0 (bukti dukung)")
        .build()
        .map_err(ApiError::internal)?;

    let mut pending = groups.into_values().collect::<Vec<_>>().into_iter();
    let mut tasks = JoinSet::new();

    for _ in 0..MAX_CONCURRENT_DOWNLOADS {
        let Some((target, ids)) = pending.next() else {
            break;
        };
        tasks.spawn(download_group(
            db.clone(),
            config.data_dir.clone(),
            import_id.to_string(),
            ids,
            target,
            client.clone(),
        ));
    }

    while let Some(result) = tasks.join_next().await {
        if let Err(error) = result {
            tracing::warn!(%error, "tugas unduhan bukti dukung berhenti");
        }
        let Some((target, ids)) = pending.next() else {
            continue;
        };
        tasks.spawn(download_group(
            db.clone(),
            config.data_dir.clone(),
            import_id.to_string(),
            ids,
            target,
            client.clone(),
        ));
    }
    Ok(())
}

async fn download_group(
    db: SqlitePool,
    data_dir: std::path::PathBuf,
    import_id: String,
    evidence_ids: Vec<String>,
    target: Url,
    client: Client,
) {
    match fetch_evidence(&client, &target).await {
        Ok(fetched) => {
            let directory = data_dir.join("evidence").join(&import_id);
            if let Err(error) = tokio::fs::create_dir_all(&directory).await {
                mark_many(
                    &db,
                    &evidence_ids,
                    "failed",
                    &format!("Gagal menyiapkan folder: {error}"),
                )
                .await;
                return;
            }
            let digest = hex::encode(Sha256::digest(target.as_str()));
            let stored_path = directory.join(format!("{}.{}", &digest[..16], fetched.extension));
            if let Err(error) = tokio::fs::write(&stored_path, &fetched.bytes).await {
                mark_many(
                    &db,
                    &evidence_ids,
                    "failed",
                    &format!("Gagal menyimpan berkas: {error}"),
                )
                .await;
                return;
            }
            for evidence_id in evidence_ids {
                let result = sqlx::query(
                    "UPDATE import_evidence
                     SET status = 'ready', filename = ?, content_type = ?, size_bytes = ?,
                         stored_path = ?, error = ''
                     WHERE id = ?",
                )
                .bind(&fetched.filename)
                .bind(&fetched.content_type)
                .bind(fetched.bytes.len() as i64)
                .bind(stored_path.to_string_lossy().to_string())
                .bind(&evidence_id)
                .execute(&db)
                .await;
                if let Err(error) = result {
                    tracing::warn!(%error, %evidence_id, "gagal mencatat bukti dukung");
                }
            }
        }
        Err(message) => mark_many(&db, &evidence_ids, "failed", &message).await,
    }
}

struct FetchedEvidence {
    bytes: Vec<u8>,
    filename: String,
    content_type: String,
    extension: &'static str,
}

async fn fetch_evidence(client: &Client, target: &Url) -> Result<FetchedEvidence, String> {
    let response = client
        .get(target.clone())
        .send()
        .await
        .map_err(|error| format!("Gagal menghubungi tautan: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Tautan membalas HTTP {status}."));
    }
    if let Some(length) = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        && length > MAX_EVIDENCE_BYTES
    {
        return Err(format!(
            "Ukuran berkas melebihi batas {} MB.",
            MAX_EVIDENCE_BYTES / 1024 / 1024
        ));
    }

    let raw_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime = raw_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    let hinted_filename = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .and_then(filename_from_headers);
    let extension = match extension_for(&mime) {
        Some(extension) => Some(extension),
        None if mime == "application/octet-stream" => hinted_filename
            .as_deref()
            .and_then(extension_of)
            .or_else(|| extension_of(target.path())),
        None => None,
    };
    let Some(extension) = extension else {
        return Err(if mime.is_empty() {
            "Tautan tidak mengirim jenis berkas yang dikenali.".to_string()
        } else {
            format!("Jenis berkas {mime} tidak didukung (hanya gambar/PDF).")
        });
    };

    let filename = hinted_filename.unwrap_or_else(|| {
        target
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(sanitize_filename)
            .filter(|name| Path::new(name).extension().is_some())
            .unwrap_or_else(|| format!("bukti-{}.{extension}", &Uuid::new_v4().to_string()[..8]))
    });
    let mut bytes = Vec::new();
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Gagal mengunduh isi berkas: {error}"))?
    {
        if bytes.len() + chunk.len() > MAX_EVIDENCE_BYTES {
            return Err(format!(
                "Ukuran berkas melebihi batas {} MB.",
                MAX_EVIDENCE_BYTES / 1024 / 1024
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err("Tautan mengirim berkas kosong.".to_string());
    }

    Ok(FetchedEvidence {
        bytes,
        filename,
        content_type: mime,
        extension,
    })
}

/// Menyaring tautan yang tidak akan menghasilkan berkas dan mengubah tautan
/// Google Drive/Docs menjadi URL unduhan langsung.
fn normalize_target(raw_url: &str) -> Result<Url, String> {
    let url = Url::parse(raw_url).map_err(|_| "Tautan tidak valid.".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Hanya tautan http/https yang dapat diunduh.".to_string());
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    if is_blocked_host(&host) {
        return Err("Tautan mengarah ke alamat lokal dan tidak diunduh.".to_string());
    }

    match host.as_str() {
        "drive.google.com" => {
            if url.path().contains("/folders/") {
                return Err("Folder Google Drive tidak dapat diunduh sebagai satu berkas.".to_string());
            }
            let id = drive_file_id(&url)
                .ok_or_else(|| "Tautan Google Drive tidak memuat ID berkas.".to_string())?;
            Url::parse(&format!(
                "https://drive.google.com/uc?export=download&id={id}"
            ))
            .map_err(|_| "Tautan Google Drive tidak valid.".to_string())
        }
        "docs.google.com" => export_google_doc(&url),
        _ if is_unsupported_host(&host, url.path()) => {
            Err("Tautan media sosial/peta tidak menyediakan berkas langsung.".to_string())
        }
        _ => Ok(url),
    }
}

fn drive_file_id(url: &Url) -> Option<String> {
    let segments: Vec<&str> = url.path_segments()?.collect();
    if let Some(index) = segments.iter().position(|segment| *segment == "d") {
        if let Some(id) = segments.get(index + 1) {
            if !id.is_empty() {
                return Some((*id).to_string());
            }
        }
    }
    url.query_pairs()
        .find(|(key, _)| key == "id")
        .map(|(_, value)| value.to_string())
}

fn export_google_doc(url: &Url) -> Result<Url, String> {
    let segments: Vec<&str> = url
        .path_segments()
        .ok_or_else(|| "Tautan Google Docs tidak valid.".to_string())?
        .collect();
    let kind = segments.first().copied().unwrap_or_default();
    let id = segments
        .iter()
        .position(|segment| *segment == "d")
        .and_then(|index| segments.get(index + 1))
        .copied()
        .filter(|id| !id.is_empty());
    let Some(id) = id else {
        return Err("Tautan Google Docs tidak memuat ID dokumen.".to_string());
    };
    let path = match kind {
        "document" => format!("document/d/{id}/export?format=pdf"),
        "spreadsheets" => format!("spreadsheets/d/{id}/export?format=pdf"),
        "presentation" => format!("presentation/d/{id}/export?format=pdf"),
        _ => return Err("Jenis Google Docs ini tidak dapat diunduh otomatis.".to_string()),
    };
    Url::parse(&format!("https://docs.google.com/{path}"))
        .map_err(|_| "Tautan Google Docs tidak valid.".to_string())
}

fn is_unsupported_host(host: &str, path: &str) -> bool {
    matches!(
        host,
        "instagram.com"
            | "facebook.com"
            | "fb.com"
            | "web.facebook.com"
            | "tiktok.com"
            | "youtube.com"
            | "youtu.be"
            | "twitter.com"
            | "x.com"
            | "maps.app.goo.gl"
            | "maps.google.com"
            | "goo.gl"
            | "share.google"
    ) || (host.ends_with("google.com") && path.starts_with("/maps"))
}

fn extension_for(mime: &str) -> Option<&'static str> {
    match mime {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "application/pdf" => Some("pdf"),
        _ => None,
    }
}

fn extension_of(value: &str) -> Option<&'static str> {
    let extension = Path::new(value)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "jpg" | "jpeg" => Some("jpg"),
        "png" => Some("png"),
        "webp" => Some("webp"),
        "gif" => Some("gif"),
        "pdf" => Some("pdf"),
        _ => None,
    }
}

fn filename_from_headers(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    if let Some(index) = lower.find("filename*=") {
        let rest = &value[index + "filename*=".len()..];
        let encoded = rest.trim_matches('"').split("''").nth(1).unwrap_or(rest);
        if let Some(name) = sanitize_filename(&percent_decode(encoded)) {
            return Some(name);
        }
    }
    let index = lower.find("filename=")?;
    let rest = &value[index + "filename=".len()..];
    let name = rest.trim_matches('"').split(';').next().unwrap_or(rest);
    sanitize_filename(name)
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = &value[index + 1..index + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                output.push(byte);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).to_string()
}

fn sanitize_filename(value: &str) -> Option<String> {
    let name = Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .trim()
        .replace(['/', '\\', '"', '\n', '\r'], "-");
    if name.is_empty() || name == "." || name == ".." {
        None
    } else {
        Some(name.chars().take(120).collect())
    }
}

fn is_blocked_host(host: &str) -> bool {
    if host.is_empty() || host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    if host.starts_with("127.") || host.starts_with("10.") || host.starts_with("192.168.") {
        return true;
    }
    if let Some(rest) = host.strip_prefix("172.")
        && let Some(second) = rest.split('.').next()
        && let Ok(octet) = second.parse::<u8>()
    {
        return (16..=31).contains(&octet);
    }
    host == "[::1]" || host == "::1" || host.starts_with("169.254.")
}

async fn mark_many(db: &SqlitePool, evidence_ids: &[String], status: &str, message: &str) {
    let message: String = message.chars().take(300).collect();
    for evidence_id in evidence_ids {
        if let Err(error) = sqlx::query(
            "UPDATE import_evidence SET status = ?, error = ? WHERE id = ?",
        )
        .bind(status)
        .bind(&message)
        .bind(evidence_id)
        .execute(db)
        .await
        {
            tracing::warn!(%error, %evidence_id, "gagal menandai bukti dukung");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_google_drive_file_links() {
        let target = normalize_target(
            "https://drive.google.com/file/d/1gdQhzNIQaeZh52xMN343z_sJQ80nFDgz/view?usp=drive_link",
        )
        .unwrap();
        assert_eq!(
            target.as_str(),
            "https://drive.google.com/uc?export=download&id=1gdQhzNIQaeZh52xMN343z_sJQ80nFDgz"
        );
    }

    #[test]
    fn skips_drive_folders_and_social_links() {
        assert!(normalize_target("https://drive.google.com/drive/folders/abc?usp=sharing").is_err());
        assert!(normalize_target("https://www.instagram.com/reel/abc/").is_err());
        assert!(normalize_target("https://maps.app.goo.gl/abc").is_err());
    }

    #[test]
    fn exports_google_docs_as_pdf() {
        let target = normalize_target("https://docs.google.com/document/d/abc/edit?usp=sharing").unwrap();
        assert_eq!(
            target.as_str(),
            "https://docs.google.com/document/d/abc/export?format=pdf"
        );
    }

    #[test]
    fn allows_direct_files_and_blocks_local_hosts() {
        assert!(normalize_target("https://contoh.go.id/bukti/foto.jpg").is_ok());
        assert!(normalize_target("http://127.0.0.1/bukti.pdf").is_err());
        assert!(normalize_target("file:///C:/bukti.pdf").is_err());
    }
}
