use std::collections::BTreeMap;

use crate::modules::imports::models::RowAnalysis;
use crate::shared::parse_count;

pub fn analyze(
    fields: &BTreeMap<String, String>,
    isian_laporan: &str,
    bukti_dukung: &str,
) -> RowAnalysis {
    let mut analysis = RowAnalysis::default();
    let mut map_url = String::new();

    for (key, value) in fields {
        let key_lower = key.to_ascii_lowercase();
        let value_lower = value.to_ascii_lowercase();

        if key_lower.contains("personel") || key_lower.contains("personil") {
            if !key_lower.contains("keluhan") {
                if let Some(count) = parse_count(value) {
                    analysis.personel += count;
                }
            }
        }
        if key_lower.contains("jumlah anggota") || key_lower.contains("anggota hadir") {
            if let Some(count) = parse_count(value) {
                analysis.personel += count;
            }
        }
        if key_lower.contains("bibit") {
            if let Some(count) = parse_count(value) {
                analysis.bibit += count;
            }
        }
        if key_lower.contains("jemaah")
            || key_lower.contains("jemaat")
            || key_lower.contains("jamaah")
            || key_lower.contains("umat")
        {
            if let Some(count) = parse_count(value) {
                analysis.jamaah += count;
            }
        }
        if key_lower.contains("luas") {
            if let Some(area) = parse_decimal(value) {
                analysis.area_m2 += area;
            }
        } else if key_lower.contains("area")
            && (value_lower.contains("m2") || value_lower.contains("m²"))
        {
            if let Some(area) = parse_decimal(value) {
                analysis.area_m2 += area;
            }
        }
        if key_lower.contains("peta") || key_lower.contains("lokasi") {
            if map_url.is_empty() {
                if let Some(link) = value.split_whitespace().find(|token| is_url(token)) {
                    map_url = clean_link(link);
                }
            }
        }
    }

    let combined = format!("{isian_laporan}\n{bukti_dukung}");
    if map_url.is_empty() {
        map_url = find_map_url(&combined).unwrap_or_default();
    }

    let coordinates = coordinates_from_url(&map_url)
        .or_else(|| coordinates_from_text(&combined))
        .or_else(|| coordinates_from_text(&fields.values().cloned().collect::<Vec<_>>().join(" ")));

    if let Some((latitude, longitude)) = coordinates {
        analysis.latitude = Some(latitude);
        analysis.longitude = Some(longitude);
    }
    analysis.map_url = map_url;
    analysis.area_m2 = round2(analysis.area_m2);
    analysis
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn is_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn clean_link(value: &str) -> String {
    value
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '<' | '>'
            )
        })
        .trim_end_matches('.')
        .to_string()
}

fn find_map_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(clean_link)
        .find(|token| {
            let lower = token.to_ascii_lowercase();
            is_url(&lower)
                && (lower.contains("maps")
                    || lower.contains("openstreetmap")
                    || lower.contains("goo.gl/maps"))
        })
}

pub fn coordinates_from_url(url: &str) -> Option<(f64, f64)> {
    if url.trim().is_empty() {
        return None;
    }
    for marker in ["@", "q=", "ll=", "center=", "destination=", "daddr="] {
        let lower = url.to_ascii_lowercase();
        let Some(marker_index) = lower.find(marker) else {
            continue;
        };
        let rest = &url[marker_index + marker.len()..];
        let candidate = rest
            .split(|ch| ch == '&' || ch == '#' || ch == '?')
            .next()
            .unwrap_or(rest);
        if let Some((latitude, longitude)) = parse_pair(candidate) {
            return Some((latitude, longitude));
        }
    }
    None
}

pub fn coordinates_from_text(text: &str) -> Option<(f64, f64)> {
    if text.trim().is_empty() {
        return None;
    }
    let lower = text.to_ascii_lowercase();

    let labeled_latitude = labeled_value(&lower, text, "latitude")
        .or_else(|| labeled_value(&lower, text, "lat"));
    let labeled_longitude = labeled_value(&lower, text, "longitude")
        .or_else(|| labeled_value(&lower, text, "lon"))
        .or_else(|| labeled_value(&lower, text, "lng"));
    if let (Some(latitude), Some(longitude)) = (labeled_latitude, labeled_longitude) {
        if is_valid_pair(latitude, longitude) {
            return Some((latitude, longitude));
        }
    }

    let tokens = number_tokens(text);
    for window in tokens.windows(2) {
        let first = &window[0];
        let second = &window[1];
        if first.decimals < 2 || second.decimals < 2 {
            continue;
        }
        if !is_valid_pair(first.value, second.value) {
            continue;
        }
        let between = &text[first.end..second.start];
        if between.contains(',')
            || between.contains(';')
            || between.to_ascii_lowercase().contains("lat")
            || between.to_ascii_lowercase().contains("lon")
        {
            return Some((first.value, second.value));
        }
    }
    None
}

