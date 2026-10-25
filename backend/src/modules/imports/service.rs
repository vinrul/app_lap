use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use sha2::{Digest, Sha256};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool, sqlite::SqliteRow};
use uuid::Uuid;

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    modules::{
        imports::{
            analysis,
            models::{
                EvidenceView, ImportPreview, ImportView, ParsedRow, RecordQuery, RecordView,
                RecordsResponse,
            },
            parser, qw,
        },
        periods,
    },
    shared::{LabeledCount, audit, extract_links, parse_fields},
};

const DEFAULT_PAGE_SIZE: i64 = 25;
const MAX_PAGE_SIZE: i64 = 100;

pub async fn list(db: &SqlitePool) -> ApiResult<Vec<ImportView>> {
    let rows = sqlx::query(
        "SELECT i.id, i.filename, i.row_count, i.skipped_rows, i.duplicate_rows,
                i.parser_version, i.date_start, i.date_end, i.warnings_json,
                i.created_at, COALESCE(u.username, 'Sistem') AS imported_by
         FROM imports i
         LEFT JOIN users u ON u.id = i.imported_by
         ORDER BY i.created_at DESC, i.id DESC",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows.iter().map(import_from_row).collect())
}

pub async fn list_for_range(
    db: &SqlitePool,
    start_date: &str,
    end_date: &str,
) -> ApiResult<Vec<ImportView>> {
    let rows = sqlx::query(
        "SELECT i.id, i.filename, i.row_count, i.skipped_rows, i.duplicate_rows,
                i.parser_version, i.date_start, i.date_end, i.warnings_json,
                i.created_at, COALESCE(u.username, 'Sistem') AS imported_by
         FROM imports i
         LEFT JOIN users u ON u.id = i.imported_by
         WHERE i.date_start <= ? AND i.date_end >= ?
         ORDER BY i.created_at DESC, i.id DESC",
    )
    .bind(end_date)
    .bind(start_date)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows.iter().map(import_from_row).collect())
}

pub async fn get(db: &SqlitePool, import_id: &str) -> ApiResult<ImportView> {
    let row = sqlx::query(
        "SELECT i.id, i.filename, i.row_count, i.skipped_rows, i.duplicate_rows,
                i.parser_version, i.date_start, i.date_end, i.warnings_json,
                i.created_at, COALESCE(u.username, 'Sistem') AS imported_by
         FROM imports i
         LEFT JOIN users u ON u.id = i.imported_by
         WHERE i.id = ?",
    )
    .bind(import_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Berkas impor tidak ditemukan."))?;
    Ok(import_from_row(&row))
}

pub async fn preview(bytes: Vec<u8>) -> ApiResult<ImportPreview> {
    tokio::task::spawn_blocking(move || parser::parse_workbook(&bytes))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::bad_request)
}

pub async fn commit(
    db: &SqlitePool,
    config: &Config,
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

    let preview = preview(bytes.clone()).await?;
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
    let archive_dir = config.data_dir.join("imports");
    tokio::fs::create_dir_all(&archive_dir)
        .await
        .map_err(ApiError::internal)?;
    let stored_path = archive_dir.join(format!("{import_id}.xlsx"));
    tokio::fs::write(&stored_path, &bytes)
        .await
        .map_err(ApiError::internal)?;

    let result = persist_import(
        db,
        user_id,
        filename,
        &digest,
        &stored_path,
        &import_id,
        &preview,
    )
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&stored_path).await;
    }
    result?;

    get(db, &import_id).await
}

