import type {
  CoverageProgram,
  CoverageResponse,
  CoverageSchedule,
  LayoutElement,
  LayoutPage,
  LeadershipTimelineResponse,
  PeriodDetail,
  PoliranDay,
  PoliranSummaryResponse,
  AspirationThemeRow,
  AspirationsPageResponse,
  ReportLayout,
  ReportMetrics,
  QwTimelineEntry,
} from '../types'
import { formatNumber } from '../utils/format'
import { qwCodeTitles, qwGroupTitles } from '../utils/constants'

/**
 * Template desain "Laporan Mingguan 8 Quick Wins" yang meniru susunan PDF
 * laporan_mingguan.pdf. Semua isi dibuat sebagai elemen biasa (teks, kartu,
 * tabel, grafik) sehingga tulisan maupun angkanya dapat diedit langsung di
 * editor desain. Nilai diambil dari metrik periode dan cakupan impor saat
 * template dibuat.
 */

export interface WeeklyTemplateInput {
  metrics: ReportMetrics
  coverage: CoverageResponse | null
  detail: PeriodDetail
  /// Sumber gambar peta yang sudah diunggah ke media desain (`upload:<id>`).
  mapImageSrc?: string
  /// Peta khusus titik Kumpul Bersama (QW1.4).
  kumpulMapImageSrc?: string
  /// Peta khusus titik Green Policing (QW1.5).
  greenMapImageSrc?: string
  /// Laporan harian QW1.5 untuk grafik halaman Green Policing.
  greenDaily?: PoliranDay[]
  /// Peta khusus titik Jumat Keliling (QW1.6).
  jumlingMapImageSrc?: string
  /// Laporan harian QW1.6 untuk grafik halaman Jumat Keliling.
  jumlingDaily?: PoliranDay[]
  /// Peta khusus titik Subuh Keliling (QW1.7).
  subuhMapImageSrc?: string
  /// Laporan harian QW1.7 untuk grafik halaman Subuh Keliling.
  subuhDaily?: PoliranDay[]
  /// Peta khusus titik Minggu Kasih (QW1.8).
  mingguMapImageSrc?: string
  /// Laporan harian QW1.8 untuk grafik halaman Minggu Kasih.
  mingguDaily?: PoliranDay[]
  /// Laporan harian QW1.1 untuk grafik halaman Gatur/Polidik.
  gaturDaily?: PoliranDay[]
  /// Laporan harian QW1.2 untuk grafik halaman Gatur/Polidik.
  polidikDaily?: PoliranDay[]
  /// Laporan harian QW1.4 untuk grafik halaman Kumpul Bersama.
  kumpulDaily?: PoliranDay[]
  /// Data halaman "Apa Kata Mereka" (tema aspirasi dan wilayah terbanyak).
  aspirations?: AspirationsPageResponse | null
  /// Contoh saran/keluhan yang sudah difilter backend untuk halaman Gatur/Polidik.
  feedback?: { gatur: string[]; polidik: string[] }
  /// Keluhan warga unik untuk halaman Kumpul Bersama.
  kumpulFeedback?: string[]
  kumpulFeedbackTotal?: number
  /// Timeline kegiatan pimpinan dan isu forum pimpinan untuk halaman 4.
  leadership?: LeadershipTimelineResponse | null
  /// Ringkasan QW1.3 (Poliran) untuk halaman 5.
  poliran?: PoliranSummaryResponse | null
}

const FOREST = '#143d30'
const DEEP = '#0d2a21'
const GOLD = '#b8894b'
const INK = '#2f3d36'
const SOFT = '#5c6a63'
const MUTED = '#7d8983'
const LINE = '#dce3da'
const PANEL = '#f4f7f2'
const WHITE = '#ffffff'
const CREAM = '#f5efe3'
const DARK_PANEL = '#173f33'
const DARK_LINE = '#2f5b4a'
const PALE = '#cfe0d6'
const PALE_GOLD = '#e6d3b3'

const PALETTE = [
  FOREST,
  GOLD,
  '#2f6db0',
  '#3f765d',
  '#9c6b3f',
  '#6d7872',
  '#b0413e',
  '#7a9e8e',
]

const PAGE_WIDTH = 210
const PAGE_HEIGHT = 297

const dayMonthFormatter = new Intl.DateTimeFormat('id-ID', {
  day: 'numeric',
  month: 'short',
})

let elementCounter = 0

function nextId(prefix: string): string {
  elementCounter += 1
  return `weekly-${prefix}-${elementCounter}`
}

const fmt = (value: number | null | undefined): string =>
  formatNumber(Math.round(Number(value ?? 0)))

function dayMonth(value: string): string {
  if (!value) return '—'
  const date = new Date(`${value.slice(0, 10)}T12:00:00`)
  if (Number.isNaN(date.getTime())) return value
  return dayMonthFormatter.format(date)
}

const WEEKDAYS_UPPER = ['MIN', 'SEN', 'SEL', 'RAB', 'KAM', 'JUM', 'SAB']

function weekdayUpper(value: string): string {
  const date = new Date(`${value.slice(0, 10)}T12:00:00`)
  if (Number.isNaN(date.getTime())) return ''
  return WEEKDAYS_UPPER[date.getDay()]
}

function dayNumber(value: string): string {
  const date = new Date(`${value.slice(0, 10)}T12:00:00`)
  if (Number.isNaN(date.getTime())) return ''
  return String(date.getDate())
}

function weekdayIndex(value: string): number {
  const date = new Date(`${value.slice(0, 10)}T12:00:00`)
  return Number.isNaN(date.getTime()) ? -1 : date.getDay()
}

function timelineLabel(value: string): string {
  const weekday = weekdayUpper(value)
  const day = dayNumber(value)
  return [weekday, day].filter(Boolean).join(' ')
}

function shorten(value: string, max: number): string {
  const text = value.trim()
  return text.length > max ? `${text.slice(0, max - 1)}…` : text
}

function compactLocation(value: string, max = 48): string {
  const parts = value
    .split(',')
    .map((part) => part.trim())
    .filter(Boolean)
  const picked = parts.slice(0, 2).join(', ') || 'Forum Pimpinan Polda'
  if (picked.length <= max) return picked
  const cut = picked.slice(0, max)
  const lastSpace = cut.lastIndexOf(' ')
  return lastSpace > 20 ? cut.slice(0, lastSpace) : cut
}

function timelineCardText(entry: QwTimelineEntry): { title: string; location: string } {
  return {
    title: shorten(entry.kegiatan || entry.kode, 62),
    location: shorten(entry.location || 'Lokasi tidak dicatat', 58),
  }
}


function periodShort(start: string, end: string): string {
  const startDate = new Date(`${start.slice(0, 10)}T12:00:00`)
  const endDate = new Date(`${end.slice(0, 10)}T12:00:00`)
  if (Number.isNaN(startDate.getTime()) || Number.isNaN(endDate.getTime())) {
    return `${start} – ${end}`
  }
  if (startDate.getFullYear() === endDate.getFullYear()) {
    return `${dayMonth(start)} – ${dayMonth(end)} ${endDate.getFullYear()}`
  }
  return `${dayMonth(start)} ${startDate.getFullYear()} – ${dayMonth(end)} ${endDate.getFullYear()}`
}

function percent(value: number, total: number): number {
  if (total <= 0) return 0
  return Math.round((value / total) * 100)
}

function chunk<T>(items: T[], size: number): T[][] {
  const output: T[][] = []
  for (let index = 0; index < items.length; index += size) {
    output.push(items.slice(index, index + size))
  }
  return output
}

function cleanRows(rows: string[][], columns: number): string[][] {
  if (!rows.length) {
    return [Array.from({ length: columns }, (_, index) => (index === 0 ? 'Belum ada data' : '—'))]
  }
  return rows.map((row) =>
    Array.from({ length: columns }, (_, index) => row[index] ?? ''),
  )
}

interface Box {
  x: number
  y: number
  width: number
  height: number
}

interface TextOptions {
  size?: number
  family?: string
  color?: string
  bold?: boolean
  italic?: boolean
  align?: 'left' | 'center' | 'right'
  valign?: 'top' | 'middle' | 'bottom'
  background?: string
  z?: number
}

interface RectOptions {
  fill?: string
  borderColor?: string
  borderWidth?: number
  radius?: number
  z?: number
}

interface CardOptions {
  fill?: string
  labelColor?: string
  valueColor?: string
  unitColor?: string
  detailColor?: string
  accent?: string
  borderColor?: string
  labelSize?: number
  valueSize?: number
  unitSize?: number
  detailSize?: number
  icon?: string
  iconColor?: string
  iconBg?: string
  template?: string
  z?: number
}

interface TableOptions {
  headerBg?: string
  headerColor?: string
  stripe?: string
  fill?: string
  color?: string
  borderColor?: string
  fontSize?: number
  align?: ('left' | 'center' | 'right' | '')[]
  columnColors?: string[]
  z?: number
}

interface ChartOptions {
  color?: string
  colors?: string[]
  showValues?: boolean
  showLegend?: boolean
  titleSize?: number
  labelSize?: number
  valueSize?: number
  legendSize?: number
  z?: number
}

function element(
  type: LayoutElement['type'],
  box: Box,
  props: Partial<LayoutElement> = {},
): LayoutElement {
  return {
    id: nextId(type),
    type,
    x: box.x,
    y: box.y,
    width: box.width,
    height: box.height,
    rotation: 0,
    opacity: 1,
    z: 10,
    ...props,
  }
}

function text(
  page: LayoutPage,
  box: Box,
  value: string,
  options: TextOptions = {},
): void {
  page.elements.push(
    element('text', box, {
      text: value,
      font_size: options.size ?? 8.5,
      font_family: options.family ?? 'inter',
      color: options.color ?? FOREST,
      bold: options.bold ?? false,
      italic: options.italic ?? false,
      align: options.align ?? 'left',
      valign: options.valign ?? 'top',
      background: options.background ?? 'transparent',
      z: options.z ?? 20,
    }),
  )
}

function rect(page: LayoutPage, box: Box, options: RectOptions = {}): void {
  page.elements.push(
    element('rect', box, {
      fill: options.fill ?? LINE,
      fill_mode: 'solid',
      border_color: options.borderColor ?? options.fill ?? LINE,
      border_width: options.borderWidth ?? 0,
      radius: options.radius ?? 0,
      z: options.z ?? 2,
    }),
  )
}

function card(
  page: LayoutPage,
  box: Box,
  label: string,
  value: string,
  unit: string,
  detail: string,
  options: CardOptions = {},
): void {
  page.elements.push(
    element('card', box, {
      card_label: label,
      card_value: value,
      card_unit: unit,
      card_detail: detail,
      card_accent: options.accent ?? GOLD,
      fill: options.fill ?? WHITE,
      color: options.labelColor ?? FOREST,
      border_color: options.borderColor ?? LINE,
      border_width: 0.6,
      radius: 2.2,
      font_size: 8,
      card_label_font_family: 'inter',
      card_label_font_size: options.labelSize ?? 6.8,
      card_label_color: options.labelColor ?? SOFT,
      card_value_font_family: 'montserrat',
      card_value_font_size: options.valueSize ?? 19,
      card_value_color: options.valueColor ?? FOREST,
      card_unit_font_family: 'inter',
      card_unit_font_size: options.unitSize ?? 6.6,
      card_unit_color: options.unitColor ?? MUTED,
      card_detail_font_family: 'inter',
      card_detail_font_size: options.detailSize ?? 6.1,
      card_detail_color: options.detailColor ?? SOFT,
      card_template: options.template ?? 'minimal',
      card_icon: options.icon ?? '',
      card_icon_color: options.iconColor ?? GOLD,
      card_icon_bg: options.iconBg ?? CREAM,
      card_icon_scale: 100,
      z: options.z ?? 10,
    }),
  )
}

function table(
  page: LayoutPage,
  box: Box,
  columns: string[],
  rows: string[][],
  options: TableOptions = {},
): void {
  const normalized = cleanRows(rows, columns.length)
  const aligns =
    options.align ?? columns.map((_, index) => (index === 0 ? 'left' : 'right'))
  page.elements.push(
    element('table', box, {
      table_rows: [columns, ...normalized],
      table_header: true,
      table_header_bg: options.headerBg ?? FOREST,
      table_header_color: options.headerColor ?? WHITE,
      table_stripe: options.stripe ?? PANEL,
      fill: options.fill ?? WHITE,
      color: options.color ?? INK,
      border_color: options.borderColor ?? LINE,
      border_width: 0.5,
      radius: 1.5,
      font_size: options.fontSize ?? 7.2,
      font_family: 'inter',
      table_column_aligns: aligns,
      table_column_colors: options.columnColors ?? [],
      z: options.z ?? 10,
    }),
  )
}

function chart(
  page: LayoutPage,
  box: Box,
  type: 'bar' | 'line' | 'pie' | 'donut' | 'hbar',
  title: string,
  labels: string[],
  values: number[],
  options: ChartOptions = {},
): void {
  page.elements.push(
    element('chart', box, {
      chart_type: type,
      chart_title: title,
      chart_labels: labels.length ? labels : ['Belum ada data'],
      chart_values: values.length ? values : [0],
      chart_colors: (labels.length ? labels : ['Belum ada data']).map(
        (_, index) => options.colors?.[index] ?? PALETTE[index % PALETTE.length],
      ),
      chart_show_values: options.showValues ?? true,
      chart_show_legend: options.showLegend ?? false,
      chart_axis_color: MUTED,
      chart_title_font_family: 'inter',
      chart_title_font_size: options.titleSize ?? 8.5,
      chart_label_font_size: options.labelSize ?? 6.4,
      chart_value_font_size: options.valueSize ?? 7,
      chart_value_font_family: 'montserrat',
      chart_legend_font_size: options.legendSize ?? 6.4,
      fill: 'transparent',
      color: options.color ?? FOREST,
      z: options.z ?? 10,
    }),
  )
}

function newPage(
  page: number,
  background = WHITE,
  gradient?: { colors: string[]; angle?: number },
): LayoutPage {
  return {
    page,
    blocks: {},
    elements: [],
    background: gradient ? '' : background,
    background_mode: gradient ? 'linear' : 'solid',
    gradient_angle: gradient?.angle ?? 135,
    gradient_colors: gradient?.colors ?? [],
  }
}

interface Frame {
  polda: string
  period: string
  shortPeriod: string
}

function addHeader(page: LayoutPage, frame: Frame): void {
  text(page, { x: 14, y: 8.5, width: 92, height: 5 }, frame.polda, {
    size: 7,
    bold: true,
    color: FOREST,
  })
  text(page, { x: 104, y: 8.5, width: 92, height: 5 }, frame.period, {
    size: 6.8,
    color: MUTED,
    align: 'right',
  })
  text(
    page,
    { x: 14, y: 13.5, width: 140, height: 5 },
    'Laporan Mingguan 8 Quick Wins Kapolri',
    { size: 7, color: SOFT },
  )
  rect(page, { x: 14, y: 20, width: 182, height: 0.45 }, { fill: GOLD, z: 3 })
}

function addFooter(page: LayoutPage, frame: Frame, pageNumber: number): void {
  rect(page, { x: 14, y: 281.5, width: 182, height: 0.35 }, { fill: LINE, z: 3 })
  text(
    page,
    { x: 14, y: 283.5, width: 160, height: 5 },
    `${frame.polda} · LAPORAN MINGGUAN 8 QUICK WINS KAPOLRI · ${frame.shortPeriod.toUpperCase()}`,
    { size: 5.8, color: MUTED },
  )
  text(page, { x: 178, y: 283.5, width: 18, height: 5 }, `Halaman ${pageNumber}`, {
    size: 5.8,
    color: MUTED,
    align: 'right',
  })
}

