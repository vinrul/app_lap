use std::path::Path;

use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    modules::{
        imports::{
            models::{
                ImportPreview, ImportView, ParsedRow, RecordQuery, RecordView, RecordsResponse,
            },
            parser,
        },
        periods,
    },
    shared::{LabeledCount, audit},
};

pub async fn list_for_period(db: &SqlitePool, period_id: &str) -> ApiResult<Vec<ImportView>> {
    periods::service::get(db, period_id).await?;
    let rows = sqlx::query(
        "SELECT i.id, i.filename, i.row_count, i.skipped_rows, i.parser_version, i.created_at,
                COALESCE(u.username, 'Sistem') AS imported_by
         FROM imports i
         LEFT JOIN users u ON u.id = i.imported_by
         WHERE i.period_id = ?
         ORDER BY i.created_at DESC",
    )
    .bind(period_id)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| ImportView {
            id: row.get("id"),
            filename: row.get("filename"),
            row_count: row.get("row_count"),
            skipped_rows: row.get("skipped_rows"),
            parser_version: row.get("parser_version"),
            created_at: row.get("created_at"),
            imported_by: row.get("imported_by"),
        })
        .collect())
}

pub async fn preview(bytes: Vec<u8>, start_date: &str, end_date: &str) -> ApiResult<ImportPreview> {
    let start = start_date.to_string();
    let end = end_date.to_string();
    tokio::task::spawn_blocking(move || parser::parse_workbook(&bytes, &start, &end))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::bad_request)
}

pub async fn commit(
    db: &SqlitePool,
    config: &Config,
    period_id: &str,
    user_id: &str,
    filename: &str,
    bytes: Vec<u8>,
) -> ApiResult<ImportView> {
    if bytes.len() > config.max_upload_bytes {
        return Err(ApiError::payload_too_large(format!(
            "Ukuran file maksimal {} MB.",
            config.max_upload_bytes / 1024 / 1024
        )));
    }
    if !filename.to_ascii_lowercase().ends_with(".xlsx") {
        return Err(ApiError::bad_request(
            "Unggah file Excel berekstensi .xlsx.",
        ));
    }

    let (start_date, end_date, _label) = periods::service::dates(db, period_id).await?;
    let preview = preview(bytes.clone(), &start_date, &end_date).await?;
    if !preview.errors.is_empty() {
        return Err(ApiError::bad_request(preview.errors.join(" ")));
    }
    if preview.rows.is_empty() {
        return Err(ApiError::bad_request(
            "Tidak ada baris valid yang dapat disimpan.",
        ));
    }

    let digest = hex::encode(Sha256::digest(&bytes));
    let import_id = Uuid::new_v4().to_string();
    let archive_dir = config.data_dir.join("imports").join(period_id);
    tokio::fs::create_dir_all(&archive_dir)
        .await
        .map_err(ApiError::internal)?;
    let stored_path = archive_dir.join(format!("{import_id}.xlsx"));
    tokio::fs::write(&stored_path, &bytes)
        .await
        .map_err(ApiError::internal)?;

    let result = persist_import(
        db,
        period_id,
        user_id,
        filename,
        &digest,
        &stored_path,
        &preview,
    )
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&stored_path).await;
    }

    let _ = result?;

    let row = sqlx::query(
        "SELECT i.id, i.filename, i.row_count, i.skipped_rows, i.parser_version, i.created_at,
                COALESCE(u.username, 'Sistem') AS imported_by
         FROM imports i LEFT JOIN users u ON u.id = i.imported_by
         WHERE i.id = ?",
    )
    .bind(&import_id)
    .fetch_one(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(ImportView {
        id: row.get("id"),
        filename: row.get("filename"),
        row_count: row.get("row_count"),
        skipped_rows: row.get("skipped_rows"),
        parser_version: row.get("parser_version"),
        created_at: row.get("created_at"),
        imported_by: row.get("imported_by"),
    })
}