async fn persist_import(
    db: &SqlitePool,
    user_id: &str,
    filename: &str,
    digest: &str,
    stored_path: &Path,
    import_id: &str,
    preview: &ImportPreview,
) -> ApiResult<()> {
    let warnings_json = serde_json::to_string(&preview.warnings).map_err(ApiError::internal)?;
    let date_start = preview.file_date_start.clone().unwrap_or_default();
    let date_end = preview.file_date_end.clone().unwrap_or_default();

    let mut tx = db.begin().await.map_err(ApiError::internal)?;

    sqlx::query(
        "INSERT INTO imports(
            id, filename, file_hash, stored_path, parser_version, row_count, skipped_rows,
            duplicate_rows, date_start, date_end, warnings_json, imported_by
         ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(import_id)
    .bind(filename)
    .bind(digest)
    .bind(stored_path.to_string_lossy().to_string())
    .bind("3")
    .bind(preview.valid_rows as i64)
    .bind(preview.skipped_rows as i64)
    .bind(preview.duplicate_rows as i64)
    .bind(&date_start)
    .bind(&date_end)
    .bind(warnings_json)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::internal)?;

    for row in &preview.rows {
        let parsed_json = serde_json::json!({
            "fields": &row.fields,
            "links": &row.links,
            "analysis": &row.analysis,
        })
        .to_string();
        let dedupe_key = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            row.tanggal.trim().to_lowercase(),
            row.waktu_kirim_wib.trim().to_lowercase(),
            row.satuan.trim().to_lowercase(),
            row.tingkat.trim().to_lowercase(),
            row.polres.trim().to_lowercase(),
            row.polsek.trim().to_lowercase(),
            row.kode.trim().to_lowercase(),
        );

        let inserted = sqlx::query(
            "INSERT INTO import_rows(
                import_id, source_row, tanggal, waktu_kirim_wib, satuan, tingkat, polda,
                polres, polsek, level, unit, kode, qw_title, kegiatan, quick_wins,
                pejabat_pelaksana, jabatan, pelaksana, pelapor, status_penerimaan,
                bukti_dukung, keluhan_personel_polri, isian_laporan,
                personel_count, bibit_count, area_m2, jamaah_count, latitude, longitude,
                map_url, raw_json, parsed_json, dedupe_key
             ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(import_id)
        .bind(row.source_row as i64)
        .bind(&row.tanggal)
        .bind(&row.waktu_kirim_wib)
        .bind(&row.satuan)
        .bind(&row.tingkat)
        .bind(&row.polda)
        .bind(&row.polres)
        .bind(&row.polsek)
        .bind(&row.level)
        .bind(&row.unit)
        .bind(&row.kode)
        .bind(&row.qw_title)
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
        .bind(row.analysis.personel)
        .bind(row.analysis.bibit)
        .bind(row.analysis.area_m2)
        .bind(row.analysis.jamaah)
        .bind(row.analysis.latitude)
        .bind(row.analysis.longitude)
        .bind(&row.analysis.map_url)
        .bind(row.raw.to_string())
        .bind(parsed_json)
        .bind(dedupe_key)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::internal)?;

        let row_id = inserted.last_insert_rowid();
        for link in &row.links {
            if !(link.starts_with("http://") || link.starts_with("https://")) {
                continue;
            }
            sqlx::query(
                "INSERT INTO import_evidence(id, import_id, row_id, source_url)
                 VALUES(?,?,?,?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(import_id)
            .bind(row_id)
            .bind(link)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::internal)?;
        }
    }

    audit::record_tx(
        &mut tx,
        Some(user_id),
        "import.commit",
        "import",
        import_id,
        serde_json::json!({
            "rows": preview.valid_rows,
            "skipped": preview.skipped_rows,
            "duplicates": preview.duplicate_rows,
            "sha256": digest,
            "date_start": date_start,
            "date_end": date_end,
        }),
    )
    .await?;

    tx.commit().await.map_err(ApiError::internal)?;
    Ok(())
}

pub async fn load_period_rows(
    db: &SqlitePool,
    start_date: &str,
    end_date: &str,
) -> ApiResult<(Vec<RecordView>, i64, i64)> {
    let mut qb = QueryBuilder::<Sqlite>::new(ranked_cte());
    qb.push(" WHERE r.tanggal BETWEEN ");
    qb.push_bind(start_date);
    qb.push(" AND ");
    qb.push_bind(end_date);
    qb.push(
        ") SELECT * FROM ranked ORDER BY tanggal ASC, level ASC, unit ASC, source_row ASC",
    );

    let rows = qb
        .build()
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)?;

    let mut result = Vec::with_capacity(rows.len());
    let mut duplicate_rows = 0_i64;
    let mut imports = std::collections::HashSet::new();
    for row in &rows {
        let rank: i64 = row.get("rn");
        if rank > 1 {
            duplicate_rows += 1;
            continue;
        }
        imports.insert(row.get::<String, _>("import_id"));
        result.push(record_from_row(row, &HashMap::new()));
    }

    Ok((result, duplicate_rows, imports.len() as i64))
}

