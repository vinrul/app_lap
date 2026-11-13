use std::collections::BTreeMap;

use chrono::{NaiveDate, NaiveDateTime, Timelike};
use sha2::{Digest, Sha256};

use crate::modules::imports::models::{
    BotDetection, BotSuspectRow, ImportedRow, MaterialUnitCount, RapidInputSuspect, RecordView,
    RepeatedMaterialSuspect,
};

const MAX_RAPID_SUSPECTS: usize = 20;
const MAX_REPEATED_SUSPECTS: usize = 10;
const MAX_ACTIVITIES: usize = 4;
const MAX_MATERIAL_DETAIL_ROWS: usize = 100;
const MAX_SOURCE_ROWS: usize = 12;
const MAX_DETAIL_ROWS: usize = 60;
const MIN_MATERIAL_CHARS: usize = 80;
const SAMPLE_CHARS: usize = 180;
const RAPID_COUNT_5_MINUTES: usize = 8;
const RAPID_COUNT_10_MINUTES: usize = 10;

/// Menandai pola input yang menyerupai pengiriman otomatis: banyak laporan
/// dalam hitungan menit dari satuan yang sama, atau materi yang identik dipakai
/// lintas satuan. Hasilnya berupa indikasi untuk diperiksa, bukan vonis.
pub fn detect(rows: &[ImportedRow]) -> BotDetection {
    let fields: Vec<BotRow<'_>> = rows.iter().map(BotRow::from_imported).collect();
    detect_rows(&fields)
}

pub fn detect_records(rows: &[&RecordView]) -> BotDetection {
    let fields: Vec<BotRow<'_>> = rows.iter().map(|row| BotRow::from_record(row)).collect();
    detect_rows(&fields)
}

struct BotRow<'a> {
    source_row: i64,
    tanggal: &'a str,
    waktu_kirim_wib: &'a str,
    satuan: &'a str,
    polsek: &'a str,
    polres: &'a str,
    unit: &'a str,
    kode: &'a str,
    kegiatan: &'a str,
    status_penerimaan: &'a str,
    isian_laporan: &'a str,
}

impl<'a> BotRow<'a> {
    fn from_imported(row: &'a ImportedRow) -> Self {
        Self {
            source_row: row.source_row as i64,
            tanggal: &row.tanggal,
            waktu_kirim_wib: &row.waktu_kirim_wib,
            satuan: &row.satuan,
            polsek: &row.polsek,
            polres: &row.polres,
            unit: &row.unit,
            kode: &row.kode,
            kegiatan: &row.kegiatan,
            status_penerimaan: &row.status_penerimaan,
            isian_laporan: &row.isian_laporan,
        }
    }

    fn from_record(row: &'a RecordView) -> Self {
        Self {
            source_row: row.source_row,
            tanggal: &row.tanggal,
            waktu_kirim_wib: &row.waktu_kirim_wib,
            satuan: &row.satuan,
            polsek: &row.polsek,
            polres: &row.polres,
            unit: &row.unit,
            kode: &row.kode,
            kegiatan: &row.kegiatan,
            status_penerimaan: &row.status_penerimaan,
            isian_laporan: &row.isian_laporan,
        }
    }

    fn detail(&self) -> BotSuspectRow {
        BotSuspectRow {
            source_row: self.source_row,
            tanggal: self.tanggal.to_string(),
            waktu_kirim_wib: self.waktu_kirim_wib.to_string(),
            satuan: self.satuan.to_string(),
            kode: self.kode.to_string(),
            kegiatan: self.kegiatan.to_string(),
            status_penerimaan: self.status_penerimaan.to_string(),
            isian_laporan: truncate_text(self.isian_laporan, 500),
        }
    }
}

