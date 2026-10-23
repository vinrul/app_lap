use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::info;

use crate::{app, config::Config, db, modules::auth::service as auth_service, state::AppState};

pub async fn run() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    let db = db::connect(&config).await?;

    if let Err(error) = auth_service::cleanup_expired_sessions(&db).await {
        tracing::warn!(%error, "gagal membersihkan sesi kedaluwarsa");
    }

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        pdf_lock: Arc::new(Mutex::new(())),
    };
    let router = app::build_router(state);

    let listener = tokio::net::TcpListener::bind((config.bind_addr, config.port)).await?;
    info!(
        "Aplikasi berjalan pada http://{}:{}",
        config.bind_addr, config.port
    );
    if !config.bind_addr.is_loopback() {
        if config.allowed_hosts.is_empty() {
            tracing::warn!(
                "Bind non-loopback tanpa LAPORAN_ALLOWED_HOSTS: semua Host header diterima. \
                 Set LAPORAN_ALLOWED_HOSTS untuk membatasi akses."
            );
        } else {
            tracing::info!(
                hosts = ?config.allowed_hosts,
                "Membatasi Host header ke daftar LAPORAN_ALLOWED_HOSTS"
            );
        }
    }
    axum::serve(listener, router).await?;
    Ok(())
}