pub async fn records_for_period(
    db: &SqlitePool,
    period_id: &str,
    query: &RecordQuery,
) -> ApiResult<RecordsResponse> {
    let (start, end, _) = periods::service::dates(db, period_id).await?;
    records_for_range(db, &start, &end, query).await
}

pub async fn records_for_import(
    db: &SqlitePool,
    import_id: &str,
    query: &RecordQuery,
) -> ApiResult<RecordsResponse> {
    let view = get(db, import_id).await?;
    let start = if view.date_start.is_empty() {
        "0000-01-01".to_string()
    } else {
        view.date_start
    };
    let end = if view.date_end.is_empty() {
        "9999-12-31".to_string()
    } else {
        view.date_end
    };
    let mut query = query.clone();
    query.import_id = Some(import_id.to_string());
    records_for_range(db, &start, &end, &query).await
}

pub async fn records_for_range(
    db: &SqlitePool,
    start_date: &str,
    end_date: &str,
    query: &RecordQuery,
) -> ApiResult<RecordsResponse> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = (page - 1) * page_size;

    let total = count_records(db, start_date, end_date, query).await?;
    let rows = query_records(db, start_date, end_date, query, page_size, offset).await?;
    let quick_wins = group_counts(db, start_date, end_date, query, "quick_wins").await?;
    let statuses = group_counts(db, start_date, end_date, query, "status").await?;
    let polres = group_counts(db, start_date, end_date, query, "polres").await?;

    let (duplicate_rows, source_count) = range_summary(db, start_date, end_date, query).await?;

    Ok(RecordsResponse {
        date_start: start_date.to_string(),
        date_end: end_date.to_string(),
        source_count,
        duplicate_rows,
        total,
        page,
        page_size,
        quick_wins,
        statuses,
        polres,
        records: rows,
    })
}

fn ranked_cte() -> &'static str {
    "WITH ranked AS (
        SELECT r.id, r.import_id, i.filename AS import_filename, r.source_row, r.tanggal,
               r.waktu_kirim_wib, r.satuan, r.tingkat, r.polda, r.polres, r.polsek,
               r.level, r.unit, r.kode, r.qw_title, r.kegiatan, r.quick_wins,
               r.pejabat_pelaksana, r.jabatan, r.pelaksana, r.pelapor, r.status_penerimaan,
               r.bukti_dukung, r.keluhan_personel_polri, r.isian_laporan,
               r.personel_count, r.bibit_count, r.area_m2, r.jamaah_count,
               r.latitude, r.longitude, r.map_url, r.parsed_json,
               ROW_NUMBER() OVER (
                   PARTITION BY lower(trim(r.dedupe_key))
                   ORDER BY i.created_at DESC, i.id DESC, r.source_row DESC
               ) AS rn
        FROM import_rows r
        JOIN imports i ON i.id = r.import_id"
}

fn apply_range(
    qb: &mut QueryBuilder<Sqlite>,
    start_date: &str,
    end_date: &str,
) {
    qb.push(" WHERE r.tanggal BETWEEN ");
    qb.push_bind(start_date.to_string());
    qb.push(" AND ");
    qb.push_bind(end_date.to_string());
    qb.push(") ");
}

