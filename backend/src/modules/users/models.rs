use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct AdminUserView {
    pub id: String,
    pub username: String,
    pub role: String,
    pub polres: String,
    pub satker: String,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
    #[serde(default)]
    pub polres: Option<String>,
    #[serde(default)]
    pub satker: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    pub active: Option<bool>,
    #[serde(default)]
    pub polres: Option<String>,
    #[serde(default)]
    pub satker: Option<String>,
    /// Kata sandi baru dari administrator. `None` atau string kosong berarti
    /// kata sandi tidak diubah.
    #[serde(default)]
    pub password: Option<String>,
}

/// Pilihan unit untuk formulir pengguna, diambil dari data yang sudah diimpor.
#[derive(Debug, Serialize)]
pub struct UserOptions {
    pub polres: Vec<String>,
    pub satker: Vec<String>,
}
