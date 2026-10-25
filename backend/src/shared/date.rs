use chrono::{Datelike, Duration, NaiveDate};

const MONTHS: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

/// Mengubah berbagai format tanggal dari Excel menjadi `YYYY-MM-DD`.
pub fn normalize_date(value: &str) -> String {
    let input = value.trim();
    if input.is_empty() {
        return String::new();
    }

    if input.len() >= 10 && input.as_bytes().get(4) == Some(&b'-') {
        let candidate = &input[..10];
        if NaiveDate::parse_from_str(candidate, "%Y-%m-%d").is_ok() {
            return candidate.to_string();
        }
    }

    for format in ["%d/%m/%Y", "%m/%d/%Y", "%d-%m-%Y", "%d.%m.%Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(input, format) {
            return date.format("%Y-%m-%d").to_string();
        }
    }

    // Excel kadang menyimpan tanggal sebagai serial number jika sel tidak diformat.
    if input.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(serial) = input.parse::<i64>() {
            if (20_000..=80_000).contains(&serial) {
                if let Some(date) = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)
                    .and_then(|epoch| epoch.checked_add_signed(Duration::days(serial)))
                {
                    return date.format("%Y-%m-%d").to_string();
                }
            }
        }
    }

    String::new()
}

pub fn format_month_label(date: &str) -> String {
    let Ok(value) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return date.to_string();
    };
    format!("{} {}", MONTHS[value.month0() as usize], value.year())
}

pub fn format_period_label(start: &str, end: &str) -> String {
    let (Ok(start_date), Ok(end_date)) = (
        NaiveDate::parse_from_str(start, "%Y-%m-%d"),
        NaiveDate::parse_from_str(end, "%Y-%m-%d"),
    ) else {
        return format!("{start} – {end}");
    };

    let start_month = MONTHS[(start_date.month0()) as usize];
    let end_month = MONTHS[(end_date.month0()) as usize];

    if start_date == end_date {
        return format!("{} {} {}", start_date.day(), start_month, start_date.year());
    }

    if start_date.year() == end_date.year() && start_date.month() == end_date.month() {
        format!(
            "{}–{} {} {}",
            start_date.day(),
            end_date.day(),
            start_month,
            start_date.year()
        )
    } else if start_date.year() == end_date.year() {
        format!(
            "{} {} – {} {} {}",
            start_date.day(),
            start_month,
            end_date.day(),
            end_month,
            start_date.year()
        )
    } else {
        format!(
            "{} {} {} – {} {} {}",
            start_date.day(),
            start_month,
            start_date.year(),
            end_date.day(),
            end_month,
            end_date.year()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_common_formats() {
        assert_eq!(normalize_date("2026-10-19"), "2026-10-19");
        assert_eq!(normalize_date("19/10/2026"), "2026-10-19");
        assert_eq!(normalize_date("2026-10-19 10:52"), "2026-10-19");
        assert_eq!(normalize_date("46314"), "2026-10-19");
        assert_eq!(normalize_date("bukan tanggal"), "");
    }

    #[test]
    fn formats_indonesian_period_labels() {
        assert_eq!(
            format_period_label("2026-10-16", "2026-10-18"),
            "16–18 Oktober 2026"
        );
        assert_eq!(
            format_period_label("2026-09-28", "2026-10-04"),
            "28 September – 4 Oktober 2026"
        );
    }
}