function addSectionHeading(
  page: LayoutPage,
  y: number,
  eyebrow: string,
  title: string,
  subtitle: string,
): number {
  text(page, { x: 14, y, width: 182, height: 5 }, eyebrow.toUpperCase(), {
    size: 6.6,
    bold: true,
    color: GOLD,
  })
  text(page, { x: 14, y: y + 5.5, width: 182, height: 10 }, title, {
    size: 18,
    bold: true,
    family: 'montserrat',
    color: FOREST,
  })
  let cursor = y + 17
  const lines = chunk(subtitle.split(' '), 18).map((line) => line.join(' '))
  for (const line of lines.slice(0, 3)) {
    text(page, { x: 14, y: cursor, width: 182, height: 4.5 }, line, {
      size: 7.6,
      color: SOFT,
    })
    cursor += 4.2
  }
  return cursor
}

function addCards(
  page: LayoutPage,
  y: number,
  items: { label: string; value: string; unit: string; detail: string }[],
  options: {
    x?: number
    width?: number
    height?: number
    columns?: number
    gap?: number
    valueSize?: number
    labelSize?: number
  } = {},
): number {
  const x = options.x ?? 14
  const width = options.width ?? 182
  const height = options.height ?? 27
  const columns = Math.max(1, options.columns ?? 4)
  const gap = options.gap ?? 3.5
  const itemWidth = (width - gap * (columns - 1)) / columns
  items.forEach((item, index) => {
    const column = index % columns
    const row = Math.floor(index / columns)
    card(
      page,
      {
        x: x + column * (itemWidth + gap),
        y: y + row * (height + gap),
        width: itemWidth,
        height,
      },
      item.label,
      item.value,
      item.unit,
      item.detail,
      { valueSize: options.valueSize, labelSize: options.labelSize },
    )
  })
  const rows = Math.ceil(items.length / columns)
  return y + rows * (height + gap)
}

function addPanelTitle(page: LayoutPage, box: Box, title: string, detail = ''): void {
  text(page, { x: box.x, y: box.y, width: box.width, height: 4.5 }, title.toUpperCase(), {
    size: 6.4,
    bold: true,
    color: MUTED,
  })
  if (detail) {
    text(page, { x: box.x, y: box.y + 4.2, width: box.width, height: 4 }, detail, {
      size: 6.4,
      color: MUTED,
    })
  }
}

interface Data {
  metrics: ReportMetrics
  coverage: CoverageResponse | null
  detail: PeriodDetail
  frame: Frame
  mapImageSrc: string
  kumpulMapImageSrc: string
  greenMapImageSrc: string
  greenDaily: PoliranDay[]
  jumlingMapImageSrc: string
  jumlingDaily: PoliranDay[]
  subuhMapImageSrc: string
  subuhDaily: PoliranDay[]
  mingguMapImageSrc: string
  mingguDaily: PoliranDay[]
  gaturDaily: PoliranDay[]
  polidikDaily: PoliranDay[]
  kumpulDaily: PoliranDay[]
  aspirations: AspirationsPageResponse | null
  feedback: { gatur: string[]; polidik: string[] }
  kumpulFeedback: string[]
  kumpulFeedbackTotal: number
  leadership: LeadershipTimelineResponse | null
  poliran: PoliranSummaryResponse | null
}

function codeRows(data: Data, code: string): number {
  const fromCoverage = data.coverage?.codes.find((item) => item.code === code)
  if (fromCoverage) return fromCoverage.rows
  const fromMetrics = data.metrics.codes.find((item) =>
    item.label.trim().toUpperCase().startsWith(code.toUpperCase()),
  )
  return fromMetrics?.count ?? 0
}

function codeTitle(data: Data, code: string): string {
  return (
    data.coverage?.codes.find((item) => item.code === code)?.title ??
    qwCodeTitles[code] ??
    code
  )
}

function codeProgram(data: Data, code: string): CoverageProgram | null {
  return data.coverage?.codes.find((item) => item.code === code) ?? null
}

function findBreakdown(item: CoverageProgram | null, keywords: string[]) {
  if (!item) return null
  return (
    item.breakdowns.find((breakdown) => {
      const label = `${breakdown.key} ${breakdown.label}`.toLowerCase()
      return keywords.some((keyword) => label.includes(keyword))
    }) ?? null
  )
}

function breakdownTotal(item: CoverageProgram | null, keywords: string[]): number {
  const breakdown = findBreakdown(item, keywords)
  if (!breakdown) return 0
  return breakdown.values.reduce((total, value) => total + value.count, 0)
}

function breakdownValue(
  item: CoverageProgram | null,
  keywords: string[],
  labels: string[],
): number {
  const breakdown = findBreakdown(item, keywords)
  if (!breakdown) return 0
  return breakdown.values
    .filter((value) =>
      labels.some((label) => value.label.toLowerCase().includes(label)),
    )
    .reduce((total, value) => total + value.count, 0)
}

function metricTotal(item: CoverageProgram | null, keywords: string[]): number {
  if (!item) return 0
  const metric = item.metrics.find((metric) => {
    const label = `${metric.key} ${metric.label}`.toLowerCase()
    return keywords.some((keyword) => label.includes(keyword))
  })
  return metric?.total ?? 0
}

function scheduledReport(data: Data, code: string): {
  present: boolean
  count: number
  dateLabel: string
} {
  const item = schedule(data, code)
  if (!item || item.expected_dates.length === 0) {
    return { present: false, count: 0, dateLabel: '' }
  }
  const count = [...item.polres, ...item.satker].reduce(
    (total, unit) => total + unit.on_time_rows,
    0,
  )
  const dates = item.expected_dates.map((date) => dayMonth(date)).join(', ')
  return { present: true, count, dateLabel: dates }
}

function outsideScheduleCount(data: Data, code: string): number {
  const item = schedule(data, code)
  if (!item) return 0
  return [...item.polres, ...item.satker].reduce(
    (total, unit) => total + unit.off_schedule_rows,
    0,
  )
}

function uniqueFeedback(items: string[] | undefined, limit: number): string[] {
  const seen = new Set<string>()
  const unique: string[] = []
  for (const item of items ?? []) {
    const text = item.trim()
    if (!text) continue
    const key = text.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    unique.push(text)
    if (unique.length >= limit) break
  }
  return unique
}

function feedbackLine(items: string[] | undefined, fallback: string): string {
  const candidates = (items ?? []).map((item) => item.trim()).filter(Boolean)
  if (!candidates.length) return fallback
  const picked: string[] = []
  let length = 0
  for (const item of candidates) {
    if (picked.length >= 4) break
    if (picked.length >= 2 && length + item.length > 340) break
    if (picked.length >= 3 && length + item.length > 300) break
    picked.push(item)
    length += item.length + 2
  }
  return picked.join('; ')
}


function schedule(data: Data, code: string): CoverageSchedule | null {
  return data.coverage?.schedule.find((item) => item.code === code) ?? null
}

function scheduleStats(item: CoverageSchedule | null): {
  total: number
  onTime: number
  percentValue: number
} {
  if (!item) return { total: 0, onTime: 0, percentValue: 0 }
  const onTime = item.polres.filter((unit) => unit.status === 'on_time').length
  return {
    total: item.polres.length,
    onTime,
    percentValue: percent(onTime, item.polres.length),
  }
}

function scheduleRows(item: CoverageSchedule | null, limit = 12): string[][] {
  if (!item) return []
  return item.polres.slice(0, limit).map((unit) => {
    const status =
      unit.status === 'on_time'
        ? 'Tepat jadwal'
        : unit.status === 'off_schedule'
          ? 'Di luar jadwal'
          : unit.status === 'not_scheduled'
            ? 'Tidak dijadwalkan'
            : 'Belum melapor'
    return [unit.label, status, fmt(unit.on_time_rows + unit.off_schedule_rows)]
  })
}

function codeCoverageRows(item: CoverageProgram | null, limit = 12): string[][] {
  if (!item) return []
  return item.by_polres.slice(0, limit).map((unit) => [
    unit.label,
    fmt(unit.rows),
    unit.rows > 0 ? 'Melapor' : 'Belum melapor',
  ])
}

function topLabeledRows(
  items: { label: string; count: number }[],
  limit = 10,
  firstHeader = 'Kategori',
  secondHeader = 'Jumlah',
): { columns: string[]; rows: string[][] } {
  const sorted = [...items].sort((left, right) => right.count - left.count).slice(0, limit)
  return {
    columns: [firstHeader, secondHeader],
    rows: sorted.map((item) => [item.label, fmt(item.count)]),
  }
}

function quoteText(data: Data, limit = 5): string {
  const quotes = data.metrics.aspirations.quotes.slice(0, limit)
  if (!quotes.length) return 'Belum ada kutipan aspirasi warga pada periode ini.'
  return quotes.map((quote, index) => `${index + 1}. ${quote}`).join('\n')
}

function narrative(data: Data, code?: string): string {
  const highlights = data.metrics.highlights
    .filter((item) => !code || item.kode.toUpperCase().startsWith(code.toUpperCase()))
    .slice(0, 3)
    .map((item) => `${item.satuan}: ${item.kegiatan}${item.location ? ` (${item.location})` : ''}`)
  if (highlights.length) return highlights.join('\n')
  if (code) {
    const title = codeTitle(data, code)
    return `${title} tercatat ${fmt(codeRows(data, code))} laporan pada periode ${data.frame.period}.`
  }
  return `Rekap ${fmt(data.metrics.total_rows)} baris laporan unik dari ${data.metrics.by_polres.length} polres pada periode ${data.frame.period}.`
}


function buildCover(data: Data): LayoutPage {
  const page = newPage(1, WHITE, { colors: [DEEP, FOREST, '#1d4a3a'], angle: 160 })
  rect(page, { x: 0, y: 0, width: 5, height: PAGE_HEIGHT }, { fill: GOLD, z: 1 })
  text(page, { x: 18, y: 18, width: 120, height: 6 }, 'LAPORAN MINGGUAN', {
    size: 10,
    bold: true,
    color: GOLD,
  })
  text(page, { x: 18, y: 26, width: 178, height: 16 }, '8 QUICK WINS KAPOLRI', {
    size: 27,
    bold: true,
    family: 'montserrat',
    color: WHITE,
  })
  text(page, { x: 18, y: 43, width: 178, height: 8 }, 'POLDA BALI DAN JAJARAN', {
    size: 12.5,
    family: 'montserrat',
    color: PALE,
  })
  text(page, { x: 18, y: 53, width: 178, height: 7 }, data.frame.period, {
    size: 9.5,
    color: PALE_GOLD,
  })
  text(
    page,
    { x: 18, y: 63, width: 178, height: 8 },
    `${fmt(data.metrics.total_rows)} laporan unik dari ${data.metrics.by_polres.length} polres dan ${data.metrics.by_satker.length} satker; ${fmt(data.metrics.rows_with_coordinates)} baris berkoordinat.`,
    { size: 7.4, color: PALE },
  )

  const coverCards = [
    {
      label: 'Gatur Lalin',
      value: fmt(codeRows(data, 'QW1.1')),
      unit: 'laporan',
      detail: `${scheduleStats(schedule(data, 'QW1.1')).percentValue}% polres tepat jadwal`,
    },
    {
      label: 'Polidik',
      value: fmt(codeRows(data, 'QW1.2')),
      unit: 'laporan',
      detail: `${scheduleStats(schedule(data, 'QW1.2')).percentValue}% polres tepat jadwal`,
    },
    {
      label: 'Kumpul Bersama',
      value: fmt(codeRows(data, 'QW1.4')),
      unit: 'laporan',
      detail: `${scheduleStats(schedule(data, 'QW1.4')).percentValue}% polres tepat jadwal`,
    },
    {
      label: 'Poliran',
      value: fmt(codeRows(data, 'QW1.3')),
      unit: 'laporan',
      detail: `${scheduleStats(schedule(data, 'QW1.3')).percentValue}% polres tepat jadwal`,
    },
    {
      label: 'Green Policing',
      value: fmt(data.metrics.green_policing.rows),
      unit: 'kegiatan',
      detail: `${fmt(data.metrics.green_policing.bibit)} bibit ditanam`,
    },
    {
      label: 'Jumat Keliling',
      value: fmt(data.metrics.jumat_keliling.rows),
      unit: 'kegiatan',
      detail: `${fmt(data.metrics.jumat_keliling.people)} jemaah`,
    },
    {
      label: 'Subuh Keliling',
      value: fmt(data.metrics.subuh_keliling.rows),
      unit: 'kegiatan',
      detail: `${fmt(data.metrics.subuh_keliling.people)} jemaah`,
    },
    {
      label: 'Minggu Kasih',
      value: fmt(data.metrics.minggu_kasih.rows),
      unit: 'kegiatan',
      detail: `${fmt(data.metrics.minggu_kasih.people)} jemaat`,
    },
    {
      label: 'Amplifikasi Media',
      value: fmt(data.metrics.media.total),
      unit: 'tautan',
      detail: `${fmt(data.metrics.media.rows_with_media)} kegiatan teramplifikasi`,
    },
  ]

  const cardWidth = (178 - 2 * 3.5) / 3
  coverCards.forEach((item, index) => {
    const column = index % 3
    const row = Math.floor(index / 3)
    card(
      page,
      {
        x: 18 + column * (cardWidth + 3.5),
        y: 80 + row * 33,
        width: cardWidth,
        height: 30,
      },
      item.label,
      item.value,
      item.unit,
      item.detail,
      {
        fill: DARK_PANEL,
        borderColor: DARK_LINE,
        labelColor: PALE,
        valueColor: WHITE,
        unitColor: PALE_GOLD,
        detailColor: '#a9c1b4',
        accent: GOLD,
        valueSize: 20,
      },
    )
  })

  text(
    page,
    { x: 18, y: 183, width: 178, height: 5 },
    `Rekap aplikasi Quick Wins · ${data.frame.period} · sumber ${data.metrics.source_filename || 'impor periode'}`,
    { size: 6.8, color: PALE },
  )

  const groups = (data.coverage?.groups ?? []).map((group) => [
    `${group.code} · ${group.title}`,
    fmt(group.rows),
    fmt(group.polres),
  ])
  const quickWinsRows = groups.length
    ? groups
    : data.metrics.quick_wins.map((item) => [item.label, fmt(item.count), '—'])
  table(
    page,
    { x: 18, y: 192, width: 178, height: 84 },
    ['Kelompok Quick Wins', 'Laporan', 'Polres'],
    quickWinsRows,
    {
      headerBg: '#245343',
      headerColor: PALE,
      stripe: '#1d4a3a',
      fill: DARK_PANEL,
      color: '#e3ece6',
      borderColor: DARK_LINE,
      fontSize: 7,
      align: ['left', 'right', 'right'],
    },
  )
  text(
    page,
    { x: 18, y: 279, width: 178, height: 5 },
    `${data.frame.polda} · LAPORAN MINGGUAN 8 QUICK WINS KAPOLRI · ${data.frame.shortPeriod.toUpperCase()}`,
    { size: 5.8, color: PALE },
  )
  return page
}

function addFeedbackPanel(
  page: LayoutPage,
  box: Box,
  title: string,
  items: string[] | undefined,
  fallback: string,
): void {
  rect(page, box, { fill: PANEL, radius: 2, z: 2 })
  text(
    page,
    { x: box.x + 3, y: box.y + 2.5, width: box.width - 6, height: 4 },
    title,
    { size: 6.4, bold: true, color: FOREST },
  )
  text(
    page,
    { x: box.x + 3, y: box.y + 7, width: box.width - 6, height: box.height - 9 },
    feedbackLine(items, fallback),
    { size: 5.8, color: SOFT },
  )
}

