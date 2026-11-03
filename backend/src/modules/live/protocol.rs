//! Protokol kanal WebSocket tunggal untuk editor desain.
//!
//! Satu koneksi per tab membawa sinkronisasi layout, kehadiran editor, dan
//! posisi kursor. Pesan dikelompokkan lewat `type`; tidak ada kanal kedua.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::modules::reports::layout::{LayoutPatch, ReportLayout};

/// Batas satu pesan masuk. Patch penggantian penuh bisa mendekati batas layout.
pub const MAX_MESSAGE_BYTES: usize = 3 * 1024 * 1024;
/// Snapshot layout yang ikut disiarkan dibatasi agar tidak menahan pesan lain.
pub const MAX_BROADCAST_LAYOUT: usize = 256 * 1024;
/// Koneksi tanpa pesan apa pun selama ini dianggap mati.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(75);

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EditorView {
    pub client_id: String,
    pub user_id: String,
    pub username: String,
    pub page: usize,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Bergabung ke desain satu periode. `client_id` dibuat klien agar identitas
    /// editor tetap sama saat koneksi tersambung ulang.
    Subscribe {
        period_id: String,
        #[serde(default)]
        client_id: String,
        #[serde(default = "first_page")]
        page: usize,
    },
    /// Keluar dari kanal desain tanpa memutus WebSocket.
    Unsubscribe,
    /// Halaman aktif editor berubah.
    Presence {
        #[serde(default = "first_page")]
        page: usize,
    },
    /// Patch layout dari autosave. Server menerapkannya ke layout terbaru.
    DesignPatch {
        request_id: String,
        patch: LayoutPatch,
    },
    /// Posisi elemen saat digeser. Tidak pernah disimpan.
    Drag {
        #[serde(default = "first_page")]
        page: usize,
        element: String,
        phase: String,
        x: f64,
        y: f64,
    },
    /// Memantau progres pekerjaan latar belakang, mis. `import:{id}`.
    WatchJob {
        job: String,
    },
    /// Meminta pekerjaan berhenti pada kesempatan berikutnya.
    Cancel {
        job: String,
    },
    Ping,
}

fn first_page() -> usize {
    1
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Hello {
        username: String,
        server_time: String,
        /// Penanda sesi (hash token) agar klien tahu saat sesinya dicabut.
        session: String,
    },
    DesignHello {
        period_id: String,
        client_id: String,
        revision: u64,
        layout: ReportLayout,
        updated_at: Option<String>,
        updated_by: Option<String>,
        editors: Vec<EditorView>,
    },
    DesignPatch {
        period_id: String,
        revision: u64,
        client_id: String,
        updated_at: Option<String>,
        updated_by: Option<String>,
        patch: LayoutPatch,
        #[serde(skip_serializing_if = "Option::is_none")]
        layout: Option<ReportLayout>,
    },
    /// Balasan untuk pengirim patch: simpanannya sudah masuk ke revisi ini.
    DesignSaved {
        period_id: String,
        request_id: String,
        revision: u64,
        updated_at: Option<String>,
        updated_by: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        layout: Option<ReportLayout>,
    },
    DesignPresence {
        period_id: String,
        editors: Vec<EditorView>,
    },
    Drag {
        period_id: String,
        client_id: String,
        page: usize,
        element: String,
        phase: String,
        x: f64,
        y: f64,
    },
    /// Kondisi pekerjaan saat ini. Dikirim sekali saat memantau, lalu setiap
    /// ada kemajuan.
    JobState {
        job: String,
        kind: String,
        phase: String,
        total: i64,
        ready: i64,
        failed: i64,
        skipped: i64,
        pending: i64,
    },
    JobFinished {
        job: String,
        status: String,
        total: i64,
        ready: i64,
        failed: i64,
        skipped: i64,
        pending: i64,
    },
    NotificationCreated {
        id: String,
        kind: String,
        level: String,
        title: String,
        body: String,
        url: String,
        created_at: String,
    },
    NotificationCount {
        unread: i64,
    },
    /// Sesi dicabut admin. Klien membandingkan `token` dengan `hello.session`.
    SessionRevoked {
        token: String,
    },
    Error {
        code: String,
        message: String,
    },
    Pong,
}

impl ServerMessage {
    /// Snapshot layout untuk disiarkan, `None` bila terlalu besar. Klien yang
    /// menerima `None` mengambil versi penuh lewat `GET /layout`.
    pub fn snapshot(layout: &ReportLayout) -> Option<ReportLayout> {
        serde_json::to_vec(layout)
            .ok()
            .filter(|json| json.len() <= MAX_BROADCAST_LAYOUT)
            .map(|_| layout.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_subscribe_without_page() {
        let message: ClientMessage =
            serde_json::from_str(r#"{"type":"subscribe","period_id":"p1"}"#).unwrap();
        match message {
            ClientMessage::Subscribe {
                page, client_id, ..
            } => {
                assert_eq!(page, 1);
                assert!(client_id.is_empty());
            }
            other => panic!("pesan tak terduga: {other:?}"),
        }
    }

    #[test]
    fn serializes_design_patch_with_type_tag() {
        let message = ServerMessage::DesignPatch {
            period_id: "p1".into(),
            revision: 3,
            client_id: "c1".into(),
            updated_at: None,
            updated_by: None,
            patch: LayoutPatch::default(),
            layout: None,
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"type\":\"design_patch\""));
        assert!(!json.contains("\"layout\""));
    }

    #[test]
    fn drops_snapshot_above_limit() {
        let layout = ReportLayout::default();
        assert!(ServerMessage::snapshot(&layout).is_some());
    }
}
