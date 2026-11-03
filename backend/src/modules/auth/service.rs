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
        "SELECT id, username, role, polres, satker, password_hash
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

    use sqlx::Row;
    Ok(AuthUser {
        id: row.get("id"),
        username: row.get("username"),
        role: row.get("role"),
        polres: row.get("polres"),
        satker: row.get("satker"),
        token: String::new(),
    })
}

pub async fn change_password(
    db: &SqlitePool,
    user_id: &str,
    current_password: &str,
    new_password: &str,
    keep_token: &str,
) -> ApiResult<()> {
    if current_password.is_empty() {
        return Err(ApiError::bad_request("Kata sandi saat ini wajib diisi."));
    }
    if new_password.chars().count() < 12 {
        return Err(ApiError::bad_request(
            "Kata sandi baru minimal 12 karakter.",
        ));
    }
    if current_password == new_password {
        return Err(ApiError::bad_request(
            "Kata sandi baru harus berbeda dari kata sandi saat ini.",
        ));
    }

    let row = sqlx::query("SELECT password_hash FROM users WHERE id = ? AND active = 1")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(ApiError::unauthorized)?;
    let stored: String = row.get("password_hash");
    if !verify_password(current_password, &stored) {
        return Err(ApiError::bad_request("Kata sandi saat ini tidak sesuai."));
    }

    let hash = hash_password(new_password)?;
    let mut transaction = db.begin().await.map_err(ApiError::internal)?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(hash)
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(ApiError::internal)?;
    // Sesi lain dicabut agar perubahan kata sandi berlaku di semua perangkat.
    sqlx::query("DELETE FROM sessions WHERE user_id = ? AND token <> ?")
        .bind(user_id)
        .bind(keep_token)
        .execute(&mut *transaction)
        .await
        .map_err(ApiError::internal)?;
    transaction.commit().await.map_err(ApiError::internal)?;
    Ok(())
}

pub async fn create_session(
    db: &SqlitePool,
    user_id: &str,
    hours: i64,
) -> ApiResult<(String, chrono::DateTime<Utc>)> {
    create_session_with_context(db, user_id, hours, "", "").await
}

pub async fn create_session_with_context(
    db: &SqlitePool,
    user_id: &str,
    hours: i64,
    user_agent: &str,
    ip: &str,
) -> ApiResult<(String, chrono::DateTime<Utc>)> {
    let token = random_token()?;
    let expires = Utc::now() + Duration::hours(hours);
    sqlx::query(
        "INSERT INTO sessions(token,user_id,expires_at,user_agent,ip,last_seen_at)
         VALUES(?,?,?,?,?,?)",
    )
    .bind(&token)
    .bind(user_id)
    .bind(expires.to_rfc3339())
    .bind(user_agent.chars().take(240).collect::<String>())
    .bind(ip.chars().take(60).collect::<String>())
    .bind(Utc::now().to_rfc3339())
    .execute(db)
    .await
    .map_err(ApiError::internal)?;
    Ok((token, expires))
}

/// Penanda sesi singkat untuk kanal realtime (tidak membocorkan token asli).
pub fn token_hash(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest).chars().take(16).collect()
}

/// Memperbarui waktu terakhir sesi dipakai, paling sering sekali per menit.
pub async fn touch_session(db: &SqlitePool, token: &str) {
    let now = Utc::now().to_rfc3339();
    let cutoff = (Utc::now() - Duration::seconds(60)).to_rfc3339();
    let _ = sqlx::query(
        "UPDATE sessions SET last_seen_at = ?
         WHERE token = ? AND (last_seen_at = '' OR last_seen_at < ?)",
    )
    .bind(now)
    .bind(token)
    .bind(cutoff)
    .execute(db)
    .await;
}

#[derive(Debug, serde::Serialize)]
pub struct SessionView {
    pub id: i64,
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub created_at: String,
    pub expires_at: String,
    pub last_seen_at: String,
    pub ip: String,
    pub user_agent: String,
}

