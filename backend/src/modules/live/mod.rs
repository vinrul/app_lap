//! Kanal WebSocket tunggal.
//!
//! Satu koneksi per tab membawa seluruh kebutuhan realtime editor desain:
//! sinkronisasi layout, kehadiran, dan posisi kursor. Patch layout diterapkan
//! ke database di dalam kanal ini, sehingga tersimpan (HTTP) dan siaran berada
//! pada urutan yang sama.

mod hub;
mod jobs;
mod protocol;

pub use hub::Hub;
pub use jobs::{JobHandle, JobRegistry};
pub use protocol::{EditorView, ServerMessage};

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::{
    modules::{auth, auth::AuthUser, imports, notifications, reports},
    state::AppState,
};

use protocol::ClientMessage;

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(upgrade))
}

async fn upgrade(ws: WebSocketUpgrade, State(state): State<AppState>, user: AuthUser) -> Response {
    ws.max_message_size(protocol::MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| serve(socket, state, user))
}

struct Session {
    conn_id: Uuid,
    client_id: String,
    room: Arc<hub::Room>,
    forward: tokio::task::JoinHandle<()>,
}

impl Session {
    async fn close(self) {
        self.forward.abort();
        self.room.leave(self.conn_id).await;
    }
}

struct JobWatch {
    job: String,
    forward: tokio::task::JoinHandle<()>,
}

#[derive(Default)]
struct Connection {
    session: Option<Session>,
    jobs: Vec<JobWatch>,
}

/// Meneruskan siaran satu ruang ke antrean tulis koneksi ini.
fn forward_room(
    room: Arc<hub::Room>,
    tx: mpsc::Sender<ServerMessage>,
) -> tokio::task::JoinHandle<()> {
    let mut receiver = room.subscribe();
    tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    if tx.send(message).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "klien realtime tertinggal");
                    let _ = tx
                        .send(ServerMessage::Error {
                            code: "lagged".to_string(),
                            message:
                                "Koneksi tertinggal; muat ulang desain untuk menyamakan versi."
                                    .to_string(),
                        })
                        .await;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

async fn serve(socket: WebSocket, state: AppState, user: AuthUser) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(64);

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let Ok(text) = serde_json::to_string(&message) else {
                continue;
            };
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    send(
        &tx,
        ServerMessage::Hello {
            username: user.username.clone(),
            server_time: chrono::Utc::now().to_rfc3339(),
            session: auth::service::token_hash(&user.token),
        },
    )
    .await;

    // Notifikasi diterima pada koneksi mana pun, tanpa perlu berlangganan manual.
    let user_room = state.hub.room(&format!("user:{}", user.id)).await;
    let user_forward = forward_room(user_room, tx.clone());
    if let Ok(unread) = notifications::unread_count(&state.db, &user.id).await {
        send(&tx, ServerMessage::NotificationCount { unread }).await;
    }

    let mut connection = Connection::default();

    loop {
        let received = tokio::select! {
            _ = state.shutdown.wait() => break,
            _ = tx.closed() => break,
            message = tokio::time::timeout(protocol::IDLE_TIMEOUT, stream.next()) => {
                match message {
                    Ok(Some(Ok(message))) => message,
                    _ => break,
                }
            }
        };

        match received {
            Message::Text(text) => {
                let Ok(message) = serde_json::from_str::<ClientMessage>(text.as_str()) else {
                    error(&tx, "invalid", "Pesan realtime tidak dikenali.").await;
                    continue;
                };
                if let Err((code, message)) =
                    handle(&state, &user, &tx, &mut connection, message).await
                {
                    error(&tx, code, &message).await;
                }
            }
            Message::Binary(_) => {
                error(&tx, "invalid", "Pesan biner tidak didukung.").await;
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }

    if let Some(session) = connection.session.take() {
        session.close().await;
    }
    for job in connection.jobs.drain(..) {
        job.forward.abort();
    }
    user_forward.abort();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), writer).await;
}

