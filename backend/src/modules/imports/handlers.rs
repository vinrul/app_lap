use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        auth::{AuthUser, DataScope},
        imports::{
            evidence,
            models::{
                AnalysisResponse, CoverageResponse, ImportPreview, ImportView,
                QuickWinDetailResponse, RecordQuery, RecordView, RecordsResponse,
                UnitDetailResponse,
            },
            service,
        },
        reports,
    },
    shared::cache,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<ImportView>>> {
    user.require_writer()?;
    Ok(Json(service::list(&state.db).await?))
}

pub async fn preview(
    State(state): State<AppState>,
    user: AuthUser,
    multipart: Multipart,
) -> ApiResult<Json<ImportPreview>> {
    user.require_writer()?;
    let (_, bytes) = read_workbook(multipart, state.config.max_upload_bytes).await?;
    Ok(Json(service::preview(bytes).await?))
}

pub async fn commit(
    State(state): State<AppState>,
    user: AuthUser,
    multipart: Multipart,
) -> ApiResult<Json<ImportView>> {
    user.require_writer()?;
    let (filename, bytes) = read_workbook(multipart, state.config.max_upload_bytes).await?;
    let view = service::commit(&state.db, &state.config, &user.id, &filename, bytes).await?;
    cache::invalidate();
    if !view.id.is_empty() {
        evidence::spawn_import_downloads(
            state.db.clone(),
            state.config.clone(),
            state.evidence_downloads.clone(),
            view.id.clone(),
            state.shutdown.clone(),
        );
    }
    Ok(Json(view))
}

