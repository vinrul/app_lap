import type {
  AdminUser,
  Dashboard,
  DesignSource,
  DesignTemplateView,
  DraftView,
  ImportPreview,
  ImportView,
  LayoutView,
  Meta,
  Period,
  PeriodDetail,
  RecordsResponse,
  ReportContent,
  ReportLayout,
  ReportMetrics,
  ReportView,
  UploadedDesignImage,
  User,
} from '../types'
import { queryString, request } from './client'

export const authApi = {
  me: () => request<User>('/auth/me'),
  login: (username: string, password: string) =>
    request<User>('/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) }),
  logout: () => request<void>('/auth/logout', { method: 'POST' }),
}

export const metaApi = {
  get: () => request<Meta>('/meta'),
}

export const dashboardApi = {
  get: () => request<Dashboard>('/dashboard'),
}

export const periodsApi = {
  list: () => request<Period[]>('/periods'),
  create: (input: { start_date: string; end_date: string; label?: string }) =>
    request<Period>('/periods', { method: 'POST', body: JSON.stringify(input) }),
  detail: (periodId: string) => request<PeriodDetail>(`/periods/${periodId}`),
  imports: (periodId: string) => request<ImportView[]>(`/periods/${periodId}/imports`),
  metrics: (periodId: string, importId?: string) =>
    request<ReportMetrics>(
      `/periods/${periodId}/metrics${queryString({ import_id: importId })}`,
    ),
}

export const importsApi = {
  preview: async (periodId: string, file: File) => {
    const form = new FormData()
    form.append('file', file)
    return request<ImportPreview>(`/periods/${periodId}/imports/preview`, {
      method: 'POST',
      body: form,
    })
  },
  commit: async (periodId: string, file: File) => {
    const form = new FormData()
    form.append('file', file)
    return request<ImportView>(`/periods/${periodId}/imports`, { method: 'POST', body: form })
  },
}

export interface RecordFilters {
  importId?: string
  search?: string
  quickWins?: string
  status?: string
  polres?: string
  page?: number
  pageSize?: number
}

export const recordsApi = {
  list: (periodId: string, filters: RecordFilters = {}) =>
    request<RecordsResponse>(
      `/periods/${periodId}/records${queryString({
        import_id: filters.importId,
        search: filters.search,
        quick_wins: filters.quickWins,
        status: filters.status,
        polres: filters.polres,
        page: filters.page,
        page_size: filters.pageSize,
      })}`,
    ),
}

export const reportsApi = {
  list: (periodId: string) => request<ReportView[]>(`/periods/${periodId}/reports`),
  getDraft: (periodId: string, importId: string) =>
    request<DraftView>(`/periods/${periodId}/drafts/${importId}`),
  saveDraft: (periodId: string, importId: string, content: ReportContent) =>
    request<DraftView>(`/periods/${periodId}/drafts/${importId}`, {
      method: 'PUT',
      body: JSON.stringify(content),
    }),
  generate: (periodId: string, importId: string, content: ReportContent) =>
    request<ReportView>(`/periods/${periodId}/reports`, {
      method: 'POST',
      body: JSON.stringify({ import_id: importId, content }),
    }),
  preview: (periodId: string, importId: string, content: ReportContent) =>
    request<{ html: string }>(`/periods/${periodId}/reports/preview`, {
      method: 'POST',
      body: JSON.stringify({ import_id: importId, content }),
    }),
  downloadUrl: (reportId: string) => `/api/reports/${reportId}/pdf`,
  previewUrl: (reportId: string) => `/api/reports/${reportId}/preview`,
}

export const designApi = {
  getLayout: (periodId: string, importId: string) =>
    request<LayoutView>(
      `/periods/${periodId}/layout${queryString({ import_id: importId })}`,
    ),
  saveLayout: (periodId: string, importId: string, layout: ReportLayout) =>
    request<LayoutView>(`/periods/${periodId}/layout`, {
      method: 'PUT',
      body: JSON.stringify({ import_id: importId, layout }),
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
  sources: (periodId: string, importId: string) =>
    request<DesignSource[]>(
      `/periods/${periodId}/design/sources${queryString({ import_id: importId })}`,
    ),
  templates: () => request<DesignTemplateView[]>('/design/templates'),
  saveTemplate: (name: string, layout: ReportLayout) =>
    request<DesignTemplateView>('/design/templates', {
      method: 'POST',
      body: JSON.stringify({ name, layout }),
    }),
  deleteTemplate: (templateId: string) =>
    request<void>(`/design/templates/${templateId}`, { method: 'DELETE' }),
  applyTemplate: (periodId: string, importId: string, templateId: string) =>
    request<LayoutView>(`/periods/${periodId}/design/templates/${templateId}/apply`, {
      method: 'POST',
      body: JSON.stringify({ import_id: importId }),
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