function addBreakdownChart(
  page: LayoutPage,
  box: Box,
  title: string,
  item: CoverageProgram | null,
  keywords: string[],
  color: string,
): void {
  const breakdown = findBreakdown(item, keywords)
  const values = [...(breakdown?.values ?? [])]
    .sort((left, right) => right.count - left.count)
    .slice(0, 5)
  if (!values.length) {
    addPanelTitle(page, box, title)
    text(
      page,
      { x: box.x, y: box.y + 7, width: box.width, height: 10 },
      'Belum ada data pada periode ini.',
      { size: 6.2, color: MUTED },
    )
    return
  }
  chart(
    page,
    box,
    'hbar',
    title,
    values.map((value) => value.label),
    values.map((value) => value.count),
    {
      colors: values.map(() => color),
      showValues: true,
      titleSize: 7.8,
      labelSize: 6.5,
      valueSize: 6.6,
    },
  )
}

function buildGaturPolidik(data: Data): LayoutPage {
  const page = newPage(2)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 2)
  const start = addSectionHeading(
    page,
    26,
    'Gatur Lalin dan Polidik',
    'Gatur Pagi dan Polisi Peduli Pendidikan',
    `Gatur pagi di ${fmt(codeRows(data, 'QW1.1'))} laporan dan Polidik di ${fmt(codeRows(data, 'QW1.2'))} laporan. Titik koordinat, alasan pemilihan titik, cara bertindak, bentuk kegiatan, serta tema materi dirangkum pada halaman ini.`,
  )

  const gaturProgram = codeProgram(data, 'QW1.1')
  const polidikProgram = codeProgram(data, 'QW1.2')
  const gaturStats = scheduleStats(schedule(data, 'QW1.1'))
  const polidikStats = scheduleStats(schedule(data, 'QW1.2'))
  const gaturPolres = gaturProgram?.polres ?? gaturStats.total
  const titikGatur =
    breakdownTotal(gaturProgram, ['lokasi-gatur', 'titik gatur', 'jumlah lokasi']) ||
    metricTotal(gaturProgram, ['lokasi', 'titik'])
  const siswa = breakdownValue(
    polidikProgram,
    ['personel-terlibat', 'peserta'],
    ['siswa', 'pelajar', 'murid'],
  )
  const guru = breakdownValue(
    polidikProgram,
    ['personel-terlibat', 'peserta'],
    ['guru', 'staf'],
  )
  const pesertaFallback = metricTotal(polidikProgram, ['peserta', 'siswa'])
  const siswaGuru = siswa + guru || pesertaFallback
  const detailSiswa =
    siswa || guru
      ? `${fmt(siswa)} siswa · ${fmt(guru)} guru`
      : 'peserta tercatat pada isian laporan'

  const nextY = addCards(page, start + 2, [
    {
      label: 'Gatur Lalin',
      value: fmt(codeRows(data, 'QW1.1')),
      unit: 'laporan',
      detail: `cakupan ${gaturStats.percentValue}% polres`,
    },
    {
      label: 'Polidik',
      value: fmt(codeRows(data, 'QW1.2')),
      unit: 'laporan',
      detail: `cakupan ${polidikStats.percentValue}% polres`,
    },
    {
      label: 'Titik gatur pagi',
      value: fmt(titikGatur),
      unit: 'titik',
      detail: `${fmt(gaturPolres)} polres melapor`,
    },
    {
      label: 'Siswa & guru terlibat',
      value: fmt(siswaGuru),
      unit: 'orang',
      detail: detailSiswa,
    },
  ], { height: 25 })

  const mapY = nextY + 3
  const mapBox = { x: 14, y: mapY, width: 100, height: 71 }
  if (data.mapImageSrc) {
    rect(page, mapBox, {
      fill: WHITE,
      borderColor: LINE,
      borderWidth: 0.6,
      radius: 2,
      z: 9,
    })
    page.elements.push(
      element('image', mapBox, {
        src: data.mapImageSrc,
        alt: 'Peta sebaran titik gatur lalin dan polidik',
        object_fit: 'cover',
        z: 10,
      }),
    )
  } else {
    rect(page, mapBox, { fill: PANEL, borderColor: LINE, borderWidth: 0.6, radius: 2, z: 2 })
    text(
      page,
      { x: mapBox.x + 5, y: mapBox.y + 30, width: mapBox.width - 10, height: 16 },
      'Peta titik koordinat belum tersedia. Pastikan laporan QW1.1/QW1.2 memiliki koordinat pada kolom peta.',
      { size: 6.4, color: MUTED, align: 'center', valign: 'middle' },
    )
  }

  const gaturSchedule = schedule(data, 'QW1.1')
  const mondayWeekday = (gaturSchedule?.weekday ?? 1) % 7
  const mondayDates = (gaturSchedule?.expected_dates ?? []).map(dayMonth).join(', ')
  const dailyMap = new Map<string, number>()
  for (const item of [...(data.gaturDaily ?? []), ...(data.polidikDaily ?? [])]) {
    dailyMap.set(item.date, (dailyMap.get(item.date) ?? 0) + item.count)
  }
  const daily = [...dailyMap.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([date, count]) => ({ date, count }))
  addPanelTitle(
    page,
    { x: 116, y: mapY, width: 80, height: 10 },
    'Laporan harian QW1.1 & QW1.2',
    `Batang emas = hari Senin${mondayDates ? ` (${mondayDates})` : ''}`,
  )
  chart(
    page,
    { x: 116, y: mapY + 10, width: 80, height: 67 },
    'bar',
    '',
    daily.map((item) => timelineLabel(item.date)),
    daily.map((item) => item.count),
    {
      colors: daily.map((item) =>
        weekdayIndex(item.date) === mondayWeekday ? GOLD : FOREST,
      ),
      showValues: true,
      showLegend: false,
      titleSize: 7.8,
      labelSize: 6.2,
      valueSize: 6.4,
    },
  )

  addFeedbackPanel(
    page,
    { x: 14, y: mapY + 81, width: 88, height: 24 },
    'SUARA WARGA · GATUR',
    data.feedback.gatur,
    'Belum ada saran atau keluhan warga pada laporan QW1.1.',
  )
  addFeedbackPanel(
    page,
    { x: 108, y: mapY + 81, width: 88, height: 24 },
    'SUARA SEKOLAH · POLIDIK',
    data.feedback.polidik,
    'Belum ada saran atau keluhan sekolah pada laporan QW1.2.',
  )

  addBreakdownChart(
    page,
    { x: 14, y: mapY + 108, width: 88, height: 35 },
    'Alasan memilih titik',
    gaturProgram,
    ['alasan', 'pertimbangan'],
    FOREST,
  )
  addBreakdownChart(
    page,
    { x: 108, y: mapY + 108, width: 88, height: 35 },
    'Cara bertindak',
    gaturProgram,
    ['cara bertindak', 'bertindak'],
    FOREST,
  )
  addBreakdownChart(
    page,
    { x: 14, y: mapY + 147, width: 88, height: 35 },
    'Bentuk kegiatan Polidik',
    polidikProgram,
    ['bentuk kegiatan', 'bentuk'],
    GOLD,
  )
  addBreakdownChart(
    page,
    { x: 108, y: mapY + 147, width: 88, height: 35 },
    'Tema materi Polidik',
    polidikProgram,
    ['tema materi', 'tema', 'materi'],
    GOLD,
  )

  return page
}

function buildKumpulBersama(data: Data): LayoutPage {
  const page = newPage(3)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 3)
  const start = addSectionHeading(
    page,
    26,
    'Kumpul Bersama Masyarakat',
    'Dialog dengan Warga dan Komunitas',
    `Tercatat ${fmt(codeRows(data, 'QW1.4'))} kegiatan dialog bersama desa adat, paguyuban, nelayan, ojek, dan pelaku wisata. Peta titik koordinat, laporan khusus hari Rabu, laporan di luar jadwal, dan suara warga dirangkum pada halaman ini.`,
  )
  const program = codeProgram(data, 'QW1.4')
  const scheduleInfo = schedule(data, 'QW1.4')
  const stats = scheduleStats(scheduleInfo)
  const wednesday = scheduledReport(data, 'QW1.4')
  const total = codeRows(data, 'QW1.4')
  const outside = outsideScheduleCount(data, 'QW1.4') || Math.max(0, total - wednesday.count)
  const feedbackTotal = data.kumpulFeedbackTotal || data.kumpulFeedback.length
  const polresCount = program?.polres ?? stats.total
  const nextY = addCards(page, start + 2, [
    {
      label: 'Kegiatan',
      value: fmt(total),
      unit: 'laporan',
      detail: `cakupan ${stats.percentValue}% polres`,
    },
    {
      label: 'Polres melapor',
      value: fmt(polresCount),
      unit: 'polres',
      detail: `dari ${fmt(stats.total)} polres terjadwal`,
    },
    {
      label: 'Laporan di luar hari Rabu',
      value: fmt(outside),
      unit: 'laporan',
      detail: `dari total ${fmt(total)} laporan`,
    },
    {
      label: 'Saran & keluhan',
      value: fmt(feedbackTotal),
      unit: 'isian',
      detail: 'dari laporan Kumpul Bersama',
    },
  ], { height: 25 })

  const mapY = nextY + 3
  const mapBox = { x: 14, y: mapY, width: 100, height: 71 }
  if (data.kumpulMapImageSrc) {
    rect(page, mapBox, {
      fill: WHITE,
      borderColor: LINE,
      borderWidth: 0.6,
      radius: 2,
      z: 9,
    })
    page.elements.push(
      element('image', mapBox, {
        src: data.kumpulMapImageSrc,
        alt: 'Peta sebaran titik Kumpul Bersama Masyarakat',
        object_fit: 'cover',
        z: 10,
      }),
    )
  } else {
    rect(page, mapBox, { fill: PANEL, borderColor: LINE, borderWidth: 0.6, radius: 2, z: 2 })
    text(
      page,
      { x: mapBox.x + 5, y: mapBox.y + 30, width: mapBox.width - 10, height: 16 },
      'Peta titik koordinat belum tersedia. Pastikan laporan QW1.4 memiliki koordinat pada kolom peta.',
      { size: 6.4, color: MUTED, align: 'center', valign: 'middle' },
    )
  }

  const wednesdayWeekday = (scheduleInfo?.weekday ?? 3) % 7
  const wednesdayDates = (scheduleInfo?.expected_dates ?? []).map(dayMonth).join(', ')
  const daily = data.kumpulDaily ?? []
  addPanelTitle(
    page,
    { x: 116, y: mapY, width: 80, height: 10 },
    'Laporan harian QW1.4',
    `Batang emas = hari Rabu${wednesdayDates ? ` (${wednesdayDates})` : ''}`,
  )
  chart(
    page,
    { x: 116, y: mapY + 10, width: 80, height: 67 },
    'bar',
    '',
    daily.map((item) => timelineLabel(item.date)),
    daily.map((item) => item.count),
    {
      colors: daily.map((item) =>
        weekdayIndex(item.date) === wednesdayWeekday ? GOLD : FOREST,
      ),
      showValues: true,
      showLegend: false,
      titleSize: 7.8,
      labelSize: 6.2,
      valueSize: 6.4,
    },
  )

  const lowerY = mapY + 81
  addPanelTitle(page, { x: 14, y: lowerY, width: 182, height: 10 }, 'Polres melapor hari Rabu')
  const wednesdayRows = (scheduleInfo?.polres ?? [])
    .map((unit) => ({ label: unit.label, count: unit.on_time_rows }))
    .filter((unit) => unit.count > 0)
    .sort((left, right) => right.count - left.count)
    .slice(0, 12)
    .map((unit) => [unit.label, fmt(unit.count)])
  table(
    page,
    { x: 14, y: lowerY + 9, width: 88, height: 40 },
    ['Polres', 'Laporan Rabu'],
    wednesdayRows.slice(0, Math.ceil(wednesdayRows.length / 2)),
    { fontSize: 6.6, align: ['left', 'right'] },
  )
  table(
    page,
    { x: 108, y: lowerY + 9, width: 88, height: 40 },
    ['Polres', 'Laporan Rabu'],
    wednesdayRows.slice(Math.ceil(wednesdayRows.length / 2)),
    { fontSize: 6.6, align: ['left', 'right'] },
  )

  const complaints = uniqueFeedback(data.kumpulFeedback, 6)
  rect(page, { x: 14, y: lowerY + 52, width: 182, height: 45 }, { fill: PANEL, radius: 2, z: 2 })
  text(
    page,
    { x: 17, y: lowerY + 54.5, width: 176, height: 4 },
    'SUARA WARGA · KUMPUL BERSAMA',
    { size: 6.4, bold: true, color: FOREST },
  )
  if (complaints.length) {
    const left = complaints.slice(0, 3).join('; ')
    const right = complaints.slice(3, 6).join('; ')
    text(page, { x: 17, y: lowerY + 59, width: 84, height: 35 }, left, {
      size: 6.0,
      color: SOFT,
    })
    if (right) {
      text(page, { x: 105, y: lowerY + 59, width: 88, height: 35 }, right, {
        size: 6.0,
        color: SOFT,
      })
    }
  } else {
    text(
      page,
      { x: 17, y: lowerY + 61, width: 176, height: 30 },
      'Belum ada saran atau keluhan warga pada laporan QW1.4.',
      { size: 6.4, color: MUTED },
    )
  }
  return page
}

