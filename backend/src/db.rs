use std::time::Duration;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::config::Config;

pub async fn connect(config: &Config) -> anyhow::Result<SqlitePool> {
    tokio::fs::create_dir_all(&config.data_dir).await?;
    let db_path = config.data_dir.join("laporan.sqlite3");
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(15));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .min_connections(1)
        .connect_with(options)
        .await?;

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}
