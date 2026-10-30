export type Role = 'admin' | 'operator'

export interface User {
  id: string
  username: string
  role: Role
}

export interface AdminUser extends User {
  active: boolean
  created_at: string
}

export interface Meta {
  polda_name: string
  app_name: string
  max_upload_bytes: number
  timezone_label: string
}

export interface Period {
  id: string
  start_date: string
  end_date: string
  label: string
  period_type: 'daily' | 'range' | 'monthly'
  created_at: string
  created_by: string
  imports: number
  rows: number
  reports: number
  archived_at: string | null
}

export interface ImportView {
  id: string
  filename: string
  row_count: number
  skipped_rows: number
  duplicate_rows: number
  parser_version: string
  date_start: string
  date_end: string
  warnings: string[]
  created_at: string
  imported_by: string
}

export interface LayoutBlockOffset {
  dx: number
  dy: number
  hidden?: boolean
  preserve_space?: boolean
  width?: number
  height?: number
  background?: string
  border_color?: string
  border_width?: number
  radius?: number
  texts?: Record<string, string>
}

export interface LayoutElement {
  id: string
  type: 'image' | 'rect' | 'text' | 'table' | 'chart' | 'card' | 'icon'
  x: number
  y: number
  width: number
  height: number
  rotation: number
  opacity: number
  z: number
  hidden?: boolean
  locked?: boolean
  src?: string
  alt?: string
  object_fit?: 'contain' | 'cover' | 'fill'
  shape?: string
  fill?: string
  fill_mode?: 'solid' | 'linear' | 'radial'
  gradient_angle?: number
  gradient_colors?: string[]
  border_color?: string
  border_width?: number
  radius?: number
  text?: string
  font_size?: number
  font_family?: string
  color?: string
  bold?: boolean
  italic?: boolean
  align?: 'left' | 'center' | 'right'
  valign?: 'top' | 'middle' | 'bottom'
  background?: string
  source_block?: string
  binding?: string
  group?: string
  table_rows?: string[][]
  table_header?: boolean
  table_header_bg?: string
  table_header_color?: string
  table_stripe?: string
  table_header_font_family?: string
  table_header_font_size?: number
  table_column_backgrounds?: string[]
  table_column_colors?: string[]
  table_column_font_families?: string[]
  table_column_font_sizes?: number[]
  table_column_aligns?: ('left' | 'center' | 'right' | '')[]
  chart_type?: 'bar' | 'line' | 'pie' | 'donut' | 'hbar'
  chart_title?: string
  chart_title_font_family?: string
  chart_title_font_size?: number
  chart_title_wrap?: boolean
  chart_labels?: string[]
  chart_values?: number[]
  chart_colors?: string[]
  chart_show_values?: boolean
  chart_show_legend?: boolean
  chart_axis_color?: string
  chart_label_font_size?: number
  chart_label_rotation?: number
  chart_value_font_family?: string
  chart_value_font_size?: number
  chart_legend_font_size?: number
  card_label?: string
  card_value?: string
  card_unit?: string
  card_detail?: string
  card_accent?: string
  card_label_font_family?: string
  card_label_font_size?: number
  card_label_color?: string
  card_value_font_family?: string
  card_value_font_size?: number
  card_value_color?: string
  card_unit_font_family?: string
  card_unit_font_size?: number
  card_unit_color?: string
  card_detail_font_family?: string
  card_detail_font_size?: number
  card_detail_color?: string
  card_template?: string
  card_icon?: string
  card_icon_color?: string
  card_icon_bg?: string
  card_icon_scale?: number
  icon_name?: string
  icon_color?: string
  icon_stroke?: number
}

export interface LayoutPage {
  page: number
  blocks: Record<string, LayoutBlockOffset>
  elements: LayoutElement[]
  background?: string
  background_mode?: 'solid' | 'linear' | 'radial' | 'image'
  gradient_angle?: number
  gradient_colors?: string[]
}

export interface ReportLayout {
  version: number
  mode: 'report' | 'blank'
  orientation: 'portrait' | 'landscape'
  pages: LayoutPage[]
}

export interface LayoutView {
  layout: ReportLayout
  updated_at: string | null
  updated_by: string | null
}

export interface UploadedDesignImage {
  id: string
  url: string
}

export interface MediaItem {
  source: 'design' | 'evidence'
  id: string
  filename: string
  content_type: string
  size_bytes: number
  preview_url: string
  created_at: string
  period_id: string
  period_label: string
  import_filename: string
  context: string
  kode: string
}

export interface MediaOption {
  label: string
  count: number
}

export interface MediaFilters {
  codes: MediaOption[]
  polres: MediaOption[]
  satker: MediaOption[]
}

export interface DesignSource {
  key: string
  label: string
  category: string
  table_columns: string[]
  table_rows: string[][]
  chart_labels: string[]
  chart_values: number[]
  chart_title: string
  can_chart: boolean
  can_table?: boolean
  can_card?: boolean
  card_label?: string
  card_value?: string
  card_unit?: string
  card_detail?: string
  card_icon?: string
}

