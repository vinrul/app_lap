use serde::{Deserialize, Serialize};

use crate::modules::imports::models::RecordView;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub role: String,
    pub polres: String,
    pub satker: String,
}

/// Batas data yang boleh dilihat oleh peran baca-saja.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataScope {
    All,
    /// Tidak ada data yang boleh dilihat (mis. akun polres tanpa nama polres).
    None,
    Polres(String),
    Polda {
        satker: Option<String>,
    },
}

impl DataScope {
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Kunci cache metrik; berbeda untuk tiap cakupan.
    pub fn key(&self) -> String {
        match self {
            Self::All => "all".to_string(),
            Self::None => "none".to_string(),
            Self::Polres(polres) => format!("polres:{polres}"),
            Self::Polda { satker } => {
                format!("polda:{}", satker.as_deref().unwrap_or_default())
            }
        }
    }

    pub fn matches(&self, row: &RecordView) -> bool {
        self.matches_fields(&row.level, &row.polres, &row.satker)
    }

    pub fn matches_fields(&self, level: &str, polres: &str, satker: &str) -> bool {
        match self {
            Self::All => true,
            Self::None => false,
            Self::Polres(target) => {
                !target.trim().is_empty() && polres.trim().eq_ignore_ascii_case(target.trim())
            }
            Self::Polda { satker: target } => {
                level.trim().eq_ignore_ascii_case("polda")
                    && target
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_none_or(|target| satker.trim().eq_ignore_ascii_case(target))
            }
        }
    }
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    /// Peran baca-saja tidak boleh mengubah data apa pun.
    pub fn is_reader(&self) -> bool {
        matches!(self.role.as_str(), "polres" | "polda")
    }

    pub fn data_scope(&self) -> DataScope {
        match self.role.as_str() {
            "polres" => {
                if self.polres.trim().is_empty() {
                    DataScope::None
                } else {
                    DataScope::Polres(self.polres.clone())
                }
            }
            "polda" => DataScope::Polda {
                satker: (!self.satker.trim().is_empty()).then(|| self.satker.clone()),
            },
            _ => DataScope::All,
        }
    }

    pub fn require_writer(&self) -> crate::error::ApiResult<()> {
        if self.is_reader() {
            Err(crate::error::ApiError::forbidden())
        } else {
            Ok(())
        }
    }

    pub fn require_admin(&self) -> crate::error::ApiResult<()> {
        if self.is_admin() {
            Ok(())
        } else {
            Err(crate::error::ApiError::forbidden())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct UserView {
    pub id: String,
    pub username: String,
    pub role: String,
    pub polres: String,
    pub satker: String,
}

impl From<AuthUser> for UserView {
    fn from(user: AuthUser) -> Self {
        Self {
            id: user.id,
            username: user.username,
            role: user.role,
            polres: user.polres,
            satker: user.satker,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DataScope;

    #[test]
    fn polres_scope_includes_polsek_rows() {
        let scope = DataScope::Polres("Polresta Denpasar".into());
        assert!(scope.matches_fields("Polres", "Polresta Denpasar", "Polresta Denpasar"));
        assert!(scope.matches_fields("Polsek", "Polresta Denpasar", "Polsek Kuta"));
        assert!(!scope.matches_fields("Polres", "Polres Badung", "Polres Badung"));
        assert!(!scope.matches_fields("Polda", "", "Ditintelkam"));
    }

    #[test]
    fn empty_polres_scope_matches_nothing() {
        assert!(!DataScope::None.matches_fields("Polda", "", "Ditintelkam"));
        assert!(!DataScope::None.matches_fields(
            "Polres",
            "Polresta Denpasar",
            "Polresta Denpasar"
        ));
        assert!(!DataScope::Polres("".into()).matches_fields("Polda", "", "Ditintelkam"));
    }

    #[test]
    fn polda_scope_matches_only_polda_level() {
        let all = DataScope::Polda { satker: None };
        assert!(all.matches_fields("Polda", "", "Ditintelkam"));
        assert!(!all.matches_fields("Polres", "Polres Badung", "Polres Badung"));

        let one = DataScope::Polda {
            satker: Some("Ditintelkam".into()),
        };
        assert!(one.matches_fields("Polda", "", "Ditintelkam"));
        assert!(!one.matches_fields("Polda", "", "Ditreskrimum"));
    }
}
