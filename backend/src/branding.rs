use std::{borrow::Cow, fs, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::config::Config;

/// Logo bawaan yang ikut dikompilasi. Dipakai bila `LAPORAN_LOGO_PATH` kosong
/// dan `<LAPORAN_DATA_DIR>/logo.png` tidak tersedia.
const DEFAULT_LOGO: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../logo_polda_bali.png"
));

pub fn logo_bytes(config: &Config) -> Cow<'static, [u8]> {
    config
        .logo_path
        .as_deref()
        .and_then(|path| fs::read(path).ok())
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(DEFAULT_LOGO))
}

pub fn logo_content_type(config: &Config) -> &'static str {
    config
        .logo_path
        .as_deref()
        .filter(|path| path.is_file())
        .map(content_type_for)
        .unwrap_or("image/png")
}

pub fn logo_data_uri(config: &Config) -> String {
    format!(
        "data:{};base64,{}",
        logo_content_type(config),
        STANDARD.encode(logo_bytes(config).as_ref())
    )
}

fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        _ => "image/png",
    }
}

#[cfg(test)]
mod tests {
    use super::{content_type_for, logo_data_uri};
    use crate::config::Config;
    use std::path::Path;

    #[test]
    fn maps_logo_extensions_to_content_types() {
        assert_eq!(content_type_for(Path::new("logo.png")), "image/png");
        assert_eq!(content_type_for(Path::new("LOGO.JPG")), "image/jpeg");
        assert_eq!(content_type_for(Path::new("logo.svg")), "image/svg+xml");
        assert_eq!(content_type_for(Path::new("logo")), "image/png");
    }

    #[test]
    fn uses_configured_logo_file_in_data_uri() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logo.svg");
        std::fs::write(&path, b"<svg></svg>").unwrap();
        let mut config = Config::test_config();
        config.logo_path = Some(path);

        assert!(logo_data_uri(&config).starts_with("data:image/svg+xml;base64,"));
    }
}
