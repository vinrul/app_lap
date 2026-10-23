use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        auth::service::hash_password,
        users::models::{AdminUserView, CreateUserRequest, UpdateUserRequest},
    },
};

pub async fn list(db: &SqlitePool) -> ApiResult<Vec<AdminUserView>> {
    let rows = sqlx::query(
        "SELECT id, username, role, active, created_at FROM users ORDER BY username COLLATE NOCASE",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminUserView {
            id: row.get("id"),
            username: row.get("username"),
            role: row.get("role"),
            active: row.get::<i64, _>("active") == 1,
            created_at: row.get("created_at"),
        })
        .collect())
}

pub async fn create(db: &SqlitePool, input: &CreateUserRequest) -> ApiResult<AdminUserView> {
    let username = input.username.trim();
    if username.len() < 3 {
        return Err(ApiError::bad_request("Nama pengguna minimal 3 karakter."));
    }
    if input.password.len() < 12 {
        return Err(ApiError::bad_request("Kata sandi minimal 12 karakter."));
    }
    if input.role != "admin" && input.role != "operator" {
        return Err(ApiError::bad_request("Peran harus admin atau operator."));
    }

    let id = Uuid::new_v4().to_string();
    let hash = hash_password(&input.password)?;

    sqlx::query("INSERT INTO users(id,username,password_hash,role) VALUES(?,?,?,?)")
        .bind(&id)
        .bind(username)
        .bind(hash)
        .bind(&input.role)
        .execute(db)
        .await
        .map_err(|error| {
            if let sqlx::Error::Database(database_error) = &error {
                if database_error.is_unique_violation() {
                    return ApiError::conflict("Nama pengguna sudah digunakan.");
                }
            }
            ApiError::internal(error)
        })?;

    let row = sqlx::query("SELECT created_at FROM users WHERE id = ?")
        .bind(&id)
        .fetch_one(db)
        .await
        .map_err(ApiError::internal)?;

    Ok(AdminUserView {
        id,
        username: username.to_string(),
        role: input.role.clone(),
        active: true,
        created_at: row.get("created_at"),
    })
}

pub async fn update(
    db: &SqlitePool,
    actor_id: &str,
    user_id: &str,
    input: &UpdateUserRequest,
) -> ApiResult<AdminUserView> {
    let row = sqlx::query("SELECT username,role,active,created_at FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Pengguna tidak ditemukan."))?;

    let username: String = row.get("username");
    let current_role: String = row.get("role");
    let current_active = row.get::<i64, _>("active") == 1;
    let role = input.role.clone().unwrap_or(current_role.clone());
    let active = input.active.unwrap_or(current_active);

    if role != "admin" && role != "operator" {
        return Err(ApiError::bad_request("Peran harus admin atau operator."));
    }
    if actor_id == user_id && (!active || role != "admin") {
        return Err(ApiError::bad_request(
            "Anda tidak dapat menonaktifkan atau menurunkan peran akun sendiri.",
        ));
    }

    if current_role == "admin" && current_active && (!active || role != "admin") {
        let admin_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role='admin' AND active=1")
                .fetch_one(db)
                .await
                .map_err(ApiError::internal)?;
        if admin_count <= 1 {
            return Err(ApiError::bad_request(
                "Minimal harus ada satu administrator aktif.",
            ));
        }
    }

    sqlx::query("UPDATE users SET role = ?, active = ? WHERE id = ?")
        .bind(&role)
        .bind(i64::from(active))
        .bind(user_id)
        .execute(db)
        .await
        .map_err(ApiError::internal)?;

    Ok(AdminUserView {
        id: user_id.to_string(),
        username,
        role,
        active,
        created_at: row.get("created_at"),
    })
}
