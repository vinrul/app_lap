use std::collections::BTreeMap;

/// Escape teks untuk disisipkan ke HTML laporan.
pub fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
    }
    output
}

/// Mengubah teks multi-baris menjadi paragraf HTML yang aman.
pub fn paragraphs(input: &str) -> String {
    let mut output = String::new();
    for block in input.split("\n\n") {
        let text = block.trim();
        if text.is_empty() {
            continue;
        }
        output.push_str("<p>");
        let mut first = true;
        for line in text.lines() {
            if !first {
                output.push_str("<br>");
            }
            output.push_str(&escape_html(line.trim()));
            first = false;
        }
        output.push_str("</p>");
    }
    output
}

pub fn truncate(input: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, ch) in input.chars().enumerate() {
        if index >= max_chars {
            output.push('…');
            break;
        }
        output.push(ch);
    }
    output
}

pub fn is_noise(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.is_empty()
        || matches!(
            value.as_str(),
            "-" | "—" | "nihil" | "tidak ada" | "tidak" | "none" | "null" | "belum ada" | "0"
        )
        || value.starts_with("nihil")
        || value.starts_with("tidak ada")
}

/// Memecah `Isian laporan` menjadi pasangan label/nilai.
///
/// Format sumber menggunakan baris seperti `c. Kegiatan: QW5.3 Gelar perkara`.
/// Label duplikat tetap dipertahankan jika berbeda (mis. `(Jumlah)` dan `(Rincian)`).
pub fn parse_fields(text: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let line = strip_item_prefix(line);
        let Some((raw_key, raw_value)) = line.split_once(':') else {
            continue;
        };
        let key = clean_key(raw_key);
        if key.len() < 2 || key.len() > 180 {
            continue;
        }
        let key_lower = key.to_ascii_lowercase();
        if key_lower == "http" || key_lower == "https" || key_lower == "ftp" {
            continue;
        }
        let value = raw_value.trim();
        fields.insert(key, value.to_string());
    }
    fields
}

fn strip_item_prefix(line: &str) -> &str {
    let bytes = line.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && (bytes[1] == b'.' || bytes[1] == b')')
        && bytes[2] == b' '
    {
        return line[3..].trim_start();
    }
    if bytes.len() >= 3 && bytes[0].is_ascii_digit() && bytes[1] == b')' && bytes[2] == b' ' {
        return line[3..].trim_start();
    }
    if let Some(rest) = line.strip_prefix("- ") {
        return rest.trim_start();
    }
    if let Some(rest) = line.strip_prefix("• ") {
        return rest.trim_start();
    }
    line
}

fn clean_key(raw: &str) -> String {
    let mut key = raw.trim();
    while key.ends_with(':') || key.ends_with('.') {
        key = key[..key.len() - 1].trim_end();
    }
    key.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Mengambil tautan dari teks. Mendukung URL http(s) dan path unggahan lokal.
pub fn extract_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    for token in text.split_whitespace() {
        let candidate = token
            .trim_matches(|c: char| {
                matches!(
                    c,
                    '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '<' | '>'
                )
            })
            .trim_end_matches('.');
        let lower = candidate.to_ascii_lowercase();
        if lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("/uploads/")
        {
            if !lower.contains("://nihil") && !lower.contains("/nihil") {
                links.push(candidate.to_string());
            }
        }
    }
    links.sort();
    links.dedup();
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lettered_report_fields() {
        let fields = parse_fields(
            "a. Pelaksana: Reskrim Polsek\r\nb. Nama dan pangkat: IPTU Budi\r\nc. Kegiatan: QW5.3 Gelar perkara",
        );
        assert_eq!(fields.get("Pelaksana").unwrap(), "Reskrim Polsek");
        assert_eq!(fields.get("Kegiatan").unwrap(), "QW5.3 Gelar perkara");
        assert_eq!(fields.len(), 3);
    }

    #[test]
    fn extracts_links_and_ignores_placeholders() {
        let links = extract_links(
            "Dokumentasi: https://drive.google.com/x https://nihil/ Link: https://instagram.com/p/abc.",
        );
        assert_eq!(links.len(), 2);
        assert!(links.iter().all(|link| !link.contains("nihil")));
    }

    #[test]
    fn escapes_html() {
        assert_eq!(
            escape_html("<script>alert('x')</script>"),
            "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"
        );
    }
}
