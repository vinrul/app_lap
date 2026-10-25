use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::info;

use crate::{
    app,
    config::Config,
    db,
    modules::{auth::service as auth_service, imports::evidence},
    state::{AppState, EvidenceDownloadLocks},
};

pub async fn run() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    let db = db::connect(&config).await?;

    if let Err(error) = auth_service::cleanup_expired_sessions(&db).await {
        tracing::warn!(%error, "gagal membersihkan sesi kedaluwarsa");
    }

    let http = reqwest::Client::builder()
        .user_agent(concat!(
            "LaporanMingguanPBM/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/vinrul/app_lap)"
        ))
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        pdf_lock: Arc::new(Mutex::new(())),
        evidence_downloads: Arc::new(EvidenceDownloadLocks::default()),
        http,
    };

    match evidence::resume_pending_downloads(
        &state.db,
        state.config.clone(),
        state.evidence_downloads.clone(),
    )
    .await
    {
        Ok(0) => {}
        Ok(count) => {
            info!(count, "melanjutkan unduhan bukti dukung yang tertunda");
        }
        Err(error) => {
            tracing::warn!(%error, "gagal melanjutkan unduhan bukti dukung yang tertunda");
        }
    }

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
