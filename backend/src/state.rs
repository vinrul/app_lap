use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub pdf_lock: Arc<Mutex<()>>,
}
