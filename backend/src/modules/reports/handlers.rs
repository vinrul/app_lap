use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde_json::json;

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        auth::AuthUser,
        live::ServerMessage,
        notifications,
        reports::{
            layout::{
                DesignSource, DesignTemplateView, ImportMediaRequest, LayoutPatch, LayoutView,
                MediaFilters, MediaItem, PatchLayoutRequest, SaveLayoutRequest,
                SaveTemplateRequest, UploadedImage,
            },
            models::{
                DraftView, GenerateReportRequest, PreviewReportRequest, PreviewResponse,
                PublicShareView, ReportContent, ReportMetrics, ReportShareView, ReportView,
            },
            service, sources,
        },
    },
    state::AppState,
};

pub async fn metrics(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<ReportMetrics>> {
    Ok(Json(
        service::metrics_for_scope(&state.db, &state.config, &period_id, &user.data_scope())
            .await?,
    ))
}

pub async fn get_draft(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<DraftView>> {
    user.require_writer()?;
    Ok(Json(service::get_draft(&state.db, &period_id).await?))
}

pub async fn save_draft(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(content): Json<ReportContent>,
) -> ApiResult<Json<DraftView>> {
    user.require_writer()?;
    Ok(Json(
        service::save_draft(&state.db, &user.id, &period_id, &content).await?,
    ))
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<Vec<ReportView>>> {
    Ok(Json(service::list_for_period(&state.db, &period_id).await?))
}

pub async fn get_layout(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<LayoutView>> {
    user.require_writer()?;
    Ok(Json(service::get_layout(&state.db, &period_id).await?))
}

pub async fn save_layout(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<SaveLayoutRequest>,
) -> ApiResult<Json<LayoutView>> {
    user.require_writer()?;
    let _guard = state.design_lock.lock().await;
    let view = service::save_layout(&state.db, &user.id, &period_id, input.layout).await?;
    state
        .hub
        .publish(
            &period_id,
            ServerMessage::DesignPatch {
                period_id: period_id.clone(),
                revision: view.revision,
                client_id: String::new(),
                updated_at: view.updated_at.clone(),
                updated_by: view.updated_by.clone(),
                patch: LayoutPatch {
                    replace: Some(view.layout.clone()),
                    ..LayoutPatch::default()
                },
                layout: None,
            },
        )
        .await;
    Ok(Json(view))
}

/// Menyimpan perubahan kecil dari editor desain. Patch diterapkan ke layout
/// terbaru di server, lalu disiarkan ke semua editor periode tersebut.
pub async fn patch_layout(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<PatchLayoutRequest>,
) -> ApiResult<Json<LayoutView>> {
    user.require_writer()?;
    let _guard = state.design_lock.lock().await;
    let view = service::apply_patch(&state.db, &user.id, &period_id, &input.patch).await?;
    let snapshot = ServerMessage::snapshot(&view.layout);
    state
        .hub
        .publish(
            &period_id,
            ServerMessage::DesignPatch {
                period_id: period_id.clone(),
                revision: view.revision,
                client_id: input.client_id,
                updated_at: view.updated_at.clone(),
                updated_by: view.updated_by.clone(),
                patch: input.patch,
                layout: snapshot,
            },
        )
        .await;
    Ok(Json(view))
}

pub async fn design_sources(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<Vec<DesignSource>>> {
    user.require_writer()?;
    let metrics = service::metrics_for(&state.db, &state.config, &period_id).await?;
    Ok(Json(sources::design_sources(&metrics)))
}

pub async fn list_templates(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<DesignTemplateView>>> {
    user.require_writer()?;
    Ok(Json(service::list_templates(&state.db).await?))
}

pub async fn save_template(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<SaveTemplateRequest>,
) -> ApiResult<Json<DesignTemplateView>> {
    user.require_writer()?;
    Ok(Json(
        service::save_template(&state.db, &user.id, &input.name, input.layout).await?,
    ))
}

pub async fn delete_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path(template_id): Path<String>,
) -> ApiResult<StatusCode> {
    user.require_writer()?;
    service::delete_template(&state.db, &user.id, &template_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn apply_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((period_id, template_id)): Path<(String, String)>,
) -> ApiResult<Json<LayoutView>> {
    user.require_writer()?;
    let _guard = state.design_lock.lock().await;
    let view =
        service::apply_template(&state.db, &state.config, &user.id, &period_id, &template_id)
            .await?;
    state
        .hub
        .publish(
            &period_id,
            ServerMessage::DesignPatch {
                period_id: period_id.clone(),
                revision: view.revision,
                client_id: String::new(),
                updated_at: view.updated_at.clone(),
                updated_by: view.updated_by.clone(),
                patch: LayoutPatch {
                    replace: Some(view.layout.clone()),
                    ..LayoutPatch::default()
                },
                layout: None,
            },
        )
        .await;
    Ok(Json(view))
}

pub async fn upload_layout_image(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    mut multipart: Multipart,
) -> ApiResult<Json<UploadedImage>> {
    user.require_writer()?;
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
    user: AuthUser,
    Path((period_id, image_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    user.require_writer()?;
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

pub async fn design_media(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Query(query): Query<MediaQuery>,
) -> ApiResult<Json<Vec<MediaItem>>> {
    user.require_writer()?;
    Ok(Json(
        service::list_media(
            &state.db,
            &period_id,
            query.kode.as_deref(),
            query.polres.as_deref(),
            query.satker.as_deref(),
        )
        .await?,
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaQuery {
    pub kode: Option<String>,
    pub polres: Option<String>,
    pub satker: Option<String>,
}

pub async fn design_media_filters(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
) -> ApiResult<Json<MediaFilters>> {
    user.require_writer()?;
    Ok(Json(
        service::list_media_filters(&state.db, &period_id).await?,
    ))
}

pub async fn import_media(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<ImportMediaRequest>,
) -> ApiResult<Json<UploadedImage>> {
    user.require_writer()?;
    Ok(Json(
        service::import_media_image(
            &state.db,
            &state.config,
            &user.id,
            &period_id,
            &input.source,
            &input.id,
        )
        .await?,
    ))
}

pub async fn generate(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<GenerateReportRequest>,
) -> ApiResult<Json<ReportView>> {
    user.require_writer()?;
    if let Some(content) = input.content.as_ref() {
        service::save_draft(&state.db, &user.id, &period_id, content).await?;
    }

    let request_id = input
        .request_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let job = format!("pdf:{request_id}");
    let handle = state.jobs.start(&job);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let task_state = state.clone();
    let task_handle = handle.clone();
    let task_job = job.clone();
    let user_id = user.id.clone();
    let task_period = period_id.clone();
    let task = tokio::spawn(async move {
        let result = service::generate(
            &task_state.db,
            &task_state.config,
            &task_state.pdf_lock,
            &task_state.hub,
            &task_job,
            &task_handle,
            &user_id,
            &task_period,
        )
        .await;
        let _ = sender.send(result);
    });
    handle.attach_task(task);

    let outcome = receiver.await;
    handle.detach_task();
    state.jobs.finish(&job);

    let (status, result) = match outcome {
        Ok(Ok(report)) => ("done", Ok(report)),
        Ok(Err(error)) => (
            if handle.is_cancelled() {
                "cancelled"
            } else {
                "failed"
            },
            Err(error),
        ),
        Err(_) => (
            "cancelled",
            Err(ApiError::bad_request("Pembuatan PDF dibatalkan.")),
        ),
    };
    state
        .hub
        .publish(
            &job,
            ServerMessage::JobFinished {
                job: job.clone(),
                status: status.to_string(),
                total: 0,
                ready: 0,
                failed: 0,
                skipped: 0,
                pending: 0,
            },
        )
        .await;

    let url = format!("/#/periods/{period_id}/design");
    match status {
        "done" => {
            notifications::notify(
                &state.db,
                &state.hub,
                &user.id,
                "report",
                "info",
                "PDF laporan siap",
                "PDF laporan berhasil dibuat dan siap dibuka.",
                &url,
            )
            .await;
        }
        "failed" => {
            notifications::notify(
                &state.db,
                &state.hub,
                &user.id,
                "report",
                "error",
                "Pembuatan PDF gagal",
                "Buka editor desain lalu coba lagi. Periksa juga pengaturan Chrome/Edge.",
                &url,
            )
            .await;
        }
        _ => {}
    }

    Ok(Json(result?))
}

pub async fn cancel_report(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_period_id, request_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    user.require_writer()?;
    let cancelled = state.jobs.cancel(&format!("pdf:{request_id}"));
    Ok(Json(serde_json::json!({ "cancelled": cancelled })))
}

pub async fn preview_render(
    State(state): State<AppState>,
    user: AuthUser,
    Path(period_id): Path<String>,
    Json(input): Json<PreviewReportRequest>,
) -> ApiResult<Json<PreviewResponse>> {
    user.require_writer()?;
    let html = service::preview_html(
        &state.db,
        &state.config,
        &period_id,
        input.content.as_ref(),
        input.layout.as_ref(),
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
    Ok(pdf_response(path, &filename, "attachment").await?)
}

pub async fn delete_report(
    State(state): State<AppState>,
    user: AuthUser,
    Path(report_id): Path<String>,
) -> ApiResult<StatusCode> {
    user.require_writer()?;
    service::delete_report(&state.db, &state.config, &user.id, &report_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_share(
    State(state): State<AppState>,
    user: AuthUser,
    Path(report_id): Path<String>,
) -> ApiResult<Json<ReportShareView>> {
    user.require_writer()?;
    Ok(Json(
        service::create_share(&state.db, &user.id, &report_id).await?,
    ))
}

pub async fn public_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<PublicShareView>> {
    Ok(Json(service::public_share(&state.db, &token).await?))
}

pub async fn public_preview(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Response> {
    let html = service::public_preview(&state.db, &token).await?;
    let mut response = Response::new(Body::from(html));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-robots-tag",
        HeaderValue::from_static("noindex, nofollow"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; img-src data: https: http:; style-src 'unsafe-inline'; font-src data:",
        ),
    );
    Ok(response)
}

pub async fn public_download(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Response> {
    let (path, filename) = service::public_download_path(&state.db, &token).await?;
    Ok(pdf_response(path, &filename, "inline").await?)
}

async fn pdf_response(
    path: std::path::PathBuf,
    filename: &str,
    disposition_type: &str,
) -> ApiResult<Response> {
    let bytes = tokio::fs::read(path).await.map_err(ApiError::internal)?;
    let disposition =
        HeaderValue::from_str(&format!("{disposition_type}; filename=\"{filename}\""))
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
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-robots-tag",
        HeaderValue::from_static("noindex, nofollow"),
    );
    Ok(response)
}

#[allow(dead_code)]
fn _json_error(message: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message }))).into_response()
}
