use std::time::Duration;

use sqlx::{
    Connection, SqlitePool,
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
        .connect_with(options.clone())
        .await?;

    // Migrasi berjalan tanpa penegakan foreign key agar tabel lama dapat
    // dibangun ulang (mis. memisahkan imports dari report_periods).
    let mut migration = sqlx::SqliteConnection::connect_with(&options.foreign_keys(false)).await?;
    sqlx::migrate!().run(&mut migration).await?;
    migration.close().await?;

    Ok(pool)
}
