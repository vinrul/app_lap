use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    io::Cursor,
};

use calamine::{Data, Reader, open_workbook_auto_from_rs};
use serde_json::{Map, Value};

use crate::{
    modules::imports::models::{HEADERS, ImportPreview, ImportedRow},
    shared::{LabeledCount, extract_links, normalize_date, parse_fields},
};

const MAX_LISTED_WARNINGS: usize = 60;

pub fn parse_workbook(
    bytes: &[u8],
    start_date: &str,
    end_date: &str,
) -> Result<ImportPreview, String> {
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

        let (quick_win, _code) = derive_quick_win(&values[9], &values[7]);
        let duplicate_key = format!(
            "{}|{}|{}|{}|{}",
            date, values[2], values[7], values[8], values[12]
        );
        if !seen.insert(duplicate_key) {
            duplicate_rows += 1;
            push_warning(
                &mut warnings,
                format!(
                    "Baris {source_row}: kemungkinan duplikat laporan (tanggal, satuan, kode, kegiatan, dan pelaksana sama)."
                ),
            );
        }

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
            fields,
            links,
            raw: Value::Object(raw),
        });
    }

    let unmatched_dates = parsed_rows
        .iter()
        .filter(|row| row.tanggal.as_str() < start_date || row.tanggal.as_str() > end_date)
        .count();
    if unmatched_dates > 0 {
        warnings.push(format!(
            "{unmatched_dates} baris berada di luar rentang periode yang dipilih dan tidak akan masuk agregat laporan."
        ));
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
        unmatched_dates,
        headers: actual_headers,
        missing_headers,
        errors: Vec::new(),
        warnings,
        date_counts: into_counts(date_counts),
        quick_wins: into_counts(quick_wins),
        statuses: into_counts(statuses),
        file_date_start: file_dates.iter().next().cloned(),
        file_date_end: file_dates.iter().next_back().cloned(),
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

fn derive_quick_win(quick_wins: &str, kode: &str) -> (String, String) {
    let code = kode.trim().to_string();
    let quick_win = quick_wins.trim();
    if !quick_win.is_empty() {
        return (quick_win.to_uppercase(), code);
    }
    let derived = code
        .split_once('.')
        .map(|(prefix, _)| prefix)
        .unwrap_or(code.as_str())
        .trim()
        .to_uppercase();
    (derived, code)
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
        let preview = parse_workbook(&bytes, "2026-10-15", "2026-10-19").unwrap();
        assert_eq!(preview.sheet, "Data");
        assert_eq!(preview.headers.len(), 18);
        assert_eq!(preview.valid_rows, 4600);
        assert!(preview.errors.is_empty());
        assert_eq!(preview.preview_rows[0].tanggal, "2026-10-19");
        assert!(!preview.preview_rows[0].isian_laporan.is_empty());
        assert!(preview.preview_rows[0].fields.contains_key("Pelaksana"));
        assert!(preview.quick_wins.iter().any(|item| item.label == "QW1"));
    }
}
