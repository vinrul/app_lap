//! Perawatan penyimpanan bukti dukung.
//!
//! Perintah ini memindahkan berkas lama ke penyimpanan berbasis isi,
//! memperbaiki jenis berkas dari magic bytes (mis. PDF yang tercatat sebagai
//! `application/octet-stream`), lalu membersihkan bukti dukung milik baris yang
//! kalah deduplikasi. Aman dijalankan berulang; berkas yang sudah rapi
//! dilewati.

use std::path::Path;

use sqlx::{Row, SqlitePool};

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    modules::imports::{evidence, evidence_store},
};

#[derive(Debug, Default, Clone)]
pub struct MaintenanceReport {
    pub dry_run: bool,
    pub files_scanned: u64,
    pub files_moved: u64,
    pub files_already_shared: u64,
    pub rows_updated: u64,
    pub files_reclassified: u64,
    pub purged_rows: u64,
    pub deleted_files: u64,
    pub bytes_freed: u64,
    pub failed_files: u64,
}

impl std::fmt::Display for MaintenanceReport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            formatter,
            "Perawatan bukti dukung{}",
            if self.dry_run { " (dry run)" } else { "" }
        )?;
        writeln!(
            formatter,
            "  Berkas dipindai       : {}",
            self.files_scanned
        )?;
        writeln!(formatter, "  Berkas dipindahkan    : {}", self.files_moved)?;
        writeln!(
            formatter,
            "  Berkas sudah bersama  : {}",
            self.files_already_shared
        )?;
        writeln!(formatter, "  Baris diperbarui      : {}", self.rows_updated)?;
        writeln!(
            formatter,
            "  Jenis berkas diperbaiki: {}",
            self.files_reclassified
        )?;
        writeln!(
            formatter,
            "  Baris tersisih dibersihkan: {}",
            self.purged_rows
        )?;
        writeln!(
            formatter,
            "  Berkas dihapus        : {}",
            self.deleted_files
        )?;
        writeln!(
            formatter,
            "  Ruang dibebaskan      : {:.1} MB",
            self.bytes_freed as f64 / 1024.0 / 1024.0
        )?;
        write!(formatter, "  Berkas gagal dibaca   : {}", self.failed_files)
    }
}

pub async fn run(db: &SqlitePool, config: &Config, dry_run: bool) -> ApiResult<MaintenanceReport> {
    let mut report = MaintenanceReport {
        dry_run,
        ..MaintenanceReport::default()
    };

    let rows = sqlx::query(
        "SELECT stored_path, MIN(import_id) AS import_id, MIN(content_type) AS content_type,
                COUNT(*) AS row_count
         FROM import_evidence
         WHERE status = 'ready' AND stored_path <> ''
         GROUP BY stored_path",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    let root = config.data_dir.join(evidence_store::ROOT);
    for (index, row) in rows.into_iter().enumerate() {
        let stored_path: String = row.get("stored_path");
        let import_id: String = row.get("import_id");
        let stored_type: String = row.get("content_type");
        let row_count: i64 = row.get("row_count");

        if index > 0 && index % 500 == 0 {
            tracing::info!(
                files = index,
                moved = report.files_moved,
                "perawatan bukti dukung berjalan"
            );
        }

        let Some(path) =
            evidence_store::resolve_path(&config.data_dir, &stored_path, &import_id).await
        else {
            report.failed_files += 1;
            tracing::warn!(%stored_path, %import_id, "berkas bukti dukung tidak ditemukan");
            continue;
        };
        let bytes = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(error) => {
                report.failed_files += 1;
                tracing::warn!(%error, path = %path.display(), "gagal membaca berkas bukti dukung");
                continue;
            }
        };
        report.files_scanned += 1;

        let sniffed = evidence_store::sniff_content(&bytes);
        let content_type = sniffed
            .map(|kind| kind.content_type.to_string())
            .unwrap_or_else(|| stored_type.clone());
        let extension = sniffed
            .map(|kind| kind.extension)
            .or_else(|| evidence_store::extension_from_name(&stored_path))
            .or_else(|| evidence_store::extension_from_name(&path.to_string_lossy()))
            .unwrap_or("bin");
        let digest = evidence_store::digest_hex(&bytes);
        let relative = evidence_store::content_relative_path(&digest, extension);
        let target = config.data_dir.join(&relative);
        let same_path = path == target;

        if same_path {
            report.files_already_shared += 1;
        } else {
            report.files_moved += 1;
        }
        if sniffed.is_some() && content_type != stored_type {
            report.files_reclassified += 1;
        }

        if dry_run {
            report.rows_updated += row_count as u64;
            continue;
        }

        if same_path && content_type == stored_type {
            continue;
        }

        if !same_path {
            if let Err(error) =
                evidence_store::store_content(&config.data_dir, &digest, extension, &bytes).await
            {
                report.failed_files += 1;
                tracing::warn!(%error, %stored_path, "gagal memindahkan berkas bukti dukung");
                continue;
            }
        }

        let updated = sqlx::query(
            "UPDATE import_evidence SET stored_path = ?, content_type = ? WHERE stored_path = ?",
        )
        .bind(&relative)
        .bind(&content_type)
        .bind(&stored_path)
        .execute(db)
        .await
        .map_err(ApiError::internal)?;
        report.rows_updated += updated.rows_affected();

        // Berkas lama dihapus setelah semua baris menunjuk berkas baru.
        if !same_path {
            let size = tokio::fs::metadata(&path)
                .await
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            if tokio::fs::remove_file(&path).await.is_ok() {
                report.deleted_files += 1;
                report.bytes_freed += size;
            }
        }
    }

    let purge = evidence::purge_superseded(
        db,
        &config.data_dir,
        None,
        config.evidence_retention_days,
        dry_run,
    )
    .await?;
    report.purged_rows = purge.rows;
    report.deleted_files += purge.files;
    report.bytes_freed += purge.bytes;

    if !dry_run {
        remove_empty_legacy_dirs(&root).await;
    }

    let (orphan_files, orphan_bytes) = cleanup_orphaned_legacy_files(db, config, dry_run).await?;
    report.deleted_files += orphan_files;
    report.bytes_freed += orphan_bytes;

    Ok(report)
}

