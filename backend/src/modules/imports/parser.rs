use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    io::Cursor,
};

use calamine::{Data, Reader, open_workbook_auto_from_rs};
use serde_json::{Map, Value};

use crate::{
    modules::imports::{
        analysis,
        models::{HEADERS, ImportPreview, ImportedRow},
        qw,
    },
    shared::{LabeledCount, extract_links, normalize_date, parse_fields},
};

const MAX_LISTED_WARNINGS: usize = 60;

pub fn parse_workbook(bytes: &[u8]) -> Result<ImportPreview, String> {
    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|error| format!("Berkas Excel tidak dapat dibuka: {error}"))?;

    let sheet_name = workbook
        .sheet_names()
        .iter()
        .find(|name| name.eq_ignore_ascii_case("Data"))
        .cloned()
        .ok_or_else(|| "Sheet 'Data' tidak ditemukan pada workbook.".to_string())?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|error| format!("Sheet 'Data' tidak dapat dibaca: {error}"))?;

    let mut rows_iter = range.rows();
    let header_cells = rows_iter
        .next()
        .ok_or_else(|| "Sheet 'Data' kosong.".to_string())?;

    let actual_headers: Vec<String> = header_cells.iter().map(cell_text).collect();
    let header_map = build_header_map(&actual_headers);
    let missing_headers: Vec<String> = HEADERS
        .iter()
        .filter(|header| !header_map.contains_key(&normalize_header(header)))
        .map(|header| (*header).to_string())
        .collect();

    if !missing_headers.is_empty() {
        return Ok(ImportPreview {
            sheet: sheet_name,
            total_rows: 0,
            valid_rows: 0,
            skipped_rows: 0,
            duplicate_rows: 0,
            unmatched_dates: 0,
            headers: actual_headers,
            errors: missing_headers
                .iter()
                .map(|header| format!("Header wajib '{header}' tidak ditemukan."))
                .collect(),
            warnings: Vec::new(),
            date_counts: Vec::new(),
            quick_wins: Vec::new(),
            statuses: Vec::new(),
            file_date_start: None,
            file_date_end: None,
            evidence_links: 0,
            rows_with_coordinates: 0,
            personel_total: 0,
            bibit_total: 0,
            area_total: 0.0,
            jamaah_total: 0,
            preview_rows: Vec::new(),
            rows: Vec::new(),
            missing_headers,
        });
    }

    let mut parsed_rows = Vec::new();
    let mut total_rows = 0_usize;
    let mut skipped_rows = 0_usize;
    let mut duplicate_rows = 0_usize;
    let mut warnings = Vec::new();
    let mut date_counts = BTreeMap::<String, i64>::new();
    let mut quick_wins = BTreeMap::<String, i64>::new();
    let mut statuses = BTreeMap::<String, i64>::new();
    let mut seen = HashSet::<String>::new();
    let mut file_dates = BTreeSet::<String>::new();
    let mut evidence_links = 0_usize;
    let mut rows_with_coordinates = 0_usize;
    let mut personel_total = 0_i64;
    let mut bibit_total = 0_i64;
    let mut area_total = 0.0_f64;
    let mut jamaah_total = 0_i64;

    for (offset, cells) in rows_iter.enumerate() {
        let source_row = (offset + 2) as u32;
        let values: Vec<String> = HEADERS
            .iter()
            .map(|header| {
                header_map
                    .get(&normalize_header(header))
                    .and_then(|index| cells.get(*index))
                    .map(cell_text)
                    .unwrap_or_default()
            })
            .collect();

        if values.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        total_rows += 1;

        let date = normalize_date(&values[0]);
        if date.is_empty() {
            skipped_rows += 1;
            push_warning(
                &mut warnings,
                format!("Baris {source_row}: tanggal kosong atau tidak dikenali."),
            );
            continue;
        }

        let key = dedupe_key(&date, &values[1], &values[2], &values[3], &values[5], &values[6], &values[7]);
        if !seen.insert(key) {
            duplicate_rows += 1;
            push_warning(
                &mut warnings,
                format!(
                    "Baris {source_row}: duplikat (tanggal, waktu kirim, satuan, tingkat, polres, polsek, dan kode sama)."
                ),
            );
            continue;
        }

        let (quick_win, qw_title) = derive_quick_win(&values[9], &values[7]);
        *date_counts.entry(date.clone()).or_default() += 1;
        file_dates.insert(date.clone());

        let quick_win_label = if quick_win.is_empty() {
            "Belum dikategorikan".to_string()
        } else {
            quick_win.clone()
        };
        *quick_wins.entry(quick_win_label).or_default() += 1;

        let status_label = if values[14].trim().is_empty() {
            "Belum diisi".to_string()
        } else {
            values[14].trim().to_string()
        };
        *statuses.entry(status_label).or_default() += 1;

        let fields = parse_fields(&values[17]);
        let mut links = extract_links(&values[17]);
        links.extend(extract_links(&values[15]));
        links.sort();
        links.dedup();
        evidence_links += links.len();

        let row_analysis = analysis::analyze(&fields, &values[17], &values[15]);
        if row_analysis.latitude.is_some() && row_analysis.longitude.is_some() {
            rows_with_coordinates += 1;
        }
        personel_total += row_analysis.personel;
        bibit_total += row_analysis.bibit;
        area_total += row_analysis.area_m2;
        jamaah_total += row_analysis.jamaah;

        let (level, unit) = hierarchy(&values[4], &values[5], &values[6], &values[2]);

        let mut raw = Map::new();
        for (index, header) in HEADERS.iter().enumerate() {
            raw.insert((*header).to_string(), Value::String(values[index].clone()));
        }

        parsed_rows.push(ImportedRow {
            source_row,
            tanggal: date,
            waktu_kirim_wib: values[1].clone(),
            satuan: values[2].clone(),
            tingkat: values[3].clone(),
            polda: values[4].clone(),
            polres: values[5].clone(),
            polsek: values[6].clone(),
            kode: values[7].clone(),
            kegiatan: values[8].clone(),
            quick_wins: quick_win,
            pejabat_pelaksana: values[10].clone(),
            jabatan: values[11].clone(),
            pelaksana: values[12].clone(),
            pelapor: values[13].clone(),
            status_penerimaan: values[14].clone(),
            bukti_dukung: values[15].clone(),
            keluhan_personel_polri: values[16].clone(),
            isian_laporan: values[17].clone(),
            level,
            unit,
            qw_title,
            fields,
            links,
            analysis: row_analysis,
            raw: Value::Object(raw),
        });
    }

    if skipped_rows > 0 {
        warnings.push(format!(
            "{skipped_rows} baris dilewati karena tanggal tidak dapat dikenali."
        ));
    }

    let preview_rows = parsed_rows.iter().take(12).cloned().collect();

    Ok(ImportPreview {
        sheet: sheet_name,
        total_rows,
        valid_rows: parsed_rows.len(),
        skipped_rows,
        duplicate_rows,
        unmatched_dates: 0,
        headers: actual_headers,
        missing_headers,
        errors: Vec::new(),
        warnings,
        date_counts: into_counts(date_counts),
        quick_wins: into_counts(quick_wins),
        statuses: into_counts(statuses),
        file_date_start: file_dates.iter().next().cloned(),
        file_date_end: file_dates.iter().next_back().cloned(),
        evidence_links,
        rows_with_coordinates,
        personel_total,
        bibit_total,
        area_total: (area_total * 100.0).round() / 100.0,
        jamaah_total,
        preview_rows,
        rows: parsed_rows,
    })
}

