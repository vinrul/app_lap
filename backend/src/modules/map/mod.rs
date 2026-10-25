use std::path::PathBuf;

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
    routing::get,
};

use crate::{
    error::{ApiError, ApiResult},
    modules::auth::AuthUser,
    state::AppState,
};

const MAX_TILE_BYTES: usize = 2 * 1024 * 1024;
const MAX_ZOOM: u32 = 19;

pub fn routes() -> Router<AppState> {
    Router::new().route("/map/tiles/{z}/{x}/{y}", get(tile))
}

async fn tile(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((z, x, y)): Path<(u32, u32, u32)>,
) -> ApiResult<Response> {
    if !valid_tile(z, x, y) {
        return Err(ApiError::bad_request("Koordinat tile peta tidak valid."));
    }

    let cache_path = tile_cache_path(&state, z, x, y);
    if let Ok(bytes) = tokio::fs::read(&cache_path).await {
        return tile_response(bytes);
    }

    let upstream = format!("https://tile.openstreetmap.org/{z}/{x}/{y}.png");
    let response = state.http.get(&upstream).send().await.map_err(|error| {
        tracing::warn!(%error, url = upstream, "gagal mengambil tile peta");
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "tile_unavailable",
            "Tile peta gagal dimuat. Periksa koneksi internet server.",
        )
    })?;

    let status = response.status();
    if !status.is_success() {
        tracing::warn!(%status, url = upstream, "server tile menolak permintaan");
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "tile_unavailable",
            format!("Penyedia peta menolak permintaan tile (HTTP {status})."),
        ));
    }
    if response.content_length().unwrap_or(0) > MAX_TILE_BYTES as u64 {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "tile_unavailable",
            "Ukuran tile peta melebihi batas.",
        ));
    }

    let bytes = response.bytes().await.map_err(|error| {
        tracing::warn!(%error, url = upstream, "gagal membaca tile peta");
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "tile_unavailable",
            "Isi tile peta tidak dapat dibaca.",
        )
    })?;
    if bytes.len() > MAX_TILE_BYTES || !bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "tile_unavailable",
            "Penyedia peta mengembalikan berkas tile yang tidak valid.",
        ));
    }

    if let Some(parent) = cache_path.parent() {
        if let Err(error) = tokio::fs::create_dir_all(parent).await {
            tracing::warn!(%error, path = %parent.display(), "gagal membuat cache tile peta");
        } else if let Err(error) = tokio::fs::write(&cache_path, &bytes).await {
            tracing::warn!(%error, path = %cache_path.display(), "gagal menyimpan cache tile peta");
        }
    }

    tile_response(bytes.to_vec())
}

fn valid_tile(z: u32, x: u32, y: u32) -> bool {
    if z > MAX_ZOOM {
        return false;
    }
    let limit = 1_u32 << z;
    x < limit && y < limit
}

fn tile_cache_path(state: &AppState, z: u32, x: u32, y: u32) -> PathBuf {
    state
        .config
        .data_dir
        .join("map-tiles")
        .join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.png"))
}

fn tile_response(bytes: Vec<u8>) -> ApiResult<Response> {
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/png"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=604800"),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    #[test]
    fn validates_tile_coordinates() {
        assert!(super::valid_tile(0, 0, 0));
        assert!(super::valid_tile(19, 524_287, 524_287));
        assert!(!super::valid_tile(20, 0, 0));
        assert!(!super::valid_tile(2, 4, 0));
        assert!(!super::valid_tile(2, 0, 4));
    }
}
