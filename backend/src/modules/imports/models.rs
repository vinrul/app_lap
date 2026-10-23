use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::shared::LabeledCount;

pub const HEADERS: [&str; 18] = [
    "Tanggal",
    "Waktu kirim (WIB)",
    "Satuan",
    "Tingkat",
    "Polda",
    "Polres",
    "Polsek",
    "Kode",
    "Kegiatan",
    "Quick Wins",
    "Pejabat pelaksana",
    "Jabatan",
    "Pelaksana",
    "Pelapor",
    "Status penerimaan",
    "Bukti dukung",
    "Keluhan personel Polri",
    "Isian laporan",
];

/// Representasi terstruktur dari kolom `Isian laporan`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParsedRow {
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
    #[serde(default)]
    pub links: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportedRow {
    pub source_row: u32,
    pub tanggal: String,
    pub waktu_kirim_wib: String,
    pub satuan: String,
    pub tingkat: String,
    pub polda: String,
    pub polres: String,
    pub polsek: String,
    pub kode: String,
    pub kegiatan: String,
    pub quick_wins: String,
    pub pejabat_pelaksana: String,
    pub jabatan: String,
    pub pelaksana: String,
    pub pelapor: String,
    pub status_penerimaan: String,
    pub bukti_dukung: String,
    pub keluhan_personel_polri: String,
    pub isian_laporan: String,
    pub fields: BTreeMap<String, String>,
    pub links: Vec<String>,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportPreview {
    pub sheet: String,
    pub total_rows: usize,
    pub valid_rows: usize,
    pub skipped_rows: usize,
    pub duplicate_rows: usize,
    pub unmatched_dates: usize,
    pub headers: Vec<String>,
    pub missing_headers: Vec<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub date_counts: Vec<LabeledCount>,
    pub quick_wins: Vec<LabeledCount>,
    pub statuses: Vec<LabeledCount>,
    pub file_date_start: Option<String>,
    pub file_date_end: Option<String>,
    pub preview_rows: Vec<ImportedRow>,
    #[serde(skip)]
    pub rows: Vec<ImportedRow>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportView {
    pub id: String,
    pub filename: String,
    pub row_count: i64,
    pub skipped_rows: i64,
    pub parser_version: String,
    pub created_at: String,
    pub imported_by: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordQuery {
    pub import_id: Option<String>,
    pub search: Option<String>,
    pub quick_wins: Option<String>,
    pub status: Option<String>,
    pub polres: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecordView {
    pub id: i64,
    pub source_row: i64,
    pub tanggal: String,
    pub waktu_kirim_wib: String,
    pub satuan: String,
    pub tingkat: String,
    pub polda: String,
    pub polres: String,
    pub polsek: String,
    pub kode: String,
    pub kegiatan: String,
    pub quick_wins: String,
    pub pejabat_pelaksana: String,
    pub jabatan: String,
    pub pelaksana: String,
    pub pelapor: String,
    pub status_penerimaan: String,
    pub bukti_dukung: String,
    pub keluhan_personel_polri: String,
    pub isian_laporan: String,
    pub fields: BTreeMap<String, String>,
    pub links: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RecordsResponse {
    pub import_id: String,
    pub import_filename: String,
    pub period_start: String,
    pub period_end: String,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub quick_wins: Vec<LabeledCount>,
    pub statuses: Vec<LabeledCount>,
    pub polres: Vec<LabeledCount>,
    pub records: Vec<RecordView>,
}
