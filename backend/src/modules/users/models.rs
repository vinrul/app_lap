use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct AdminUserView {
    pub id: String,
    pub username: String,
    pub role: String,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    pub active: Option<bool>,
}
