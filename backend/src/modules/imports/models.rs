use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    modules::reports::models::ReportMetrics,
    shared::{LabeledCount, UnitBreakdown},
};

/// Kolom yang dikenali saat impor. Hanya `REQUIRED_HEADERS` yang wajib ada;
/// kolom lain boleh hilang dari workbook.
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

pub const REQUIRED_HEADERS: [&str; 7] = [
    "Tanggal",
    "Waktu kirim (WIB)",
    "Satuan",
    "Tingkat",
    "Polres",
    "Polsek",
    "Kode",
];

/// Kolom tambahan yang opsional dan dipakai untuk mengambil tautan peta.
pub const MAP_HEADERS: [&str; 6] = [
    "Tautan peta 1",
    "Tautan peta 2",
    "Tautan peta 3",
    "Tautan peta 4",
    "Tautan peta 5",
    "Tautan peta 6",
];

/// Representasi terstruktur dari kolom `Isian laporan`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParsedRow {
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub analysis: RowAnalysis,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RowAnalysis {
    #[serde(default)]
    pub personel: i64,
    #[serde(default)]
    pub bibit: i64,
    #[serde(default)]
    pub area_m2: f64,
    #[serde(default)]
    pub jamaah: i64,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub map_url: String,
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
    pub level: String,
    pub unit: String,
    pub qw_title: String,
    pub fields: BTreeMap<String, String>,
    pub links: Vec<String>,
    pub analysis: RowAnalysis,
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
    pub evidence_links: usize,
    pub rows_with_coordinates: usize,
    pub personel_total: i64,
    pub bibit_total: i64,
    pub area_total: f64,
    pub jamaah_total: i64,
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
    pub duplicate_rows: i64,
    pub parser_version: String,
    pub date_start: String,
    pub date_end: String,
    pub warnings: Vec<String>,
    pub created_at: String,
    pub imported_by: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RecordQuery {
    pub import_id: Option<String>,
    pub search: Option<String>,
    pub quick_wins: Option<String>,
    pub status: Option<String>,
    pub polres: Option<String>,
    pub level: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecordView {
    pub id: i64,
    pub source_row: i64,
    pub import_id: String,
    pub import_filename: String,
    pub tanggal: String,
    pub waktu_kirim_wib: String,
    pub satuan: String,
    pub tingkat: String,
    pub polda: String,
    pub polres: String,
    pub polsek: String,
    pub level: String,
    pub unit: String,
    pub kode: String,
    pub qw_title: String,
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
    pub personel_count: i64,
    pub bibit_count: i64,
    pub area_m2: f64,
    pub jamaah_count: i64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub map_url: String,
    pub fields: BTreeMap<String, String>,
    pub links: Vec<String>,
    pub evidence: Vec<EvidenceView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EvidenceView {
    pub id: String,
    pub source_url: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub status: String,
    pub error: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct RecordsResponse {
    pub date_start: String,
    pub date_end: String,
    pub source_count: i64,
    pub duplicate_rows: i64,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub quick_wins: Vec<LabeledCount>,
    pub statuses: Vec<LabeledCount>,
    pub polres: Vec<LabeledCount>,
    pub records: Vec<RecordView>,
}

#[derive(Debug, Serialize)]
pub struct AnalysisResponse {
    pub date_start: String,
    pub date_end: String,
    pub source_count: i64,
    pub total_rows: i64,
    pub duplicate_rows: i64,
    pub personel_total: i64,
    pub bibit_total: i64,
    pub area_total: f64,
    pub jamaah_total: i64,
    pub rows_with_coordinates: i64,
    pub evidence_links: i64,
    pub by_group: Vec<LabeledCount>,
    pub by_code: Vec<LabeledCount>,
    pub by_level: Vec<LabeledCount>,
    pub by_unit: Vec<LabeledCount>,
    pub by_satker: Vec<UnitBreakdown>,
    pub by_polres_breakdown: Vec<UnitBreakdown>,
    pub by_date: Vec<LabeledCount>,
}

#[derive(Debug, Serialize)]
pub struct UnitDetailResponse {
    pub scope: String,
    pub label: String,
    pub date_start: String,
    pub date_end: String,
    pub polsek_count: i64,
    pub metrics: ReportMetrics,
    pub by_polsek: Vec<UnitBreakdown>,
}

#[derive(Debug, Serialize)]
pub struct QuickWinCodeCoverage {
    pub code: String,
    pub title: String,
    pub rows: i64,
    pub polres: i64,
    pub satker: i64,
}

#[derive(Debug, Serialize)]
pub struct QuickWinActivity {
    pub record_id: i64,
    pub tanggal: String,
    pub kode: String,
    pub title: String,
    pub polres: String,
    pub polsek: String,
    pub satker: String,
    pub kegiatan: String,
    pub pelaksana: String,
    pub personel: i64,
    pub bibit: i64,
    pub area_m2: f64,
    pub jamaah: i64,
    pub map_url: String,
}

#[derive(Debug, Serialize)]
pub struct QuickWinDetailResponse {
    pub code: String,
    pub group: String,
    pub title: String,
    pub group_title: String,
    pub date_start: String,
    pub date_end: String,
    pub total_rows: i64,
    pub personel: i64,
    pub polres_count: i64,
    pub satker_count: i64,
    pub by_polres: Vec<UnitBreakdown>,
    pub by_satker: Vec<UnitBreakdown>,
    pub codes: Vec<QuickWinCodeCoverage>,
    /// Jumlah seluruh kegiatan yang cocok dengan filter (bisa lebih dari 100).
    pub activities_total: i64,
    pub activities: Vec<QuickWinActivity>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageResponse {
    pub date_start: String,
    pub date_end: String,
    pub total_rows: i64,
    pub active_polres: i64,
    pub active_satker: i64,
    pub known_polres: Vec<String>,
    pub known_satker: Vec<String>,
    /// Semua polres pada periode, dipakai sebagai pilihan filter.
    pub polres_options: Vec<String>,
    /// Polres yang sedang difilter; kosong berarti seluruh Polda.
    pub filter_polres: String,
    pub groups: Vec<CoverageGroup>,
    pub programs: Vec<CoverageProgram>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageGroup {
    pub code: String,
    pub title: String,
    pub rows: i64,
    pub polres: i64,
    pub satker: i64,
    pub polres_missing: Vec<String>,
    pub satker_missing: Vec<String>,
    pub by_polres: Vec<CoverageUnitCount>,
    pub by_satker: Vec<CoverageUnitCount>,
    pub codes: Vec<CoverageCode>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageCode {
    pub code: String,
    pub title: String,
    pub rows: i64,
    pub polres: i64,
    pub satker: i64,
    pub polres_missing: i64,
    pub satker_missing: i64,
    pub by_polres: Vec<CoverageUnitCount>,
    pub by_satker: Vec<CoverageUnitCount>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageUnitCount {
    pub label: String,
    pub rows: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageProgram {
    pub key: String,
    pub title: String,
    pub code: String,
    pub rows: i64,
    pub polres: i64,
    pub satker: i64,
    pub polres_total: i64,
    pub satker_total: i64,
    pub polres_missing: Vec<String>,
    pub satker_missing: Vec<String>,
    pub metrics: Vec<CoverageMetric>,
    pub by_polres: Vec<CoverageUnit>,
    pub by_satker: Vec<CoverageUnit>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageMetric {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub total: f64,
    pub filled_rows: i64,
    pub missing_rows: i64,
    pub filled_polres: i64,
    pub filled_satker: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageUnit {
    pub label: String,
    pub rows: i64,
    pub values: Vec<CoverageUnitValue>,
    pub missing: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageUnitValue {
    pub key: String,
    pub value: f64,
    pub filled: bool,
}
