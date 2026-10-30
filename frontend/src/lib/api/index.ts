import type {
  AdminUser,
  AnalysisResponse,
  CoverageResponse,
  Dashboard,
  DesignSource,
  DesignTemplateView,
  DraftView,
  ImportPreview,
  ImportView,
  LayoutView,
  MediaItem,
  Meta,
  Period,
  PeriodDetail,
  QuickWinDetail,
  RecordItem,
  RecordsResponse,
  PublicShareView,
  ReportContent,
  ReportLayout,
  ReportMetrics,
  ReportShareView,
  ReportView,
  UnitDetail,
  UploadedDesignImage,
  User,
} from '../types'
import { queryString, request } from './client'

export const authApi = {
  me: () => request<User>('/auth/me'),
  login: (username: string, password: string) =>
    request<User>('/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) }),
  logout: () => request<void>('/auth/logout', { method: 'POST' }),
  changePassword: (currentPassword: string, newPassword: string) =>
    request<void>('/auth/password', {
      method: 'POST',
      body: JSON.stringify({
        current_password: currentPassword,
        new_password: newPassword,
      }),
    }),
}

export const metaApi = {
  get: () => request<Meta>('/meta'),
}

export const dashboardApi = {
  get: () => request<Dashboard>('/dashboard'),
}

export const periodsApi = {
  list: (includeArchived = false) =>
    request<Period[]>(`/periods${includeArchived ? '?include_archived=true' : ''}`),
  create: (input: {
    start_date: string
    end_date: string
    label?: string
    period_type?: 'daily' | 'range' | 'monthly'
  }) => request<Period>('/periods', { method: 'POST', body: JSON.stringify(input) }),
  update: (
    periodId: string,
    input: {
      start_date?: string
      end_date?: string
      label?: string
      period_type?: 'daily' | 'range' | 'monthly'
    },
  ) =>
    request<Period>(`/periods/${periodId}`, {
      method: 'PATCH',
      body: JSON.stringify(input),
    }),
  archive: (periodId: string) =>
    request<Period>(`/periods/${periodId}/archive`, { method: 'POST' }),
  restore: (periodId: string) =>
    request<Period>(`/periods/${periodId}/restore`, { method: 'POST' }),
  detail: (periodId: string) => request<PeriodDetail>(`/periods/${periodId}`),
  metrics: (periodId: string) =>
    request<ReportMetrics>(`/periods/${periodId}/metrics`),
}

export const importsApi = {
  list: () => request<ImportView[]>('/imports'),
  preview: async (file: File) => {
    const form = new FormData()
    form.append('file', file)
    return request<ImportPreview>('/imports/preview', {
      method: 'POST',
      body: form,
    })
  },
  commit: async (file: File) => {
    const form = new FormData()
    form.append('file', file)
    return request<ImportView>('/imports', { method: 'POST', body: form })
  },
  records: (importId: string, filters: RecordFilters = {}) =>
    request<RecordsResponse>(
      `/imports/${importId}/records${queryString({
        search: filters.search,
        quick_wins: filters.quickWins,
        status: filters.status,
        polres: filters.polres,
        level: filters.level,
        page: filters.page,
        page_size: filters.pageSize,
      })}`,
    ),
  analysis: (periodId: string) =>
    request<AnalysisResponse>(`/periods/${periodId}/analysis`),
  coverage: (periodId: string) =>
    request<CoverageResponse>(`/periods/${periodId}/coverage`),
  retryEvidence: (importId: string) =>
    request<{ queued: number }>(`/imports/${importId}/evidence/retry`, { method: 'POST' }),
  unitDetail: (periodId: string, scope: 'polres' | 'satker', label: string) =>
    request<UnitDetail>(
      `/periods/${periodId}/unit-detail${queryString({ scope, label })}`,
    ),
  quickWinDetail: (
    periodId: string,
    code: string,
    unit?: { scope: 'polres' | 'satker'; label: string },
  ) =>
    request<QuickWinDetail>(
      `/periods/${periodId}/quick-wins/${encodeURIComponent(code)}${queryString({
        scope: unit?.scope,
        label: unit?.label,
      })}`,
    ),
  recordDetail: (periodId: string, recordId: number) =>
    request<RecordItem>(`/periods/${periodId}/records/${recordId}`),
}

export interface RecordFilters {
  search?: string
  quickWins?: string
  status?: string
  polres?: string
  level?: string
  page?: number
  pageSize?: number
}

export const recordsApi = {
  list: (periodId: string, filters: RecordFilters = {}) =>
    request<RecordsResponse>(
      `/periods/${periodId}/records${queryString({
        search: filters.search,
        quick_wins: filters.quickWins,
        status: filters.status,
        polres: filters.polres,
        level: filters.level,
        page: filters.page,
        page_size: filters.pageSize,
      })}`,
    ),
}

export const reportsApi = {
  list: (periodId: string) => request<ReportView[]>(`/periods/${periodId}/reports`),
  getDraft: (periodId: string) =>
    request<DraftView>(`/periods/${periodId}/draft`),
  saveDraft: (periodId: string, content: ReportContent) =>
    request<DraftView>(`/periods/${periodId}/draft`, {
      method: 'PUT',
      body: JSON.stringify(content),
    }),
  generate: (periodId: string, content: ReportContent) =>
    request<ReportView>(`/periods/${periodId}/reports`, {
      method: 'POST',
      body: JSON.stringify({ content }),
    }),
  preview: (periodId: string, content: ReportContent, layout?: ReportLayout) =>
    request<{ html: string }>(`/periods/${periodId}/reports/preview`, {
      method: 'POST',
      body: JSON.stringify({ content, layout }),
    }),
  downloadUrl: (reportId: string) => `/api/reports/${reportId}/pdf`,
  previewUrl: (reportId: string) => `/api/reports/${reportId}/preview`,
  share: (reportId: string) =>
    request<ReportShareView>(`/reports/${reportId}/share`, { method: 'POST' }),
  remove: (reportId: string) =>
    request<void>(`/reports/${reportId}`, { method: 'DELETE' }),
}

export const publicShareApi = {
  info: (token: string) =>
    request<PublicShareView>(`/public/shares/${encodeURIComponent(token)}`),
  previewUrl: (token: string) =>
    `/api/public/shares/${encodeURIComponent(token)}/preview`,
  pdfUrl: (token: string) =>
    `/api/public/shares/${encodeURIComponent(token)}/pdf`,
}

export const designApi = {
  getLayout: (periodId: string) =>
    request<LayoutView>(`/periods/${periodId}/layout`),
  saveLayout: (periodId: string, layout: ReportLayout, keepalive = false) =>
    request<LayoutView>(`/periods/${periodId}/layout`, {
      method: 'PUT',
      body: JSON.stringify({ layout }),
      keepalive,
    }),
  uploadImage: async (periodId: string, file: File) => {
    const form = new FormData()
    form.append('file', file)
    return request<UploadedDesignImage>(`/periods/${periodId}/layout/images`, {
      method: 'POST',
      body: form,
    })
  },
  imageUrl: (periodId: string, imageId: string) =>
    `/api/periods/${periodId}/layout/images/${imageId}`,
  sources: (periodId: string) =>
    request<DesignSource[]>(`/periods/${periodId}/design/sources`),
  media: (periodId: string) =>
    request<MediaItem[]>(`/periods/${periodId}/design/media`),
  importMedia: (periodId: string, source: 'design' | 'evidence', id: string) =>
    request<UploadedDesignImage>(`/periods/${periodId}/layout/images/from-media`, {
      method: 'POST',
      body: JSON.stringify({ source, id }),
    }),
  templates: () => request<DesignTemplateView[]>('/design/templates'),
  saveTemplate: (name: string, layout: ReportLayout) =>
    request<DesignTemplateView>('/design/templates', {
      method: 'POST',
      body: JSON.stringify({ name, layout }),
    }),
  deleteTemplate: (templateId: string) =>
    request<void>(`/design/templates/${templateId}`, { method: 'DELETE' }),
  applyTemplate: (periodId: string, templateId: string) =>
    request<LayoutView>(`/periods/${periodId}/design/templates/${templateId}/apply`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
}

export const usersApi = {
  list: () => request<AdminUser[]>('/admin/users'),
  create: (input: { username: string; password: string; role: string }) =>
    request<AdminUser>('/admin/users', { method: 'POST', body: JSON.stringify(input) }),
  update: (userId: string, input: { role?: string; active?: boolean }) =>
    request<AdminUser>(`/admin/users/${userId}`, {
      method: 'PATCH',
      body: JSON.stringify(input),
    }),
}

export async function downloadReport(report: ReportView): Promise<void> {
  const response = await fetch(reportsApi.downloadUrl(report.id), { credentials: 'same-origin' })
  if (!response.ok) throw new Error('PDF tidak dapat diunduh.')
  const blob = await response.blob()
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = `laporan-mingguan-${report.period_label.replace(/\s+/g, '-')}.pdf`
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  URL.revokeObjectURL(url)
}

export function openReportPreview(report: ReportView): void {
  window.open(reportsApi.previewUrl(report.id), '_blank', 'noopener')
}

export function openHtmlPreview(html: string): void {
  const blob = new Blob([html], { type: 'text/html' })
  const url = URL.createObjectURL(blob)
  window.open(url, '_blank', 'noopener')
  window.setTimeout(() => URL.revokeObjectURL(url), 60_000)
}
