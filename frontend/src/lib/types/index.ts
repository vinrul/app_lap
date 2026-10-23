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
  created_at: string
  created_by: string
  imports: number
  reports: number
}

export interface ImportView {
  id: string
  filename: string
  row_count: number
  skipped_rows: number
  parser_version: string
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
  src?: string
  alt?: string
  object_fit?: 'contain' | 'cover' | 'fill'
  fill?: string
  border_color?: string
  border_width?: number
  radius?: number
  text?: string
  font_size?: number
  font_family?: 'sans' | 'serif'
  color?: string
  bold?: boolean
  italic?: boolean
  align?: 'left' | 'center' | 'right'
  background?: string
  source_block?: string
  binding?: string
  group?: string
  table_rows?: string[][]
  table_header?: boolean
  table_header_bg?: string
  table_header_color?: string
  table_stripe?: string
  chart_type?: 'bar' | 'line' | 'pie' | 'donut' | 'hbar'
  chart_title?: string
  chart_labels?: string[]
  chart_values?: number[]
  chart_colors?: string[]
  chart_show_values?: boolean
  chart_show_legend?: boolean
  chart_axis_color?: string
  card_label?: string
  card_value?: string
  card_unit?: string
  card_detail?: string
  card_accent?: string
  icon_name?: string
  icon_color?: string
  icon_stroke?: number
}

export interface LayoutPage {
  page: number
  blocks: Record<string, LayoutBlockOffset>
  elements: LayoutElement[]
}

export interface ReportLayout {
  version: number
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
  import_id: string
  created_at: string
  source_filename: string
  period_label: string
  pdf_url: string
  preview_url: string
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
  latest_import_id: string | null
  draft: DraftView | null
}

export interface LabeledCount {
  label: string
  count: number
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
  import_id: string
  source_filename: string
  polda_name: string
  period_label: string
  period_start: string
  period_end: string
  generated_at: string
  total_rows: number
  outside_period: number
  cards: MetricCard[]
  quick_wins: LabeledCount[]
  codes: LabeledCount[]
  statuses: LabeledCount[]
  daily: DailyCount[]
  by_polres: LabeledCount[]
  by_level: LabeledCount[]
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
  preview_rows: PreviewRow[]
}

export interface RecordItem {
  id: number
  source_row: number
  tanggal: string
  waktu_kirim_wib: string
  satuan: string
  tingkat: string
  polda: string
  polres: string
  polsek: string
  kode: string
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
  fields: Record<string, string>
  links: string[]
}

export interface RecordsResponse {
  import_id: string
  import_filename: string
  period_start: string
  period_end: string
  total: number
  page: number
  page_size: number
  quick_wins: LabeledCount[]
  statuses: LabeledCount[]
  polres: LabeledCount[]
  records: RecordItem[]
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