fn push_filters(qb: &mut QueryBuilder<Sqlite>, query: &RecordQuery) {
    if let Some(import_id) = query
        .import_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        qb.push(" AND import_id = ")
            .push_bind(import_id.to_string());
    }
    if let Some(level) = query.level.as_deref().filter(|value| !value.trim().is_empty()) {
        qb.push(" AND level = ").push_bind(level.to_string());
    }
    if let Some(polres) = query
        .polres
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        if polres.eq_ignore_ascii_case("polda") {
            qb.push(" AND trim(polres) = ''");
        } else {
            qb.push(" AND polres = ").push_bind(polres.to_string());
        }
    }
    if let Some(quick_wins) = query
        .quick_wins
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        qb.push(" AND quick_wins = ")
            .push_bind(quick_wins.to_string());
    }
    if let Some(status) = query.status.as_deref().filter(|value| !value.trim().is_empty()) {
        if status.eq_ignore_ascii_case("belum diisi") {
            qb.push(" AND trim(status_penerimaan) = ''");
        } else {
            qb.push(" AND status_penerimaan = ")
                .push_bind(status.to_string());
        }
    }
    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let pattern = format!("%{search}%");
        qb.push(" AND (kegiatan LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR satuan LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR unit LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR polres LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR polsek LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR kode LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR isian_laporan LIKE ");
        qb.push_bind(pattern);
        qb.push(")");
    }
}

async fn count_records(
    db: &SqlitePool,
    start_date: &str,
    end_date: &str,
    query: &RecordQuery,
) -> ApiResult<i64> {
    let mut qb = QueryBuilder::<Sqlite>::new(ranked_cte());
    apply_range(&mut qb, start_date, end_date);
    qb.push(" SELECT COUNT(*) FROM ranked WHERE rn = 1");
    push_filters(&mut qb, query);
    qb.build_query_scalar()
        .fetch_one(db)
        .await
        .map_err(ApiError::internal)
}

async fn query_records(
    db: &SqlitePool,
    start_date: &str,
    end_date: &str,
    query: &RecordQuery,
    page_size: i64,
    offset: i64,
) -> ApiResult<Vec<RecordView>> {
    let mut qb = QueryBuilder::<Sqlite>::new(ranked_cte());
    apply_range(&mut qb, start_date, end_date);
    qb.push(" SELECT * FROM ranked WHERE rn = 1");
    push_filters(&mut qb, query);
    qb.push(" ORDER BY tanggal DESC, id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb
        .build()
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)?;
    let ids: Vec<i64> = rows.iter().map(|row| row.get("id")).collect();
    let evidence = if ids.is_empty() {
        HashMap::new()
    } else {
        load_evidence_for_rows(db, &ids).await?
    };
    Ok(rows
        .iter()
        .map(|row| record_from_row(row, &evidence))
        .collect())
}

async fn group_counts(
    db: &SqlitePool,
    start_date: &str,
    end_date: &str,
    query: &RecordQuery,
    column: &str,
) -> ApiResult<Vec<LabeledCount>> {
    let expression = match column {
        "quick_wins" => "COALESCE(NULLIF(trim(quick_wins), ''), 'Belum dikategorikan')",
        "status" => "COALESCE(NULLIF(trim(status_penerimaan), ''), 'Belum diisi')",
        "polres" => "COALESCE(NULLIF(trim(polres), ''), 'Polda')",
        _ => return Ok(Vec::new()),
    };
    let mut qb = QueryBuilder::<Sqlite>::new(ranked_cte());
    apply_range(&mut qb, start_date, end_date);
    qb.push(format!(
        " SELECT {expression} AS label, COUNT(*) AS count FROM ranked WHERE rn = 1"
    ));
    push_filters(&mut qb, query);
    qb.push(" GROUP BY label ORDER BY count DESC, label ASC LIMIT 100");
    let rows = qb
        .build()
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)?;
    Ok(rows
        .iter()
        .map(|row| LabeledCount::new(row.get::<String, _>("label"), row.get("count")))
        .collect())
}

async fn range_summary(
    db: &SqlitePool,
    start_date: &str,
    end_date: &str,
    _query: &RecordQuery,
) -> ApiResult<(i64, i64)> {
    let mut qb = QueryBuilder::<Sqlite>::new(ranked_cte());
    apply_range(&mut qb, start_date, end_date);
    qb.push(" SELECT COUNT(*) AS duplicates FROM ranked WHERE rn > 1");
    let duplicates: i64 = qb
        .build_query_scalar()
        .fetch_one(db)
        .await
        .map_err(ApiError::internal)?;

    let mut qb = QueryBuilder::<Sqlite>::new(ranked_cte());
    apply_range(&mut qb, start_date, end_date);
    qb.push(" SELECT COUNT(DISTINCT import_id) FROM ranked WHERE rn = 1");
    let sources: i64 = qb
        .build_query_scalar()
        .fetch_one(db)
        .await
        .map_err(ApiError::internal)?;
    Ok((duplicates, sources))
}

