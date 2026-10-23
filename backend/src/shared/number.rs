/// Mengubah teks angka gaya Indonesia menjadi bilangan bulat.
///
/// Contoh: `1.415` -> 1415, `2,4 jt` -> 2, `Rp 1.200.000` -> 1200000.
/// Nilai yang bukan angka atau berisi daftar nama akan menghasilkan `None`.
pub fn parse_count(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if is_non_count(&lower) {
        return None;
    }

    // Hindari teks panjang yang kebetulan memuat angka (daftar nama, uraian).
    if trimmed.chars().count() > 40 {
        return None;
    }

    // Daftar nama/nomor urut seperti "1. Budi, 2. Ani" bukan angka tunggal.
    if trimmed.matches(", ").count() > 0 && trimmed.contains('.') {
        return None;
    }
    if trimmed.matches(',').count() > 1 {
        return None;
    }

    let mut normalized = trimmed.replace("Rp", "").replace("rp", "").replace(' ', "");

    // Titik sebagai pemisah ribuan: 1.415 / 1.200.000
    let dot_groups: Vec<&str> = normalized.split('.').collect();
    if dot_groups.len() > 1
        && dot_groups.iter().skip(1).all(|group| group.len() == 3)
        && dot_groups[0].chars().all(|c| c.is_ascii_digit())
    {
        normalized = dot_groups.join("");
    } else if let Some((integer, _)) = normalized.split_once(',') {
        // Koma sebagai desimal: 2,4 jt -> 2
        normalized = integer.to_string();
    }

    let digits: String = normalized
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .filter(|c| c.is_ascii_digit())
        .collect();

    if digits.is_empty() || digits.len() > 12 {
        return None;
    }
    digits.parse::<i64>().ok()
}

fn is_non_count(lower: &str) -> bool {
    matches!(
        lower,
        "" | "-" | "nihil" | "tidak ada" | "tidak" | "none" | "null" | "belum ada"
    ) || lower.starts_with("nihil")
        || lower.starts_with("tidak ada")
}

/// Format angka dengan pemisah ribuan Indonesia.
pub fn format_count(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let mut output = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            output.push('.');
        }
        output.push(ch);
    }
    if negative {
        output.insert(0, '-');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_indonesian_numbers() {
        assert_eq!(parse_count("1.415"), Some(1415));
        assert_eq!(parse_count("2,4 jt"), Some(2));
        assert_eq!(parse_count("Rp 1.200.000"), Some(1_200_000));
        assert_eq!(parse_count("0"), Some(0));
        assert_eq!(parse_count("Nihil"), None);
        assert_eq!(parse_count("1. Budi, 2. Ani"), None);
    }

    #[test]
    fn formats_thousands() {
        assert_eq!(format_count(4_600), "4.600");
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(1_234_567), "1.234.567");
    }
}
