//! Penyimpanan berkas bukti dukung berbasis isi.
//!
//! Setiap berkas disimpan satu kali berdasarkan hash SHA-256 isinya sehingga
//! tautan yang sama pada beberapa import tidak lagi menghasilkan salinan baru.
//! Path yang dicatat di database bersifat relatif terhadap `LAPORAN_DATA_DIR`
//! agar data dapat dipindahkan tanpa menulis ulang seluruh baris.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::error::{ApiError, ApiResult};

/// Folder akar bukti dukung relatif terhadap direktori data.
pub const ROOT: &str = "evidence";
/// Folder berkas berbasis isi. Folder lama `evidence/<import_id>` tetap
/// dikenali agar data sebelum perubahan ini masih dapat dibaca.
pub const CONTENT_ROOT: &str = "evidence/content";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentKind {
    pub content_type: &'static str,
    pub extension: &'static str,
}

#[derive(Debug, Default)]
pub struct Released {
    pub files: u64,
    pub bytes: u64,
}

pub fn digest_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Path relatif yang disimpan di tabel `import_evidence`.
pub fn content_relative_path(digest: &str, extension: &str) -> String {
    let prefix = digest.get(..2).unwrap_or(digest);
    format!("{CONTENT_ROOT}/{prefix}/{digest}.{extension}")
}

/// Menulis berkas bila belum ada. Isi yang identik akan menunjuk berkas yang
/// sama, sehingga penulisan kedua dan seterusnya tidak memakan ruang.
pub async fn store_content(
    data_dir: &Path,
    digest: &str,
    extension: &str,
    bytes: &[u8],
) -> std::io::Result<PathBuf> {
    let path = data_dir.join(content_relative_path(digest, extension));
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let temp = path.with_file_name(format!("{digest}.{}.tmp", uuid::Uuid::new_v4()));
    tokio::fs::write(&temp, bytes).await?;
    match tokio::fs::rename(&temp, &path).await {
        Ok(()) => Ok(path),
        // Worker lain mungkin menulis isi yang sama lebih dulu.
        Err(_) if tokio::fs::try_exists(&path).await.unwrap_or(false) => {
            let _ = tokio::fs::remove_file(&temp).await;
            Ok(path)
        }
        Err(error) => {
            let _ = tokio::fs::remove_file(&temp).await;
            Err(error)
        }
    }
}

/// Menemukan berkas dari nilai `stored_path`.
///
/// Baris lama menyimpan path absolut. Bila direktori data pernah dipindah,
/// berkas dicari juga pada folder import asal dengan nama berkas yang sama.
pub async fn resolve_path(data_dir: &Path, stored_path: &str, import_id: &str) -> Option<PathBuf> {
    if stored_path.trim().is_empty() {
        return None;
    }
    let stored = Path::new(stored_path);
    let candidate = if stored.is_absolute() {
        stored.to_path_buf()
    } else {
        data_dir.join(stored)
    };
    if tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
        return Some(candidate);
    }

    let name = stored.file_name()?;
    let fallback = data_dir.join(ROOT).join(import_id).join(name);
    tokio::fs::try_exists(&fallback)
        .await
        .ok()
        .filter(|exists| *exists)
        .map(|_| fallback)
}

/// Menghapus berkas yang tidak lagi dirujuk baris mana pun. Dipakai saat import
/// dihapus dan saat bukti dukung baris tersisih dibersihkan.
pub async fn release_unreferenced(
    db: &SqlitePool,
    data_dir: &Path,
    paths: &[String],
) -> ApiResult<Released> {
    let mut seen = std::collections::HashSet::new();
    let mut released = Released::default();
    for stored_path in paths {
        if stored_path.trim().is_empty() || !seen.insert(stored_path.clone()) {
            continue;
        }
        let referenced: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM import_evidence WHERE stored_path = ?")
                .bind(stored_path)
                .fetch_one(db)
                .await
                .map_err(ApiError::internal)?;
        if referenced > 0 {
            continue;
        }

        let path = if Path::new(stored_path).is_absolute() {
            PathBuf::from(stored_path)
        } else {
            data_dir.join(stored_path)
        };
        let size = tokio::fs::metadata(&path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {
                released.files += 1;
                released.bytes += size;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "gagal menghapus berkas bukti dukung");
            }
        }
    }
    Ok(released)
}

/// Menentukan jenis berkas dari beberapa byte awal. Banyak server membalas
/// `application/octet-stream` untuk PDF/gambar, sehingga isi berkas lebih
/// dipercaya daripada header HTTP.
pub fn sniff_content(bytes: &[u8]) -> Option<ContentKind> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(ContentKind {
            content_type: "image/jpeg",
            extension: "jpg",
        });
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(ContentKind {
            content_type: "image/png",
            extension: "png",
        });
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(ContentKind {
            content_type: "image/gif",
            extension: "gif",
        });
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some(ContentKind {
            content_type: "image/webp",
            extension: "webp",
        });
    }
    if bytes.starts_with(b"%PDF") {
        return Some(ContentKind {
            content_type: "application/pdf",
            extension: "pdf",
        });
    }
    None
}

pub fn extension_for_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "application/pdf" => Some("pdf"),
        _ => None,
    }
}

pub fn mime_for_extension(extension: &str) -> Option<&'static str> {
    match extension {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "pdf" => Some("application/pdf"),
        _ => None,
    }
}

pub fn extension_from_name(value: &str) -> Option<&'static str> {
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

    #[test]
    fn recognizes_supported_content_from_magic_bytes() {
        assert_eq!(
            sniff_content(&[0xFF, 0xD8, 0xFF, 0xE0]).map(|kind| kind.extension),
            Some("jpg")
        );
        assert_eq!(
            sniff_content(b"\x89PNG\r\n\x1a\n0000").map(|kind| kind.extension),
            Some("png")
        );
        assert_eq!(
            sniff_content(b"%PDF-1.7").map(|kind| kind.content_type),
            Some("application/pdf")
        );
        assert!(sniff_content(b"<html><body>").is_none());
    }

    #[test]
    fn builds_content_addressed_relative_paths() {
        let digest = "ab".to_string() + &"c".repeat(62);
        assert_eq!(
            content_relative_path(&digest, "pdf"),
            format!("evidence/content/ab/{digest}.pdf")
        );
    }

    #[test]
    fn maps_between_mime_and_extension() {
        assert_eq!(mime_for_extension("jpg"), Some("image/jpeg"));
        assert_eq!(extension_for_mime("application/pdf"), Some("pdf"));
        assert_eq!(extension_from_name("Foto.JPEG"), Some("jpg"));
    }

    #[tokio::test]
    async fn stores_identical_content_once() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"%PDF-1.7 isi sama";
        let digest = digest_hex(bytes);
        let first = store_content(dir.path(), &digest, "pdf", bytes)
            .await
            .unwrap();
        let second = store_content(dir.path(), &digest, "pdf", bytes)
            .await
            .unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with(dir.path().join(CONTENT_ROOT)));
        assert!(tokio::fs::try_exists(&first).await.unwrap());
    }
}