fn labeled_value(lower: &str, text: &str, label: &str) -> Option<f64> {
    let index = lower.find(label)?;
    let start = index + label.len();
    number_tokens(&text[start..])
        .into_iter()
        .find(|token| token.decimals >= 2)
        .map(|token| token.value)
}

fn is_valid_pair(latitude: f64, longitude: f64) -> bool {
    latitude >= -90.0
        && latitude <= 90.0
        && longitude >= -180.0
        && longitude <= 180.0
        && !(latitude.abs() < f64::EPSILON && longitude.abs() < f64::EPSILON)
}

fn parse_pair(value: &str) -> Option<(f64, f64)> {
    let tokens = number_tokens(value);
    if tokens.len() < 2 {
        return None;
    }
    for window in tokens.windows(2) {
        let (latitude, longitude) = (window[0].value, window[1].value);
        if is_valid_pair(latitude, longitude) {
            return Some((latitude, longitude));
        }
    }
    None
}

struct NumberToken {
    value: f64,
    start: usize,
    end: usize,
    decimals: usize,
}

fn number_tokens(value: &str) -> Vec<NumberToken> {
    let bytes = value.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let current = bytes[index] as char;
        let negative = current == '-'
            && index + 1 < bytes.len()
            && (bytes[index + 1] as char).is_ascii_digit();
        if !current.is_ascii_digit() && !negative {
            index += 1;
            continue;
        }
        let start = index;
        if negative {
            index += 1;
        }
        let mut decimals = 0;
        let mut in_fraction = false;
        while index < bytes.len() {
            let ch = bytes[index] as char;
            if ch.is_ascii_digit() {
                if in_fraction {
                    decimals += 1;
                }
                index += 1;
            } else if ch == '.' {
                in_fraction = true;
                index += 1;
            } else {
                break;
            }
        }
        let raw = &value[start..index];
        if let Ok(parsed) = raw.parse::<f64>() {
            tokens.push(NumberToken {
                value: parsed,
                start,
                end: index,
                decimals,
            });
        }
    }
    tokens
}

pub fn parse_decimal(value: &str) -> Option<f64> {
    let cleaned: String = value
        .chars()
        .filter(|ch| ch.is_ascii_digit() || *ch == '.' || *ch == ',')
        .collect();
    if cleaned.is_empty() {
        return None;
    }

    let dots = cleaned.matches('.').count();
    let commas = cleaned.matches(',').count();
    let normalized = if dots > 0 && commas > 0 {
        let last_dot = cleaned.rfind('.').unwrap_or(0);
        let last_comma = cleaned.rfind(',').unwrap_or(0);
        if last_comma > last_dot {
            cleaned.replace('.', "").replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else if commas > 0 {
        cleaned.replace(',', ".")
    } else if dots == 1 {
        let (integer, fraction) = cleaned.split_once('.').unwrap_or(("", ""));
        if fraction.len() == 3 && integer.len() <= 3 {
            cleaned.replace('.', "")
        } else {
            cleaned
        }
    } else {
        cleaned
    };

    normalized.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_coordinates_from_text_and_url() {
        assert_eq!(
            coordinates_from_text("Titik koordinat: MASJID (-8.74382, 115.17813)"),
            Some((-8.74382, 115.17813))
        );
        assert_eq!(
            coordinates_from_text("latitude: -8.570384 longitude: 115.410378"),
            Some((-8.570384, 115.410378))
        );
        assert_eq!(
            coordinates_from_url("https://www.google.com/maps?q=-8.570384,115.410378"),
            Some((-8.570384, 115.410378))
        );
        assert_eq!(
            coordinates_from_url("https://maps.google.com/@-8.65,115.21,15z"),
            Some((-8.65, 115.21))
        );
    }

    #[test]
    fn extracts_analysis_numbers() {
        let fields = BTreeMap::from([
            ("Jumlah personel".to_string(), "12 orang".to_string()),
            ("Jumlah bibit".to_string(), "1.415".to_string()),
            ("Luas area yang ditanami".to_string(), "2,5 m²".to_string()),
            ("Jumlah jemaah".to_string(), "120".to_string()),
        ]);
        let analysis = analyze(
            &fields,
            "Tautan peta 1: https://www.google.com/maps?q=-8.570384,115.410378",
            "",
        );
        assert_eq!(analysis.personel, 12);
        assert_eq!(analysis.bibit, 1415);
        assert_eq!(analysis.area_m2, 2.5);
        assert_eq!(analysis.jamaah, 120);
        assert_eq!(analysis.latitude, Some(-8.570384));
        assert_eq!(analysis.longitude, Some(115.410378));
        assert!(analysis.map_url.contains("google.com/maps"));
    }
}
