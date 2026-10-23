const numberFormatter = new Intl.NumberFormat('id-ID')
const dateFormatter = new Intl.DateTimeFormat('id-ID', { day: 'numeric', month: 'long', year: 'numeric' })
const shortDateFormatter = new Intl.DateTimeFormat('id-ID', { day: 'numeric', month: 'short', year: 'numeric' })
const dateTimeFormatter = new Intl.DateTimeFormat('id-ID', {
  day: 'numeric',
  month: 'short',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

export const formatNumber = (value: number | null | undefined): string =>
  numberFormatter.format(value ?? 0)

export const formatDate = (value: string | null | undefined): string => {
  if (!value) return '—'
  const date = new Date(`${value.slice(0, 10)}T12:00:00`)
  if (Number.isNaN(date.getTime())) return value
  return dateFormatter.format(date)
}

export const formatShortDate = (value: string | null | undefined): string => {
  if (!value) return '—'
  const date = new Date(`${value.slice(0, 10)}T12:00:00`)
  if (Number.isNaN(date.getTime())) return value
  return shortDateFormatter.format(date)
}

export const formatDateTime = (value: string | null | undefined): string => {
  if (!value) return '—'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return dateTimeFormatter.format(date)
}

export const formatRange = (start: string, end: string): string =>
  `${formatDate(start)} – ${formatDate(end)}`

export const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export const percentage = (value: number, total: number): number =>
  total > 0 ? Math.round((value / total) * 100) : 0

export const toIsoDate = (date: Date): string => {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export const startOfWeek = (date: Date): Date => {
  const copy = new Date(date)
  const day = copy.getDay() || 7
  copy.setDate(copy.getDate() - day + 1)
  copy.setHours(12, 0, 0, 0)
  return copy
}

export const addDays = (date: Date, days: number): Date => {
  const copy = new Date(date)
  copy.setDate(copy.getDate() + days)
  return copy
}

export const statusOfPeriod = (period: { imports: number; reports: number }): {
  label: string
  tone: 'neutral' | 'progress' | 'success'
} => {
  if (period.reports > 0) return { label: 'PDF tersedia', tone: 'success' }
  if (period.imports > 0) return { label: 'Siap disusun', tone: 'progress' }
  return { label: 'Menunggu impor', tone: 'neutral' }
}
