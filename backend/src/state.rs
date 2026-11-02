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

    /// Melepas lock tanpa menjalankan putaran tambahan, dipakai saat shutdown.
    pub fn release(&self, import_id: &str) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.running.remove(import_id);
        state.rerun.remove(import_id);
    }

    pub fn is_idle(&self) -> bool {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.running.is_empty()
    }
}

/// Sinyal berhenti yang dibagikan ke server dan pekerja latar belakang.
/// Memakai `watch` agar penunggu yang datang setelah sinyal tidak melewatkannya.
#[derive(Clone)]
pub struct ShutdownSignal {
    sender: tokio::sync::watch::Sender<bool>,
    receiver: tokio::sync::watch::Receiver<bool>,
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownSignal {
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::watch::channel(false);
        Self { sender, receiver }
    }

    pub fn trigger(&self) {
        let _ = self.sender.send(true);
    }

    pub fn is_shutdown(&self) -> bool {
        *self.receiver.borrow()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub pdf_lock: Arc<Mutex<()>>,
    pub evidence_downloads: Arc<EvidenceDownloadLocks>,
    pub shutdown: ShutdownSignal,
    pub http: reqwest::Client,
}

#[cfg(test)]
mod tests {
    use super::{EvidenceDownloadLocks, ShutdownSignal};

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

    #[test]
    fn releases_locks_on_shutdown() {
        let locks = EvidenceDownloadLocks::default();
        assert!(locks.claim("import-a"));
        assert!(!locks.is_idle());
        locks.release("import-a");
        assert!(locks.is_idle());
    }

    #[test]
    fn shutdown_signal_sets_flag() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_shutdown());
        signal.trigger();
        assert!(signal.is_shutdown());
    }
}
