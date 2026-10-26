use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
};

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        auth::AuthUser,
        imports::{
            evidence,
            models::{
                AnalysisResponse, CoverageResponse, ImportPreview, ImportView,
                QuickWinDetailResponse, RecordQuery,
                RecordsResponse, UnitDetailResponse,
            },
            service,
        },
    },
    state::AppState,
};

pub async fn list(State(state): State<AppState>, _user: AuthUser) -> ApiResult<Json<Vec<ImportView>>> {
    Ok(Json(service::list(&state.db).await?))
}

pub async fn preview(
    State(state): State<AppState>,
    _user: AuthUser,
    multipart: Multipart,
) -> ApiResult<Json<ImportPreview>> {
    let (_, bytes) = read_workbook(multipart, state.config.max_upload_bytes).await?;
    Ok(Json(service::preview(bytes).await?))
}

pub async fn commit(
    State(state): State<AppState>,
    user: AuthUser,
    multipart: Multipart,
) -> ApiResult<Json<ImportView>> {
    let (filename, bytes) = read_workbook(multipart, state.config.max_upload_bytes).await?;
    let view = service::commit(&state.db, &state.config, &user.id, &filename, bytes).await?;
    if !view.id.is_empty() {
        evidence::spawn_import_downloads(
            state.db.clone(),
            state.config.clone(),
            state.evidence_downloads.clone(),
            view.id.clone(),
        );
    }
    Ok(Json(view))
}

pub async fn retry_evidence(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(import_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let queued = evidence::retry_import(
        &state.db,
        state.config.clone(),
        state.evidence_downloads.clone(),
        &import_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "queued": queued })))
}

pub async fn evidence_file(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(evidence_id): Path<String>,
) -> ApiResult<Response> {
    let (bytes, content_type, filename) = evidence::open(&state.db, &evidence_id).await?;
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
        if safe_name.is_empty() { "bukti-dukung" } else { &safe_name }
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
    _user: AuthUser,
    Path(import_id): Path<String>,
    Query(query): Query<RecordQuery>,
) -> ApiResult<Json<RecordsResponse>> {
    Ok(Json(
        service::records_for_import(&state.db, &import_id, &query).await?,
    ))
}

pub async fn period_records(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<RecordQuery>,
) -> ApiResult<Json<RecordsResponse>> {
    Ok(Json(
        service::records_for_period(&state.db, &period_id, &query).await?,
    ))
}

pub async fn analysis(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<AnalysisResponse>> {
    Ok(Json(service::analysis_for_period(&state.db, &period_id).await?))
}

pub async fn coverage(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<CoverageResponse>> {
    Ok(Json(service::coverage_for_period(&state.db, &period_id).await?))
}

#[derive(Debug, serde::Deserialize)]
pub struct UnitDetailQuery {
    pub scope: String,
    pub label: String,
}

pub async fn unit_detail(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<UnitDetailQuery>,
) -> ApiResult<Json<UnitDetailResponse>> {
    Ok(Json(
        service::unit_detail_for_period(
            &state.db,
            &state.config,
            &period_id,
            &query.scope,
            &query.label,
        )
        .await?,
    ))
}

pub async fn quick_win_detail(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((period_id, code)): Path<(String, String)>,
) -> ApiResult<Json<QuickWinDetailResponse>> {
    Ok(Json(
        service::quick_win_detail_for_period(&state.db, &period_id, &code).await?,
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