/// Folder `evidence/<import_id>` dari penyimpanan lama dibiarkan kosong setelah
/// berkasnya dipindahkan. Folder `content` tidak disentuh karena berisi berkas.
async fn remove_empty_legacy_dirs(root: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(root).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.file_name().to_string_lossy() == "content" {
            continue;
        }
        if entry
            .file_type()
            .await
            .map(|kind| kind.is_dir())
            .unwrap_or(false)
        {
            let _ = tokio::fs::remove_dir(entry.path()).await;
        }
    }
}

/// Menghapus berkas di folder import lama yang sudah tidak dirujuk baris mana
/// pun. Ini menangani salinan yang tertinggal ketika direktori data pernah
/// dipindah atau disalin.
async fn cleanup_orphaned_legacy_files(
    db: &SqlitePool,
    config: &Config,
    dry_run: bool,
) -> ApiResult<(u64, u64)> {
    let stored: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT stored_path FROM import_evidence WHERE stored_path <> ''",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;
    let stored: std::collections::HashSet<String> = stored
        .into_iter()
        .map(|path| path.replace('\\', "/").to_ascii_lowercase())
        .collect();

    let root = config.data_dir.join(evidence_store::ROOT);
    let Ok(mut entries) = tokio::fs::read_dir(&root).await else {
        return Ok((0, 0));
    };

    let mut files = 0_u64;
    let mut bytes = 0_u64;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let folder = entry.file_name().to_string_lossy().to_string();
        if folder == "content"
            || !entry
                .file_type()
                .await
                .map(|kind| kind.is_dir())
                .unwrap_or(false)
        {
            continue;
        }

        let Ok(mut children) = tokio::fs::read_dir(entry.path()).await else {
            continue;
        };
        while let Ok(Some(child)) = children.next_entry().await {
            if !child
                .file_type()
                .await
                .map(|kind| kind.is_file())
                .unwrap_or(false)
            {
                continue;
            }
            let name = child.file_name().to_string_lossy().to_string();
            let relative = format!("{}/{folder}/{name}", evidence_store::ROOT)
                .replace('\\', "/")
                .to_ascii_lowercase();
            let absolute = child
                .path()
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            if stored.contains(&relative) || stored.contains(&absolute) {
                continue;
            }

            let size = child
                .metadata()
                .await
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            files += 1;
            bytes += size;
            if !dry_run {
                let _ = tokio::fs::remove_file(child.path()).await;
            }
        }
        if !dry_run {
            let _ = tokio::fs::remove_dir(entry.path()).await;
        }
    }
    Ok((files, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrates_legacy_files_and_removes_orphans() {
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
            "INSERT INTO imports(
                id,filename,file_hash,stored_path,parser_version,row_count,skipped_rows,
                duplicate_rows,date_start,date_end,warnings_json,imported_by
             ) VALUES('imp1','x.xlsx','hash','path','4',1,0,0,'2026-10-25','2026-10-25','[]','u1')",
        )
        .execute(&db)
        .await
        .unwrap();
        let row_id = sqlx::query(
            "INSERT INTO import_rows(import_id,source_row,tanggal,dedupe_key,raw_json)
             VALUES('imp1',2,'2026-10-25','uji-1','{}')",
        )
        .execute(&db)
        .await
        .unwrap()
        .last_insert_rowid();

        let legacy_dir = config.data_dir.join("evidence/imp1");
        tokio::fs::create_dir_all(&legacy_dir).await.unwrap();
        let png = b"\x89PNG\r\n\x1a\nisi";
        let legacy = legacy_dir.join("lama.jpg");
        let orphan = legacy_dir.join("yatim.jpg");
        tokio::fs::write(&legacy, png).await.unwrap();
        tokio::fs::write(&orphan, b"tidak dirujuk").await.unwrap();

        sqlx::query(
            "INSERT INTO import_evidence(id,import_id,row_id,source_url,filename,content_type,
                stored_path,size_bytes,status)
             VALUES('ev1','imp1',?,'https://contoh/foto.jpg','lama.jpg','application/octet-stream',?,?,'ready')",
        )
        .bind(row_id)
        .bind(legacy.to_string_lossy().to_string())
        .bind(png.len() as i64)
        .execute(&db)
        .await
        .unwrap();

        let report = run(&db, &config, false).await.unwrap();
        assert_eq!(report.files_scanned, 1);
        assert_eq!(report.files_moved, 1);
        assert!(report.deleted_files >= 2);

        let stored_path: String =
            sqlx::query_scalar("SELECT stored_path FROM import_evidence WHERE id = 'ev1'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert!(stored_path.starts_with("evidence/content/"));
        assert!(
            tokio::fs::try_exists(config.data_dir.join(&stored_path))
                .await
                .unwrap()
        );

        let content_type: String =
            sqlx::query_scalar("SELECT content_type FROM import_evidence WHERE id = 'ev1'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(content_type, "image/png");
        assert!(!tokio::fs::try_exists(&legacy).await.unwrap());
        assert!(!tokio::fs::try_exists(&orphan).await.unwrap());
    }
}
