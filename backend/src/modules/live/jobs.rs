//! Registry pekerjaan latar belakang yang dapat dipantau dan dibatalkan.
//!
//! Pembatalan bersifat kooperatif: worker memeriksa flag di antara satuan
//! pekerjaan, lalu berhenti tanpa memulai tugas baru. Pekerjaan yang berjalan
//! sebagai task (mis. pembuatan PDF) juga dihentikan lewat `abort`, sehingga
//! proses Chrome ikut dimatikan.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use tokio::task::JoinHandle;

pub struct JobHandle {
    cancel: AtomicBool,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl JobHandle {
    fn new() -> Self {
        Self {
            cancel: AtomicBool::new(false),
            task: Mutex::new(None),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Menandai batal dan menghentikan task pekerjaan bila ada.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        let task = self
            .task
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(task) = task {
            task.abort();
        }
    }

    /// Menautkan task pekerjaan agar dapat dihentikan saat dibatalkan.
    pub fn attach_task(&self, task: JoinHandle<()>) {
        let mut slot = self
            .task
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = Some(task);
    }

    /// Melepas task setelah selesai agar `cancel` tidak menghentikan task lain.
    pub fn detach_task(&self) {
        self.task
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
    }
}

#[derive(Default)]
pub struct JobRegistry {
    jobs: Mutex<HashMap<String, Arc<JobHandle>>>,
}

impl JobRegistry {
    /// Mendaftarkan pekerjaan baru dan mengembalikan handle-nya.
    pub fn start(&self, job: &str) -> Arc<JobHandle> {
        let handle = Arc::new(JobHandle::new());
        self.lock().insert(job.to_string(), handle.clone());
        handle
    }

    pub fn finish(&self, job: &str) {
        self.lock().remove(job);
    }

    /// Mengembalikan `true` bila pekerjaan sedang berjalan dan dibatalkan.
    pub fn cancel(&self, job: &str) -> bool {
        match self.lock().get(job) {
            Some(handle) => {
                handle.cancel();
                true
            }
            None => false,
        }
    }

    pub fn is_running(&self, job: &str) -> bool {
        self.lock().contains_key(job)
    }

    pub fn running(&self) -> Vec<String> {
        let mut jobs = self.lock().keys().cloned().collect::<Vec<_>>();
        jobs.sort();
        jobs
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<JobHandle>>> {
        self.jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_and_cancels_jobs() {
        let registry = JobRegistry::default();
        let handle = registry.start("import:i-1");
        assert!(registry.is_running("import:i-1"));
        assert!(!handle.is_cancelled());

        assert!(registry.cancel("import:i-1"));
        assert!(handle.is_cancelled());
        assert!(!registry.cancel("import:i-2"));

        registry.finish("import:i-1");
        assert!(!registry.is_running("import:i-1"));
        assert!(registry.running().is_empty());
    }

    #[tokio::test]
    async fn aborts_attached_task() {
        let handle = Arc::new(JobHandle::new());
        let task = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        });
        handle.attach_task(task);
        handle.cancel();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(handle.is_cancelled());
    }
}