pub async fn delete_import(
    State(state): State<AppState>,
    user: AuthUser,
    Path(import_id): Path<String>,
) -> ApiResult<StatusCode> {
    user.require_admin()?;
    service::delete(&state.db, &state.config, &user.id, &import_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn retry_evidence(
    State(state): State<AppState>,
    user: AuthUser,
    Path(import_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    user.require_writer()?;
    let queued = evidence::retry_import(
        &state.db,
        state.config.clone(),
        state.evidence_downloads.clone(),
        &import_id,
        state.shutdown.clone(),
    )
    .await?;
    Ok(Json(serde_json::json!({ "queued": queued })))
}

pub async fn evidence_file(
    State(state): State<AppState>,
    user: AuthUser,
    Path(evidence_id): Path<String>,
) -> ApiResult<Response> {
    user.require_writer()?;
    let (bytes, content_type, filename) =
        evidence::open(&state.db, &state.config, &evidence_id).await?;
    let safe_name: String = filename
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | ' ') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let disposition = format!(
        "inline; filename=\"{}\"",
        if safe_name.is_empty() {
            "bukti-dukung"
        } else {
            &safe_name
        }
    );
    let headers = [
        (header::CONTENT_TYPE, content_type),
        (header::CONTENT_DISPOSITION, disposition),
        (header::CACHE_CONTROL, "private, max-age=3600".to_string()),
    ];
    Ok((headers, bytes).into_response())
}

pub async fn records(
    State(state): State<AppState>,
    user: AuthUser,
    Path(import_id): Path<String>,
    Query(query): Query<RecordQuery>,
) -> ApiResult<Json<RecordsResponse>> {
    user.require_writer()?;
    Ok(Json(
        service::records_for_import(&state.db, &import_id, &query).await?,
    ))
}

/// Membatasi filter klien sesuai peran. Pembaca tidak dapat memperluas
/// cakupan dengan mengirim parameter polres/level/unit sendiri.
fn scoped_query(user: &AuthUser, mut query: RecordQuery) -> RecordQuery {
    match user.data_scope() {
        DataScope::All => query,
        DataScope::None => {
            query.polres = Some("__tidak_ada__".to_string());
            query.level = None;
            query.unit = None;
            query
        }
        DataScope::Polres(polres) => {
            query.polres = Some(polres);
            query.level = None;
            query.satker = None;
            query
        }
        DataScope::Polda { satker } => {
            query.polres = Some("polda".to_string());
            query.level = Some("Polda".to_string());
            query.unit = None;
            query.satker = satker;
            query
        }
    }
}

pub async fn period_records(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<RecordQuery>,
) -> ApiResult<Json<RecordsResponse>> {
    let query = scoped_query(&user, query);
    Ok(Json(
        service::records_for_period(&state.db, &period_id, &query).await?,
    ))
}

pub async fn record_detail(
    State(state): State<AppState>,
    user: AuthUser,
    Path((period_id, record_id)): Path<(String, i64)>,
) -> ApiResult<Json<RecordView>> {
    Ok(Json(
        service::record_detail_for_period(&state.db, &period_id, record_id, &user.data_scope())
            .await?,
    ))
}

pub async fn analysis(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<AnalysisResponse>> {
    Ok(Json(
        service::analysis_for_period(&state.db, &period_id, &user.data_scope()).await?,
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct CoverageQuery {
    pub polres: Option<String>,
}

pub async fn coverage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<CoverageQuery>,
) -> ApiResult<Json<CoverageResponse>> {
    let scope = user.data_scope();
    let polres = if scope.is_all() {
        query
            .polres
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    } else {
        None
    };
    if polres.is_none() {
        // Cakupan tanpa filter sudah ikut dihitung bersama metrik periode,
        // sehingga permintaan dari panel desain tidak perlu memindai ulang.
        let metrics =
            reports::service::metrics_for_scope(&state.db, &state.config, &period_id, &scope)
                .await?;
        if let Some(coverage) = metrics.coverage {
            return Ok(Json(coverage));
        }
    }
    Ok(Json(
        service::coverage_for_period(&state.db, &period_id, polres, &scope).await?,
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct UnitDetailQuery {
    pub scope: String,
    pub label: String,
}

pub async fn unit_detail(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<UnitDetailQuery>,
) -> ApiResult<Json<UnitDetailResponse>> {
    Ok(Json(
        service::unit_detail_for_period(
            &state.db,
            &state.config,
            &period_id,
            &user.data_scope(),
            &query.scope,
            &query.label,
        )
        .await?,
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct QuickWinActivityQuery {
    pub scope: Option<String>,
    pub label: Option<String>,
    pub polres: Option<String>,
}

pub async fn quick_win_detail(
    State(state): State<AppState>,
    user: AuthUser,
    Path((period_id, code)): Path<(String, String)>,
    Query(query): Query<QuickWinActivityQuery>,
) -> ApiResult<Json<QuickWinDetailResponse>> {
    let unit = match query.scope.as_deref() {
        Some(scope) if !scope.trim().is_empty() => {
            Some((scope, query.label.as_deref().unwrap_or_default()))
        }
        _ => None,
    };
    let data_scope = user.data_scope();
    // Pembaca tidak boleh mempersempit ke polres lain lewat parameter klien.
    let polres = if data_scope.is_all() {
        query.polres.as_deref()
    } else {
        None
    };
    Ok(Json(
        service::quick_win_detail_for_period(
            &state.db,
            &period_id,
            &code,
            unit,
            polres,
            &data_scope,
        )
        .await?,
    ))
}

async fn read_workbook(mut multipart: Multipart, max_bytes: usize) -> ApiResult<(String, Vec<u8>)> {
    let mut filename = None;
    let mut bytes = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request(format!("Form unggahan tidak valid: {error}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let name = field.file_name().unwrap_or("unggahan.xlsx").to_string();
        if !name.to_ascii_lowercase().ends_with(".xlsx") {
            return Err(ApiError::bad_request(
                "Unggah file Excel berekstensi .xlsx.",
            ));
        }
        let data = field.bytes().await.map_err(ApiError::internal)?;
        if data.len() > max_bytes {
            return Err(ApiError::payload_too_large(format!(
                "Ukuran file maksimal {} MB.",
                max_bytes / 1024 / 1024
            )));
        }
        if !data.starts_with(b"PK") {
            return Err(ApiError::bad_request(
                "Berkas bukan workbook .xlsx yang valid.",
            ));
        }
        filename = Some(name);
        bytes = Some(data.to_vec());
    }

    let filename = filename.ok_or_else(|| ApiError::bad_request("File Excel belum dipilih."))?;
    let bytes = bytes.ok_or_else(|| ApiError::bad_request("File Excel belum dipilih."))?;
    Ok((filename, bytes))
}