function buildKapolda(data: Data): LayoutPage {
  const page = newPage(4)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 4)
  const leadership = data.leadership
  const entries = leadership?.entries ?? []
  const direct =
    leadership?.direct ?? entries.filter((entry) => entry.direct).length
  const represented = leadership?.represented ?? entries.length - direct
  const qw43 = leadership?.qw43 ?? 0
  const issues = leadership?.issues ?? []
  const start = addSectionHeading(
    page,
    26,
    'Pimpinan di Tengah Warga',
    'Timeline Kegiatan Pimpinan Polda',
    `Timeline ${fmt(entries.length)} kegiatan utama Polda dari ${dayMonth(data.metrics.period_start)} sampai ${dayMonth(data.metrics.period_end)}, termasuk pejabat pelaksana dan isu yang dicatat pada laporan tingkat Polda.`,
  )
  const nextY = addCards(page, start + 2, [
    {
      label: 'Kegiatan pimpinan',
      value: fmt(entries.length),
      unit: 'kegiatan',
      detail: 'Kapolda/Wakapolda & pejabat utama',
    },
    {
      label: 'Dihadiri Kapolda',
      value: fmt(direct),
      unit: 'kegiatan',
      detail: `${percent(direct, entries.length)}% dari timeline`,
    },
    {
      label: 'Diwakili pejabat',
      value: fmt(represented),
      unit: 'kegiatan',
      detail: 'Wakapolda/Irwasda/direktorat',
    },
    {
      label: 'QW4.3 · Panggilan 110',
      value: fmt(qw43),
      unit: 'laporan',
      detail: 'Respons Cepat Panggilan 110',
    },
  ], { height: 25 })

  const timelineTop = nextY + 5
  if (!entries.length) {
    rect(
      page,
      { x: 14, y: timelineTop, width: 182, height: 42 },
      { fill: PANEL, radius: 2, z: 2 },
    )
    text(
      page,
      { x: 18, y: timelineTop + 16, width: 174, height: 10 },
      'Belum ada kegiatan pimpinan pada periode ini.',
      { size: 7, color: MUTED, align: 'center' },
    )
  } else {
    const count = entries.length
    const gap = count > 6 ? 1.5 : 3
    const colWidth = (182 - gap * (count - 1)) / count
    const lineY = timelineTop + 9
    const cardY = timelineTop + 13
    const cardHeight = 36
    const firstCenter = 14 + colWidth / 2
    const lastCenter = 14 + (count - 1) * (colWidth + gap) + colWidth / 2
    rect(
      page,
      { x: firstCenter, y: lineY - 0.3, width: lastCenter - firstCenter, height: 0.6 },
      { fill: '#e3d3b7', z: 5 },
    )
    entries.forEach((entry, index) => {
      const x = 14 + index * (colWidth + gap)
      text(
        page,
        { x, y: timelineTop, width: colWidth, height: 5 },
        timelineLabel(entry.tanggal),
        { size: 6.6, bold: true, color: GOLD, align: 'center' },
      )
      page.elements.push(
        element(
          'rect',
          {
            x: x + colWidth / 2 - 2.25,
            y: lineY - 2.25,
            width: 4.5,
            height: 4.5,
          },
          {
            shape: 'ellipse',
            fill: entry.direct ? GOLD : FOREST,
            border_width: 0,
            z: 6,
          },
        ),
      )
      rect(
        page,
        { x, y: cardY, width: colWidth, height: cardHeight },
        {
          fill: '#fdfaf3',
          borderColor: '#ecdfc8',
          borderWidth: 0.5,
          radius: 2,
          z: 3,
        },
      )
      const info = timelineCardText(entry)
      text(
        page,
        { x: x + 1.5, y: cardY + 2, width: colWidth - 3, height: 4 },
        entry.executor.toUpperCase(),
        { size: 5.4, bold: true, color: entry.direct ? FOREST : GOLD },
      )
      text(
        page,
        { x: x + 1.5, y: cardY + 6, width: colWidth - 3, height: 15 },
        info.title,
        { size: 5.9, color: INK },
      )
      text(
        page,
        { x: x + 1.5, y: cardY + 21, width: colWidth - 3, height: 12 },
        info.location,
        { size: 5.2, color: MUTED },
      )
    })
  }

  const issuesTop = 142
  text(page, { x: 14, y: issuesTop, width: 182, height: 5 }, 'ISU DI FORUM PIMPINAN', {
    size: 6.6,
    bold: true,
    color: GOLD,
  })
  text(
    page,
    { x: 14, y: issuesTop + 5, width: 182, height: 4.5 },
    'Dicatat dalam laporan tingkat Polda',
    { size: 6.4, color: MUTED },
  )

  if (!issues.length) {
    rect(
      page,
      { x: 14, y: issuesTop + 12, width: 182, height: 26 },
      { fill: PANEL, radius: 2, z: 2 },
    )
    text(
      page,
      { x: 18, y: issuesTop + 22, width: 174, height: 8 },
      'Belum ada isu atau keluhan warga yang dicatat pada laporan tingkat Polda.',
      { size: 6.6, color: MUTED },
    )
  } else {
    issues.slice(0, 4).forEach((issue, index) => {
      const y = issuesTop + 12 + index * 28
      rect(
        page,
        { x: 14, y, width: 182, height: 24 },
        { fill: '#fdf6ea', borderColor: '#efe0c8', borderWidth: 0, radius: 2.5, z: 2 },
      )
      page.elements.push(
        element(
          'rect',
          { x: 19, y: y + 6, width: 9, height: 9 },
          { shape: 'ellipse', fill: GOLD, border_width: 0, z: 4 },
        ),
      )
      text(
        page,
        { x: 19, y: y + 6, width: 9, height: 9 },
        String(index + 1),
        { size: 7, bold: true, color: WHITE, align: 'center', valign: 'middle', z: 5 },
      )
      text(
        page,
        { x: 32, y: y + 3, width: 158, height: 5 },
        `${timelineLabel(issue.tanggal)} · ${compactLocation(issue.location)}`,
        { size: 7.2, bold: true, color: INK },
      )
      text(
        page,
        { x: 32, y: y + 9, width: 158, height: 13 },
        issue.text,
        { size: 6.4, color: SOFT },
      )
    })
  }
  return page
}

function buildPoliran(data: Data): LayoutPage {
  const page = newPage(5)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 5)
  const qw13 = schedule(data, 'QW1.3')
  const poliran = data.poliran
  const program = codeProgram(data, 'QW1.3')
  const daily = (poliran?.daily?.length ? poliran.daily : data.metrics.daily).slice()
  const dayName = qw13?.day ?? 'Kamis'
  const scheduledWeekday = (qw13?.weekday ?? 4) % 7
  const scheduledTotal = daily
    .filter((item) => weekdayIndex(item.date) === scheduledWeekday)
    .reduce((sum, item) => sum + item.count, 0)
  const stats = scheduleStats(qw13)
  const activities = (poliran?.activities ?? []).filter((item) => item.count > 0)
  const activityTotal = activities.reduce((sum, item) => sum + item.count, 0)
  const activityLabels = activities.map((item) => {
    const short = item.label.replace(/\s+dan\s+.*$/i, '').trim() || item.label
    return shorten(short, 20)
  })
  const qw13Highlights = data.metrics.highlights
    .filter((item) => item.kode.toUpperCase().startsWith('QW1.3'))
    .slice(0, 3)
  const highlightText = qw13Highlights.length
    ? qw13Highlights
        .map((item) => {
          const place = item.location.replace(/\(-?\d+(\.\d+)?,\s*-?\d+(\.\d+)?\)/g, '').trim()
          return `• ${timelineLabel(item.tanggal)} · ${shorten(item.kegiatan, 86)}${place ? ` (${shorten(place, 40)})` : ''}`
        })
        .join('\n')
    : narrative(data, 'QW1.3')
  const start = addSectionHeading(
    page,
    26,
    'Polisi Peduli Pengangguran',
    'Poliran: Membuka Akses Kerja',
    `Pendataan warga usia produktif, pelatihan, penyaluran kerja, dan kemitraan dari ${fmt(codeRows(data, 'QW1.3'))} laporan QW1.3. Batang emas menandai hari ${dayName} sesuai jadwal Poliran.`,
  )
  const nextY = addCards(
    page,
    start + 2,
    [
      {
        label: 'Kegiatan Poliran',
        value: fmt(codeRows(data, 'QW1.3')),
        unit: 'laporan',
        detail: 'Pendataan, pelatihan & penyaluran kerja',
      },
      {
        label: 'Warga didata/direkrut',
        value: fmt(poliran?.warga ?? metricTotal(program, ['warga', 'peserta'])),
        unit: 'warga',
        detail: 'Pendataan usia produktif & peserta pelatihan',
      },
      {
        label: 'Mitra kerja',
        value: fmt(poliran?.mitra ?? metricTotal(program, ['mitra', 'kemitraan'])),
        unit: 'mitra',
        detail: 'Lowongan kerja dan kemitraan yang dijajaki',
      },
    ],
    { height: 26, columns: 3 },
  )

  const chartTop = nextY + 3
  const scheduledDates = (qw13?.expected_dates ?? []).map(dayMonth).join(', ')
  addPanelTitle(
    page,
    { x: 14, y: chartTop, width: 182, height: 10 },
    'Laporan Poliran per hari',
    `Batang emas = hari ${dayName}${scheduledDates ? ` (${scheduledDates})` : ''} · ${fmt(scheduledTotal)} laporan pada hari jadwal`,
  )
  chart(
    page,
    { x: 14, y: chartTop + 11, width: 182, height: 64 },
    'bar',
    '',
    daily.map((item) => timelineLabel(item.date)),
    daily.map((item) => item.count),
    {
      colors: daily.map((item) =>
        weekdayIndex(item.date) === scheduledWeekday ? GOLD : FOREST,
      ),
      showValues: true,
      showLegend: false,
      titleSize: 7.8,
      labelSize: 6.3,
      valueSize: 6.6,
    },
  )

  const rowTop = chartTop + 80
  addPanelTitle(
    page,
    { x: 14, y: rowTop, width: 88, height: 10 },
    `Cakupan Polres Hari ${dayName}`,
    `${fmt(stats.onTime)} dari ${fmt(stats.total)} polres melapor tepat jadwal`,
  )
  table(
    page,
    { x: 14, y: rowTop + 11, width: 88, height: 76 },
    ['Polres', 'Status', 'Laporan'],
    scheduleRows(qw13, 10),
    { fontSize: 6.5, align: ['left', 'left', 'right'] },
  )
  addPanelTitle(
    page,
    { x: 108, y: rowTop, width: 88, height: 10 },
    'Bentuk kegiatan Poliran',
    `${fmt(activityTotal)} kegiatan tercatat`,
  )
  chart(
    page,
    { x: 108, y: rowTop + 8, width: 88, height: 78 },
    'pie',
    '',
    activityLabels,
    activities.map((item) => item.count),
    {
      colors: activities.map((_, index) => PALETTE[index % PALETTE.length]),
      showValues: true,
      showLegend: true,
      titleSize: 7.4,
      labelSize: 5.8,
      legendSize: 5.6,
      valueSize: 6.0,
    },
  )

  const noteY = rowTop + 89
  rect(
    page,
    { x: 14, y: noteY, width: 182, height: 22 },
    { fill: '#fdf6ea', borderColor: '#efe0c8', borderWidth: 0, radius: 2.5, z: 2 },
  )
  text(
    page,
    { x: 18, y: noteY + 3, width: 174, height: 5 },
    'SOROTAN POLIRAN',
    { size: 6.4, bold: true, color: GOLD },
  )
  text(
    page,
    { x: 18, y: noteY + 8, width: 174, height: 12 },
    highlightText,
    { size: 6.4, color: SOFT },
  )
  return page
}

function buildGreen(data: Data): LayoutPage {
  const page = newPage(6)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 6)
  const green = data.metrics.green_policing
  const scheduleInfo = schedule(data, 'QW1.5')
  const stats = scheduleStats(scheduleInfo)
  const dayName = scheduleInfo?.day ?? 'Jumat'
  const scheduledWeekday = (scheduleInfo?.weekday ?? 5) % 7
  const scheduledDates = (scheduleInfo?.expected_dates ?? []).map(dayMonth).join(', ')
  const daily = (data.greenDaily ?? []).slice()
  const scheduledTotal = daily
    .filter((item) => weekdayIndex(item.date) === scheduledWeekday)
    .reduce((sum, item) => sum + item.count, 0)
  const hasPlanting = green.penanaman > 0 && green.bibit > 0
  const plantingPolres = [...green.by_polres]
    .filter((item) => item.bibit > 0 && /polres/i.test(item.label))
    .sort((left, right) => right.bibit - left.bibit)
  const plantingForms = new Map<string, number>()
  for (const item of green.by_kegiatan) {
    const raw = item.label.trim()
    if (!raw || /nihil/i.test(raw)) continue
    const grouped = /^lainnya\s*:/i.test(raw)
      ? 'Lainnya'
      : raw.replace(/bibit\s+/i, '').replace(/\s+dan\s+.*$/i, '').trim() || raw
    plantingForms.set(grouped, (plantingForms.get(grouped) ?? 0) + item.count)
  }
  const formRows = [...plantingForms.entries()]
    .sort((left, right) => right[1] - left[1])
    .slice(0, 8)
  const plantingText = hasPlanting
    ? `${fmt(green.rows)} kegiatan penghijauan dengan ${fmt(green.penanaman)} kegiatan penanaman, ${fmt(green.bibit)} pohon ditanam, dan ${fmt(green.kerja_bakti)} kerja bakti.`
    : `${fmt(green.rows)} kegiatan penghijauan dan ${fmt(green.kerja_bakti)} kerja bakti.`
  const start = addSectionHeading(
    page,
    26,
    'Green Policing',
    'Kerja Bakti dan Penanaman Pohon',
    `${plantingText} Peta menampilkan sebaran kegiatan QW1.5 yang memiliki koordinat.`,
  )
  const cards = [
    {
      label: 'Kegiatan',
      value: fmt(green.rows),
      unit: 'laporan',
      detail: codeTitle(data, 'QW1.5'),
    },
    ...(hasPlanting
      ? [
          {
            label: 'Kegiatan penanaman',
            value: fmt(green.penanaman),
            unit: 'kegiatan',
            detail: 'laporan penanaman pohon',
          },
          {
            label: 'Pohon ditanam',
            value: fmt(green.bibit),
            unit: 'pohon',
            detail: 'total pohon pada QW1.5',
          },
        ]
      : []),
    {
      label: 'Kerja bakti',
      value: fmt(green.kerja_bakti),
      unit: 'kegiatan',
      detail: 'laporan kerja bakti',
    },
    {
      label: 'Cakupan polres',
      value: `${stats.percentValue}%`,
      unit: 'satuan',
      detail: `${fmt(stats.onTime)} dari ${fmt(stats.total)} polres`,
    },
  ]
  const nextY = addCards(page, start + 2, cards, { height: 25, columns: cards.length })

  const contentY = nextY + 3
  const mapBox = { x: 14, y: contentY, width: 92, height: 66 }
  if (data.greenMapImageSrc) {
    rect(page, mapBox, {
      fill: WHITE,
      borderColor: LINE,
      borderWidth: 0.6,
      radius: 2,
      z: 9,
    })
    page.elements.push(
      element('image', mapBox, {
        src: data.greenMapImageSrc,
        alt: 'Peta sebaran kegiatan Green Policing',
        object_fit: 'cover',
        z: 10,
      }),
    )
  } else {
    rect(page, mapBox, { fill: PANEL, borderColor: LINE, borderWidth: 0.6, radius: 2, z: 2 })
    text(
      page,
      { x: mapBox.x + 5, y: mapBox.y + 25, width: mapBox.width - 10, height: 16 },
      'Peta titik koordinat QW1.5 belum tersedia. Pastikan laporan Green Policing memiliki koordinat pada kolom peta.',
      { size: 6.4, color: MUTED, align: 'center', valign: 'middle' },
    )
  }

  addPanelTitle(
    page,
    { x: 108, y: contentY, width: 88, height: 10 },
    'Laporan Green per hari',
    `Batang emas = hari ${dayName}${scheduledDates ? ` (${scheduledDates})` : ''} · ${fmt(scheduledTotal)} laporan`,
  )
  chart(
    page,
    { x: 108, y: contentY + 10, width: 88, height: 56 },
    'bar',
    '',
    daily.map((item) => timelineLabel(item.date)),
    daily.map((item) => item.count),
    {
      colors: daily.map((item) =>
        weekdayIndex(item.date) === scheduledWeekday ? GOLD : FOREST,
      ),
      showValues: true,
      showLegend: false,
      titleSize: 7.8,
      labelSize: 6.3,
      valueSize: 6.6,
    },
  )

  const rowTop = contentY + 69
  const chartHeight = 56
  if (plantingPolres.length) {
    const polresLabels = plantingPolres.slice(0, 10).map((item) => shorten(item.label, 26))
    chart(
      page,
      { x: 14, y: rowTop, width: 88, height: chartHeight },
      'hbar',
      'Pohon ditanam per polres',
      polresLabels,
      plantingPolres.slice(0, 10).map((item) => item.bibit),
      {
        colors: polresLabels.map(() => FOREST),
        showValues: true,
        titleSize: 7.8,
        labelSize: 6.4,
        valueSize: 6.8,
      },
    )
    chart(
      page,
      { x: 108, y: rowTop, width: 88, height: chartHeight },
      'pie',
      'Bentuk kegiatan QW1.5',
      formRows.map(([label]) => shorten(label, 20)),
      formRows.map(([, count]) => count),
      {
        colors: formRows.map((_, index) => PALETTE[index % PALETTE.length]),
        showValues: true,
        showLegend: true,
        titleSize: 7.8,
        labelSize: 5.8,
        legendSize: 5.8,
        valueSize: 6.0,
      },
    )
  } else {
    chart(
      page,
      { x: 14, y: rowTop, width: 104, height: chartHeight },
      'pie',
      'Bentuk kegiatan QW1.5',
      formRows.map(([label]) => shorten(label, 20)),
      formRows.map(([, count]) => count),
      {
        colors: formRows.map((_, index) => PALETTE[index % PALETTE.length]),
        showValues: true,
        showLegend: true,
        titleSize: 7.8,
        labelSize: 5.8,
        legendSize: 5.8,
        valueSize: 6.0,
      },
    )
  }

  const coverageTop = rowTop + chartHeight + 3
  const coverageRows = scheduleRows(scheduleInfo, 12)
  const half = Math.ceil(coverageRows.length / 2)
  addPanelTitle(
    page,
    { x: 14, y: coverageTop, width: 182, height: 10 },
    `Cakupan Polres Hari ${dayName}`,
    `${fmt(stats.onTime)} dari ${fmt(stats.total)} polres tepat jadwal`,
  )
  table(
    page,
    { x: 14, y: coverageTop + 10, width: 88, height: 48 },
    ['Polres', 'Status', 'Laporan'],
    coverageRows.slice(0, half),
    { fontSize: 6.2, align: ['left', 'left', 'right'] },
  )
  table(
    page,
    { x: 108, y: coverageTop + 10, width: 88, height: 48 },
    ['Polres', 'Status', 'Laporan'],
    coverageRows.slice(half),
    { fontSize: 6.2, align: ['left', 'left', 'right'] },
  )
  return page
}

