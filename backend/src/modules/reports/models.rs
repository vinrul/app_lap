use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    modules::{imports::models::CoverageResponse, reports::layout::ReportLayout},
    shared::types::{LabeledCount, UnitBreakdown},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportContent {
    pub summary: String,
    pub narkoba: String,
    pub green_policing: String,
    pub jumat_keliling: String,
    pub subuh_keliling: String,
    pub minggu_kasih: String,
    pub aspirations: String,
    pub follow_up: String,
    pub media: String,
    pub closing: String,
    pub signatory_name: String,
    pub signatory_rank: String,
    pub signatory_nip: String,
    pub report_city: String,
    pub report_date: String,
}

impl ReportContent {
    pub fn from_value(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DraftView {
    pub content: ReportContent,
    pub updated_at: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportView {
    pub id: String,
    pub period_id: String,
    pub created_at: String,
    pub source_filename: String,
    pub period_label: String,
    pub pdf_url: String,
    pub preview_url: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportShareView {
    pub token: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicShareView {
    pub token: String,
    pub title: String,
    pub period_label: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GenerateReportRequest {
    #[serde(default)]
    pub content: Option<ReportContent>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewReportRequest {
    #[serde(default)]
    pub content: Option<ReportContent>,
    #[serde(default)]
    pub layout: Option<ReportLayout>,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub html: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MetricCard {
    pub key: String,
    pub label: String,
    pub value: i64,
    pub unit: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GroupMetrics {
    pub label: String,
    pub rows: i64,
    pub people: i64,
    pub locations: i64,
    pub bibit: i64,
    pub area_m2: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RowSummary {
    pub source_row: i64,
    pub tanggal: String,
    pub satuan: String,
    pub polres: String,
    pub kode: String,
    pub kegiatan: String,
    pub location: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DailyCount {
    pub date: String,
    pub count: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NarkobaMetrics {
    pub rows: i64,
    pub orang_diamankan: i64,
    pub kawasan: i64,
    pub rehabilitasi: i64,
    pub peserta_penyuluhan: i64,
    pub by_code: Vec<LabeledCount>,
    pub by_polres: Vec<GroupMetrics>,
    pub top_locations: Vec<LabeledCount>,
    pub activities: Vec<RowSummary>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GreenMetrics {
    pub rows: i64,
    pub bibit: i64,
    pub area_m2: i64,
    pub kerja_bakti: i64,
    pub penanaman: i64,
    pub peserta: i64,
    pub by_polres: Vec<GroupMetrics>,
    pub by_kegiatan: Vec<LabeledCount>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReligiousMetrics {
    pub rows: i64,
    pub people: i64,
    pub locations: i64,
    pub polres: i64,
    pub polsek: i64,
    pub by_polres: Vec<GroupMetrics>,
    pub by_religion: Vec<LabeledCount>,
    pub top_locations: Vec<LabeledCount>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AspirationMetrics {
    pub total: i64,
    pub themes: Vec<LabeledCount>,
    pub statuses: Vec<LabeledCount>,
    pub by_source: Vec<LabeledCount>,
    pub quotes: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FollowUpMetrics {
    pub total: i64,
    pub forms: Vec<LabeledCount>,
    pub activities: Vec<RowSummary>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MediaMetrics {
    pub publications: i64,
    pub documentation: i64,
    pub total: i64,
    pub rows_with_media: i64,
    pub by_channel: Vec<LabeledCount>,
    pub by_activity: Vec<LabeledCount>,
    pub top_domains: Vec<LabeledCount>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportMetrics {
    pub source_count: i64,
    pub source_filename: String,
    pub polda_name: String,
    pub period_label: String,
    pub period_start: String,
    pub period_end: String,
    pub generated_at: String,
    pub total_rows: i64,
    pub outside_period: i64,
    pub duplicate_rows: i64,
    pub personel_total: i64,
    pub bibit_total: i64,
    pub area_total: f64,
    pub jamaah_total: i64,
    pub rows_with_coordinates: i64,
    pub evidence_links: i64,
    pub cards: Vec<MetricCard>,
    pub quick_wins: Vec<LabeledCount>,
    pub codes: Vec<LabeledCount>,
    pub statuses: Vec<LabeledCount>,
    pub daily: Vec<DailyCount>,
    pub by_polres: Vec<LabeledCount>,
    pub by_level: Vec<LabeledCount>,
    pub by_satker: Vec<UnitBreakdown>,
    pub by_polres_breakdown: Vec<UnitBreakdown>,
    pub narkoba: NarkobaMetrics,
    pub green_policing: GreenMetrics,
    pub jumat_keliling: ReligiousMetrics,
    pub subuh_keliling: ReligiousMetrics,
    pub minggu_kasih: ReligiousMetrics,
    pub aspirations: AspirationMetrics,
    pub follow_up: FollowUpMetrics,
    pub media: MediaMetrics,
    pub highlights: Vec<RowSummary>,
    /// Ringkasan cakupan per program, dipakai katalog sumber data desain.
    /// Tidak ikut diserialisasi karena hanya dibutuhkan saat menyusun sumber.
    #[serde(skip)]
    pub coverage: Option<CoverageResponse>,
}
