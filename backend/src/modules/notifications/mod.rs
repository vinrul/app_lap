//! Notifikasi per pengguna.
//!
//! Setiap notifikasi disimpan di database agar bisa dibaca ulang, lalu
//! disiarkan ke ruang `user:{id}` pada kanal WebSocket.

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        auth::AuthUser,
        live::{Hub, ServerMessage},
    },
    state::AppState,
};

const MAX_ITEMS: i64 = 50;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list))
        .route("/notifications/read", post(mark_read))
}

#[derive(Clone, Debug, Serialize)]
pub struct NotificationView {
    pub id: String,
    pub kind: String,
    pub level: String,
    pub title: String,
    pub body: String,
    pub url: String,
    pub read_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct NotificationList {
    pub items: Vec<NotificationView>,
    pub unread: i64,
}

#[derive(Serialize)]
pub struct UnreadCount {
    pub unread: i64,
}

#[derive(Deserialize)]
pub struct ReadRequest {
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub all: bool,
}

pub async fn create(
    db: &SqlitePool,
    hub: &Hub,
    user_id: &str,
    kind: &str,
    level: &str,
    title: &str,
    body: &str,
    url: &str,
) -> ApiResult<()> {
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO notifications(id, user_id, kind, level, title, body, url, created_at)
         VALUES(?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(kind)
    .bind(level)
    .bind(title)
    .bind(body)
    .bind(url)
    .bind(&created_at)
    .execute(db)
    .await
    .map_err(ApiError::internal)?;

    hub.publish(
        &room(user_id),
        ServerMessage::NotificationCreated {
            id,
            kind: kind.to_string(),
            level: level.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            url: url.to_string(),
            created_at,
        },
    )
    .await;
    let unread = unread_count(db, user_id).await.unwrap_or(0);
    hub.publish(&room(user_id), ServerMessage::NotificationCount { unread })
        .await;
    Ok(())
}

/// Seperti [`create`], tetapi kegagalan hanya dicatat agar tidak menggagalkan
/// pekerjaan utama (impor, PDF, dan sebagainya).
pub async fn notify(
    db: &SqlitePool,
    hub: &Hub,
    user_id: &str,
    kind: &str,
    level: &str,
    title: &str,
    body: &str,
    url: &str,
) {
    if let Err(error) = create(db, hub, user_id, kind, level, title, body, url).await {
        tracing::warn!(%error, "gagal menyimpan notifikasi");
    }
}

pub async fn unread_count(db: &SqlitePool, user_id: &str) -> ApiResult<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id = ? AND read_at IS NULL")
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(ApiError::internal)
}

/// Menghapus notifikasi lama saat server start.
pub async fn prune(db: &SqlitePool, days: i64) -> ApiResult<u64> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    let result = sqlx::query("DELETE FROM notifications WHERE created_at < ?")
        .bind(cutoff)
        .execute(db)
        .await
        .map_err(ApiError::internal)?;
    Ok(result.rows_affected())
}

fn room(user_id: &str) -> String {
    format!("user:{user_id}")
}

async fn list(State(state): State<AppState>, user: AuthUser) -> ApiResult<Json<NotificationList>> {
    let rows = sqlx::query(
        "SELECT id, kind, level, title, body, url, read_at, created_at
         FROM notifications WHERE user_id = ?
         ORDER BY created_at DESC LIMIT ?",
    )
    .bind(&user.id)
    .bind(MAX_ITEMS)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?;

    let items = rows
        .into_iter()
        .map(|row| NotificationView {
            id: row.get("id"),
            kind: row.get("kind"),
            level: row.get("level"),
            title: row.get("title"),
            body: row.get("body"),
            url: row.get("url"),
            read_at: row.try_get("read_at").ok(),
            created_at: row.get("created_at"),
        })
        .collect();
    let unread = unread_count(&state.db, &user.id).await?;
    Ok(Json(NotificationList { items, unread }))
}

async fn mark_read(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<ReadRequest>,
) -> ApiResult<Json<UnreadCount>> {
    let now = chrono::Utc::now().to_rfc3339();
    if input.all {
        sqlx::query("UPDATE notifications SET read_at = ? WHERE user_id = ? AND read_at IS NULL")
            .bind(&now)
            .bind(&user.id)
            .execute(&state.db)
            .await
            .map_err(ApiError::internal)?;
    } else {
        for id in input.ids.iter().take(MAX_ITEMS as usize) {
            sqlx::query("UPDATE notifications SET read_at = ? WHERE id = ? AND user_id = ?")
                .bind(&now)
                .bind(id)
                .bind(&user.id)
                .execute(&state.db)
                .await
                .map_err(ApiError::internal)?;
        }
    }
    let unread = unread_count(&state.db, &user.id).await?;
    state
        .hub
        .publish(&room(&user.id), ServerMessage::NotificationCount { unread })
        .await;
    Ok(Json(UnreadCount { unread }))
}