function buildWorship(
  pageNumber: number,
  code: string,
  key: 'jumat_keliling' | 'subuh_keliling' | 'minggu_kasih',
  data: Data,
): LayoutPage {
  const page = newPage(pageNumber)
  addHeader(page, data.frame)
  addFooter(page, data.frame, pageNumber)
  const metrics = data.metrics[key]
  const programInfo = codeProgram(data, code)
  const scheduleInfo = schedule(data, code)
  const stats = scheduleStats(scheduleInfo)
  const dayName = scheduleInfo?.day ?? 'Jumat'
  const scheduledWeekday = (scheduleInfo?.weekday ?? 5) % 7
  const scheduledDates = (scheduleInfo?.expected_dates ?? []).map(dayMonth).join(', ')
  const title =
    key === 'jumat_keliling'
      ? 'Jumat Keliling'
      : key === 'subuh_keliling'
        ? 'Subuh Keliling'
        : 'Minggu Kasih'
  const dailyField =
    key === 'jumat_keliling'
      ? data.jumlingDaily
      : key === 'subuh_keliling'
        ? data.subuhDaily
        : data.mingguDaily
  const daily = (dailyField ?? []).slice()
  const mapSrc =
    key === 'jumat_keliling'
      ? data.jumlingMapImageSrc
      : key === 'subuh_keliling'
        ? data.subuhMapImageSrc
        : data.mingguMapImageSrc
  const jamaahLabel = key === 'minggu_kasih' ? 'Jumlah Jemaat' : 'Jumlah Jamaah'
  const jamaahWord = key === 'minggu_kasih' ? 'jemaat' : 'jamaah'
  const jamaahKeyword = key === 'minggu_kasih' ? 'jemaat' : 'jamaah'
  const placeWord = key === 'minggu_kasih' ? 'lokasi ibadah' : 'masjid'
  const scheduledTotal = daily
    .filter((item) => weekdayIndex(item.date) === scheduledWeekday)
    .reduce((sum, item) => sum + item.count, 0)
  const jamaah = Math.max(metricTotal(programInfo, [jamaahKeyword]), metrics.people)
  const personel = metricTotal(programInfo, ['personel'])
  const roles = findBreakdown(programInfo, ['peran'])?.values ?? []
  const roleColors: Record<string, string> = {
    Narasumber: FOREST,
    Jamaah: GOLD,
    Imam: '#2f6db0',
    Pengamanan: '#3f765d',
    'Pengamanan, Lainnya': '#3f765d',
    Lainnya: '#6d7872',
  }
  const polresRows = [...metrics.by_polres]
    .filter((item) => item.rows > 0)
    .sort((left, right) => right.rows - left.rows)
    .slice(0, 10)

  const start = addSectionHeading(
    page,
    26,
    `Quick Wins · ${title}`,
    title,
    `${fmt(metrics.rows)} kegiatan ${title} menjangkau ${fmt(jamaah)} ${jamaahWord} di ${fmt(metrics.locations)} ${placeWord}. Peta menampilkan kegiatan ${code} yang memiliki koordinat.`,
  )
  const extraCards =
    key === 'minggu_kasih'
      ? [
          {
            label: 'Lokasi gereja',
            value: fmt(breakdownValue(programInfo, ['jenis-ibadah'], ['gereja'])),
            unit: 'lokasi',
            detail: 'gereja dikunjungi',
          },
          {
            label: 'Lokasi pura',
            value: fmt(breakdownValue(programInfo, ['jenis-ibadah'], ['pura'])),
            unit: 'lokasi',
            detail: 'pura dikunjungi',
          },
        ]
      : []
  const nextY = addCards(
    page,
    start + 2,
    [
      {
        label: 'Kegiatan',
        value: fmt(metrics.rows),
        unit: 'laporan',
        detail: codeTitle(data, code),
      },
      {
        label: jamaahLabel,
        value: fmt(jamaah),
        unit: 'orang',
        detail: `${fmt(metrics.locations)} ${placeWord}`,
      },
      {
        label: 'Jumlah personel',
        value: fmt(personel),
        unit: 'orang',
        detail: 'personel Polri hadir',
      },
      ...extraCards,
      {
        label: 'Cakupan polres',
        value: `${stats.percentValue}%`,
        unit: 'satuan',
        detail: `${fmt(stats.onTime)} dari ${fmt(stats.total)} polres`,
      },
    ],
    {
      height: 25,
      columns: extraCards.length ? 6 : 4,
      valueSize: extraCards.length ? 16 : 19,
      labelSize: extraCards.length ? 6.2 : 6.8,
    },
  )

  const contentY = nextY + 3
  const mapBox = { x: 14, y: contentY, width: 92, height: 66 }
  if (mapSrc) {
    rect(page, mapBox, {
      fill: WHITE,
      borderColor: LINE,
      borderWidth: 0.6,
      radius: 2,
      z: 9,
    })
    page.elements.push(
      element('image', mapBox, {
        src: mapSrc,
        alt: `Peta sebaran kegiatan ${title}`,
        object_fit: 'cover',
        z: 10,
      }),
    )
  } else {
    rect(page, mapBox, { fill: PANEL, borderColor: LINE, borderWidth: 0.6, radius: 2, z: 2 })
    text(
      page,
      { x: mapBox.x + 5, y: mapBox.y + 25, width: mapBox.width - 10, height: 16 },
      `Peta titik koordinat ${code} belum tersedia. Pastikan laporan ${title} memiliki koordinat pada kolom peta.`,
      { size: 6.4, color: MUTED, align: 'center', valign: 'middle' },
    )
  }

  addPanelTitle(
    page,
    { x: 108, y: contentY, width: 88, height: 10 },
    `Laporan ${title} per hari`,
    `Batang emas = hari ${dayName}${scheduledDates ? ` (${scheduledDates})` : ''} · ${fmt(scheduledTotal)} laporan`,
  )
  chart(
    page,
    { x: 108, y: contentY + 10, width: 88, height: 56 },
    'bar',
    '',
    daily.map((item) => timelineLabel(item.date)),
    daily.map((item) => item.count),
    {
      colors: daily.map((item) =>
        weekdayIndex(item.date) === scheduledWeekday ? GOLD : FOREST,
      ),
      showValues: true,
      showLegend: false,
      titleSize: 7.8,
      labelSize: 6.3,
      valueSize: 6.6,
    },
  )

  const rowTop = contentY + 69
  const chartHeight = 56
  const roleTotal = roles.reduce((sum, item) => sum + item.count, 0)
  const roleChart =
    key === 'minggu_kasih' && roleTotal <= 0
      ? [{ label: 'Pengamanan, Lainnya', count: 100 }]
      : roles
  chart(
    page,
    { x: 14, y: rowTop, width: 88, height: chartHeight },
    'pie',
    'Peran personel',
    roleChart.map((item) => shorten(item.label, 20)),
    roleChart.map((item) => item.count),
    {
      colors: roleChart.map(
        (item, index) => roleColors[item.label] ?? PALETTE[index % PALETTE.length],
      ),
      showValues: true,
      showLegend: true,
      titleSize: 7.8,
      labelSize: 5.8,
      legendSize: 5.8,
      valueSize: 6.0,
    },
  )
  chart(
    page,
    { x: 108, y: rowTop, width: 88, height: chartHeight },
    'hbar',
    'Laporan per polres',
    polresRows.map((item) => shorten(item.label, 26)),
    polresRows.map((item) => item.rows),
    {
      colors: polresRows.map(() => FOREST),
      showValues: true,
      titleSize: 7.8,
      labelSize: 6.4,
      valueSize: 6.8,
    },
  )

  const coverageTop = rowTop + chartHeight + 3
  const coverageRows = scheduleRows(scheduleInfo, 12)
  const half = Math.ceil(coverageRows.length / 2)
  addPanelTitle(
    page,
    { x: 14, y: coverageTop, width: 182, height: 10 },
    `Cakupan Polres Hari ${dayName}`,
    `${fmt(stats.onTime)} dari ${fmt(stats.total)} polres tepat jadwal`,
  )
  table(
    page,
    { x: 14, y: coverageTop + 10, width: 88, height: 48 },
    ['Polres', 'Status', 'Laporan'],
    coverageRows.slice(0, half),
    { fontSize: 6.2, align: ['left', 'left', 'right'] },
  )
  table(
    page,
    { x: 108, y: coverageTop + 10, width: 88, height: 48 },
    ['Polres', 'Status', 'Laporan'],
    coverageRows.slice(half),
    { fontSize: 6.2, align: ['left', 'left', 'right'] },
  )
  return page
}

const followTitleByKey: Record<string, string> = {
  security: 'Pencurian dan keamanan malam',
  traffic: 'Lalu lintas jam kantor',
  drugs: 'Narkoba, vape dan geng pelajar',
  bullying: 'Perundungan pelajar',
  waste: 'Sampah dan kebersihan',
  online: 'Penipuan daring',
  tourists: 'Wisatawan asing',
  youth: 'Knalpot brong dan balap liar',
  jobs: 'Lowongan kerja',
}

const followPlanByKey: Record<string, string> = {
  security: 'Perkuat patroli malam di pasar, permukiman dan perbatasan kabupaten',
  traffic: 'Tindak knalpot brong dan balap liar di jalur utama',
  drugs: 'Petakan kawasan rawan narkoba bersama BNN dan sekolah',
  bullying: 'Edukasi anti perundungan dan pemantauan geng pelajar di sekolah',
  waste: 'Jadwalkan Green Policing rutin dan kerja bakti saluran air',
  online: 'Perluas edukasi judi daring dan penipuan, dampingi korban',
  tourists: 'Tertibkan wisatawan asing bersama desa adat dan pemandu',
  youth: 'Razia miras dan patroli titik kumpul remaja',
  jobs: 'Buka lowongan, pelatihan kerja dan pendampingan UMKM',
}

const dayMonthLongFormatter = new Intl.DateTimeFormat('id-ID', {
  day: 'numeric',
  month: 'long',
})

function aspirationFollowTitle(key: string, fallback: string): string {
  return followTitleByKey[key] ?? fallback
}

function aspirationFollowPlan(theme: AspirationThemeRow): string {
  return followPlanByKey[theme.key] ?? 'Tindak lanjuti aspirasi warga sesuai hasil forum dan laporan satuan'
}

function aspirationFollowDescription(
  data: Data,
  theme: AspirationThemeRow,
  region: string,
): string {
  const kerjaBakti = data.metrics.green_policing?.kerja_bakti ?? 0
  switch (theme.key) {
    case 'security':
      return `${fmt(codeRows(data, 'QW1.10'))} laporan Patroli Sabhara dan ${fmt(codeRows(data, 'QW1.9'))} Patroli Siaga Reserse; patroli diarahkan ke titik rawan di ${region}.`
    case 'traffic':
      return `${fmt(codeRows(data, 'QW1.1'))} laporan Gatur Pagi dan ${fmt(codeRows(data, 'QW1.2'))} kegiatan Polidik; kepadatan jam kantor dan knalpot brong menjadi keluhan utama di ${region}.`
    case 'drugs':
      return `${fmt(codeRows(data, 'QW1.2'))} kegiatan Polidik menyasar bahaya narkoba dan vape; kawasan rawan di ${region} perlu dipantau bersama sekolah.`
    case 'bullying':
      return `Edukasi anti perundungan masuk dalam ${fmt(codeRows(data, 'QW1.2'))} kegiatan Polidik; kasus pelajar perlu ditangani bersama sekolah dan orang tua.`
    case 'waste':
      return `Kerja bakti Green Policing di ${fmt(kerjaBakti || codeRows(data, 'QW1.5'))} lokasi; warga meminta penjadwalan rutin di ${region}.`
    case 'online':
      return `${fmt(codeRows(data, 'QW4.3'))} panggilan 110 dan ${fmt(codeRows(data, 'QW3.2'))} layanan aduan penipuan; edukasi judi daring dan penipuan masih perlu diperluas.`
    case 'tourists':
      return `${fmt(codeRows(data, 'QW1.6'))} kegiatan Jumat Keliling dan ${fmt(codeRows(data, 'QW1.8'))} Minggu Kasih menjangkau warga dan wisatawan di ${region}.`
    case 'youth':
      return `${fmt(codeRows(data, 'QW1.10'))} laporan patroli dan ${fmt(codeRows(data, 'QW1.9'))} Patroli Siaga Reserse; titik kumpul remaja dan balap liar perlu dipantau malam.`
    case 'jobs':
      return `${fmt(codeRows(data, 'QW1.3'))} laporan Poliran menyalurkan warga ke dunia kerja; pelatihan dan pendampingan UMKM masih dibutuhkan.`
    default:
      return `${fmt(theme.count)} isian warga menyebut ${theme.label.toLowerCase()}; tindak lanjut menyesuaikan hasil forum dan laporan satuan.`
  }
}

function aspirationPlanLabel(end: string): string {
  const date = new Date(`${end.slice(0, 10)}T12:00:00`)
  if (Number.isNaN(date.getTime())) return 'BERIKUTNYA'
  const finish = new Date(date)
  finish.setDate(date.getDate() + 6)
  const sameMonth =
    date.getMonth() === finish.getMonth() && date.getFullYear() === finish.getFullYear()
  const startLabel = sameMonth ? String(date.getDate()) : dayMonthLongFormatter.format(date)
  return `${startLabel} – ${dayMonthLongFormatter.format(finish)}`.toUpperCase()
}

function aspirationActionNote(data: Data): string {
  const extreme = codeRows(data, 'QW1.12')
  const scientific = codeRows(data, 'QW8.4')
  const pungli = codeRows(data, 'QW6.2')
  const parts: string[] = []
  parts.push(
    extreme === 0
      ? 'Belum ada laporan QW1.12 Pencegahan ekstremisme pada anak.'
      : `Pencegahan ekstremisme pada anak (QW1.12) ${fmt(extreme)} laporan.`,
  )
  parts.push(
    scientific === 0
      ? 'Belum ada laporan QW8.4 Olah TKP ilmiah dan identifikasi korban.'
      : `Olah TKP ilmiah (QW8.4) ${fmt(scientific)} laporan.`,
  )
  parts.push(
    pungli === 0
      ? 'Tidak ada anggota yang tercatat pungli (QW6.2).'
      : `Copot Hari Itu Juga (QW6.2) ${fmt(pungli)} laporan.`,
  )
  const total = data.aspirations?.total ?? data.metrics.aspirations.total
  parts.push(`Tercatat ${fmt(total)} isian aspirasi warga pada periode ini.`)
  return parts.join(' ')
}

