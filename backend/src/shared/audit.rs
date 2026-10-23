use serde_json::Value;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::ApiResult;

/// Mencatat aksi penting tanpa menghentikan alur utama jika audit gagal.
pub async fn record(
    db: &SqlitePool,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    details: Value,
) -> ApiResult<()> {
    let result = sqlx::query(
        "INSERT INTO audit_events(user_id,action,entity_type,entity_id,details_json)
         VALUES(?,?,?,?,?)",
    )
    .bind(user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details.to_string())
    .execute(db)
    .await;

    log_failure(result, action);
    Ok(())
}

/// Versi transaksi agar audit ikut rollback bila aksi utama gagal.
pub async fn record_tx(
    tx: &mut Transaction<'_, Sqlite>,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    details: Value,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO audit_events(user_id,action,entity_type,entity_id,details_json)
         VALUES(?,?,?,?,?)",
    )
    .bind(user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|error| crate::error::ApiError::internal(error))?;
    Ok(())
}

fn log_failure(result: Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error>, action: &str) {
    if let Err(error) = result {
        tracing::warn!(%error, action, "gagal menulis audit");
    }
}