fn record_from_row(row: &SqliteRow, evidence: &HashMap<i64, Vec<EvidenceView>>) -> RecordView {
    let isian_laporan: String = row.get("isian_laporan");
    let bukti_dukung: String = row.get("bukti_dukung");
    let parsed: ParsedRow =
        serde_json::from_str(&row.get::<String, _>("parsed_json")).unwrap_or_default();
    let fields = if parsed.fields.is_empty() {
        parse_fields(&isian_laporan)
    } else {
        parsed.fields
    };
    let mut links = if parsed.links.is_empty() {
        extract_links(&isian_laporan)
    } else {
        parsed.links
    };
    links.extend(extract_links(&bukti_dukung));
    links.sort();
    links.dedup();

    let stored_analysis = parsed.analysis;
    let analysis = if stored_analysis.personel == 0
        && stored_analysis.bibit == 0
        && stored_analysis.area_m2.abs() < f64::EPSILON
        && stored_analysis.jamaah == 0
        && stored_analysis.latitude.is_none()
        && stored_analysis.map_url.is_empty()
    {
        analysis::analyze(&fields, &isian_laporan, &bukti_dukung)
    } else {
        stored_analysis
    };

    let id: i64 = row.get("id");
    let kode: String = row.get("kode");
    let qw_title: String = row.get("qw_title");
    RecordView {
        id,
        source_row: row.get("source_row"),
        import_id: row.get("import_id"),
        import_filename: row.get("import_filename"),
        tanggal: row.get("tanggal"),
        waktu_kirim_wib: row.get("waktu_kirim_wib"),
        satuan: row.get("satuan"),
        tingkat: row.get("tingkat"),
        polda: row.get("polda"),
        polres: row.get("polres"),
        polsek: row.get("polsek"),
        level: row.get("level"),
        unit: row.get("unit"),
        kode: kode.clone(),
        qw_title: if qw_title.is_empty() {
            qw::title(&kode).to_string()
        } else {
            qw_title
        },
        kegiatan: row.get("kegiatan"),
        quick_wins: row.get("quick_wins"),
        pejabat_pelaksana: row.get("pejabat_pelaksana"),
        jabatan: row.get("jabatan"),
        pelaksana: row.get("pelaksana"),
        pelapor: row.get("pelapor"),
        status_penerimaan: row.get("status_penerimaan"),
        bukti_dukung,
        keluhan_personel_polri: row.get("keluhan_personel_polri"),
        isian_laporan,
        personel_count: analysis.personel,
        bibit_count: analysis.bibit,
        area_m2: analysis.area_m2,
        jamaah_count: analysis.jamaah,
        latitude: analysis.latitude,
        longitude: analysis.longitude,
        map_url: analysis.map_url,
        fields,
        links,
        evidence: evidence.get(&id).cloned().unwrap_or_default(),
    }
}

async fn load_evidence_for_rows(
    db: &SqlitePool,
    ids: &[i64],
) -> ApiResult<HashMap<i64, Vec<EvidenceView>>> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT id, row_id, source_url, filename, content_type, size_bytes, status, error FROM import_evidence WHERE row_id IN (",
    );
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(*id);
    }
    separated.push_unseparated(") ORDER BY created_at ASC, id ASC");
    let rows = qb
        .build()
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)?;
    let mut grouped: HashMap<i64, Vec<EvidenceView>> = HashMap::new();
    for row in rows {
        let id: String = row.get("id");
        let row_id: i64 = row.get("row_id");
        grouped.entry(row_id).or_default().push(EvidenceView {
            url: format!("/api/evidence/{id}"),
            id,
            source_url: row.get("source_url"),
            filename: row.get("filename"),
            content_type: row.get("content_type"),
            size_bytes: row.get("size_bytes"),
            status: row.get("status"),
            error: row.get("error"),
        });
    }
    Ok(grouped)
}

