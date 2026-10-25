//! Penemuan berkas pada folder Google Drive publik.
//!
//! Hanya folder publik yang dapat dibaca dengan API key. Pencarian dibatasi
//! pada folder akar dan satu tingkat subfolder; subfolder yang lebih dalam
//! tidak ditelusuri.

use std::path::Path;

use reqwest::{Client, Url};
use serde::Deserialize;

const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const MAX_FOLDER_ENTRIES: usize = 500;
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";

#[derive(Clone, Debug)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size: i64,
}

impl DriveFile {
    pub fn is_folder(&self) -> bool {
        self.mime_type == FOLDER_MIME
    }
}

pub struct FolderListing {
    pub files: Vec<DriveFile>,
    pub truncated: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveListResponse {
    #[serde(default)]
    files: Vec<DriveListFile>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveListFile {
    id: String,
    name: String,
    #[serde(default)]
    mime_type: String,
    #[serde(default)]
    size: Option<String>,
}

/// Mengambil ID folder dari tautan `drive.google.com/drive/folders/<id>`.
pub fn folder_id(raw_url: &str) -> Option<String> {
    let url = Url::parse(raw_url).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if host != "drive.google.com" {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.collect();
    let index = segments.iter().position(|segment| *segment == "folders")?;
    let id = segments.get(index + 1)?.trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

/// Menemukan berkas di folder akar dan satu tingkat subfolder.
pub async fn discover(
    client: &Client,
    api_key: &str,
    root_id: &str,
) -> Result<FolderListing, String> {
    let (root_children, mut truncated) =
        list_children(client, api_key, root_id, MAX_FOLDER_ENTRIES).await?;

    let mut files = Vec::new();
    let mut subfolders = Vec::new();
    for child in root_children {
        if child.is_folder() {
            subfolders.push(child);
        } else {
            files.push(child);
        }
    }

    for subfolder in subfolders {
        if files.len() >= MAX_FOLDER_ENTRIES {
            truncated = true;
            break;
        }
        let remaining = MAX_FOLDER_ENTRIES - files.len();
        let (children, sub_truncated) =
            match list_children(client, api_key, &subfolder.id, remaining).await {
                Ok(result) => result,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        folder = %subfolder.name,
                        "subfolder Google Drive dilewati"
                    );
                    truncated = true;
                    continue;
                }
            };
        truncated |= sub_truncated;
        for child in children {
            // Folder di dalam subfolder tidak ditelusuri (hanya satu tingkat).
            if child.is_folder() {
                continue;
            }
            if files.len() >= MAX_FOLDER_ENTRIES {
                truncated = true;
                break;
            }
            files.push(child);
        }
    }

    Ok(FolderListing { files, truncated })
}

async fn list_children(
    client: &Client,
    api_key: &str,
    folder_id: &str,
    limit: usize,
) -> Result<(Vec<DriveFile>, bool), String> {
    let mut files = Vec::new();
    let mut page_token: Option<String> = None;
    let query = format!("'{folder_id}' in parents and trashed = false");

    loop {
        let mut request = client.get(DRIVE_FILES_URL).query(&[
            ("q", query.as_str()),
            ("fields", "nextPageToken,files(id,name,mimeType,size)"),
            ("pageSize", "1000"),
            ("orderBy", "name"),
            ("supportsAllDrives", "true"),
            ("includeItemsFromAllDrives", "true"),
            ("key", api_key),
        ]);
        if let Some(token) = page_token.as_deref() {
            request = request.query(&[("pageToken", token)]);
        }

        let response = request.send().await.map_err(|error| {
            tracing::warn!(
                error = %error.without_url(),
                "gagal menghubungi Google Drive API"
            );
            "Gagal menghubungi Google Drive API.".to_string()
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(drive_error(status.as_u16()));
        }

        let body = response.text().await.map_err(|error| {
            tracing::warn!(
                error = %error.without_url(),
                "gagal membaca respons Google Drive API"
            );
            "Respons Google Drive API tidak dapat dibaca.".to_string()
        })?;
        let page: DriveListResponse = serde_json::from_str(&body)
            .map_err(|error| format!("Respons Google Drive API tidak valid: {error}"))?;

        for file in page.files {
            if files.len() >= limit {
                return Ok((files, true));
            }
            files.push(DriveFile {
                id: file.id,
                name: file.name,
                mime_type: file.mime_type,
                size: file
                    .size
                    .and_then(|value| value.parse::<i64>().ok())
                    .unwrap_or(0),
            });
        }

        page_token = page.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok((files, false))
}

fn drive_error(status: u16) -> String {
    match status {
        401 | 403 => {
            "Akses Google Drive ditolak. Pastikan folder publik dan API key diizinkan untuk Google Drive API."
                .to_string()
        }
        400 => "Permintaan Google Drive ditolak. Periksa API key dan ID folder.".to_string(),
        404 => "Folder Google Drive tidak ditemukan atau tidak dapat diakses publik.".to_string(),
        429 => "Kuota Google Drive API sedang tercapai. Coba lagi beberapa saat lagi.".to_string(),
        _ => format!("Google Drive API membalas HTTP {status}."),
    }
}

/// Ekstensi yang didukung. Google Drive kadang mengirim MIME generik, jadi
/// ekstensi nama berkas dipakai sebagai cadangan.
pub fn supported_extension(file: &DriveFile) -> Option<&'static str> {
    match file.mime_type.as_str() {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "application/pdf" => Some("pdf"),
        _ => extension_of(&file.name),
    }
}

pub fn content_type_for(file: &DriveFile, extension: &str) -> String {
    if !file.mime_type.is_empty() && file.mime_type != "application/octet-stream" {
        return file.mime_type.clone();
    }
    match extension {
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
    .to_string()
}

pub fn file_url(id: &str) -> String {
    format!("https://drive.google.com/file/d/{id}/view")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, mime_type: &str) -> DriveFile {
        DriveFile {
            id: "id".to_string(),
            name: name.to_string(),
            mime_type: mime_type.to_string(),
            size: 0,
        }
    }

    #[test]
    fn extracts_folder_id_from_public_links() {
        assert_eq!(
            folder_id("https://drive.google.com/drive/folders/abc123?usp=sharing"),
            Some("abc123".to_string())
        );
        assert_eq!(
            folder_id("https://drive.google.com/drive/u/0/folders/xyz789"),
            Some("xyz789".to_string())
        );
        assert_eq!(
            folder_id("https://drive.google.com/file/d/abc123/view"),
            None
        );
        assert_eq!(folder_id("https://example.com/drive/folders/abc"), None);
    }

    #[test]
    fn only_supports_images_and_pdf() {
        assert_eq!(
            supported_extension(&file("foto.JPG", "image/jpeg")),
            Some("jpg")
        );
        assert_eq!(
            supported_extension(&file("dokumen.pdf", "application/pdf")),
            Some("pdf")
        );
        assert_eq!(
            supported_extension(&file("foto.png", "application/octet-stream")),
            Some("png")
        );
        assert_eq!(
            supported_extension(&file(
                "catatan.docx",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            )),
            None
        );
        assert_eq!(supported_extension(&file("folder", FOLDER_MIME)), None);
    }

    #[test]
    fn builds_public_file_links() {
        assert_eq!(
            file_url("abc123"),
            "https://drive.google.com/file/d/abc123/view"
        );
    }

    #[test]
    fn parses_drive_listing_payload() {
        let page: DriveListResponse = serde_json::from_str(
            r#"{"nextPageToken":"next","files":[{"id":"1","name":"foto.jpg","mimeType":"image/jpeg","size":"123"}]}"#,
        )
        .unwrap();
        assert_eq!(page.next_page_token.as_deref(), Some("next"));
        assert_eq!(page.files.len(), 1);
        assert_eq!(page.files[0].name, "foto.jpg");
        assert_eq!(page.files[0].mime_type, "image/jpeg");
        assert_eq!(page.files[0].size.as_deref(), Some("123"));
    }
}
