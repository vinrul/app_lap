//! Sinkronisasi desain laporan secara realtime.
//!
//! Setiap periode memiliki satu kanal siaran. Klien menerima snapshot awal
//! lewat SSE, lalu menerima patch kecil setiap kali pengguna lain menyimpan.
//! Patch diterapkan ke layout terbaru di server sehingga edit pada elemen yang
//! berbeda tidak saling menimpa.

use std::{
    collections::HashMap,
    convert::Infallible,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::response::sse::Event;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};

use crate::{
    modules::reports::layout::{LayoutPatch, LayoutView, ReportLayout},
    state::ShutdownSignal,
};

/// Jumlah event yang ditahan untuk klien lambat. Setiap patch membawa snapshot
/// layout, jadi antrean dibuat pendek; klien tertinggal akan diminta resync.
const EVENT_CAPACITY: usize = 64;
const PRESENCE_TTL: Duration = Duration::from_secs(30);
/// Selang pengiriman `ping`. Klien memakainya sebagai penanda bahwa kanal
/// siaran masih hidup; tanpa ini koneksi setengah terbuka tidak terdeteksi.
const PING_INTERVAL: Duration = Duration::from_secs(20);
/// Batas ukuran snapshot yang ikut disiarkan pada setiap patch.
const MAX_BROADCAST_LAYOUT: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct EditorView {
    pub client_id: String,
    pub user_id: String,
    pub username: String,
    pub page: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DesignEvent {
    Hello {
        revision: u64,
        layout: ReportLayout,
        updated_at: Option<String>,
        updated_by: Option<String>,
        editors: Vec<EditorView>,
    },
    Patch {
        revision: u64,
        client_id: String,
        updated_at: Option<String>,
        updated_by: Option<String>,
        patch: LayoutPatch,
        /// Snapshot penuh setelah patch diterapkan. Klien memakainya langsung
        /// sehingga tidak perlu menempelkan patch ke basis yang mungkin basi.
        #[serde(skip_serializing_if = "Option::is_none")]
        layout: Option<ReportLayout>,
    },
    Presence {
        editors: Vec<EditorView>,
    },
}

impl DesignEvent {
    /// Membuat event patch beserta snapshot layout hasil penggabungan di server.
    /// Snapshot tidak dikirim bila sudah berupa penggantian penuh atau terlalu
    /// besar; klien lalu memakai patch seperti biasa.
    pub fn patch(view: &LayoutView, client_id: String, patch: LayoutPatch) -> Self {
        let layout = if patch.replace.is_some() {
            None
        } else {
            serde_json::to_vec(&view.layout)
                .ok()
                .filter(|json| json.len() <= MAX_BROADCAST_LAYOUT)
                .map(|_| view.layout.clone())
        };
        Self::Patch {
            revision: view.revision,
            client_id,
            updated_at: view.updated_at.clone(),
            updated_by: view.updated_by.clone(),
            patch,
            layout,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LiveQuery {
    #[serde(default)]
    pub client_id: String,
    pub page: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PresenceRequest {
    pub client_id: String,
    pub page: Option<usize>,
}

#[derive(Default)]
pub struct DesignLive {
    rooms: Mutex<HashMap<String, Arc<DesignRoom>>>,
}

pub struct DesignRoom {
    sender: broadcast::Sender<DesignEvent>,
    editors: Mutex<HashMap<String, (EditorView, Instant)>>,
}

impl DesignLive {
    pub async fn room(&self, period_id: &str) -> Arc<DesignRoom> {
        let mut rooms = self.rooms.lock().await;
        rooms
            .entry(period_id.to_string())
            .or_insert_with(|| Arc::new(DesignRoom::new()))
            .clone()
    }

    pub async fn publish(&self, period_id: &str, event: DesignEvent) {
        self.room(period_id).await.publish(event);
    }
}

impl DesignRoom {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            sender,
            editors: Mutex::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DesignEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: DesignEvent) {
        let _ = self.sender.send(event);
    }

    /// Mendaftarkan koneksi editor dan menyiarkan daftar editor terbaru.
    pub async fn join(&self, editor: EditorView) -> Vec<EditorView> {
        let mut editors = self.editors.lock().await;
        prune_editors(&mut editors);
        editors.insert(editor.client_id.clone(), (editor, Instant::now()));
        let list = editor_list(&editors);
        drop(editors);
        self.publish(DesignEvent::Presence {
            editors: list.clone(),
        });
        list
    }

    /// Memperbarui halaman aktif editor. Siaran hanya dikirim bila halaman
    /// berubah agar heartbeat tidak membanjiri kanal.
    pub async fn heartbeat(&self, editor: EditorView) -> Option<Vec<EditorView>> {
        let mut editors = self.editors.lock().await;
        let pruned = prune_editors(&mut editors);
        let changed = editors
            .get(&editor.client_id)
            .map(|(current, _)| current.page != editor.page || current.username != editor.username)
            .unwrap_or(true);
        editors.insert(editor.client_id.clone(), (editor, Instant::now()));
        if !changed && !pruned {
            return None;
        }
        let list = editor_list(&editors);
        drop(editors);
        self.publish(DesignEvent::Presence {
            editors: list.clone(),
        });
        Some(list)
    }

    /// Menghapus editor saat tab ditutup atau halaman dimuat ulang. Tanpa ini
    /// entri lama baru hilang setelah TTL dan sempat tampil sebagai editor lain.
    pub async fn leave(&self, client_id: &str) -> Option<Vec<EditorView>> {
        let mut editors = self.editors.lock().await;
        if editors.remove(client_id).is_none() {
            return None;
        }
        let list = editor_list(&editors);
        drop(editors);
        self.publish(DesignEvent::Presence {
            editors: list.clone(),
        });
        Some(list)
    }
}

fn prune_editors(editors: &mut HashMap<String, (EditorView, Instant)>) -> bool {
    let before = editors.len();
    editors.retain(|_, (_, seen)| seen.elapsed() < PRESENCE_TTL);
    editors.len() != before
}

fn editor_list(editors: &HashMap<String, (EditorView, Instant)>) -> Vec<EditorView> {
    let mut list: Vec<EditorView> = editors.values().map(|(view, _)| view.clone()).collect();
    list.sort_by(|left, right| {
        left.username
            .to_lowercase()
            .cmp(&right.username.to_lowercase())
            .then_with(|| left.client_id.cmp(&right.client_id))
    });
    list
}

/// Aliran event SSE. Snapshot awal dikirim lebih dulu, lalu setiap patch.
/// Saat server berhenti, aliran ditutup agar graceful shutdown tidak tertahan.
pub fn stream(
    initial: DesignEvent,
    receiver: broadcast::Receiver<DesignEvent>,
    shutdown: ShutdownSignal,
) -> impl Stream<Item = Result<Event, Infallible>> {
    futures_util::stream::unfold(
        (Some(initial), receiver, shutdown),
        |(initial, mut receiver, shutdown)| async move {
            if let Some(event) = initial {
                return Some((Ok(sse_event(&event)), (None, receiver, shutdown)));
            }
            loop {
                tokio::select! {
                    _ = shutdown.wait() => return None,
                    _ = tokio::time::sleep(PING_INTERVAL) => {
                        return Some((
                            Ok(Event::default().event("ping").data("{}")),
                            (None, receiver, shutdown),
                        ));
                    }
                    result = receiver.recv() => match result {
                        Ok(event) => {
                            return Some((Ok(sse_event(&event)), (None, receiver, shutdown)));
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!(skipped, "klien desain tertinggal; minta sinkronisasi ulang");
                            return Some((
                                Ok(Event::default().event("resync").data("{}")),
                                (None, receiver, shutdown),
                            ));
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                    }
                }
            }
        },
    )
}

fn sse_event(event: &DesignEvent) -> Event {
    let name = match event {
        DesignEvent::Hello { .. } => "hello",
        DesignEvent::Patch { .. } => "patch",
        DesignEvent::Presence { .. } => "presence",
    };
    let data = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
    Event::default().event(name).data(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::{
        config::Config,
        modules::auth::service::COOKIE_NAME,
        state::{AppState, EvidenceDownloadLocks},
    };

    fn editor(client_id: &str, page: usize) -> EditorView {
        EditorView {
            client_id: client_id.to_string(),
            user_id: "u1".to_string(),
            username: "tester".to_string(),
            page,
        }
    }

    #[tokio::test]
    async fn tracks_editors_and_page_changes() {
        let room = DesignRoom::new();
        let list = room.join(editor("c1", 1)).await;
        assert_eq!(list.len(), 1);

        let mut receiver = room.subscribe();
        assert!(room.heartbeat(editor("c1", 1)).await.is_none());
        assert!(room.heartbeat(editor("c1", 2)).await.is_some());
        let event = receiver.try_recv().expect("siaran presence");
        match event {
            DesignEvent::Presence { editors } => assert_eq!(editors[0].page, 2),
            other => panic!("event tak terduga: {other:?}"),
        }

        assert!(room.leave("c1").await.is_some());
        assert!(room.leave("c1").await.is_none());
    }

    struct SseReader {
        response: reqwest::Response,
        buffer: String,
    }

    impl SseReader {
        fn new(response: reqwest::Response) -> Self {
            Self {
                response,
                buffer: String::new(),
            }
        }

        async fn next(&mut self) -> String {
            loop {
                if let Some(index) = self.buffer.find("\n\n") {
                    let event = self.buffer[..index].to_string();
                    self.buffer.drain(..index + 2);
                    return event;
                }
                let chunk = tokio::time::timeout(Duration::from_secs(5), self.response.chunk())
                    .await
                    .expect("SSE timeout")
                    .expect("SSE error")
                    .expect("SSE tertutup");
                self.buffer.push_str(&String::from_utf8_lossy(&chunk));
            }
        }
    }

    #[tokio::test]
    async fn streams_hello_and_broadcasts_patches() {
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
             VALUES('p1','2026-10-19','2026-10-25','19–25 Oktober 2026','u1')",
        )
        .execute(&db)
        .await
        .unwrap();

        let state = AppState {
            db: db.clone(),
            config: Arc::new(config),
            pdf_lock: Arc::new(tokio::sync::Mutex::new(())),
            evidence_downloads: Arc::new(EvidenceDownloadLocks::default()),
            design_live: Arc::new(DesignLive::default()),
            design_lock: Arc::new(tokio::sync::Mutex::new(())),
            shutdown: ShutdownSignal::new(),
            http: reqwest::Client::new(),
        };
        let router = crate::app::build_router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

        let client = reqwest::Client::new();
        let cookie = format!("{COOKIE_NAME}=token-uji");
        let live = client
            .get(format!(
                "http://{address}/api/periods/p1/design/live?client_id=klien-1&page=1"
            ))
            .header(reqwest::header::COOKIE, &cookie)
            .send()
            .await
            .unwrap();
        assert!(live.status().is_success());
        let mut live = SseReader::new(live);
        let hello = live.next().await;
        assert!(hello.contains("event: hello"));
        assert!(hello.contains("\"revision\":0"));

        let patch = serde_json::json!({
            "client_id": "klien-2",
            "patch": {
                "base_revision": 0,
                "pages": [{
                    "page": 1,
                    "elements_upsert": [{
                        "id": "teks-1",
                        "type": "text",
                        "x": 10,
                        "y": 10,
                        "width": 50,
                        "height": 10,
                        "z": 1,
                        "text": "Halo kolaborasi"
                    }]
                }]
            }
        });
        let response = client
            .patch(format!("http://{address}/api/periods/p1/layout"))
            .header(reqwest::header::COOKIE, &cookie)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(patch.to_string())
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let body: serde_json::Value =
            serde_json::from_str(&response.text().await.unwrap()).unwrap();
        assert_eq!(body["revision"], 1);

        let broadcast = loop {
            let event = live.next().await;
            if event.contains("event: patch") {
                break event;
            }
        };
        assert!(broadcast.contains("Halo kolaborasi"));
        // Siaran membawa snapshot hasil penggabungan di server, bukan hanya patch.
        assert!(broadcast.contains("\"layout\":{"));

        // Patch kedua dari klien lain harus digabung tanpa menghapus elemen pertama.
        let second_patch = serde_json::json!({
            "client_id": "klien-3",
            "patch": {
                "base_revision": 1,
                "pages": [{
                    "page": 1,
                    "elements_upsert": [{
                        "id": "teks-2",
                        "type": "text",
                        "x": 20,
                        "y": 30,
                        "width": 40,
                        "height": 12,
                        "z": 2,
                        "text": "Elemen kedua"
                    }]
                }]
            }
        });
        let response = client
            .patch(format!("http://{address}/api/periods/p1/layout"))
            .header(reqwest::header::COOKIE, &cookie)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(second_patch.to_string())
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());

        let layout = client
            .get(format!("http://{address}/api/periods/p1/layout"))
            .header(reqwest::header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let layout: serde_json::Value = serde_json::from_str(&layout).unwrap();
        let elements = layout["layout"]["pages"][0]["elements"].as_array().unwrap();
        assert!(elements.iter().any(|element| element["id"] == "teks-1"));
        assert!(elements.iter().any(|element| element["id"] == "teks-2"));

        let response = client
            .post(format!(
                "http://{address}/api/periods/p1/design/presence/leave"
            ))
            .header(reqwest::header::COOKIE, &cookie)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::json!({ "client_id": "klien-1" }).to_string())
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let presence = loop {
            let event = live.next().await;
            if event.contains("event: presence") {
                break event;
            }
        };
        assert!(presence.contains("\"editors\":[]"));

        server.abort();
    }
}