async fn persist_import(
    db: &SqlitePool,
    period_id: &str,
    user_id: &str,
    filename: &str,
    digest: &str,
    stored_path: &Path,
    preview: &ImportPreview,
) -> ApiResult<()> {
    let import_id = stored_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ApiError::internal("nama berkas impor tidak valid"))?
        .to_string();

    let warnings_json = serde_json::to_string(&preview.warnings).map_err(ApiError::internal)?;

    let mut tx = db.begin().await.map_err(ApiError::internal)?;

    sqlx::query(
        "INSERT INTO imports(
            id, period_id, filename, file_hash, stored_path, parser_version,
            row_count, skipped_rows, warnings_json, imported_by
         ) VALUES(?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&import_id)
    .bind(period_id)
    .bind(filename)
    .bind(digest)
    .bind(stored_path.to_string_lossy().to_string())
    .bind("2")
    .bind(preview.valid_rows as i64)
    .bind(preview.skipped_rows as i64)
    .bind(warnings_json)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::internal)?;

    for row in &preview.rows {
        let parsed_json = serde_json::json!({
            "fields": &row.fields,
            "links": &row.links,
        })
        .to_string();

        sqlx::query(
            "INSERT INTO import_rows(
                import_id, source_row, tanggal, waktu_kirim_wib, satuan, tingkat, polda,
                polres, polsek, kode, kegiatan, quick_wins, pejabat_pelaksana, jabatan,
                pelaksana, pelapor, status_penerimaan, bukti_dukung,
                keluhan_personel_polri, isian_laporan, raw_json, parsed_json
             ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&import_id)
        .bind(row.source_row as i64)
        .bind(&row.tanggal)
        .bind(&row.waktu_kirim_wib)
        .bind(&row.satuan)
        .bind(&row.tingkat)
        .bind(&row.polda)
        .bind(&row.polres)
        .bind(&row.polsek)
        .bind(&row.kode)
        .bind(&row.kegiatan)
        .bind(&row.quick_wins)
        .bind(&row.pejabat_pelaksana)
        .bind(&row.jabatan)
        .bind(&row.pelaksana)
        .bind(&row.pelapor)
        .bind(&row.status_penerimaan)
        .bind(&row.bukti_dukung)
        .bind(&row.keluhan_personel_polri)
        .bind(&row.isian_laporan)
        .bind(row.raw.to_string())
        .bind(parsed_json)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::internal)?;
    }

    sqlx::query(
        "INSERT INTO report_drafts(id, period_id, import_id, content_json, updated_by)
         VALUES(?,?,?,'{}',?)
         ON CONFLICT(period_id, import_id) DO NOTHING",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(period_id)
    .bind(&import_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::internal)?;

    audit::record_tx(
        &mut tx,
        Some(user_id),
        "import.commit",
        "import",
        &import_id,
        serde_json::json!({
            "rows": preview.valid_rows,
            "skipped": preview.skipped_rows,
            "duplicates": preview.duplicate_rows,
            "sha256": digest,
        }),
    )
    .await?;

    tx.commit().await.map_err(ApiError::internal)?;
    Ok(())
}

pub async fn ensure_import(
    db: &SqlitePool,
    period_id: &str,
    import_id: &str,
) -> ApiResult<(String, String, String)> {
    let row = sqlx::query(
        "SELECT i.filename, p.start_date, p.end_date
         FROM imports i JOIN report_periods p ON p.id = i.period_id
         WHERE i.id = ? AND i.period_id = ?",
    )
    .bind(import_id)
    .bind(period_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Versi impor tidak ditemukan pada periode ini."))?;
    Ok((
        row.get("filename"),
        row.get("start_date"),
        row.get("end_date"),
    ))
}

pub async fn latest_import_id(db: &SqlitePool, period_id: &str) -> ApiResult<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT id FROM imports WHERE period_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(period_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Periode ini belum memiliki data impor."))
}

pub async fn records(
    db: &SqlitePool,
    period_id: &str,
    query: &RecordQuery,
) -> ApiResult<RecordsResponse> {
    periods::service::get(db, period_id).await?;
    let import_id = match query.import_id.as_deref() {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => latest_import_id(db, period_id).await?,
    };
    let (import_filename, period_start, period_end) =
        ensure_import(db, period_id, &import_id).await?;

    let start_date = query
        .start_date
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&period_start)
        .to_string();
    let end_date = query
        .end_date
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&period_end)
        .to_string();

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(25).clamp(10, 100);

    let total = count_records(db, &import_id, &start_date, &end_date, query).await?;

    let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT id, source_row, tanggal, waktu_kirim_wib, satuan, tingkat, polda, polres,
                polsek, kode, kegiatan, quick_wins, pejabat_pelaksana, jabatan, pelaksana,
                pelapor, status_penerimaan, bukti_dukung, keluhan_personel_polri,
                isian_laporan, parsed_json
         FROM import_rows",
    );
    apply_filters(&mut qb, &import_id, &start_date, &end_date, query);
    qb.push(" ORDER BY tanggal DESC, source_row ASC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page - 1) * page_size);

    let rows = qb
        .build_query_as::<RecordRow>()
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)?;

    let records = rows
        .into_iter()
        .map(|row| {
            let parsed: ParsedRow = serde_json::from_str(&row.parsed_json).unwrap_or_default();
            let fields = if parsed.fields.is_empty() {
                crate::shared::parse_fields(&row.isian_laporan)
            } else {
                parsed.fields
            };
            let links = if parsed.links.is_empty() {
                crate::shared::extract_links(&row.isian_laporan)
            } else {
                parsed.links
            };
            RecordView {
                id: row.id,
                source_row: row.source_row,
                tanggal: row.tanggal,
                waktu_kirim_wib: row.waktu_kirim_wib,
                satuan: row.satuan,
                tingkat: row.tingkat,
                polda: row.polda,
                polres: row.polres,
                polsek: row.polsek,
                kode: row.kode,
                kegiatan: row.kegiatan,
                quick_wins: row.quick_wins,
                pejabat_pelaksana: row.pejabat_pelaksana,
                jabatan: row.jabatan,
                pelaksana: row.pelaksana,
                pelapor: row.pelapor,
                status_penerimaan: row.status_penerimaan,
                bukti_dukung: row.bukti_dukung,
                keluhan_personel_polri: row.keluhan_personel_polri,
                isian_laporan: row.isian_laporan,
                fields,
                links,
            }
        })
        .collect();

    let quick_wins = group_counts(db, "quick_wins", &import_id, &start_date, &end_date).await?;
    let statuses =
        group_counts(db, "status_penerimaan", &import_id, &start_date, &end_date).await?;
    let polres = group_counts(db, "polres", &import_id, &start_date, &end_date).await?;

    Ok(RecordsResponse {
        import_id,
        import_filename,
        period_start,
        period_end,
        total,
        page,
        page_size,
        quick_wins,
        statuses,
        polres,
        records,
    })
}