export interface DesignTemplateView {
  id: string
  name: string
  layout: ReportLayout
  created_at: string
  created_by: string | null
}

export interface ReportView {
  id: string
  period_id: string
  created_at: string
  source_filename: string
  period_label: string
  pdf_url: string
  preview_url: string
}

export interface ReportShareView {
  token: string
  created_at: string
}

export interface PublicShareView {
  token: string
  title: string
  period_label: string
  created_at: string
}

export interface ReportContent {
  summary: string
  narkoba: string
  green_policing: string
  jumat_keliling: string
  subuh_keliling: string
  minggu_kasih: string
  aspirations: string
  follow_up: string
  media: string
  closing: string
  signatory_name: string
  signatory_rank: string
  signatory_nip: string
  report_city: string
  report_date: string
}

export interface DraftView {
  content: ReportContent
  updated_at: string | null
  updated_by: string | null
}

export interface PeriodDetail {
  period: Period
  imports: ImportView[]
  reports: ReportView[]
  draft: DraftView | null
}

export interface LabeledCount {
  label: string
  count: number
}

export interface UnitBreakdown {
  label: string
  rows: number
  polsek: number
  personel: number
  bibit: number
  area_m2: number
  jamaah: number
}

export interface MetricCard {
  key: string
  label: string
  value: number
  unit: string
  detail: string
}

export interface GroupMetrics {
  label: string
  rows: number
  people: number
  locations: number
  bibit: number
  area_m2: number
}

export interface RowSummary {
  source_row: number
  tanggal: string
  satuan: string
  polres: string
  kode: string
  kegiatan: string
  location: string
  detail: string
}

export interface DailyCount {
  date: string
  count: number
}

export interface NarkobaMetrics {
  rows: number
  orang_diamankan: number
  kawasan: number
  rehabilitasi: number
  peserta_penyuluhan: number
  by_code: LabeledCount[]
  by_polres: GroupMetrics[]
  top_locations: LabeledCount[]
  activities: RowSummary[]
}

export interface GreenMetrics {
  rows: number
  bibit: number
  area_m2: number
  kerja_bakti: number
  penanaman: number
  peserta: number
  by_polres: GroupMetrics[]
  by_kegiatan: LabeledCount[]
}

export interface ReligiousMetrics {
  rows: number
  people: number
  locations: number
  polres: number
  polsek: number
  by_polres: GroupMetrics[]
  by_religion: LabeledCount[]
  top_locations: LabeledCount[]
}

export interface AspirationMetrics {
  total: number
  themes: LabeledCount[]
  statuses: LabeledCount[]
  by_source: LabeledCount[]
  quotes: string[]
}

export interface FollowUpMetrics {
  total: number
  forms: LabeledCount[]
  activities: RowSummary[]
}

export interface MediaMetrics {
  publications: number
  documentation: number
  total: number
  rows_with_media: number
  by_channel: LabeledCount[]
  by_activity: LabeledCount[]
  top_domains: LabeledCount[]
}

export interface ReportMetrics {
  source_count: number
  source_filename: string
  polda_name: string
  period_label: string
  period_start: string
  period_end: string
  generated_at: string
  total_rows: number
  outside_period: number
  duplicate_rows: number
  personel_total: number
  bibit_total: number
  area_total: number
  jamaah_total: number
  rows_with_coordinates: number
  evidence_links: number
  cards: MetricCard[]
  quick_wins: LabeledCount[]
  codes: LabeledCount[]
  statuses: LabeledCount[]
  daily: DailyCount[]
  by_polres: LabeledCount[]
  by_level: LabeledCount[]
  by_satker: UnitBreakdown[]
  by_polres_breakdown: UnitBreakdown[]
  narkoba: NarkobaMetrics
  green_policing: GreenMetrics
  jumat_keliling: ReligiousMetrics
  subuh_keliling: ReligiousMetrics
  minggu_kasih: ReligiousMetrics
  aspirations: AspirationMetrics
  follow_up: FollowUpMetrics
  media: MediaMetrics
  highlights: RowSummary[]
}

export interface PreviewRow {
  source_row: number
  tanggal: string
  satuan: string
  kode: string
  kegiatan: string
  status_penerimaan: string
}

export interface ImportPreview {
  sheet: string
  total_rows: number
  valid_rows: number
  skipped_rows: number
  duplicate_rows: number
  unmatched_dates: number
  headers: string[]
  missing_headers: string[]
  errors: string[]
  warnings: string[]
  date_counts: LabeledCount[]
  quick_wins: LabeledCount[]
  statuses: LabeledCount[]
  file_date_start: string | null
  file_date_end: string | null
  evidence_links: number
  rows_with_coordinates: number
  personel_total: number
  bibit_total: number
  area_total: number
  jamaah_total: number
  preview_rows: PreviewRow[]
}

