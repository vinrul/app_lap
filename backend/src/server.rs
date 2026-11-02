use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;
use tracing::info;

use crate::{
    app,
    config::Config,
    db,
    modules::{auth::service as auth_service, imports::evidence, reports},
    state::{AppState, EvidenceDownloadLocks, ShutdownSignal},
};

pub async fn run() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    if config.google_drive_api_key.is_none() {
        tracing::info!(
            "LAPORAN_GOOGLE_DRIVE_API_KEY tidak diatur; tautan folder Google Drive tidak akan diperluas."
        );
    } else {
        tracing::info!(
            "LAPORAN_GOOGLE_DRIVE_API_KEY terdeteksi; dukungan folder Google Drive publik aktif."
        );
    }
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
        design_live: Arc::new(reports::live::DesignLive::default()),
        design_lock: Arc::new(Mutex::new(())),
        shutdown: ShutdownSignal::new(),
        http,
    };

    match evidence::resume_pending_downloads(
        &state.db,
        state.config.clone(),
        state.evidence_downloads.clone(),
        state.shutdown.clone(),
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

    let router = app::build_router(state.clone());

    // Permintaan pertama setelah server hidup tidak perlu menanggung
    // pembacaan disk dari nol untuk periode yang paling sering dibuka.
    tokio::spawn(warm_latest_period(state.clone()));

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
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal(state.shutdown.clone()))
        .await?;

    info!("server berhenti menerima permintaan; menyelesaikan pekerjaan latar belakang");
    state.shutdown.trigger();

    if !wait_for_evidence(&state.evidence_downloads, Duration::from_secs(15)).await {
        tracing::warn!("worker bukti dukung belum selesai dalam batas waktu; keluar");
    }

    match tokio::time::timeout(Duration::from_secs(5), state.db.close()).await {
        Ok(()) => info!("database ditutup dengan rapi"),
        Err(_) => tracing::warn!("penutupan database melewati batas waktu"),
    }

    info!("server berhenti");
    Ok(())
}

/// Menunggu Ctrl+C atau SIGTERM (dikirim systemd saat stop/restart).
async fn shutdown_signal(shutdown: ShutdownSignal) {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "gagal mendengarkan sinyal Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "gagal mendengarkan sinyal SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    info!("sinyal berhenti diterima; menutup server dengan rapi");
    shutdown.trigger();
}

async fn wait_for_evidence(locks: &EvidenceDownloadLocks, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if locks.is_idle() {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn warm_latest_period(state: AppState) {
    let period_id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM report_periods
         WHERE archived_at IS NULL
         ORDER BY start_date DESC, created_at DESC
         LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await;

    let Ok(Some(period_id)) = period_id else {
        return;
    };

    let started = std::time::Instant::now();
    match reports::service::metrics_for(&state.db, &state.config, &period_id).await {
        Ok(metrics) => info!(
            period_id = %period_id,
            rows = metrics.total_rows,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "cache periode terbaru siap"
        ),
        Err(error) => tracing::warn!(%error, "gagal memanaskan cache periode terbaru"),
    }
}
