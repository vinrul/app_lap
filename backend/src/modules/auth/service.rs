use std::env;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use rand::TryRng;
use sqlx::{Row, SqlitePool};
use tracing::info;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    modules::auth::models::AuthUser,
};

pub const COOKIE_NAME: &str = "laporan_session";

pub fn random_token() -> ApiResult<String> {
    let mut bytes = [0_u8; 32];
    rand::rngs::SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(ApiError::internal)?;
    Ok(hex::encode(bytes))
}

pub fn hash_password(password: &str) -> ApiResult<String> {
    Argon2::default()
        .hash_password_with_rng(&mut rand::rngs::SysRng, password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(ApiError::internal)
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub async fn authenticate(db: &SqlitePool, username: &str, password: &str) -> ApiResult<AuthUser> {
    let row = sqlx::query(
        "SELECT id, username, role, password_hash
         FROM users WHERE username = ? AND active = 1",
    )
    .bind(username.trim())
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(ApiError::unauthorized)?;

    let stored: String = row.get("password_hash");
    if !verify_password(password, &stored) {
        return Err(ApiError::unauthorized());
    }

    Ok(AuthUser {
        id: row.get("id"),
        username: row.get("username"),
        role: row.get("role"),
    })
}

pub async fn create_session(
    db: &SqlitePool,
    user_id: &str,
    hours: i64,
) -> ApiResult<(String, chrono::DateTime<Utc>)> {
    let token = random_token()?;
    let expires = Utc::now() + Duration::hours(hours);
    sqlx::query("INSERT INTO sessions(token,user_id,expires_at) VALUES(?,?,?)")
        .bind(&token)
        .bind(user_id)
        .bind(expires.to_rfc3339())
        .execute(db)
        .await
        .map_err(ApiError::internal)?;
    Ok((token, expires))
}

pub async fn delete_session(db: &SqlitePool, token: &str) -> ApiResult<()> {
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(token)
        .execute(db)
        .await
        .map_err(ApiError::internal)?;
    Ok(())
}

pub async fn cleanup_expired_sessions(db: &SqlitePool) -> ApiResult<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(Utc::now().to_rfc3339())
        .execute(db)
        .await
        .map_err(ApiError::internal)?;
    Ok(result.rows_affected())
}

pub async fn bootstrap_admin(db: &SqlitePool) -> anyhow::Result<()> {
    let username = env::var("LAPORAN_ADMIN_USER").map_err(|_| {
        anyhow::anyhow!(
            "Atur LAPORAN_ADMIN_USER dan LAPORAN_ADMIN_PASSWORD untuk membuat admin pertama."
        )
    })?;
    let password = env::var("LAPORAN_ADMIN_PASSWORD").map_err(|_| {
        anyhow::anyhow!(
            "Atur LAPORAN_ADMIN_USER dan LAPORAN_ADMIN_PASSWORD untuk membuat admin pertama."
        )
    })?;

    if username.trim().len() < 3 {
        anyhow::bail!("Nama pengguna admin minimal 3 karakter.");
    }
    if password.len() < 12 {
        anyhow::bail!("Kata sandi admin pertama minimal 12 karakter.");
    }

    let admin_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin' AND active = 1")
            .fetch_one(db)
            .await?;
    if admin_count > 0 {
        anyhow::bail!("Administrator sudah tersedia; bootstrap ditolak.");
    }

    sqlx::query("INSERT INTO users(id,username,password_hash,role) VALUES(?,?,?,'admin')")
        .bind(Uuid::new_v4().to_string())
        .bind(username.trim())
        .bind(hash_password(&password)?)
        .execute(db)
        .await?;

    info!(%username, "administrator awal berhasil dibuat");
    Ok(())
}