async fn count_records(
    db: &SqlitePool,
    import_id: &str,
    start_date: &str,
    end_date: &str,
    query: &RecordQuery,
) -> ApiResult<i64> {
    let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new("SELECT COUNT(*) FROM import_rows");
    apply_filters(&mut qb, import_id, start_date, end_date, query);
    qb.build_query_scalar::<i64>()
        .fetch_one(db)
        .await
        .map_err(ApiError::internal)
}

fn apply_filters(
    qb: &mut sqlx::QueryBuilder<sqlx::Sqlite>,
    import_id: &str,
    start_date: &str,
    end_date: &str,
    query: &RecordQuery,
) {
    qb.push(" WHERE import_id = ")
        .push_bind(import_id.to_string())
        .push(" AND tanggal BETWEEN ")
        .push_bind(start_date.to_string())
        .push(" AND ")
        .push_bind(end_date.to_string());

    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        let pattern = format!("%{search}%");
        qb.push(" AND (satuan LIKE ")
            .push_bind(pattern.clone())
            .push(" OR polda LIKE ")
            .push_bind(pattern.clone())
            .push(" OR polres LIKE ")
            .push_bind(pattern.clone())
            .push(" OR polsek LIKE ")
            .push_bind(pattern.clone())
            .push(" OR kode LIKE ")
            .push_bind(pattern.clone())
            .push(" OR kegiatan LIKE ")
            .push_bind(pattern.clone())
            .push(" OR quick_wins LIKE ")
            .push_bind(pattern.clone())
            .push(" OR pelaksana LIKE ")
            .push_bind(pattern.clone())
            .push(" OR isian_laporan LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if let Some(value) = query.quick_wins.as_deref().filter(|v| !v.is_empty()) {
        qb.push(" AND quick_wins = ").push_bind(value.to_string());
    }
    if let Some(value) = query.status.as_deref().filter(|v| !v.is_empty()) {
        qb.push(" AND status_penerimaan = ")
            .push_bind(value.to_string());
    }
    if let Some(value) = query.polres.as_deref().filter(|v| !v.is_empty()) {
        qb.push(" AND polres = ").push_bind(value.to_string());
    }
}

async fn group_counts(
    db: &SqlitePool,
    column: &str,
    import_id: &str,
    start_date: &str,
    end_date: &str,
) -> ApiResult<Vec<LabeledCount>> {
    // Query statis dipilih dari daftar tetap, sehingga tidak ada SQL dinamis dari input pengguna.
    let sql = match column {
        "quick_wins" => {
            "SELECT quick_wins AS label, COUNT(*) AS count
             FROM import_rows
             WHERE import_id = ? AND tanggal BETWEEN ? AND ?
             GROUP BY quick_wins ORDER BY count DESC, label ASC"
        }
        "status_penerimaan" => {
            "SELECT status_penerimaan AS label, COUNT(*) AS count
             FROM import_rows
             WHERE import_id = ? AND tanggal BETWEEN ? AND ?
             GROUP BY status_penerimaan ORDER BY count DESC, label ASC"
        }
        "polres" => {
            "SELECT polres AS label, COUNT(*) AS count
             FROM import_rows
             WHERE import_id = ? AND tanggal BETWEEN ? AND ?
             GROUP BY polres ORDER BY count DESC, label ASC"
        }
        _ => return Ok(Vec::new()),
    };
    let rows = sqlx::query(sql)
        .bind(import_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let raw_label: String = row.get("label");
            let label = if raw_label.trim().is_empty() {
                "Belum diisi".to_string()
            } else {
                raw_label.trim().to_string()
            };
            let count: i64 = row.get("count");
            LabeledCount::new(label, count)
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct RecordRow {
    id: i64,
    source_row: i64,
    tanggal: String,
    waktu_kirim_wib: String,
    satuan: String,
    tingkat: String,
    polda: String,
    polres: String,
    polsek: String,
    kode: String,
    kegiatan: String,
    quick_wins: String,
    pejabat_pelaksana: String,
    jabatan: String,
    pelaksana: String,
    pelapor: String,
    status_penerimaan: String,
    bukti_dukung: String,
    keluhan_personel_polri: String,
    isian_laporan: String,
    parsed_json: String,
}
