use std::{
    env,
    net::IpAddr,
    path::{Path, PathBuf},
};

/// Konfigurasi proses. Semua nilai dapat diubah lewat environment variable
/// agar deployment lokal tidak memerlukan perubahan kode.
#[derive(Clone)]
pub struct Config {
    pub data_dir: PathBuf,
    pub frontend_dir: PathBuf,
    pub bind_addr: IpAddr,
    pub allowed_hosts: Vec<String>,
    pub port: u16,
    pub chrome_path: String,
    pub secure_cookie: bool,
    pub polda_name: String,
    pub app_name: String,
    pub max_upload_bytes: usize,
    pub session_hours: i64,
    pub timezone_label: String,
    pub google_drive_api_key: Option<String>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("data_dir", &self.data_dir)
            .field("frontend_dir", &self.frontend_dir)
            .field("bind_addr", &self.bind_addr)
            .field("allowed_hosts", &self.allowed_hosts)
            .field("port", &self.port)
            .field("chrome_path", &self.chrome_path)
            .field("secure_cookie", &self.secure_cookie)
            .field("polda_name", &self.polda_name)
            .field("app_name", &self.app_name)
            .field("max_upload_bytes", &self.max_upload_bytes)
            .field("session_hours", &self.session_hours)
            .field("timezone_label", &self.timezone_label)
            .field(
                "google_drive_api_key",
                &self.google_drive_api_key.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

impl Config {
    #[cfg(test)]
    pub fn test_config() -> Self {
        Self {
            data_dir: PathBuf::new(),
            frontend_dir: PathBuf::new(),
            bind_addr: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            allowed_hosts: Vec::new(),
            port: 0,
            chrome_path: String::new(),
            secure_cookie: false,
            polda_name: "POLDA BALI".into(),
            app_name: "Laporan Mingguan PBM".into(),
            max_upload_bytes: 32 * 1024 * 1024,
            session_hours: 12,
            timezone_label: "WITA".into(),
            google_drive_api_key: None,
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let root = project_root()?;
        let data_dir = env::var("LAPORAN_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join("data"));
        let frontend_dir = env::var("LAPORAN_FRONTEND_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join("frontend").join("dist"));
        let port = env::var("LAPORAN_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(4317);
        let bind_addr = env::var("LAPORAN_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1".to_string())
            .parse::<IpAddr>()
            .map_err(|_| {
                anyhow::anyhow!(
                    "LAPORAN_BIND_ADDR harus berupa alamat IP, misalnya 127.0.0.1 atau 0.0.0.0."
                )
            })?;
        let allowed_hosts = env::var("LAPORAN_ALLOWED_HOSTS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                value
                    .trim_matches(|c| c == '[' || c == ']')
                    .to_ascii_lowercase()
            })
            .collect();
        let max_upload_mb = env::var("LAPORAN_MAX_UPLOAD_MB")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(32)
            .clamp(1, 256);
        let session_hours = env::var("LAPORAN_SESSION_HOURS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(12)
            .clamp(1, 24 * 30);
        let google_drive_api_key = env::var("LAPORAN_GOOGLE_DRIVE_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Ok(Self {
            data_dir,
            frontend_dir,
            bind_addr,
            allowed_hosts,
            port,
            chrome_path: discover_chrome(),
            secure_cookie: env::var("LAPORAN_SECURE_COOKIE").as_deref() == Ok("true"),
            polda_name: env::var("LAPORAN_POLDA_NAME")
                .unwrap_or_else(|_| "POLDA BALI".to_string())
                .trim()
                .to_uppercase(),
            app_name: env::var("LAPORAN_APP_NAME")
                .unwrap_or_else(|_| "Laporan Mingguan PBM".to_string()),
            max_upload_bytes: max_upload_mb * 1024 * 1024,
            session_hours,
            timezone_label: env::var("LAPORAN_TIMEZONE_LABEL")
                .unwrap_or_else(|_| "WITA".to_string()),
            google_drive_api_key,
        })
    }
}

/// Menemukan root proyek baik ketika dijalankan dari root maupun dari `backend/`.
pub fn project_root() -> anyhow::Result<PathBuf> {
    let current = env::current_dir()?;
    if current.join("frontend").exists() || current.join("backend").exists() {
        return Ok(current);
    }
    if let Some(parent) = current.parent() {
        if parent.join("frontend").exists() || parent.join("backend").exists() {
            return Ok(parent.to_path_buf());
        }
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn debug_redacts_google_drive_api_key() {
        let mut config = Config::test_config();
        config.google_drive_api_key = Some("rahasia".to_string());
        let output = format!("{config:?}");
        assert!(!output.contains("rahasia"));
        assert!(output.contains("<redacted>"));
    }
}

fn discover_chrome() -> String {
    if let Ok(value) = env::var("LAPORAN_CHROME") {
        return value;
    }
    for path in [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ] {
        if Path::new(path).exists() {
            return path.to_string();
        }
    }
    "chrome".to_string()
}