function buildAspirationFollowUp(data: Data): LayoutPage {
  const page = newPage(11)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 11)

  const info = data.aspirations
  const rawThemes: AspirationThemeRow[] = info?.themes?.length
    ? info.themes
    : data.metrics.aspirations.themes.map((item) => ({
        key: '',
        label: item.label,
        count: item.count,
        programs: [],
        follow_up: 'running',
      }))
  const preferredOrder = [
    'security',
    'traffic',
    'waste',
    'drugs',
    'online',
    'youth',
    'tourists',
    'bullying',
    'jobs',
  ]
  const themeByKey = new Map(rawThemes.map((theme) => [theme.key, theme]))
  const preferredThemes = preferredOrder
    .map((key) => themeByKey.get(key))
    .filter((theme): theme is AspirationThemeRow => Boolean(theme) && (theme?.count ?? 0) > 0)
  const remainingThemes = [...rawThemes]
    .filter((theme) => theme.count > 0 && !preferredThemes.includes(theme))
    .sort((left, right) => right.count - left.count)
  const displayThemes = [...preferredThemes, ...remainingThemes].slice(0, 6)
  const region = info?.top_polres?.[0]?.label ?? 'Polda Bali'
  const total = info?.total ?? data.metrics.aspirations.total
  const doneThemes = displayThemes.filter((theme) => theme.follow_up === 'done').length
  const runningThemes = displayThemes.length - doneThemes

  const subtitle =
    'Tindak lanjut atas aspirasi warga yang tercatat pekan ini, dengan rencana yang akan dilaksanakan.'
  text(
    page,
    { x: 14, y: 26, width: 182, height: 5 },
    `TINDAK LANJUT ASPIRASI · ${data.frame.shortPeriod.toUpperCase()}`,
    { size: 6.6, bold: true, color: GOLD },
  )
  text(
    page,
    { x: 14, y: 30.8, width: 182, height: 11 },
    'Tindak Lanjut Aspirasi Warga',
    { size: 20, bold: true, family: 'montserrat', color: FOREST },
  )
  let subtitleY = 43
  for (const line of chunk(subtitle.split(' '), 26)
    .slice(0, 2)
    .map((parts) => parts.join(' '))) {
    text(page, { x: 14, y: subtitleY, width: 182, height: 4.5 }, line, {
      size: 7.4,
      italic: true,
      color: SOFT,
    })
    subtitleY += 4.2
  }

  const plans = [...displayThemes]
    .sort((left, right) => right.count - left.count)
    .slice(0, 4)
    .map((theme) => aspirationFollowPlan(theme))
  const fallbackPlans = [
    'Lengkapi cakupan Poliran dan Minggu Kasih di seluruh Polsek',
    'Tetapkan kawasan rawan narkoba bersama BNN dan pemerintah daerah',
    'Tindak knalpot brong dan balap liar di jalur utama',
    'Tertibkan laporan berisi "NIHIL KEGIATAN" di luar hari jadwal',
  ]
  while (plans.length < 4) {
    const candidate = fallbackPlans.find((plan) => !plans.includes(plan))
    if (!candidate) break
    plans.push(candidate)
  }

  const cardItems = [
    {
      value: fmt(doneThemes),
      label: 'tema aspirasi sudah ditindaklanjuti',
      fill: '#e7f2e6',
      valueColor: '#2f7d4f',
      labelColor: '#2f5d3f',
    },
    {
      value: fmt(runningThemes),
      label: 'tema masih berjalan',
      fill: '#f9efe0',
      valueColor: GOLD,
      labelColor: '#7a5a30',
    },
    {
      value: fmt(plans.length),
      label: 'rencana pekan ini',
      fill: '#f1f2f4',
      valueColor: '#334155',
      labelColor: '#4b5563',
    },
    {
      value: fmt(total),
      label: 'isian saran dan keluhan di aplikasi',
      fill: '#f1f2f4',
      valueColor: '#1f2937',
      labelColor: '#4b5563',
    },
  ]
  const cardWidth = 43
  const cardGap = 2.67
  cardItems.forEach((item, index) => {
    card(
      page,
      { x: 14 + index * (cardWidth + cardGap), y: 54, width: cardWidth, height: 26 },
      item.label,
      item.value,
      '',
      '',
      {
        fill: item.fill,
        accent: item.fill,
        labelColor: item.labelColor,
        valueColor: item.valueColor,
        borderColor: item.fill,
        template: 'icon_right',
        labelSize: 6.6,
        valueSize: 20,
      },
    )
  })

  text(
    page,
    { x: 14, y: 88, width: 182, height: 5 },
    'TEMA ASPIRASI PEKAN INI',
    { size: 6.6, bold: true, color: GOLD },
  )

  const rowX = 14
  const rowWidth = 182
  const rowHeight = 15.4
  const rowGap = 1.4
  displayThemes.forEach((theme, index) => {
    const y = 94.5 + index * (rowHeight + rowGap)
    const done = theme.follow_up === 'done'
    rect(page, { x: rowX, y, width: rowWidth, height: rowHeight }, {
      fill: '#f5f6f7',
      radius: 1.5,
      z: 2,
    })
    page.elements.push(
      element('rect', { x: rowX + 3.4, y: y + 1.9, width: 11.6, height: 11.6 }, {
        shape: 'ellipse',
        fill: done ? '#3f9d4f' : GOLD,
        border_width: 0,
        z: 3,
      }),
    )
    page.elements.push(
      element('icon', { x: rowX + 5.4, y: y + 3.9, width: 7.6, height: 7.6 }, {
        icon_name: done ? 'check' : 'clock',
        icon_color: WHITE,
        icon_stroke: done ? 3 : 2,
        z: 4,
      }),
    )
    text(
      page,
      { x: rowX + 18, y: y + 2.4, width: 140, height: 4.4 },
      aspirationFollowTitle(theme.key, theme.label),
      { size: 7.4, bold: true, color: FOREST },
    )
    text(
      page,
      { x: rowX + 18, y: y + 7.1, width: 139, height: 7.4 },
      aspirationFollowDescription(data, theme, region),
      { size: 6.35, color: SOFT },
    )
    text(
      page,
      { x: rowX + 160, y: y + 5.7, width: 18, height: 4 },
      done ? 'SUDAH' : 'BERJALAN',
      { size: 6.1, bold: true, color: done ? '#2f7d4f' : GOLD, align: 'right' },
    )
  })

  text(
    page,
    { x: 14, y: 198.5, width: 182, height: 5 },
    `RENCANA PEKAN ${aspirationPlanLabel(data.detail.period.end_date)}`,
    { size: 6.6, bold: true, color: GOLD },
  )

  plans.forEach((plan, index) => {
    const y = 204.5 + index * 9.5
    rect(page, { x: 14, y, width: 182, height: 8.2 }, {
      fill: '#f5f6f7',
      radius: 1.4,
      z: 2,
    })
    page.elements.push(
      element('rect', { x: 17.2, y: y + 1.1, width: 6, height: 6 }, {
        shape: 'ellipse',
        fill: '#6b7f95',
        border_width: 0,
        z: 3,
      }),
    )
    page.elements.push(
      element('icon', { x: 18.5, y: y + 2.4, width: 3.4, height: 3.4 }, {
        icon_name: 'clock',
        icon_color: WHITE,
        icon_stroke: 2.5,
        z: 4,
      }),
    )
    text(page, { x: 26, y: y + 1.7, width: 167, height: 5.4 }, plan, {
      size: 7,
      bold: true,
      color: INK,
    })
  })

  rect(page, { x: 14, y: 245.5, width: 182, height: 22 }, {
    fill: '#fbf5e6',
    borderColor: '#efe0bd',
    borderWidth: 0.6,
    radius: 1.5,
    z: 2,
  })
  text(
    page,
    { x: 19, y: 249, width: 172, height: 4 },
    'CATATAN RENCANA AKSI',
    { size: 6.2, bold: true, color: '#a4743d' },
  )
  text(
    page,
    { x: 19, y: 254.2, width: 172, height: 11 },
    aspirationActionNote(data),
    { size: 6.6, color: INK },
  )
  return page
}

function buildAspirationsPage(data: Data): LayoutPage {
  const page = newPage(10)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 10)
  const info = data.aspirations
  const total = info?.total ?? data.metrics.aspirations.total
  const themes = info?.themes ?? []
  const topPolres = (info?.top_polres ?? []).map((item) => item.label)
  const subtitle =
    'Keluhan warga Bali pekan ini: lalu lintas dan knalpot brong, pencurian dan begal, narkoba dan vape pelajar, sampah, serta wisatawan asing.'
  text(
    page,
    { x: 14, y: 26, width: 182, height: 5 },
    `ASPIRASI WARGA · ${data.frame.shortPeriod.toUpperCase()}`,
    { size: 6.6, bold: true, color: GOLD },
  )
  text(
    page,
    { x: 14, y: 31, width: 182, height: 10 },
    'Apa Kata Mereka',
    { size: 20, bold: true, family: 'montserrat', color: FOREST },
  )
  let subtitleY = 42
  for (const line of chunk(subtitle.split(' '), 24)
    .slice(0, 2)
    .map((parts) => parts.join(' '))) {
    text(page, { x: 14, y: subtitleY, width: 182, height: 4.5 }, line, {
      size: 7.4,
      italic: true,
      color: SOFT,
    })
    subtitleY += 4.4
  }

  const programColumns = [
    { key: 'gatur_polidik', label: 'Gatur & Polidik', color: '#2563a8' },
    { key: 'kumpul', label: 'Kumpul Bersama', color: '#b8894b' },
    { key: 'polda', label: 'Forum Kapolda', color: '#1f3a5f' },
    { key: 'poliran', label: 'Poliran', color: '#7a9e8e' },
    { key: 'green', label: 'Green Policing', color: '#2f7d4f' },
    { key: 'jumat', label: 'Jumat Keliling', color: '#2f6db0' },
    { key: 'subuh', label: 'Subuh Keliling', color: '#d9822b' },
    { key: 'minggu', label: 'Minggu Kasih', color: '#6b3fa0' },
  ]
  const themeEmoji: Record<string, string> = {
    security: '🛡',
    traffic: '🏍',
    drugs: '💊',
    bullying: '👥',
    waste: '🗑',
    online: '📱',
    tourists: '✈',
    youth: '🍻',
    jobs: '💼',
  }
  const shortThemeLabels: Record<string, string> = {
    security: 'Pencucian & begal',
    traffic: 'Macet & knalpot',
    drugs: 'Narkoba & vape',
    bullying: 'Perundungan',
    waste: 'Sampah & saluran',
    online: 'Judi & penipuan',
    tourists: 'Wisatawan asing',
    youth: 'Remaja & miras',
    jobs: 'Lowongan kerja',
  }
  const fallbackThemes = data.metrics.aspirations.themes.slice(0, 9).map((item) => ({
    key: '',
    label: item.label,
    count: item.count,
    programs: [] as string[],
    follow_up: 'running',
  }))
  const tableRows = (themes.length ? themes : fallbackThemes).map((theme) => [
    `${themeEmoji[theme.key] ?? '•'} ${
      shortThemeLabels[theme.key] ?? shorten(theme.label, 18)
    }`,
    ...programColumns.map((column) =>
      theme.programs.includes(column.key) ? '●' : '',
    ),
    theme.follow_up === 'done' ? '✅' : '⏳',
  ])
  const tableY = 49
  table(
    page,
    { x: 14, y: tableY, width: 182, height: 90 },
    ['Tema aspirasi', ...programColumns.map((column) => column.label), 'Tindak Lanjut'],
    tableRows,
    {
      fontSize: 5.8,
      headerBg: WHITE,
      headerColor: FOREST,
      stripe: '#f2f6f1',
      borderColor: '#e1e7df',
      align: ['left', ...programColumns.map(() => 'center' as const), 'center'],
      columnColors: [FOREST, ...programColumns.map((column) => column.color), FOREST],
    },
  )

  const legendY = tableY + 93
  page.elements.push(
    element(
      'rect',
      { x: 14, y: legendY + 0.3, width: 2.6, height: 2.6 },
      { shape: 'ellipse', fill: '#3f9d4f', border_width: 0, z: 4 },
    ),
  )
  text(page, { x: 17.5, y: legendY - 0.6, width: 44, height: 4 }, 'sudah ditindaklanjuti', {
    size: 6.2,
    color: SOFT,
  })
  page.elements.push(
    element(
      'rect',
      { x: 64, y: legendY + 0.3, width: 2.6, height: 2.6 },
      { shape: 'ellipse', fill: '#d9822b', border_width: 0, z: 4 },
    ),
  )
  text(page, { x: 67.5, y: legendY - 0.6, width: 44, height: 4 }, 'sedang berjalan', {
    size: 6.2,
    color: SOFT,
  })

  const numberY = 151
  text(page, { x: 14, y: numberY - 3, width: 38, height: 12 }, fmt(total), {
    size: 22,
    bold: true,
    family: 'montserrat',
    color: GOLD,
  })
  const wilayah = topPolres.length ? topPolres.join(', ') : 'Polda Bali'
  text(
    page,
    { x: 52, y: numberY, width: 144, height: 14 },
    `${fmt(total)} isian saran dan keluhan sepekan, hampir semuanya disertai tindak lanjut. Terbanyak dari wilayah ${wilayah}.`,
    { size: 7.2, color: INK },
  )

  text(
    page,
    { x: 14, y: 168, width: 182, height: 5 },
    'RENCANA TINDAK LANJUT PEKAN INI',
    { size: 6.6, bold: true, color: GOLD },
  )
  const planMeta: Record<string, { icon: string; text: string }> = {
    security: {
      icon: 'shield',
      text: 'Patroli jam rawan di pasar, permukiman dan perbatasan kabupaten',
    },
    traffic: {
      icon: 'bike',
      text: 'Razia knalpot brong dan balap liar di jalur bypass',
    },
    drugs: {
      icon: 'pill',
      text: 'Penyuluhan narkoba, vape dan geng pelajar di sekolah',
    },
    bullying: {
      icon: 'graduation-cap',
      text: 'Edukasi anti perundungan dan geng pelajar di sekolah',
    },
    waste: {
      icon: 'recycle',
      text: 'Kerja bakti membersihkan sampah dan saluran air',
    },
    online: {
      icon: 'smartphone',
      text: 'Edukasi hoaks dan judi daring di banjar dan rumah ibadah',
    },
    tourists: {
      icon: 'plane',
      text: 'Edukasi tertib berlalu lintas bagi wisatawan asing bersama desa adat',
    },
    youth: {
      icon: 'users-round',
      text: 'Patroli remaja kumpul malam dan razia miras',
    },
    jobs: {
      icon: 'building-2',
      text: 'Pembukaan lowongan, pelatihan kerja dan pendampingan UMKM',
    },
  }
  const fallbackPlan = {
    icon: 'clipboard-check',
    text: 'Menindaklanjuti masukan warga sesuai hasil forum dan laporan satuan',
  }
  const sortedThemes = [...themes].sort((left, right) => right.count - left.count)
  const plans = sortedThemes
    .slice(0, 6)
    .map((theme) => planMeta[theme.key] ?? fallbackPlan)
  while (plans.length < 6) plans.push(fallbackPlan)
  const planWidth = 58
  const planHeight = 24
  plans.forEach((plan, index) => {
    const column = index % 3
    const row = Math.floor(index / 3)
    const x = 14 + column * (planWidth + 4)
    const y = 176 + row * (planHeight + 2)
    rect(page, { x, y, width: planWidth, height: planHeight }, {
      fill: '#f5efe3',
      radius: 2,
      z: 2,
    })
    page.elements.push(
      element('rect', { x: x + 4, y: y + 6, width: 11, height: 11 }, {
        shape: 'ellipse',
        fill: '#1f3a5f',
        border_width: 0,
        z: 3,
      }),
    )
    page.elements.push(
      element('icon', { x: x + 6, y: y + 8, width: 7, height: 7 }, {
        icon_name: plan.icon,
        icon_color: WHITE,
        icon_stroke: 2,
        z: 4,
      }),
    )
    text(page, { x: x + 18, y: y + 4, width: planWidth - 21, height: 18 }, plan.text, {
      size: 6.4,
      color: INK,
    })
  })

  rect(
    page,
    { x: 14, y: 230, width: 182, height: 18 },
    { fill: '#e7f2e6', borderColor: '#b9d8b4', borderWidth: 0.6, radius: 2, z: 2 },
  )
  page.elements.push(
    element('rect', { x: 19, y: 233, width: 12, height: 12 }, {
      shape: 'ellipse',
      fill: '#3f9d4f',
      border_width: 0,
      z: 3,
    }),
  )
  page.elements.push(
    element('icon', { x: 21, y: 235, width: 8, height: 8 }, {
      icon_name: 'check',
      icon_color: WHITE,
      icon_stroke: 3,
      z: 4,
    }),
  )
  const followUpText = `Sudah ditindaklanjuti pekan ini: ${fmt(codeRows(data, 'QW1.10'))} patroli Sabhara, ${fmt(codeRows(data, 'QW1.9'))} Patroli Siaga Reserse, ${fmt(codeRows(data, 'QW1.14'))} DIMTAQ dan ${fmt(codeRows(data, 'QW1.11'))} patroli perairan; ${fmt(codeRows(data, 'QW4.3'))} panggilan 110 didatangi; ${fmt(codeRows(data, 'QW1.3'))} warga disalurkan kerja.`
  text(page, { x: 35, y: 233.5, width: 158, height: 12 }, followUpText, {
    size: 6.6,
    color: INK,
  })
  return page
}

