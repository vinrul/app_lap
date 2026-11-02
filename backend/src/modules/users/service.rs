use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        auth::service::hash_password,
        users::models::{AdminUserView, CreateUserRequest, UpdateUserRequest, UserOptions},
    },
};

const ROLES: [&str; 4] = ["admin", "operator", "polres", "polda"];

fn validate_role(role: &str) -> ApiResult<()> {
    if ROLES.contains(&role) {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "Peran harus admin, operator, polres, atau polda.",
        ))
    }
}

/// Peran `polres` wajib menunjuk satu polres; peran `polda` boleh menunjuk
/// satu satker atau seluruh satker Polda. Peran lain tidak memakai keduanya.
fn assignment(role: &str, polres: &str, satker: &str) -> ApiResult<(String, String)> {
    let polres = polres.trim().to_string();
    let satker = satker.trim().to_string();
    match role {
        "polres" => {
            if polres.is_empty() {
                return Err(ApiError::bad_request(
                    "Peran polres wajib memiliki nama polres.",
                ));
            }
            Ok((polres, String::new()))
        }
        "polda" => Ok((String::new(), satker)),
        _ => Ok((String::new(), String::new())),
    }
}

pub async fn list(db: &SqlitePool) -> ApiResult<Vec<AdminUserView>> {
    let rows = sqlx::query(
        "SELECT id, username, role, polres, satker, active, created_at
         FROM users ORDER BY username COLLATE NOCASE",
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
            polres: row.get("polres"),
            satker: row.get("satker"),
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
    validate_role(&input.role)?;
    let (polres, satker) = assignment(
        &input.role,
        input.polres.as_deref().unwrap_or_default(),
        input.satker.as_deref().unwrap_or_default(),
    )?;

    let id = Uuid::new_v4().to_string();
    let hash = hash_password(&input.password)?;

    sqlx::query(
        "INSERT INTO users(id,username,password_hash,role,polres,satker)
         VALUES(?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(username)
    .bind(hash)
    .bind(&input.role)
    .bind(&polres)
    .bind(&satker)
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
        polres,
        satker,
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
    let row = sqlx::query(
        "SELECT username, role, polres, satker, active, created_at
         FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Pengguna tidak ditemukan."))?;

    let username: String = row.get("username");
    let current_role: String = row.get("role");
    let current_polres: String = row.get("polres");
    let current_satker: String = row.get("satker");
    let current_active = row.get::<i64, _>("active") == 1;
    let role = input.role.clone().unwrap_or(current_role.clone());
    let active = input.active.unwrap_or(current_active);

    validate_role(&role)?;
    let (polres, satker) = assignment(
        &role,
        input.polres.as_deref().unwrap_or(&current_polres),
        input.satker.as_deref().unwrap_or(&current_satker),
    )?;

    let new_password = input.password.as_deref().filter(|value| !value.is_empty());
    if let Some(password) = new_password {
        if password.chars().count() < 12 {
            return Err(ApiError::bad_request("Kata sandi minimal 12 karakter."));
        }
        // Reset oleh administrator hanya untuk akun operasional. Kata sandi
        // administrator tetap diubah pemiliknya lewat menu Ganti kata sandi.
        if current_role == "admin" || role == "admin" {
            return Err(ApiError::bad_request(
                "Kata sandi akun administrator hanya dapat diubah oleh pemiliknya.",
            ));
        }
    }
    let password_hash = new_password.map(hash_password).transpose()?;

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

    let mut transaction = db.begin().await.map_err(ApiError::internal)?;
    sqlx::query(
        "UPDATE users
         SET role = ?, polres = ?, satker = ?, active = ?,
             password_hash = COALESCE(?, password_hash)
         WHERE id = ?",
    )
    .bind(&role)
    .bind(&polres)
    .bind(&satker)
    .bind(i64::from(active))
    .bind(password_hash.as_deref())
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    .map_err(ApiError::internal)?;

    // Reset kata sandi mengeluarkan pengguna dari semua perangkat.
    if password_hash.is_some() {
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *transaction)
            .await
            .map_err(ApiError::internal)?;
    }
    transaction.commit().await.map_err(ApiError::internal)?;

    Ok(AdminUserView {
        id: user_id.to_string(),
        username,
        role,
        polres,
        satker,
        active,
        created_at: row.get("created_at"),
    })
}

/// Pilihan dropdown polres dan satker Polda dari baris import yang tersimpan.
pub async fn options(db: &SqlitePool, polda_name: &str) -> ApiResult<UserOptions> {
    let polres = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT TRIM(polres) AS label
         FROM import_rows
         WHERE TRIM(polres) <> ''
         ORDER BY label COLLATE NOCASE",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    let satker: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT TRIM(satker) AS label
         FROM import_rows
         WHERE TRIM(satker) <> ''
         ORDER BY label COLLATE NOCASE",
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?
    .into_iter()
    .filter(|value| !value.eq_ignore_ascii_case(polda_name.trim()))
    .collect();

    Ok(UserOptions { polres, satker })
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
        (dir, db)
    }

    fn request(
        username: &str,
        role: &str,
        polres: Option<&str>,
        satker: Option<&str>,
    ) -> CreateUserRequest {
        CreateUserRequest {
            username: username.to_string(),
            password: "kata-sandi-panjang".to_string(),
            role: role.to_string(),
            polres: polres.map(str::to_string),
            satker: satker.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn validates_scoped_roles_and_assignments() {
        let (_dir, db) = setup().await;

        // Peran polres wajib menunjuk satu polres.
        assert!(
            create(&db, &request("polres1", "polres", None, None))
                .await
                .is_err()
        );

        let polres = create(
            &db,
            &request(
                "polres1",
                "polres",
                Some("Polresta Denpasar"),
                Some("Diabaikan"),
            ),
        )
        .await
        .unwrap();
        assert_eq!(polres.polres, "Polresta Denpasar");
        assert_eq!(polres.satker, "");

        let polda = create(
            &db,
            &request("polda1", "polda", Some("Diabaikan"), Some("Ditintelkam")),
        )
        .await
        .unwrap();
        assert_eq!(polda.polres, "");
        assert_eq!(polda.satker, "Ditintelkam");

        assert!(
            create(&db, &request("lain", "superadmin", None, None))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn lists_polres_and_polda_satker_options() {
        let (_dir, db) = setup().await;
        sqlx::query(
            "INSERT INTO users(id,username,password_hash,role) VALUES('u1','tester','x','admin')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO imports(
                id,filename,file_hash,stored_path,parser_version,row_count,skipped_rows,
                duplicate_rows,date_start,date_end,warnings_json,imported_by
             ) VALUES('imp1','x.xlsx','hash','path','4',3,0,0,'2026-10-26','2026-10-26','[]','u1')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO import_rows(import_id,source_row,tanggal,polres,level,unit,satker,dedupe_key,raw_json)
             VALUES
               ('imp1',1,'2026-10-26','Polresta Denpasar','Polres','Polresta Denpasar','','k1','{}'),
               ('imp1',2,'2026-10-26','','Polda','Polda Bali','Ditintelkam','k2','{}'),
               ('imp1',3,'2026-10-26','','Polda','Polda Bali','Ditintelkam','k3','{}')",
        )
        .execute(&db)
        .await
        .unwrap();

        let options = options(&db, "POLDA BALI").await.unwrap();
        assert_eq!(options.polres, vec!["Polresta Denpasar"]);
        assert_eq!(options.satker, vec!["Ditintelkam"]);
    }

    #[tokio::test]
    async fn resets_non_admin_password_and_revokes_sessions() {
        let (_dir, db) = setup().await;
        let created = create(&db, &request("operator1", "operator", None, None))
            .await
            .unwrap();
        let (token, _) = crate::modules::auth::service::create_session(&db, &created.id, 1)
            .await
            .unwrap();

        let input = UpdateUserRequest {
            role: None,
            active: None,
            polres: None,
            satker: None,
            password: Some("kata-sandi-baru-123".to_string()),
        };
        update(&db, "actor", &created.id, &input).await.unwrap();

        let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = ?")
            .bind(&created.id)
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(sessions, 0);
        let token_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE token = ?")
            .bind(&token)
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(token_exists, 0);

        assert!(
            crate::modules::auth::service::authenticate(&db, "operator1", "kata-sandi-baru-123")
                .await
                .is_ok()
        );
        assert!(
            crate::modules::auth::service::authenticate(&db, "operator1", "kata-sandi-panjang")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn rejects_admin_password_reset_and_short_password() {
        let (_dir, db) = setup().await;
        let admin = create(&db, &request("admin2", "admin", None, None))
            .await
            .unwrap();
        let reset_admin = UpdateUserRequest {
            role: None,
            active: None,
            polres: None,
            satker: None,
            password: Some("kata-sandi-baru-123".to_string()),
        };
        assert!(update(&db, "actor", &admin.id, &reset_admin).await.is_err());

        let operator = create(&db, &request("operator2", "operator", None, None))
            .await
            .unwrap();
        let short = UpdateUserRequest {
            role: None,
            active: None,
            polres: None,
            satker: None,
            password: Some("pendek".to_string()),
        };
        assert!(update(&db, "actor", &operator.id, &short).await.is_err());
    }
}