fn build_header_map(headers: &[String]) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    for (index, header) in headers.iter().enumerate() {
        map.entry(normalize_header(header)).or_insert(index);
    }
    map
}

fn normalize_header(value: &str) -> String {
    value
        .trim_start_matches('\u{feff}')
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        Data::DateTime(value) => {
            let (year, month, day, ..) = value.to_ymd_hms_milli();
            format!("{year:04}-{month:02}-{day:02}")
        }
        Data::DateTimeIso(value) | Data::DurationIso(value) => value.trim().to_string(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => {
            if value.fract().abs() < f64::EPSILON {
                format!("{}", *value as i64)
            } else {
                value.to_string()
            }
        }
        Data::Bool(value) => value.to_string(),
        _ => cell.to_string().trim().to_string(),
    }
}

fn dedupe_key(
    tanggal: &str,
    waktu: &str,
    satuan: &str,
    tingkat: &str,
    polres: &str,
    polsek: &str,
    kode: &str,
) -> String {
    [
        tanggal, waktu, satuan, tingkat, polres, polsek, kode,
    ]
    .iter()
    .map(|value| value.trim().to_lowercase())
    .collect::<Vec<_>>()
    .join("|")
}

fn hierarchy(polda: &str, polres: &str, polsek: &str, satuan: &str) -> (String, String) {
    let polda = polda.trim();
    let polres = polres.trim();
    let polsek = polsek.trim();
    let satuan = satuan.trim();
    if !polsek.is_empty() {
        return ("Polsek".to_string(), polsek.to_string());
    }
    if !polres.is_empty() {
        return ("Polres".to_string(), polres.to_string());
    }
    let unit = if !satuan.is_empty() { satuan } else { polda };
    ("Polda".to_string(), unit.to_string())
}

