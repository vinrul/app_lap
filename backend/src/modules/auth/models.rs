use serde::{Deserialize, Serialize};

use crate::modules::imports::models::RecordView;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub role: String,
    pub polres: String,
    pub satker: String,
    /// Token sesi mentah; dipakai untuk penanda sesi pada kanal realtime.
    pub token: String,
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

    /// Menulis data global (impor, periode, dokumentasi).
    pub fn is_global_writer(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "operator")
    }

    /// Hanya peran polda yang benar-benar baca-saja. Akun polres boleh menyusun
    /// desain dan PDF untuk polresnya sendiri.
    pub fn is_reader(&self) -> bool {
        self.role == "polda"
    }

    /// Peran yang boleh menyunting desain dan menerbitkan PDF laporan.
    pub fn can_design(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "operator" | "polres")
    }

    pub fn data_scope(&self) -> DataScope {
        match self.role.as_str() {
            "admin" | "operator" => DataScope::All,
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
            // Peran tak dikenal (mis. sisa migrasi lama) tidak mendapat data.
            _ => DataScope::None,
        }
    }

    /// Menyimpan desain dan menerbitkan PDF. Akun polres hanya untuk
    /// polresnya; admin dan operator untuk cakupan global.
    pub fn require_writer(&self) -> crate::error::ApiResult<()> {
        if self.can_design() {
            Ok(())
        } else {
            Err(crate::error::ApiError::forbidden())
        }
    }

    /// Menulis data global (impor, periode, dokumentasi) hanya untuk admin dan
    /// operator. Akun polres tidak ikut.
    pub fn require_global_writer(&self) -> crate::error::ApiResult<()> {
        if self.is_global_writer() {
            Ok(())
        } else {
            Err(crate::error::ApiError::forbidden())
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
    use super::{AuthUser, DataScope};

    fn user(role: &str, polres: &str, satker: &str) -> AuthUser {
        AuthUser {
            id: "u1".into(),
            username: "tester".into(),
            role: role.into(),
            polres: polres.into(),
            satker: satker.into(),
            token: "token".into(),
        }
    }

    #[test]
    fn unknown_role_gets_no_data_scope() {
        let legacy = user("lain", "Polresta Surakarta", "Ditintelkam");
        assert_eq!(legacy.data_scope(), DataScope::None);
        assert!(!legacy.can_design());
        assert!(!legacy.is_global_writer());
    }

    #[test]
    fn polres_can_design_only_within_its_scope() {
        let polres = user("polres", "Polresta Surakarta", "");
        assert_eq!(
            polres.data_scope(),
            DataScope::Polres("Polresta Surakarta".into())
        );
        assert!(polres.can_design());
        assert!(polres.require_writer().is_ok());
        // Impor, periode, dan administrasi pengguna tetap tertutup.
        assert!(polres.require_global_writer().is_err());
        assert!(!polres.is_admin());
        assert!(!polres.is_reader());

        let empty = user("polres", "", "");
        assert_eq!(empty.data_scope(), DataScope::None);
    }

    #[test]
    fn polda_remains_read_only() {
        let polda = user("polda", "", "Ditintelkam");
        assert!(polda.is_reader());
        assert!(!polda.can_design());
        assert!(polda.require_writer().is_err());
        assert_eq!(
            polda.data_scope(),
            DataScope::Polda {
                satker: Some("Ditintelkam".into())
            }
        );
    }

    #[test]
    fn global_writer_is_reserved_for_admin_and_operator() {
        assert!(user("admin", "", "").require_global_writer().is_ok());
        assert!(user("operator", "", "").require_global_writer().is_ok());
        assert!(
            user("polres", "Polresta Denpasar", "")
                .require_global_writer()
                .is_err()
        );
        assert!(user("polda", "", "").require_global_writer().is_err());
    }

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
