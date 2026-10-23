mod app;
mod config;
mod db;
mod error;
mod middleware;
mod modules;
mod server;
mod shared;
mod state;

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let command = env::args().nth(1).unwrap_or_else(|| "serve".into());
    match command.as_str() {
        "serve" => server::run().await,
        "bootstrap-admin" => {
            let config = config::Config::from_env()?;
            let db = db::connect(&config).await?;
            modules::auth::service::bootstrap_admin(&db).await
        }
        other => anyhow::bail!(
            "Perintah tidak dikenal: {other}. Gunakan 'serve' atau 'bootstrap-admin'."
        ),
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,laporan_mingguan=debug".into());
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