fn import_from_row(row: &SqliteRow) -> ImportView {
    let warnings: Vec<String> =
        serde_json::from_str(&row.get::<String, _>("warnings_json")).unwrap_or_default();
    ImportView {
        id: row.get("id"),
        filename: row.get("filename"),
        row_count: row.get("row_count"),
        skipped_rows: row.get("skipped_rows"),
        duplicate_rows: row.get("duplicate_rows"),
        parser_version: row.get("parser_version"),
        date_start: row.get("date_start"),
        date_end: row.get("date_end"),
        warnings,
        created_at: row.get("created_at"),
        imported_by: row.get("imported_by"),
    }
}

pub async fn analysis_for_period(
    db: &SqlitePool,
    period_id: &str,
) -> ApiResult<crate::modules::imports::models::AnalysisResponse> {
    let (start, end, _) = periods::service::dates(db, period_id).await?;
    let (rows, duplicate_rows, source_count) = load_period_rows(db, &start, &end).await?;
    Ok(build_analysis(&start, &end, source_count, duplicate_rows, &rows))
}

fn build_analysis(
    start: &str,
    end: &str,
    source_count: i64,
    duplicate_rows: i64,
    rows: &[RecordView],
) -> crate::modules::imports::models::AnalysisResponse {
    let mut personel_total = 0_i64;
    let mut bibit_total = 0_i64;
    let mut area_total = 0.0_f64;
    let mut jamaah_total = 0_i64;
    let mut rows_with_coordinates = 0_i64;
    let mut evidence_links = 0_i64;
    let mut groups = BTreeMap::<String, i64>::new();
    let mut codes = BTreeMap::<String, i64>::new();
    let mut levels = BTreeMap::<String, i64>::new();
    let mut units = BTreeMap::<String, i64>::new();
    let mut dates = BTreeMap::<String, i64>::new();

    for row in rows {
        personel_total += row.personel_count;
        bibit_total += row.bibit_count;
        area_total += row.area_m2;
        jamaah_total += row.jamaah_count;
        if row.latitude.is_some() && row.longitude.is_some() {
            rows_with_coordinates += 1;
        }
        evidence_links += row.links.len() as i64;
        let group = if row.quick_wins.trim().is_empty() {
            "Belum dikategorikan".to_string()
        } else {
            format!(
                "{} · {}",
                row.quick_wins.trim(),
                qw::group_title(&row.quick_wins)
            )
        };
        *groups.entry(group).or_default() += 1;
        let code = if row.kode.trim().is_empty() {
            "Tanpa kode".to_string()
        } else {
            format!("{} · {}", row.kode.trim(), row.qw_title)
        };
        *codes.entry(code).or_default() += 1;
        levels.entry(if row.level.is_empty() {
            "Polda".to_string()
        } else {
            row.level.clone()
        }).or_default();
        units.entry(if row.unit.is_empty() {
            row.satuan.clone()
        } else {
            row.unit.clone()
        }).or_default();
        *dates.entry(row.tanggal.clone()).or_default() += 1;
    }

    crate::modules::imports::models::AnalysisResponse {
        date_start: start.to_string(),
        date_end: end.to_string(),
        source_count,
        total_rows: rows.len() as i64,
        duplicate_rows,
        personel_total,
        bibit_total,
        area_total: (area_total * 100.0).round() / 100.0,
        jamaah_total,
        rows_with_coordinates,
        evidence_links,
        by_group: top_counts(groups, 20),
        by_code: top_counts(codes, 50),
        by_level: top_counts(levels, 10),
        by_unit: top_counts(units, 30),
        by_date: top_counts(dates, 400),
    }
}

fn top_counts(map: BTreeMap<String, i64>, limit: usize) -> Vec<LabeledCount> {
    let mut items: Vec<LabeledCount> = map
        .into_iter()
        .map(|(label, count)| LabeledCount::new(label, count))
        .collect();
    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(limit);
    items
}