pub async fn list_sessions(db: &SqlitePool) -> ApiResult<Vec<SessionView>> {
    let rows = sqlx::query(
        "SELECT sessions.rowid AS id, sessions.user_id, users.username, users.role,
                sessions.created_at, sessions.expires_at, sessions.last_seen_at,
                sessions.ip, sessions.user_agent
         FROM sessions
         JOIN users ON users.id = sessions.user_id
         WHERE sessions.expires_at > ?
         ORDER BY sessions.last_seen_at DESC, sessions.created_at DESC",
    )
    .bind(Utc::now().to_rfc3339())
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| SessionView {
            id: row.get("id"),
            user_id: row.get("user_id"),
            username: row.get("username"),
            role: row.get("role"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
            last_seen_at: row.get("last_seen_at"),
            ip: row.get("ip"),
            user_agent: row.get("user_agent"),
        })
        .collect())
}

/// Menghapus satu sesi. Mengembalikan `(user_id, token)` bila ada.
pub async fn revoke_session(db: &SqlitePool, id: i64) -> ApiResult<Option<(String, String)>> {
    let row = sqlx::query("SELECT user_id, token FROM sessions WHERE rowid = ?")
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let user_id: String = row.get("user_id");
    let token: String = row.get("token");
    sqlx::query("DELETE FROM sessions WHERE rowid = ?")
        .bind(id)
        .execute(db)
        .await
        .map_err(ApiError::internal)?;
    Ok(Some((user_id, token)))
}

/// Menghapus semua sesi pengguna dan mengembalikan token-tokennya.
pub async fn revoke_user_sessions(db: &SqlitePool, user_id: &str) -> ApiResult<Vec<String>> {
    let tokens: Vec<String> = sqlx::query_scalar("SELECT token FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(db)
        .await
        .map_err(ApiError::internal)?;
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(ApiError::internal)?;
    Ok(tokens)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    async fn setup() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::test_config();
        config.data_dir = dir.path().to_path_buf();
        let db = crate::db::connect(&config).await.unwrap();
        let hash = hash_password("password-lama-123").unwrap();
        sqlx::query(
            "INSERT INTO users(id,username,password_hash,role)
             VALUES('u1','tester',?,'operator')",
        )
        .bind(hash)
        .execute(&db)
        .await
        .unwrap();
        (dir, db)
    }

    #[tokio::test]
    async fn changes_own_password_and_revokes_other_sessions() {
        let (_dir, db) = setup().await;
        let (current, _) = create_session(&db, "u1", 1).await.unwrap();
        let (other, _) = create_session(&db, "u1", 1).await.unwrap();

        let error = change_password(&db, "u1", "salah", "password-baru-123", &current)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Kata sandi saat ini tidak sesuai. (bad_request)"
        );

        change_password(
            &db,
            "u1",
            "password-lama-123",
            "password-baru-123",
            &current,
        )
        .await
        .unwrap();

        let stored: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = 'u1'")
            .fetch_one(&db)
            .await
            .unwrap();
        assert!(verify_password("password-baru-123", &stored));
        assert!(!verify_password("password-lama-123", &stored));

        let current_exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE token = ?")
                .bind(&current)
                .fetch_one(&db)
                .await
                .unwrap();
        let other_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE token = ?")
            .bind(&other)
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(current_exists, 1);
        assert_eq!(other_exists, 0);
    }

    #[tokio::test]
    async fn rejects_invalid_new_password() {
        let (_dir, db) = setup().await;
        let error = change_password(&db, "u1", "password-lama-123", "pendek", "token")
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Kata sandi baru minimal 12 karakter. (bad_request)"
        );

        let error = change_password(&db, "u1", "password-lama-123", "password-lama-123", "token")
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Kata sandi baru harus berbeda dari kata sandi saat ini. (bad_request)"
        );
    }
}