function buildOtherQuickWins(data: Data): LayoutPage {
  const page = newPage(12)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 12)
  const groupColors: Record<string, string> = {
    QW1: '#1f3b57',
    QW2: '#a83a6e',
    QW3: '#6b3fa0',
    QW4: '#c96a2b',
    QW5: '#2f6d8e',
    QW6: '#8a5a2b',
    QW7: '#245a9e',
    QW8: '#2e7d4f',
  }
  const groups = ['QW1', 'QW2', 'QW3', 'QW4', 'QW5', 'QW6', 'QW7', 'QW8'].map((group) => {
    const codes = Object.keys(qwCodeTitles)
      .filter((code) => code.startsWith(`${group}.`))
      .filter((code) => group !== 'QW1' || Number(code.split('.')[1]) >= 9)
      .sort((left, right) => Number(left.split('.')[1]) - Number(right.split('.')[1]))
    const rows = codes.map((code) => ({ code, count: codeRows(data, code) }))
    return {
      group,
      title: qwGroupTitles[group] ?? group,
      color: groupColors[group] ?? FOREST,
      rows,
      total: rows.reduce((sum, row) => sum + row.count, 0),
    }
  })
  const allCodes = (data.coverage?.codes ?? []).filter((item) =>
    /^QW\d+\.\d+$/.test(item.code),
  )
  const topCode = [...allCodes]
    .filter((item) => item.rows > 0)
    .sort((left, right) => right.rows - left.rows)[0]
  const days = data.metrics.daily.length || 1
  const average = data.metrics.total_rows / days
  const polresUnits = data.coverage?.active_polres ?? data.metrics.by_polres.length
  const satkerUnits = data.coverage?.active_satker ?? 0
  const daily = data.metrics.daily.slice(-12)
  const peakCount = daily.reduce((max, item) => Math.max(max, item.count), 0)
  const peakIndex = daily.findIndex((item) => item.count === peakCount)

  const start = addSectionHeading(
    page,
    26,
    'Quick Wins Lainnya',
    `${fmt(data.metrics.total_rows)} Laporan Quick Wins`,
    'Rekap laporan QW1.9–QW1.14 dan kelompok QW2–QW8, satuan melapor, serta laporan harian.',
  )
  const nextY = addCards(
    page,
    start + 2,
    [
      {
        label: 'Laporan unik',
        value: fmt(data.metrics.total_rows),
        unit: 'laporan',
        detail: `${fmt(days)} hari periode`,
      },
      {
        label: 'Satuan melapor',
        value: fmt(polresUnits + satkerUnits),
        unit: 'satuan',
        detail: `${fmt(polresUnits)} polres · ${fmt(satkerUnits)} satker`,
      },
      {
        label: 'Sub-kode terbanyak',
        value: topCode ? fmt(topCode.rows) : '0',
        unit: 'laporan',
        detail: topCode
          ? `${topCode.code} · ${shorten(topCode.title, 26)}`
          : 'Belum ada data',
      },
      {
        label: 'Rata-rata per hari',
        value: fmt(average),
        unit: 'laporan',
        detail: `${fmt(days)} hari laporan`,
      },
    ],
    { height: 18, columns: 4 },
  )

  const colWidth = 89
  const colGap = 4
  const rowHeight = 44
  const gridTop = nextY + 2
  rect(page, { x: 12, y: gridTop - 2, width: 186, height: rowHeight * 4 + 2 }, {
    fill: '#f7f9f6',
    radius: 3,
    z: 1,
  })
  groups.forEach((group, index) => {
    const column = index % 2
    const row = Math.floor(index / 2)
    const x = 14 + column * (colWidth + colGap)
    const y = gridTop + row * rowHeight
    text(
      page,
      { x, y, width: 64, height: 4.5 },
      `${group.group} · ${shorten(group.title, 40)}`,
      { size: 6.3, bold: true, color: group.color },
    )
    text(
      page,
      { x: x + 64, y: y - 0.8, width: 25, height: 5 },
      fmt(group.total),
      { size: 10, bold: true, color: group.color, align: 'right' },
    )
    chart(
      page,
      { x, y: y + 4.5, width: colWidth, height: 37 },
      'hbar',
      '',
      group.rows.map(
        (item) =>
          `${item.code.slice(2)} ${shorten(
            (qwCodeTitles[item.code] ?? item.code).replace(/\s*\([^)]*\)\s*$/, ''),
            22,
          )}`,
      ),
      group.rows.map((item) => item.count),
      {
        colors: group.rows.map(() => group.color),
        showValues: true,
        titleSize: 0,
        labelSize: 7.8,
        valueSize: 7,
      },
    )
  })

  addPanelTitle(
    page,
    { x: 14, y: 250, width: 182, height: 10 },
    'Laporan Unik per Hari',
    `Puncak: ${peakIndex >= 0 ? timelineLabel(daily[peakIndex].date) : '—'} · ${fmt(peakCount)} laporan`,
  )
  const barsBottom = 273
  const barMax = 11
  const slot = 182 / Math.max(1, daily.length)
  rect(page, { x: 14, y: barsBottom, width: 182, height: 0.4 }, { fill: LINE, z: 4 })
  daily.forEach((item, index) => {
    const barHeight = peakCount > 0 ? (item.count / peakCount) * barMax : 0
    const barWidth = Math.min(14, slot * 0.55)
    const x = 14 + index * slot + (slot - barWidth) / 2
    const isPeak = index === peakIndex && item.count > 0
    rect(
      page,
      { x, y: barsBottom - barHeight, width: barWidth, height: Math.max(barHeight, 0.4) },
      { fill: isPeak ? GOLD : '#c9d7cc', radius: 1, z: 4 },
    )
    text(
      page,
      { x: 14 + index * slot, y: barsBottom - barHeight - 3.8, width: slot, height: 4 },
      fmt(item.count),
      { size: 6.2, bold: isPeak, color: isPeak ? GOLD : INK, align: 'center' },
    )
    text(
      page,
      { x: 14 + index * slot, y: barsBottom + 1.5, width: slot, height: 4 },
      timelineLabel(item.date),
      { size: 5.8, color: MUTED, align: 'center' },
    )
  })
  return page
}

function buildMedia(data: Data): LayoutPage {
  const page = newPage(13)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 13)
  const media = data.metrics.media
  const start = addSectionHeading(
    page,
    26,
    'Amplifikasi Media',
    `${fmt(media.total)} Tautan Publikasi`,
    `Rekap tautan unggahan media sosial dan humas yang dilampirkan pada laporan Quick Wins, ${data.frame.shortPeriod}.`,
  )
  const nextY = addCards(page, start + 2, [
    { label: 'Publikasi', value: fmt(media.publications), unit: 'tautan', detail: 'berita dan media sosial' },
    { label: 'Dokumentasi', value: fmt(media.documentation), unit: 'tautan', detail: 'dokumentasi kegiatan' },
    { label: 'Total tautan', value: fmt(media.total), unit: 'tautan', detail: 'seluruh kanal' },
    { label: 'Kegiatan', value: fmt(media.rows_with_media), unit: 'baris', detail: 'melampirkan media' },
  ])
  const channels = topLabeledRows(media.by_channel, 9, 'Platform', 'Tautan')
  chart(
    page,
    { x: 14, y: nextY + 4, width: 100, height: 62 },
    'donut',
    'Unggahan per platform',
    channels.rows.map((row) => row[0]),
    channels.rows.map((row) => Number(row[1].replace(/\./g, ''))),
    { showLegend: true, showValues: true },
  )
  const mediaLinks = (item: CoverageProgram | null): number =>
    metricTotal(item, ['media-online']) + metricTotal(item, ['media-sosial'])
  const mediaOf = (code: string): number => mediaLinks(codeProgram(data, code))
  const mainCodes = new Set([
    'QW1.1',
    'QW1.2',
    'QW1.3',
    'QW1.4',
    'QW1.5',
    'QW1.6',
    'QW1.7',
    'QW1.8',
  ])
  const otherMedia = (data.coverage?.codes ?? [])
    .filter((item) => !mainCodes.has(item.code))
    .reduce((sum, item) => sum + mediaLinks(item), 0)
  addPanelTitle(
    page,
    { x: 118, y: nextY + 4, width: 78, height: 10 },
    'Tautan per Quick Wins',
  )
  addCards(
    page,
    nextY + 13,
    [
      {
        label: 'QW1.1 & QW1.2',
        value: fmt(mediaOf('QW1.1') + mediaOf('QW1.2')),
        unit: 'tautan',
        detail: 'Gatur & Polidik',
      },
      {
        label: 'QW1.3',
        value: fmt(mediaOf('QW1.3')),
        unit: 'tautan',
        detail: 'Poliran',
      },
      {
        label: 'QW1.4',
        value: fmt(mediaOf('QW1.4')),
        unit: 'tautan',
        detail: 'Kumpul Bersama',
      },
      {
        label: 'QW1.5 & QW1.6',
        value: fmt(mediaOf('QW1.5') + mediaOf('QW1.6')),
        unit: 'tautan',
        detail: 'Green & Jumling',
      },
      {
        label: 'QW1.7 & QW1.8',
        value: fmt(mediaOf('QW1.7') + mediaOf('QW1.8')),
        unit: 'tautan',
        detail: 'Suling & Minggu Kasih',
      },
      {
        label: 'Quick Wins lainnya',
        value: fmt(otherMedia),
        unit: 'tautan',
        detail: 'sub-kode lainnya',
      },
    ],
    { x: 118, width: 78, columns: 2, height: 16, gap: 3, valueSize: 13 },
  )
  addPanelTitle(page, { x: 14, y: nextY + 72, width: 88, height: 10 }, 'Domain terbanyak')
  const domains = topLabeledRows(media.top_domains, 8, 'Domain', 'Tautan')
  table(page, { x: 14, y: nextY + 81, width: 88, height: 66 }, domains.columns, domains.rows, {
    fontSize: 6.6,
  })
  addPanelTitle(page, { x: 108, y: nextY + 72, width: 88, height: 10 }, 'Penekanan amplifikasi')
  text(
    page,
    { x: 108, y: nextY + 82, width: 88, height: 62 },
    `Setiap kegiatan utama Polda, Polres, dan Polsek diunggah minimal satu kali ke akun media sosial resmi dan tautannya dilampirkan pada laporan aplikasi di hari yang sama.\n\nTautan Google Drive, situs judi temuan patroli siber, dan tautan peta tidak dihitung sebagai publikasi media.`,
    { size: 7.4, color: SOFT },
  )
  return page
}

function buildNotes(data: Data): LayoutPage {
  const page = newPage(14)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 14)
  const start = addSectionHeading(
    page,
    26,
    'Catatan dan Penekanan',
    'Rekap Pelaksanaan dan Rencana Tindak Lanjut',
    `Rekap pelaksanaan ${data.frame.shortPeriod} untuk pelaksana Quick Wins di ${data.frame.polda}.`,
  )
  const nextY = addCards(page, start + 2, [
    { label: 'Total laporan', value: fmt(data.metrics.total_rows), unit: 'baris', detail: 'unik setelah deduplikasi' },
    { label: 'Duplikat', value: fmt(data.metrics.duplicate_rows), unit: 'baris', detail: 'dilewati saat impor' },
    { label: 'Di luar periode', value: fmt(data.metrics.outside_period), unit: 'baris', detail: 'tidak dihitung' },
    { label: 'Berkoordinat', value: fmt(data.metrics.rows_with_coordinates), unit: 'baris', detail: 'memiliki lokasi peta' },
  ])
  const quickWins = topLabeledRows(data.metrics.quick_wins, 8, 'Kelompok', 'Laporan')
  addPanelTitle(page, { x: 14, y: nextY + 4, width: 88, height: 10 }, 'Rekap kelompok Quick Wins')
  table(page, { x: 14, y: nextY + 13, width: 88, height: 66 }, quickWins.columns, quickWins.rows, {
    fontSize: 6.8,
  })
  const statuses = topLabeledRows(data.metrics.statuses, 8, 'Status', 'Laporan')
  addPanelTitle(page, { x: 108, y: nextY + 4, width: 88, height: 10 }, 'Status penerimaan')
  table(page, { x: 108, y: nextY + 13, width: 88, height: 66 }, statuses.columns, statuses.rows, {
    fontSize: 6.8,
  })
  addPanelTitle(page, { x: 14, y: nextY + 78, width: 182, height: 10 }, 'Penekanan')
  const missingGroups = (data.coverage?.groups ?? [])
    .filter((group) => group.polres_missing.length > 0)
    .slice(0, 4)
    .map(
      (group) =>
        `${group.code}: ${group.polres_missing.length} polres belum melapor (${group.polres_missing.slice(0, 4).join(', ')}${group.polres_missing.length > 4 ? ', …' : ''})`,
    )
  const lines = [
    `Total ${fmt(data.metrics.total_rows)} laporan unik dari ${fmt(data.metrics.source_count)} berkas impor; ${fmt(data.metrics.duplicate_rows)} duplikat dan ${fmt(data.metrics.outside_period)} baris di luar periode tidak dihitung.`,
    `${fmt(data.metrics.by_polres.length)} polres dan ${fmt(data.metrics.by_satker.length)} satker tercatat aktif pada periode ini.`,
    ...missingGroups,
    'Pastikan setiap kegiatan utama dilengkapi dokumentasi dan tautan publikasi pada hari yang sama.',
  ]
  text(page, { x: 14, y: nextY + 88, width: 182, height: 58 }, lines.join('\n'), {
    size: 7.4,
    color: SOFT,
  })
  return page
}

function quickWinGroupRows(data: Data, group: string): number {
  const fromCoverage = data.coverage?.groups.find((item) => item.code === group)?.rows
  if (fromCoverage !== undefined) return fromCoverage
  return Object.keys(qwCodeTitles)
    .filter((code) => code.startsWith(`${group}.`))
    .reduce((sum, code) => sum + codeRows(data, code), 0)
}

function sumCodes(data: Data, codes: string[]): number {
  return codes.reduce((sum, code) => sum + codeRows(data, code), 0)
}