async fn handle(
    state: &AppState,
    user: &AuthUser,
    tx: &mpsc::Sender<ServerMessage>,
    connection: &mut Connection,
    message: ClientMessage,
) -> Result<(), (&'static str, String)> {
    match message {
        ClientMessage::Ping => {
            send(tx, ServerMessage::Pong).await;
            Ok(())
        }
        ClientMessage::WatchJob { job } => {
            user.require_writer().map_err(|_| {
                (
                    "forbidden",
                    "Hanya pengguna dengan izin menyunting yang dapat memantau pekerjaan."
                        .to_string(),
                )
            })?;
            let Some(import_id) = job.strip_prefix("import:") else {
                if job.contains(':') {
                    // Pekerjaan lain (pdf, dokumentasi) bisa didaftarkan beberapa
                    // saat setelah klien mengirim `watch_job`; langganan tetap diterima.
                    let room = state.hub.room(&job).await;
                    if let Some(index) = connection.jobs.iter().position(|item| item.job == job) {
                        connection.jobs.remove(index).forward.abort();
                    }
                    let forward = forward_room(room, tx.clone());
                    connection.jobs.push(JobWatch {
                        job: job.clone(),
                        forward,
                    });
                    send(
                        tx,
                        ServerMessage::JobState {
                            job: job.clone(),
                            kind: job.split(':').next().unwrap_or("job").to_string(),
                            phase: "queued".to_string(),
                            total: 0,
                            ready: 0,
                            failed: 0,
                            skipped: 0,
                            pending: 0,
                        },
                    )
                    .await;
                    return Ok(());
                }
                return Err(("unsupported", "Jenis pekerjaan tidak dikenal.".to_string()));
            };
            let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM imports WHERE id = ?")
                .bind(import_id)
                .fetch_optional(&state.db)
                .await
                .map_err(|error| ("internal", error.to_string()))?;
            if exists.is_none() {
                return Err(("not_found", "Import tidak ditemukan.".to_string()));
            }
            if let Some(index) = connection.jobs.iter().position(|item| item.job == job) {
                connection.jobs.remove(index).forward.abort();
            }
            // Batasi jumlah langganan agar satu koneksi tidak menumpuk forwarder.
            while connection.jobs.len() >= 8 {
                connection.jobs.remove(0).forward.abort();
            }
            let room = state.hub.room(&job).await;
            let forward = forward_room(room, tx.clone());
            connection.jobs.push(JobWatch {
                job: job.clone(),
                forward,
            });
            if let Some(snapshot) = imports::evidence::job_state(&state.db, &job)
                .await
                .map_err(|error| ("internal", error.to_string()))?
            {
                send(tx, snapshot).await;
            }
            Ok(())
        }
        ClientMessage::Cancel { job } => {
            user.require_writer().map_err(|_| {
                (
                    "forbidden",
                    "Hanya pengguna dengan izin menyunting yang dapat membatalkan pekerjaan."
                        .to_string(),
                )
            })?;
            if state.jobs.cancel(&job) {
                Ok(())
            } else {
                Err((
                    "not_running",
                    "Pekerjaan tidak sedang berjalan.".to_string(),
                ))
            }
        }
        ClientMessage::Subscribe {
            period_id,
            client_id,
            page,
        } => {
            user.require_writer().map_err(|_| {
                (
                    "forbidden",
                    "Hanya pengguna dengan izin menyunting yang dapat bergabung.".to_string(),
                )
            })?;
            if let Some(previous) = connection.session.take() {
                previous.close().await;
            }
            let snapshot = reports::service::get_layout(&state.db, &period_id)
                .await
                .map_err(|error| ("not_found", error.to_string()))?;
            let room = state.hub.room(&period_id).await;
            let conn_id = Uuid::new_v4();
            let client_id = if client_id.trim().is_empty() {
                Uuid::new_v4().to_string()
            } else {
                client_id
            };

            let forward = forward_room(room.clone(), tx.clone());

            let editors = room
                .join(
                    conn_id,
                    EditorView {
                        client_id: client_id.clone(),
                        user_id: user.id.clone(),
                        username: user.username.clone(),
                        page: page.max(1),
                    },
                )
                .await;

            send(
                tx,
                ServerMessage::DesignHello {
                    period_id: room.period_id().to_string(),
                    client_id: client_id.clone(),
                    revision: snapshot.revision,
                    layout: snapshot.layout,
                    updated_at: snapshot.updated_at,
                    updated_by: snapshot.updated_by,
                    editors,
                },
            )
            .await;

            connection.session = Some(Session {
                conn_id,
                client_id,
                room,
                forward,
            });
            Ok(())
        }
        ClientMessage::Unsubscribe => {
            if let Some(active) = connection.session.take() {
                active.close().await;
            }
            Ok(())
        }
        ClientMessage::Presence { page } => {
            if let Some(active) = connection.session.as_ref() {
                active
                    .room
                    .set_page(active.conn_id, page.max(1), &user.username)
                    .await;
            }
            Ok(())
        }
        ClientMessage::DesignPatch { request_id, patch } => {
            let Some(active) = connection.session.as_ref() else {
                return Err((
                    "not_subscribed",
                    "Belum bergabung ke desain periode.".to_string(),
                ));
            };
            let period_id = active.room.period_id().to_string();
            let client_id = active.client_id.clone();
            let _guard = state.design_lock.lock().await;
            let view = reports::service::apply_patch(&state.db, &user.id, &period_id, &patch)
                .await
                .map_err(|error| ("save_failed", error.to_string()))?;
            let snapshot = ServerMessage::snapshot(&view.layout);
            active.room.publish(ServerMessage::DesignPatch {
                period_id: period_id.clone(),
                revision: view.revision,
                client_id: client_id.clone(),
                updated_at: view.updated_at.clone(),
                updated_by: view.updated_by.clone(),
                patch,
                layout: snapshot.clone(),
            });
            send(
                tx,
                ServerMessage::DesignSaved {
                    period_id,
                    request_id,
                    revision: view.revision,
                    updated_at: view.updated_at,
                    updated_by: view.updated_by,
                    layout: snapshot,
                },
            )
            .await;
            Ok(())
        }
        ClientMessage::Drag {
            page,
            element,
            phase,
            x,
            y,
        } => {
            let Some(active) = connection.session.as_ref() else {
                return Ok(());
            };
            if !x.is_finite() || !y.is_finite() || element.len() > 80 {
                return Ok(());
            }
            if !matches!(phase.as_str(), "start" | "move" | "end") {
                return Ok(());
            }
            active.room.publish(ServerMessage::Drag {
                period_id: active.room.period_id().to_string(),
                client_id: active.client_id.clone(),
                page: page.max(1),
                element,
                phase,
                x,
                y,
            });
            Ok(())
        }
    }
}