export interface RecordItem {
  id: number
  source_row: number
  import_id: string
  import_filename: string
  tanggal: string
  waktu_kirim_wib: string
  satuan: string
  tingkat: string
  polda: string
  polres: string
  polsek: string
  level: string
  unit: string
  kode: string
  qw_title: string
  kegiatan: string
  quick_wins: string
  pejabat_pelaksana: string
  jabatan: string
  pelaksana: string
  pelapor: string
  status_penerimaan: string
  bukti_dukung: string
  keluhan_personel_polri: string
  isian_laporan: string
  personel_count: number
  bibit_count: number
  area_m2: number
  jamaah_count: number
  latitude: number | null
  longitude: number | null
  map_url: string
  fields: Record<string, string>
  links: string[]
  evidence: EvidenceItem[]
}

export interface EvidenceItem {
  id: string
  source_url: string
  filename: string
  content_type: string
  size_bytes: number
  status: string
  error: string
  url: string
}

export interface RecordsResponse {
  date_start: string
  date_end: string
  source_count: number
  duplicate_rows: number
  total: number
  page: number
  page_size: number
  quick_wins: LabeledCount[]
  statuses: LabeledCount[]
  polres: LabeledCount[]
  records: RecordItem[]
}

export interface AnalysisResponse {
  date_start: string
  date_end: string
  source_count: number
  total_rows: number
  duplicate_rows: number
  personel_total: number
  bibit_total: number
  area_total: number
  jamaah_total: number
  rows_with_coordinates: number
  evidence_links: number
  by_group: LabeledCount[]
  by_code: LabeledCount[]
  by_level: LabeledCount[]
  by_unit: LabeledCount[]
  by_satker: UnitBreakdown[]
  by_polres_breakdown: UnitBreakdown[]
  by_date: LabeledCount[]
}

export interface UnitDetail {
  scope: 'polres' | 'satker'
  label: string
  date_start: string
  date_end: string
  polsek_count: number
  metrics: ReportMetrics
  by_polsek: UnitBreakdown[]
}

export interface QuickWinCodeCoverage {
  code: string
  title: string
  rows: number
  polres: number
  satker: number
}

export interface QuickWinActivity {
  record_id: number
  tanggal: string
  kode: string
  title: string
  polres: string
  polsek: string
  satker: string
  kegiatan: string
  pelaksana: string
  personel: number
  bibit: number
  area_m2: number
  jamaah: number
  map_url: string
}

export interface QuickWinDetail {
  code: string
  group: string
  title: string
  group_title: string
  date_start: string
  date_end: string
  total_rows: number
  personel: number
  polres_count: number
  satker_count: number
  communities: LabeledCount[]
  by_polres: UnitBreakdown[]
  by_satker: UnitBreakdown[]
  codes: QuickWinCodeCoverage[]
  polres_missing: string[]
  satker_missing: string[]
  activities_total: number
  activities: QuickWinActivity[]
}

export interface CoverageResponse {
  date_start: string
  date_end: string
  total_rows: number
  active_polres: number
  active_satker: number
  known_polres: string[]
  known_satker: string[]
  polres_options: string[]
  filter_polres: string
  groups: CoverageGroup[]
  programs: CoverageProgram[]
  codes: CoverageProgram[]
}

export interface CoverageGroup {
  code: string
  title: string
  rows: number
  polres: number
  satker: number
  polres_total: number
  satker_total: number
  polres_missing: string[]
  satker_missing: string[]
  by_polres: CoverageUnitCount[]
  by_satker: CoverageUnitCount[]
  codes: CoverageCode[]
}

export interface CoverageCode {
  code: string
  title: string
  rows: number
  polres: number
  satker: number
  polres_missing: number
  satker_missing: number
  by_polres: CoverageUnitCount[]
  by_satker: CoverageUnitCount[]
}

export interface CoverageUnitCount {
  label: string
  rows: number
}

export interface CoverageProgram {
  key: string
  title: string
  code: string
  rows: number
  polres: number
  satker: number
  polres_total: number
  satker_total: number
  polres_missing: string[]
  satker_missing: string[]
  metrics: CoverageMetric[]
  by_polres: CoverageUnit[]
  by_satker: CoverageUnit[]
  breakdowns: CoverageBreakdown[]
}

export interface CoverageBreakdown {
  key: string
  label: string
  filled_rows: number
  values: { label: string; count: number }[]
}

export interface CoverageMetric {
  key: string
  label: string
  unit: string
  total: number
  filled_rows: number
  missing_rows: number
  filled_polres: number
  filled_satker: number
}

export interface CoverageUnit {
  label: string
  rows: number
  values: CoverageUnitValue[]
  missing: string[]
}

export interface CoverageUnitValue {
  key: string
  value: number
  filled: boolean
}

export interface DashboardCounts {
  periods: number
  imports: number
  reports: number
  rows: number
}

export interface Dashboard {
  polda_name: string
  app_name: string
  counts: DashboardCounts
  latest_period: Period | null
  latest_import: ImportView | null
  metrics: ReportMetrics | null
  recent_periods: Period[]
  recent_reports: ReportView[]
}