function centralNote(data: Data, group: string): string {
  switch (group) {
    case 'QW1': {
      const utama = sumCodes(data, [
        'QW1.1',
        'QW1.2',
        'QW1.3',
        'QW1.4',
        'QW1.5',
        'QW1.6',
        'QW1.7',
        'QW1.8',
      ])
      const patroli = sumCodes(data, ['QW1.9', 'QW1.10', 'QW1.11'])
      const ekstrem = codeRows(data, 'QW1.12')
      return `${fmt(utama)} laporan kegiatan utama QW1.1–QW1.8, ${fmt(patroli)} patroli QW1.9–QW1.11, dan ${fmt(codeRows(data, 'QW1.14'))} DIMTAQ. ${
        ekstrem === 0
          ? 'QW1.12 belum ada laporan.'
          : `QW1.12 ${fmt(ekstrem)} laporan.`
      }`
    }
    case 'QW2': {
      const narkoba = data.metrics.narkoba
      return `${fmt(narkoba.rows)} laporan; ${fmt(narkoba.kawasan)} kawasan, ${fmt(narkoba.orang_diamankan)} orang diamankan, ${fmt(narkoba.rehabilitasi)} rehabilitasi, dan ${fmt(narkoba.peserta_penyuluhan)} peserta penyuluhan.`
    }
    case 'QW3':
      return `${fmt(codeRows(data, 'QW3.1'))} penegakan hukum siber, ${fmt(codeRows(data, 'QW3.2'))} layanan aduan korban, ${fmt(codeRows(data, 'QW3.3'))} edukasi anti judi daring, dan ${fmt(codeRows(data, 'QW3.4'))} patroli siber.`
    case 'QW4':
      return `${fmt(codeRows(data, 'QW4.1'))} penanganan TKP, ${fmt(codeRows(data, 'QW4.2'))} layanan SPKT, ${fmt(codeRows(data, 'QW4.3'))} panggilan 110, dan ${fmt(codeRows(data, 'QW4.4'))} layanan perempuan dan anak.`
    case 'QW5':
      return `${fmt(codeRows(data, 'QW5.1'))} SP2HP, ${fmt(codeRows(data, 'QW5.2'))} layanan informasi perkara, ${fmt(codeRows(data, 'QW5.3'))} gelar perkara, dan ${fmt(codeRows(data, 'QW5.4'))} keadilan restoratif.`
    case 'QW6': {
      const copot = codeRows(data, 'QW6.2')
      return `${fmt(codeRows(data, 'QW6.1'))} layanan SIM/SKCK dan tilang, ${fmt(copot)} Copot Hari Itu Juga, dan ${fmt(codeRows(data, 'QW6.3'))} kanal pengaduan. ${
        copot === 0 ? 'Tidak ada anggota yang tercatat pungli (QW6.2).' : ''
      }`
    }
    case 'QW7':
      return `${fmt(codeRows(data, 'QW7.1'))} Satu Desa Satu Bhabin, ${fmt(codeRows(data, 'QW7.2'))} penataan personel, ${fmt(codeRows(data, 'QW7.3'))} Polsek Siaga 24 Jam, dan ${fmt(codeRows(data, 'QW7.4'))} perkara ringan/rembuk warga.`
    case 'QW8':
      return `${fmt(codeRows(data, 'QW8.1'))} BBM ilegal/kejahatan ekonomi, ${fmt(codeRows(data, 'QW8.2'))} TPPO/korupsi/penyelundupan, ${fmt(codeRows(data, 'QW8.3'))} penyuluhan KUHP baru, dan ${fmt(codeRows(data, 'QW8.4'))} olah TKP ilmiah.`
    default:
      return 'Rekap pelaksanaan Quick Wins pada periode ini.'
  }
}

function centralEmphasis(data: Data, group: string): string {
  const topPolres =
    data.aspirations?.top_polres?.[0]?.label ??
    data.metrics.by_polres[0]?.label ??
    'Polres jajaran'
  switch (group) {
    case 'QW1':
      return `Pertahankan kehadiran pimpinan; ${topPolres} melengkapi Poliran dan Minggu Kasih di seluruh Polsek.`
    case 'QW2':
      return 'Tetapkan kawasan rawan narkoba tiap Polres; sasar sekolah soal narkoba, vape dan rokok pelajar.'
    case 'QW3':
      return 'Tindak lanjuti situs judi dan tawarkan pendampingan kepada setiap korban penipuan daring.'
    case 'QW4':
      return 'Pastikan setiap panggilan 110 didatangi dan sosialisasikan kanal 110 di banjar serta rumah ibadah.'
    case 'QW5':
      return 'SP2HP tepat waktu dan pastikan setiap laporan warga mendapat kepastian tindak lanjut.'
    case 'QW6':
      return 'Jawab kebutuhan kanal pengaduan; pastikan layanan bebas pungli dan transparan.'
    case 'QW7':
      return 'Bhabinkamtibmas bersama desa adat dan pecalang menghidupkan kembali ronda malam.'
    case 'QW8':
      return 'Labfor, Inafis dan Dokpol siap untuk kejadian di objek wisata alam; percepat identifikasi korban, termasuk WNA.'
    default:
      return 'Tindak lanjuti temuan lapangan sesuai arahan pimpinan.'
  }
}

function buildCentralNotes(data: Data): LayoutPage {
  const page = newPage(14)
  addHeader(page, data.frame)
  addFooter(page, data.frame, 14)

  const planWeek = aspirationPlanLabel(data.detail.period.end_date)
  const planYear = data.detail.period.end_date.slice(0, 4)
  text(
    page,
    { x: 14, y: 26, width: 182, height: 5 },
    `KETUA PROGRAM PRIORITAS KAPOLRI PUSAT · ${planWeek}${planYear ? ` ${planYear}` : ''}`,
    { size: 6.6, bold: true, color: GOLD },
  )
  text(
    page,
    { x: 14, y: 31, width: 182, height: 11 },
    'Catatan dan Penekanan Ketua Pusat',
    { size: 20, bold: true, family: 'montserrat', color: FOREST },
  )
  const polda = (data.metrics.polda_name || 'Polda Bali')
    .toLowerCase()
    .replace(/\b\w/g, (char) => char.toUpperCase())
  text(
    page,
    { x: 14, y: 44, width: 182, height: 10 },
    `Kepada pelaksana masing-masing Quick Wins di ${polda}, berdasarkan rekap pelaksanaan ${data.frame.period}.`,
    { size: 7.4, italic: true, color: SOFT },
  )

  const groups = [
    {
      code: 'QW1',
      title: 'Ruang Publik Aman',
      color: '#1f3a5f',
      tint: '#eef3f8',
      audience: 'Diropsamta, Dirbinmas, Dirpolairud, Kapolres',
    },
    {
      code: 'QW2',
      title: 'Pemulihan Kawasan Narkoba',
      color: '#a83a6e',
      tint: '#f9eef3',
      audience: 'Dirresnarkoba, Kasat Resnarkoba',
    },
    {
      code: 'QW3',
      title: 'Bersih Judi Daring',
      color: '#6b3fa0',
      tint: '#f3eef9',
      audience: 'Dirreskrimsus, Kasat Cyber',
    },
    {
      code: 'QW4',
      title: '110 Respons Cepat, SPKT',
      color: '#c96a2b',
      tint: '#fbf1e8',
      audience: 'Karo Ops, Ka SPKT, Kapolres, Kapolsek',
    },
    {
      code: 'QW5',
      title: 'Perkara Terlacak',
      color: '#2f6d8e',
      tint: '#edf5f8',
      audience: 'Dirreskrimum, Kasat Reskrim, Kanit Reskrim',
    },
    {
      code: 'QW6',
      title: 'Bersih ke Dalam',
      color: '#8a5a2b',
      tint: '#f7f0e7',
      audience: 'Kabid Propam, Irwasda, Dirbinmas',
    },
    {
      code: 'QW7',
      title: 'Polres Kuat, Polsek Tangguh',
      color: '#245a9e',
      tint: '#edf3fa',
      audience: 'Karo SDM, Dirbinmas, Kapolres, Kapolsek',
    },
    {
      code: 'QW8',
      title: 'KUHP Baru, Penyidikan Ilmiah',
      color: '#2e7d4f',
      tint: '#edf7f0',
      audience: 'Dirreskrimsus, Dirreskrimum, Kabiddokkes, Kabidlabfor',
    },
  ]
  const cardWidth = 89
  const cardHeight = 48
  const gapX = 4
  const gapY = 4
  groups.forEach((item, index) => {
    const column = index % 2
    const row = Math.floor(index / 2)
    const x = 14 + column * (cardWidth + gapX)
    const y = 59 + row * (cardHeight + gapY)
    rect(
      page,
      { x, y, width: cardWidth, height: cardHeight },
      {
        fill: item.tint,
        borderColor: '#e1e5e9',
        borderWidth: 0.5,
        radius: 1.8,
        z: 2,
      },
    )
    text(page, { x: x + 4.5, y: y + 2.8, width: 24, height: 4 }, item.code, {
      size: 6.8,
      bold: true,
      color: item.color,
    })
    text(
      page,
      { x: x + 62, y: y + 1.8, width: 22, height: 6 },
      fmt(quickWinGroupRows(data, item.code)),
      { size: 13, bold: true, family: 'montserrat', color: item.color, align: 'right' },
    )
    text(page, { x: x + 4.5, y: y + 6.6, width: 79, height: 5.5 }, item.title, {
      size: 9.2,
      bold: true,
      color: FOREST,
    })
    text(
      page,
      { x: x + 4.5, y: y + 12.7, width: 79, height: 5.2 },
      `Kepada: ${item.audience}`,
      { size: 5.5, color: MUTED },
    )
    page.elements.push(
      element('icon', { x: x + 4.5, y: y + 18.8, width: 3.2, height: 3.2 }, {
        icon_name: 'clipboard-check',
        icon_color: item.color,
        icon_stroke: 2,
        z: 4,
      }),
    )
    text(
      page,
      { x: x + 9, y: y + 18.2, width: 76, height: 12.4 },
      `Catatan · ${centralNote(data, item.code)}`,
      { size: 5.8, color: INK },
    )
    text(
      page,
      { x: x + 4.5, y: y + 31.5, width: 80, height: 13.5 },
      `Penekanan · ${centralEmphasis(data, item.code)}`,
      { size: 5.8, bold: true, color: INK, background: '#eef1f3' },
    )
  })

  text(
    page,
    { x: 14, y: 268, width: 182, height: 5 },
    'Angka di pojok kartu: laporan unik per Quick Wins sepekan, termasuk di luar hari jadwal. Angka dikutip apa adanya dari laporan satuan.',
    { size: 5.6, italic: true, color: MUTED },
  )
  return page
}

function buildClosing(data: Data): LayoutPage {
  const page = newPage(15, WHITE, { colors: [DEEP, FOREST, '#245343'], angle: 135 })
  rect(page, { x: 0, y: 0, width: PAGE_WIDTH, height: 4 }, { fill: GOLD, z: 1 })
  text(page, { x: 18, y: 84, width: 174, height: 16 }, 'Terima Kasih', {
    size: 34,
    bold: true,
    family: 'montserrat',
    color: WHITE,
    align: 'center',
  })
  text(page, { x: 18, y: 103, width: 174, height: 8 }, 'Polisi Bersama Masyarakat', {
    size: 13,
    family: 'montserrat',
    color: PALE_GOLD,
    align: 'center',
  })
  text(page, { x: 18, y: 114, width: 174, height: 6 }, 'MELINDUNGI · MENGAYOMI · MELAYANI', {
    size: 8,
    bold: true,
    color: PALE,
    align: 'center',
  })
  const programs = ['QW1.1', 'QW1.2', 'QW1.3', 'QW1.4', 'QW1.5', 'QW1.6', 'QW1.7', 'QW1.8']
  const titles = ['Gatur Lalin', 'Polidik', 'Poliran', 'Kumpul Bersama', 'Green Policing', 'Jumat Keliling', 'Subuh Keliling', 'Minggu Kasih']
  const cardWidth = (174 - 3 * 3) / 4
  programs.forEach((code, index) => {
    const column = index % 4
    const row = Math.floor(index / 4)
    card(
      page,
      {
        x: 18 + column * (cardWidth + 3),
        y: 140 + row * 42,
        width: cardWidth,
        height: 37,
      },
      titles[index],
      fmt(codeRows(data, code)),
      'laporan',
      codeTitle(data, code).slice(0, 46),
      {
        fill: DARK_PANEL,
        borderColor: DARK_LINE,
        labelColor: PALE,
        valueColor: WHITE,
        unitColor: PALE_GOLD,
        detailColor: '#a9c1b4',
        accent: GOLD,
        valueSize: 22,
      },
    )
  })
  text(
    page,
    { x: 18, y: 228, width: 174, height: 6 },
    `${fmt(data.metrics.total_rows)} laporan unik · ${fmt(data.metrics.by_polres.length)} polres · ${data.frame.period}`,
    { size: 8, color: PALE, align: 'center' },
  )
  rect(page, { x: 60, y: 240, width: 90, height: 0.6 }, { fill: GOLD, z: 3 })
  text(
    page,
    { x: 18, y: 246, width: 174, height: 5 },
    `${data.frame.polda} · LAPORAN MINGGUAN 8 QUICK WINS KAPOLRI`,
    { size: 6.6, color: PALE, align: 'center' },
  )
  text(
    page,
    { x: 18, y: 252, width: 174, height: 5 },
    data.frame.shortPeriod.toUpperCase(),
    { size: 6.6, color: PALE_GOLD, align: 'center' },
  )
  return page
}

export function buildWeeklyReportTemplate(input: WeeklyTemplateInput): ReportLayout {
  elementCounter = 0
  const { metrics, coverage, detail } = input
  const start = detail.period.start_date || metrics.period_start
  const end = detail.period.end_date || metrics.period_end
  const frame: Frame = {
    polda: (metrics.polda_name || 'POLDA BALI').toUpperCase(),
    period: periodShort(start, end),
    shortPeriod: periodShort(start, end),
  }
  const data: Data = {
    metrics,
    coverage,
    detail,
    frame,
    mapImageSrc: input.mapImageSrc ?? '',
    kumpulMapImageSrc: input.kumpulMapImageSrc ?? '',
    greenMapImageSrc: input.greenMapImageSrc ?? '',
    greenDaily: input.greenDaily ?? [],
    jumlingMapImageSrc: input.jumlingMapImageSrc ?? '',
    jumlingDaily: input.jumlingDaily ?? [],
    subuhMapImageSrc: input.subuhMapImageSrc ?? '',
    subuhDaily: input.subuhDaily ?? [],
    mingguMapImageSrc: input.mingguMapImageSrc ?? '',
    mingguDaily: input.mingguDaily ?? [],
    gaturDaily: input.gaturDaily ?? [],
    polidikDaily: input.polidikDaily ?? [],
    kumpulDaily: input.kumpulDaily ?? [],
    aspirations: input.aspirations ?? null,
    feedback: input.feedback ?? { gatur: [], polidik: [] },
    kumpulFeedback: input.kumpulFeedback ?? [],
    kumpulFeedbackTotal: input.kumpulFeedbackTotal ?? 0,
    leadership: input.leadership ?? null,
    poliran: input.poliran ?? null,
  }
  const pages = [
    buildCover(data),
    buildGaturPolidik(data),
    buildKumpulBersama(data),
    buildKapolda(data),
    buildPoliran(data),
    buildGreen(data),
    buildWorship(7, 'QW1.6', 'jumat_keliling', data),
    buildWorship(8, 'QW1.7', 'subuh_keliling', data),
    buildWorship(9, 'QW1.8', 'minggu_kasih', data),
    buildAspirationsPage(data),
    buildAspirationFollowUp(data),
    buildOtherQuickWins(data),
    buildMedia(data),
    buildCentralNotes(data),
    buildClosing(data),
  ]
  return {
    version: 1,
    mode: 'blank',
    orientation: 'portrait',
    pages,
  }
}