fn derive_quick_win(quick_wins: &str, kode: &str) -> (String, String) {
    let code = kode.trim();
    if let Some(group) = qw::group_of(code) {
        return (group.code.to_string(), qw::title(code).to_string());
    }
    let explicit = quick_wins.trim();
    if let Some(group) = qw::group_of(explicit) {
        return (group.code.to_string(), group.title.to_string());
    }
    let derived = explicit
        .split_once('.')
        .map(|(prefix, _)| prefix)
        .unwrap_or(explicit)
        .trim()
        .to_uppercase();
    if derived.starts_with("QW") {
        return (derived, String::new());
    }
    (String::new(), String::new())
}

fn push_warning(warnings: &mut Vec<String>, message: String) {
    if warnings.len() < MAX_LISTED_WARNINGS {
        warnings.push(message);
    } else if warnings.len() == MAX_LISTED_WARNINGS {
        warnings.push("… peringatan lain disembunyikan.".to_string());
    }
}

fn into_counts(map: BTreeMap<String, i64>) -> Vec<LabeledCount> {
    let mut items: Vec<LabeledCount> = map
        .into_iter()
        .map(|(label, count)| LabeledCount::new(label, count))
        .collect();
    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sample_workbook_structure() {
        let bytes = std::fs::read("../report-2026-10-19 (1).xlsx").unwrap();
        let preview = parse_workbook(&bytes).unwrap();
        assert_eq!(preview.sheet, "Data");
        assert_eq!(preview.headers.len(), 18);
        assert!(!preview.rows.is_empty());
        assert!(preview.errors.is_empty());
        assert_eq!(preview.preview_rows[0].tanggal, "2026-10-19");
        assert!(!preview.preview_rows[0].isian_laporan.is_empty());
        assert!(preview.preview_rows[0].fields.contains_key("Pelaksana"));
        assert!(preview.quick_wins.iter().any(|item| item.label == "QW1"));
        assert!(preview.duplicate_rows > 0);
        assert!(preview.rows.iter().any(|row| row.qw_title.contains("Jumat")));
        assert!(preview.rows.iter().any(|row| row.analysis.jamaah > 0));
    }

    #[test]
    fn deduplicates_by_operational_key() {
        let first = dedupe_key(
            "2026-10-19",
            "10:00",
            "Polsek Kuta",
            "Polsek",
            "Polresta Denpasar",
            "Polsek Kuta",
            "QW3.3",
        );
        let second = dedupe_key(
            "2026-10-19 ",
            " 10:00",
            "polsek kuta",
            "POLSEK",
            "polresta denpasar",
            "polsek kuta",
            "qw3.3",
        );
        assert_eq!(first, second);
    }

    #[test]
    fn normalizes_unit_hierarchy() {
        assert_eq!(
            hierarchy("Polda Bali", "", "", "Ditreskrimsus"),
            ("Polda".to_string(), "Ditreskrimsus".to_string())
        );
        assert_eq!(
            hierarchy("Polda Bali", "Polresta Denpasar", "", ""),
            ("Polres".to_string(), "Polresta Denpasar".to_string())
        );
        assert_eq!(
            hierarchy("Polda Bali", "Polresta Denpasar", "Polsek Kuta", ""),
            ("Polsek".to_string(), "Polsek Kuta".to_string())
        );
    }
}