async fn send(tx: &mpsc::Sender<ServerMessage>, message: ServerMessage) {
    let _ = tx.send(message).await;
}

async fn error(tx: &mpsc::Sender<ServerMessage>, code: &str, message: &str) {
    send(
        tx,
        ServerMessage::Error {
            code: code.to_string(),
            message: message.to_string(),
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::{SinkExt, StreamExt};
    use serde_json::{Value, json};
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{
            Message,
            client::IntoClientRequest,
            http::{HeaderValue, Request},
        },
    };

    use crate::{
        config::Config,
        modules::{auth::service::COOKIE_NAME, live::Hub},
        state::{AppState, EvidenceDownloadLocks, ShutdownSignal},
    };

    type Socket = tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >;

    async fn next_json(socket: &mut Socket) -> Value {
        loop {
            let message = tokio::time::timeout(std::time::Duration::from_secs(5), socket.next())
                .await
                .expect("pesan realtime tidak datang")
                .expect("kanal tertutup")
                .expect("kanal gagal");
            match message {
                Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
                Message::Ping(_) | Message::Pong(_) => continue,
                other => panic!("pesan tak terduga: {other:?}"),
            }
        }
    }

    async fn wait_for(socket: &mut Socket, kind: &str) -> Value {
        loop {
            let value = next_json(socket).await;
            if value["type"] == kind {
                return value;
            }
        }
    }

    fn client_request(address: &str, cookie: &str) -> Request<()> {
        let url = format!("ws://{address}/api/ws");
        let mut request = url.as_str().into_client_request().unwrap();
        request
            .headers_mut()
            .insert("cookie", HeaderValue::from_str(cookie).expect("cookie"));
        request
    }

    #[tokio::test]
    async fn syncs_design_patch_over_single_socket() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::test_config();
        config.data_dir = dir.path().to_path_buf();
        let db = crate::db::connect(&config).await.unwrap();
        sqlx::query(
            "INSERT INTO users(id,username,password_hash,role) VALUES('u1','tester','x','admin')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO sessions(token,user_id,expires_at) VALUES('token-uji','u1',?)")
            .bind((chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339())
            .execute(&db)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO report_periods(id,start_date,end_date,label,created_by)
             VALUES('p1','2026-10-19','2026-10-25','19-25 Oktober 2026','u1')",
        )
        .execute(&db)
        .await
        .unwrap();

        let state = AppState {
            db: db.clone(),
            config: Arc::new(config),
            pdf_lock: Arc::new(tokio::sync::Mutex::new(())),
            evidence_downloads: Arc::new(EvidenceDownloadLocks::default()),
            hub: Arc::new(Hub::default()),
            jobs: Arc::new(crate::modules::live::JobRegistry::default()),
            design_lock: Arc::new(tokio::sync::Mutex::new(())),
            shutdown: ShutdownSignal::new(),
            http: reqwest::Client::new(),
        };
        let router = crate::app::build_router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

        let cookie = format!("{COOKIE_NAME}=token-uji");
        let (mut socket, _) = connect_async(client_request(&address.to_string(), &cookie))
            .await
            .unwrap();
        let hello = next_json(&mut socket).await;
        assert_eq!(hello["type"], "hello");

        socket
            .send(Message::Text(
                json!({
                    "type": "subscribe",
                    "period_id": "p1",
                    "client_id": "klien-1",
                    "page": 1
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        let hello = wait_for(&mut socket, "design_hello").await;
        assert_eq!(hello["revision"], 0);
        assert_eq!(hello["layout"]["pages"].as_array().unwrap().len(), 9);

        socket
            .send(Message::Text(
                json!({
                    "type": "design_patch",
                    "request_id": "req-1",
                    "patch": {
                        "base_revision": 0,
                        "pages": [{
                            "page": 1,
                            "elements_upsert": [{
                                "id": "kartu-1",
                                "type": "card",
                                "x": 10,
                                "y": 20,
                                "width": 48,
                                "height": 33,
                                "z": 10,
                                "card_label": "Bibit ditanam",
                                "card_value": "120"
                            }]
                        }]
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

        let saved = wait_for(&mut socket, "design_saved").await;
        assert_eq!(saved["revision"], 1);
        assert_eq!(saved["request_id"], "req-1");
        assert_eq!(saved["layout"]["pages"][0]["elements"][0]["id"], "kartu-1");

        // Patch kedua dari klien lain harus disiarkan ke koneksi ini.
        let second = reqwest::Client::new();
        let response = second
            .patch(format!("http://{address}/api/periods/p1/layout"))
            .header(reqwest::header::COOKIE, &cookie)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(
                json!({
                    "client_id": "klien-2",
                    "patch": {
                        "base_revision": 1,
                        "pages": [{
                            "page": 1,
                            "elements_upsert": [{
                                "id": "kartu-2",
                                "type": "card",
                                "x": 60,
                                "y": 20,
                                "width": 48,
                                "height": 33,
                                "z": 11,
                                "card_label": "Personel",
                                "card_value": "40"
                            }]
                        }]
                    }
                })
                .to_string(),
            )
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());

        let broadcast = loop {
            let value = next_json(&mut socket).await;
            if value["type"] == "design_patch" && value["client_id"] == "klien-2" {
                break value;
            }
        };
        assert_eq!(broadcast["revision"], 2);

        server.abort();
    }
}
