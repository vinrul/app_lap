use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        auth::AuthUser,
        reports::{
            layout::{
                ApplyTemplateRequest, DesignSource, DesignTemplateView, LayoutView,
                SaveLayoutRequest, SaveTemplateRequest, UploadedImage,
            },
            models::{
                DraftView, GenerateReportRequest, PreviewReportRequest, PreviewResponse,
                ReportContent, ReportMetrics, ReportView,
            },
            service, sources,
        },
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub import_id: Option<String>,
}

pub async fn metrics(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<MetricsQuery>,
) -> ApiResult<Json<ReportMetrics>> {
    Ok(Json(
        service::metrics_for(
            &state.db,
            &state.config,
            &period_id,
            query.import_id.as_deref(),
        )
        .await?,
    ))
}

pub async fn get_draft(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((period_id, import_id)): Path<(String, String)>,
) -> ApiResult<Json<DraftView>> {
    Ok(Json(
        service::get_draft(&state.db, &period_id, &import_id).await?,
    ))
}

pub async fn save_draft(
    State(state): State<AppState>,
    user: AuthUser,
    Path((period_id, import_id)): Path<(String, String)>,
    Json(content): Json<ReportContent>,
) -> ApiResult<Json<DraftView>> {
    Ok(Json(
        service::save_draft(&state.db, &user.id, &period_id, &import_id, &content).await?,
    ))
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<Vec<ReportView>>> {
    Ok(Json(service::list_for_period(&state.db, &period_id).await?))
}

#[derive(Debug, Deserialize)]
pub struct LayoutQuery {
    pub import_id: Option<String>,
}

pub async fn get_layout(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<LayoutQuery>,
) -> ApiResult<Json<LayoutView>> {
    let import_id = query
        .import_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("Versi impor wajib dipilih."))?;
    Ok(Json(
        service::get_layout(&state.db, &period_id, &import_id).await?,
    ))
}

pub async fn save_layout(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<SaveLayoutRequest>,
) -> ApiResult<Json<LayoutView>> {
    if input.import_id.trim().is_empty() {
        return Err(ApiError::bad_request("Versi impor wajib dipilih."));
    }
    Ok(Json(
        service::save_layout(
            &state.db,
            &user.id,
            &period_id,
            &input.import_id,
            input.layout,
        )
        .await?,
    ))
}

pub async fn design_sources(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<LayoutQuery>,
) -> ApiResult<Json<Vec<DesignSource>>> {
    let import_id = query
        .import_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("Versi impor wajib dipilih."))?;
    let metrics =
        service::metrics_for(&state.db, &state.config, &period_id, Some(&import_id)).await?;
    Ok(Json(sources::design_sources(&metrics)))
}

pub async fn list_templates(
    State(state): State<AppState>,
    _user: AuthUser,
) -> ApiResult<Json<Vec<DesignTemplateView>>> {
    Ok(Json(service::list_templates(&state.db).await?))
}

pub async fn save_template(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<SaveTemplateRequest>,
) -> ApiResult<Json<DesignTemplateView>> {
    Ok(Json(
        service::save_template(&state.db, &user.id, &input.name, input.layout).await?,
    ))
}

pub async fn delete_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path(template_id): Path<String>,
) -> ApiResult<StatusCode> {
    service::delete_template(&state.db, &user.id, &template_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn apply_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((period_id, template_id)): Path<(String, String)>,
    Json(input): Json<ApplyTemplateRequest>,
) -> ApiResult<Json<LayoutView>> {
    if input.import_id.trim().is_empty() {
        return Err(ApiError::bad_request("Versi impor wajib dipilih."));
    }
    Ok(Json(
        service::apply_template(
            &state.db,
            &state.config,
            &user.id,
            &period_id,
            &input.import_id,
            &template_id,
        )
        .await?,
    ))
}

pub async fn upload_layout_image(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    mut multipart: Multipart,
) -> ApiResult<Json<UploadedImage>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request(format!("Berkas tidak dapat dibaca: {error}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("gambar").to_string();
        let bytes = field.bytes().await.map_err(|error| {
            ApiError::bad_request(format!("Berkas tidak dapat dibaca: {error}"))
        })?;
        return Ok(Json(
            service::save_layout_image(
                &state.db,
                &state.config,
                &user.id,
                &period_id,
                &filename,
                &bytes,
            )
            .await?,
        ));
    }
    Err(ApiError::bad_request(
        "Berkas gambar wajib diunggah pada field 'file'.",
    ))
}

pub async fn layout_image(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((period_id, image_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    let (path, content_type, _filename) =
        service::layout_image(&state.db, &period_id, &image_id).await?;
    let bytes = tokio::fs::read(path).await.map_err(ApiError::internal)?;
    let content_type = HeaderValue::from_str(&content_type).map_err(ApiError::internal)?;

    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    Ok(response)
}

pub async fn generate(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<GenerateReportRequest>,
) -> ApiResult<Json<ReportView>> {
    if input.import_id.trim().is_empty() {
        return Err(ApiError::bad_request("Versi impor wajib dipilih."));
    }
    if let Some(content) = input.content.as_ref() {
        service::save_draft(&state.db, &user.id, &period_id, &input.import_id, content).await?;
    }

    let report = service::generate(
        &state.db,
        &state.config,
        &state.pdf_lock,
        &user.id,
        &period_id,
        &input.import_id,
    )
    .await?;

    Ok(Json(report))
}

pub async fn preview_render(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<PreviewReportRequest>,
) -> ApiResult<Json<PreviewResponse>> {
    if input.import_id.trim().is_empty() {
        return Err(ApiError::bad_request("Versi impor wajib dipilih."));
    }
    let html = service::preview_html(
        &state.db,
        &state.config,
        &period_id,
        &input.import_id,
        input.content.as_ref(),
    )
    .await?;
    Ok(Json(PreviewResponse { html }))
}

pub async fn preview_stored(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(report_id): Path<String>,
) -> ApiResult<Html<String>> {
    Ok(Html(
        service::stored_preview(&state.db, &state.config, &report_id).await?,
    ))
}

pub async fn download(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(report_id): Path<String>,
) -> ApiResult<Response> {
    let (path, filename) = service::download_path(&state.db, &state.config, &report_id).await?;
    let bytes = tokio::fs::read(path).await.map_err(ApiError::internal)?;
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        .map_err(ApiError::internal)?;

    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    response
        .headers_mut()
        .insert(header::CONTENT_DISPOSITION, disposition);
    Ok(response)
}

#[allow(dead_code)]
fn _json_error(message: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message }))).into_response()
}