fn detect_rows(rows: &[BotRow<'_>]) -> BotDetection {
    BotDetection {
        rapid_input: detect_rapid_input(rows),
        repeated_material: detect_repeated_material(rows),
    }
}

#[derive(Clone, Copy)]
struct Window {
    start: usize,
    end: usize,
    count: usize,
    span_seconds: i64,
}

fn better_window(candidate: Window, current: Window) -> bool {
    candidate.count > current.count
        || (candidate.count == current.count && candidate.span_seconds < current.span_seconds)
}

fn best_window(
    items: &[(usize, NaiveDateTime)],
    max_seconds: i64,
    min_count: usize,
) -> Option<Window> {
    let mut left = 0;
    let mut best: Option<Window> = None;
    for right in 0..items.len() {
        while (items[right].1 - items[left].1).num_seconds() > max_seconds {
            left += 1;
        }
        let count = right - left + 1;
        if count < min_count {
            continue;
        }
        let window = Window {
            start: left,
            end: right,
            count,
            span_seconds: (items[right].1 - items[left].1).num_seconds(),
        };
        if best.is_none_or(|current| better_window(window, current)) {
            best = Some(window);
        }
    }
    best
}

fn detect_rapid_input(rows: &[BotRow<'_>]) -> Vec<RapidInputSuspect> {
    let mut groups: BTreeMap<(String, NaiveDate), Vec<(usize, NaiveDateTime)>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(time) = parse_timestamp(&row.waktu_kirim_wib) else {
            continue;
        };
        groups
            .entry((report_unit(row), time.date()))
            .or_default()
            .push((index, time));
    }

    let mut suspects = Vec::new();
    for ((satuan, date), mut items) in groups {
        items.sort_by_key(|(_, time)| *time);
        let five = best_window(&items, 5 * 60, RAPID_COUNT_5_MINUTES);
        let ten = best_window(&items, 10 * 60, RAPID_COUNT_10_MINUTES);
        let window = match (five, ten) {
            (Some(five), Some(ten)) => {
                if better_window(five, ten) {
                    Some(five)
                } else {
                    Some(ten)
                }
            }
            (Some(five), None) => Some(five),
            (None, Some(ten)) => Some(ten),
            (None, None) => None,
        };
        let Some(window) = window else {
            continue;
        };

        let mut activity_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut minute_counts: BTreeMap<String, usize> = BTreeMap::new();
        for (index, time) in &items[window.start..=window.end] {
            *activity_counts
                .entry(activity_label(&rows[*index]))
                .or_default() += 1;
            *minute_counts
                .entry(time.format("%Y-%m-%d %H:%M").to_string())
                .or_default() += 1;
        }

        let first = items[window.start].1;
        let last = items[window.end].1;
        suspects.push(RapidInputSuspect {
            satuan,
            tanggal: date.format("%Y-%m-%d").to_string(),
            report_count: window.count,
            window_seconds: window.span_seconds,
            first_time: clock_label(&first),
            last_time: clock_label(&last),
            same_minute_count: minute_counts.values().copied().max().unwrap_or(0),
            activities: top_labels(activity_counts, MAX_ACTIVITIES),
            source_rows: items[window.start..=window.end]
                .iter()
                .take(MAX_SOURCE_ROWS)
                .map(|(index, _)| rows[*index].source_row)
                .collect(),
            rows: items[window.start..=window.end]
                .iter()
                .take(MAX_DETAIL_ROWS)
                .map(|(index, _)| rows[*index].detail())
                .collect(),
        });
    }

    suspects.sort_by(|left, right| {
        right
            .report_count
            .cmp(&left.report_count)
            .then_with(|| left.window_seconds.cmp(&right.window_seconds))
            .then_with(|| left.satuan.cmp(&right.satuan))
            .then_with(|| left.tanggal.cmp(&right.tanggal))
    });
    suspects.truncate(MAX_RAPID_SUSPECTS);
    suspects
}

#[derive(Default)]
struct MaterialGroup {
    rows: Vec<usize>,
    units: BTreeMap<String, usize>,
    activities: BTreeMap<String, usize>,
}

fn detect_repeated_material(rows: &[BotRow<'_>]) -> Vec<RepeatedMaterialSuspect> {
    let mut groups: BTreeMap<String, MaterialGroup> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(key) = material_digest(&row.isian_laporan) else {
            continue;
        };
        let group = groups.entry(key).or_default();
        group.rows.push(index);
        *group.units.entry(report_unit(row)).or_default() += 1;
        *group.activities.entry(activity_label(row)).or_default() += 1;
    }

    let mut suspects: Vec<RepeatedMaterialSuspect> = groups
        .into_iter()
        .filter(|(_, group)| group.rows.len() >= 3 && group.units.len() >= 2)
        .map(|(_, group)| {
            let activity = top_labels(group.activities, 1)
                .into_iter()
                .next()
                .unwrap_or_else(|| "Tanpa kegiatan".to_string());
            let mut units: Vec<(String, usize)> = group.units.into_iter().collect();
            units.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let unit_count = units.len();
            let unit_counts = units
                .iter()
                .map(|(satuan, rows)| MaterialUnitCount {
                    satuan: satuan.clone(),
                    rows: *rows,
                })
                .collect();
            let units = units.into_iter().map(|(unit, _)| unit).collect();
            let sample = group
                .rows
                .first()
                .map(|index| truncate_text(&rows[*index].isian_laporan, SAMPLE_CHARS))
                .unwrap_or_default();
            let source_rows = group
                .rows
                .iter()
                .take(MAX_SOURCE_ROWS)
                .map(|index| rows[*index].source_row)
                .collect();
            let detail_rows = group
                .rows
                .iter()
                .take(MAX_MATERIAL_DETAIL_ROWS)
                .map(|index| rows[*index].detail())
                .collect();
            RepeatedMaterialSuspect {
                activity,
                report_count: group.rows.len(),
                unit_count,
                units,
                unit_counts,
                sample,
                source_rows,
                rows: detail_rows,
            }
        })
        .collect();

    suspects.sort_by(|left, right| {
        right
            .unit_count
            .cmp(&left.unit_count)
            .then_with(|| right.report_count.cmp(&left.report_count))
            .then_with(|| left.activity.cmp(&right.activity))
    });
    suspects.truncate(MAX_REPEATED_SUSPECTS);
    suspects
}

fn parse_timestamp(value: &str) -> Option<NaiveDateTime> {
    let value = value.trim();
    for format in [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
    ] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(value, format) {
            return Some(parsed);
        }
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
}

fn report_unit(row: &BotRow<'_>) -> String {
    for value in [&row.satuan, &row.polsek, &row.polres, &row.unit] {
        let value = value.trim();
        if !value.is_empty() {
            return value.to_string();
        }
    }
    "Tanpa satuan".to_string()
}

fn activity_label(row: &BotRow<'_>) -> String {
    let code = row.kode.trim();
    let kegiatan = row.kegiatan.trim();
    match (code.is_empty(), kegiatan.is_empty()) {
        (true, true) => "Tanpa kegiatan".to_string(),
        (true, false) => kegiatan.to_string(),
        (false, true) => code.to_string(),
        (false, false) => format!("{code} · {kegiatan}"),
    }
}

fn clock_label(time: &NaiveDateTime) -> String {
    if time.second() == 0 {
        time.format("%H:%M").to_string()
    } else {
        time.format("%H:%M:%S").to_string()
    }
}

fn top_labels(counts: BTreeMap<String, usize>, limit: usize) -> Vec<String> {
    let mut items: Vec<(String, usize)> = counts.into_iter().collect();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items
        .into_iter()
        .take(limit)
        .map(|(label, _)| label)
        .collect()
}

fn normalize_material(value: &str) -> String {
    value
        .split_whitespace()
        .filter(|token| {
            let lower = token.to_ascii_lowercase();
            !lower.contains("http://") && !lower.contains("https://")
        })
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn material_digest(value: &str) -> Option<String> {
    let normalized = normalize_material(value);
    if normalized.chars().count() < MIN_MATERIAL_CHARS {
        return None;
    }
    Some(hex::encode(Sha256::digest(normalized.as_bytes())))
}

fn truncate_text(value: &str, limit: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let truncated: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::detect;
    use crate::modules::imports::models::ImportedRow;
    use std::collections::BTreeMap;

    fn row(
        source_row: u32,
        time: &str,
        satuan: &str,
        kode: &str,
        kegiatan: &str,
        isian: &str,
    ) -> ImportedRow {
        ImportedRow {
            source_row,
            tanggal: time.chars().take(10).collect(),
            waktu_kirim_wib: time.to_string(),
            satuan: satuan.to_string(),
            tingkat: "Polsek".to_string(),
            polda: "POLDA BALI".to_string(),
            polres: "Polres Contoh".to_string(),
            polsek: satuan.to_string(),
            kode: kode.to_string(),
            kegiatan: kegiatan.to_string(),
            quick_wins: String::new(),
            pejabat_pelaksana: String::new(),
            jabatan: String::new(),
            pelaksana: String::new(),
            pelapor: String::new(),
            status_penerimaan: String::new(),
            bukti_dukung: String::new(),
            keluhan_personel_polri: String::new(),
            isian_laporan: isian.to_string(),
            level: "Polsek".to_string(),
            unit: String::new(),
            qw_title: String::new(),
            fields: BTreeMap::new(),
            links: Vec::new(),
            analysis: Default::default(),
            raw: serde_json::Value::Null,
        }
    }

    #[test]
    fn flags_rapid_input_per_unit() {
        let mut rows = Vec::new();
        for index in 0..8 {
            let time = if index < 4 {
                "2026-11-09 08:00"
            } else {
                "2026-11-09 08:01"
            };
            rows.push(row(
                index + 1,
                time,
                "Polsek Cepat",
                "QW5.2",
                "Layanan Informasi Perkara",
                "materi berbeda",
            ));
        }

        let detection = detect(&rows);
        assert_eq!(detection.rapid_input.len(), 1);
        let suspect = &detection.rapid_input[0];
        assert_eq!(suspect.satuan, "Polsek Cepat");
        assert_eq!(suspect.report_count, 8);
        assert_eq!(suspect.same_minute_count, 4);
        assert_eq!(suspect.window_seconds, 60);
        assert_eq!(suspect.rows.len(), 8);
        assert_eq!(suspect.rows[0].source_row, 1);
        assert_eq!(suspect.rows[0].kode, "QW5.2");
    }

    #[test]
    fn flags_identical_material_across_units() {
        let content = "a. pelaksana: kanit reskrim polsek b. kegiatan yang dilaksanakan: qw5.4 keadilan restoratif untuk perkara ringan c. hasil yang dicapai: perdamaian para pihak";
        let rows = vec![
            row(
                1,
                "2026-11-09 08:00",
                "Polsek A",
                "QW5.4",
                "Keadilan Restoratif",
                content,
            ),
            row(
                2,
                "2026-11-09 09:00",
                "Polsek B",
                "QW5.4",
                "Keadilan Restoratif",
                content,
            ),
            row(
                3,
                "2026-11-09 10:00",
                "Polsek A",
                "QW5.4",
                "Keadilan Restoratif",
                content,
            ),
        ];

        let detection = detect(&rows);
        assert_eq!(detection.repeated_material.len(), 1);
        let suspect = &detection.repeated_material[0];
        assert_eq!(suspect.report_count, 3);
        assert_eq!(suspect.unit_count, 2);
        assert_eq!(suspect.unit_counts.len(), 2);
        assert_eq!(suspect.unit_counts[0].satuan, "Polsek A");
        assert_eq!(suspect.unit_counts[0].rows, 2);
        assert!(suspect.activity.contains("QW5.4"));
    }
}
