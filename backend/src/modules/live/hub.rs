//! Registry ruang siaran per periode.
//!
//! Setiap periode memiliki satu kanal `broadcast`. Daftar editor disimpan di
//! ruang tersebut sehingga kehadiran hanya punya satu sumber kebenaran.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use super::protocol::{EditorView, ServerMessage};

const ROOM_CAPACITY: usize = 64;

#[derive(Default)]
pub struct Hub {
    rooms: Mutex<HashMap<String, Arc<Room>>>,
}

impl Hub {
    pub async fn room(&self, period_id: &str) -> Arc<Room> {
        let mut rooms = self.rooms.lock().await;
        rooms
            .entry(period_id.to_string())
            .or_insert_with(|| Arc::new(Room::new(period_id)))
            .clone()
    }

    /// Dipakai penangan HTTP (simpan layout penuh atau terapkan template) agar
    /// perubahan ikut disiarkan ke editor lain.
    pub async fn publish(&self, period_id: &str, message: ServerMessage) {
        self.room(period_id).await.publish(message);
    }
}

pub struct Room {
    period_id: String,
    sender: broadcast::Sender<ServerMessage>,
    editors: Mutex<HashMap<Uuid, EditorView>>,
}

impl Room {
    fn new(period_id: &str) -> Self {
        let (sender, _) = broadcast::channel(ROOM_CAPACITY);
        Self {
            period_id: period_id.to_string(),
            sender,
            editors: Mutex::new(HashMap::new()),
        }
    }

    pub fn period_id(&self) -> &str {
        &self.period_id
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.sender.subscribe()
    }

    pub fn publish(&self, message: ServerMessage) {
        let _ = self.sender.send(message);
    }

    /// Mendaftarkan editor dan menyiarkan daftar terbaru.
    pub async fn join(&self, conn_id: Uuid, editor: EditorView) -> Vec<EditorView> {
        let mut editors = self.editors.lock().await;
        editors.insert(conn_id, editor);
        let list = editor_list(&editors);
        drop(editors);
        self.publish_presence(&list);
        list
    }

    /// Menghapus editor saat koneksi ditutup.
    pub async fn leave(&self, conn_id: Uuid) -> Option<Vec<EditorView>> {
        let mut editors = self.editors.lock().await;
        editors.remove(&conn_id)?;
        let list = editor_list(&editors);
        drop(editors);
        self.publish_presence(&list);
        Some(list)
    }

    /// Memperbarui halaman aktif. Siaran hanya dikirim bila halaman berubah.
    pub async fn set_page(
        &self,
        conn_id: Uuid,
        page: usize,
        username: &str,
    ) -> Option<Vec<EditorView>> {
        let mut editors = self.editors.lock().await;
        let editor = editors.get_mut(&conn_id)?;
        if editor.page == page && editor.username == username {
            return None;
        }
        editor.page = page;
        editor.username = username.to_string();
        let list = editor_list(&editors);
        drop(editors);
        self.publish_presence(&list);
        Some(list)
    }

    fn publish_presence(&self, editors: &[EditorView]) {
        self.publish(ServerMessage::DesignPresence {
            period_id: self.period_id.clone(),
            editors: editors.to_vec(),
        });
    }
}

fn editor_list(editors: &HashMap<Uuid, EditorView>) -> Vec<EditorView> {
    let mut list: Vec<EditorView> = editors.values().cloned().collect();
    list.sort_by(|left, right| {
        left.username
            .to_lowercase()
            .cmp(&right.username.to_lowercase())
            .then_with(|| left.client_id.cmp(&right.client_id))
    });
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(client_id: &str, page: usize) -> EditorView {
        EditorView {
            client_id: client_id.to_string(),
            user_id: "u1".to_string(),
            username: "tester".to_string(),
            page,
        }
    }

    #[tokio::test]
    async fn joins_leaves_and_updates_pages() {
        let hub = Hub::default();
        let room = hub.room("p1").await;
        let mut receiver = room.subscribe();

        let conn = Uuid::new_v4();
        let editors = room.join(conn, editor("c1", 1)).await;
        assert_eq!(editors.len(), 1);

        let event = receiver.try_recv().expect("siaran presence");
        match event {
            ServerMessage::DesignPresence { editors, .. } => assert_eq!(editors[0].page, 1),
            other => panic!("pesan tak terduga: {other:?}"),
        }

        assert!(room.set_page(conn, 1, "tester").await.is_none());
        let editors = room
            .set_page(conn, 2, "tester")
            .await
            .expect("halaman berubah");
        assert_eq!(editors[0].page, 2);

        assert!(room.leave(conn).await.is_some());
        assert!(room.leave(conn).await.is_none());
    }

    #[tokio::test]
    async fn broadcasts_to_every_subscriber() {
        let hub = Hub::default();
        let room = hub.room("p1").await;
        let mut first = room.subscribe();
        let mut second = room.subscribe();

        hub.publish("p1", ServerMessage::Pong).await;

        assert!(matches!(first.try_recv(), Ok(ServerMessage::Pong)));
        assert!(matches!(second.try_recv(), Ok(ServerMessage::Pong)));
    }

    #[test]
    fn sorts_editors_by_username() {
        let mut editors = HashMap::new();
        editors.insert(
            Uuid::new_v4(),
            EditorView {
                username: "budi".into(),
                ..editor("c2", 1)
            },
        );
        editors.insert(
            Uuid::new_v4(),
            EditorView {
                username: "ani".into(),
                ..editor("c1", 1)
            },
        );
        let list = editor_list(&editors);
        assert_eq!(list[0].username, "ani");
    }
}
