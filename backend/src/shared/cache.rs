use std::{
    collections::HashMap,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use sqlx::SqlitePool;

use crate::modules::reports::models::ReportMetrics;

/// Metrik satu periode berukuran kecil (ratusan KB), sehingga beberapa entri
/// masih murah dibandingkan menghitung ulang dari 50 ribu baris.
const MAX_ENTRIES: usize = 8;

type Key = (String, String);

struct Entry {
    value: Arc<ReportMetrics>,
    stored_at: Instant,
}

static ENTRIES: LazyLock<Mutex<HashMap<Key, Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Dinaikkan setiap ada perubahan data. Perhitungan yang dimulai sebelum
/// perubahan tidak boleh menulis hasilnya ke cache.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Panel desain meminta katalog sumber, cakupan, dan pratinjau secara paralel.
/// Kunci ini membuat permintaan yang datang bersamaan menunggu satu
/// perhitungan, bukan mengulanginya masing-masing.
static COMPUTE: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));

fn database_key(db: &SqlitePool) -> String {
    db.connect_options()
        .get_filename()
        .to_string_lossy()
        .into_owned()
}

fn key(db: &SqlitePool, period_id: &str) -> Key {
    (database_key(db), period_id.to_string())
}

pub fn metrics(db: &SqlitePool, period_id: &str) -> Option<Arc<ReportMetrics>> {
    let entries = ENTRIES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    entries
        .get(&key(db, period_id))
        .map(|entry| entry.value.clone())
}

pub fn generation() -> u64 {
    GENERATION.load(Ordering::SeqCst)
}

pub fn store_metrics(db: &SqlitePool, period_id: &str, value: Arc<ReportMetrics>, generation: u64) {
    if GENERATION.load(Ordering::SeqCst) != generation {
        return;
    }
    let mut entries = ENTRIES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let key = key(db, period_id);
    if entries.len() >= MAX_ENTRIES && !entries.contains_key(&key) {
        if let Some(oldest) = entries
            .iter()
            .min_by_key(|(_, entry)| entry.stored_at)
            .map(|(key, _)| key.clone())
        {
            entries.remove(&oldest);
        }
    }
    entries.insert(
        key,
        Entry {
            value,
            stored_at: Instant::now(),
        },
    );
}

/// Dipanggil setelah impor atau perubahan periode agar metrik dihitung ulang.
pub fn invalidate() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    let mut entries = ENTRIES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    entries.clear();
}

pub async fn lock_compute() -> tokio::sync::MutexGuard<'static, ()> {
    COMPUTE.lock().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, db};

    #[tokio::test]
    async fn stores_metrics_and_rejects_stale_results() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::test_config();
        config.data_dir = dir.path().to_path_buf();
        let pool = db::connect(&config).await.unwrap();

        let value = Arc::new(ReportMetrics::default());
        let generation = generation();
        store_metrics(&pool, "p1", value.clone(), generation);
        assert!(metrics(&pool, "p1").is_some());

        invalidate();
        assert!(metrics(&pool, "p1").is_none());

        // Perhitungan yang dimulai sebelum invalidasi tidak boleh masuk cache.
        store_metrics(&pool, "p1", value, generation);
        assert!(metrics(&pool, "p1").is_none());
    }
}
