use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        auth::AuthUser,
        imports::{
            models::{ImportPreview, ImportView, RecordQuery, RecordsResponse},
            service,
        },
        periods,
    },
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<Vec<ImportView>>> {
    Ok(Json(service::list_for_period(&state.db, &period_id).await?))
}

pub async fn preview(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
    multipart: Multipart,
) -> ApiResult<Json<ImportPreview>> {
    let (start_date, end_date, _) = periods::service::dates(&state.db, &period_id).await?;
    let (_, bytes) = read_workbook(multipart, state.config.max_upload_bytes).await?;
    Ok(Json(service::preview(bytes, &start_date, &end_date).await?))
}

pub async fn commit(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    multipart: Multipart,
) -> ApiResult<Json<ImportView>> {
    let (filename, bytes) = read_workbook(multipart, state.config.max_upload_bytes).await?;
    Ok(Json(
        service::commit(
            &state.db,
            &state.config,
            &period_id,
            &user.id,
            &filename,
            bytes,
        )
        .await?,
    ))
}

pub async fn records(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<RecordQuery>,
) -> ApiResult<Json<RecordsResponse>> {
    Ok(Json(service::records(&state.db, &period_id, &query).await?))
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
