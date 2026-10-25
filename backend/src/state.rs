use std::{
    collections::HashSet,
    sync::{Arc, Mutex as StdMutex},
};

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::config::Config;

#[derive(Default)]
pub struct EvidenceDownloadLocks {
    inner: StdMutex<EvidenceDownloadLockState>,
}

#[derive(Default)]
struct EvidenceDownloadLockState {
    running: HashSet<String>,
    rerun: HashSet<String>,
}

impl EvidenceDownloadLocks {
    /// Mengembalikan `true` jika worker baru boleh dijalankan. Bila worker untuk
    /// import yang sama sedang berjalan, permintaan ditandai agar worker
    /// tersebut mengulang satu putaran setelah putaran saat ini selesai.
    pub fn claim(&self, import_id: &str) -> bool {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.running.contains(import_id) {
            state.rerun.insert(import_id.to_string());
            return false;
        }
        state.running.insert(import_id.to_string());
        state.rerun.remove(import_id);
        true
    }

    /// Dipanggil worker setelah satu putaran. Mengembalikan `true` bila ada
    /// permintaan rerun; jika tidak, lock import dilepas.
    pub fn continue_or_finish(&self, import_id: &str) -> bool {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.rerun.remove(import_id) {
            return true;
        }
        state.running.remove(import_id);
        false
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub pdf_lock: Arc<Mutex<()>>,
    pub evidence_downloads: Arc<EvidenceDownloadLocks>,
    pub http: reqwest::Client,
}

#[cfg(test)]
mod tests {
    use super::EvidenceDownloadLocks;

    #[test]
    fn prevents_duplicate_workers_and_requests_rerun() {
        let locks = EvidenceDownloadLocks::default();
        assert!(locks.claim("import-a"));
        assert!(!locks.claim("import-a"));
        assert!(locks.continue_or_finish("import-a"));
        assert!(!locks.continue_or_finish("import-a"));
        assert!(locks.claim("import-b"));
        assert!(!locks.continue_or_finish("import-a"));
    }
}
