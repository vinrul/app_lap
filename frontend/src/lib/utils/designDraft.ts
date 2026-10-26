import type { ReportLayout } from '../types'

const STORAGE_PREFIX = 'laporan:design-draft:v1:'
const MAX_DRAFT_BYTES = 2 * 1024 * 1024

export interface DesignDraft {
  version: 1
  periodId: string
  ownerId: string
  savedAt: string
  baseUpdatedAt: string | null
  json: string
}

function storage(): Storage | null {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage
  } catch {
    return null
  }
}

function keyFor(periodId: string, ownerId: string): string {
  return `${STORAGE_PREFIX}${encodeURIComponent(ownerId)}:${periodId}`
}

export function readDesignDraft(periodId: string, ownerId: string): DesignDraft | null {
  const store = storage()
  if (!store) return null
  let raw: string | null
  try {
    raw = store.getItem(keyFor(periodId, ownerId))
  } catch {
    return null
  }
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as Partial<DesignDraft>
    if (
      parsed.version !== 1 ||
      parsed.periodId !== periodId ||
      parsed.ownerId !== ownerId ||
      typeof parsed.savedAt !== 'string' ||
      typeof parsed.json !== 'string' ||
      (parsed.baseUpdatedAt !== null &&
        parsed.baseUpdatedAt !== undefined &&
        typeof parsed.baseUpdatedAt !== 'string')
    ) {
      clearDesignDraft(periodId, ownerId)
      return null
    }
    return {
      version: 1,
      periodId,
      ownerId,
      savedAt: parsed.savedAt,
      baseUpdatedAt: parsed.baseUpdatedAt ?? null,
      json: parsed.json,
    }
  } catch {
    clearDesignDraft(periodId, ownerId)
    return null
  }
}

export function writeDesignDraft(
  periodId: string,
  layout: ReportLayout,
  baseUpdatedAt: string | null,
  ownerId: string,
): DesignDraft | null {
  const store = storage()
  if (!store) return null
  let json: string
  try {
    json = JSON.stringify(layout)
  } catch {
    return null
  }
  const draft: DesignDraft = {
    version: 1,
    periodId,
    ownerId,
    savedAt: new Date().toISOString(),
    baseUpdatedAt,
    json,
  }
  let record: string
  try {
    record = JSON.stringify(draft)
  } catch {
    return null
  }
  // localStorage menyimpan string UTF-16; batasi agar tidak menabrak kuota.
  if (record.length * 2 > MAX_DRAFT_BYTES) return null
  try {
    store.setItem(keyFor(periodId, ownerId), record)
    return draft
  } catch {
    return null
  }
}

export function clearDesignDraft(periodId: string, ownerId: string): void {
  const store = storage()
  if (!store) return
  try {
    store.removeItem(keyFor(periodId, ownerId))
  } catch {
    // Draft lokal bersifat best-effort; kegagalan menghapus tidak fatal.
  }
}

export function parseDesignDraftLayout(draft: DesignDraft): ReportLayout | null {
  try {
    const parsed = JSON.parse(draft.json) as Partial<ReportLayout>
    if (!parsed || typeof parsed !== 'object' || !Array.isArray(parsed.pages)) return null
    return parsed as ReportLayout
  } catch {
    return null
  }
}

export function layoutsMatch(left: ReportLayout, right: ReportLayout): boolean {
  return stableJson(left) === stableJson(right)
}

function stableJson(value: unknown): string {
  return JSON.stringify(sortKeys(value))
}

function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeys)
  if (value && typeof value === 'object') {
    const record = value as Record<string, unknown>
    const sorted: Record<string, unknown> = {}
    for (const key of Object.keys(record).sort()) {
      sorted[key] = sortKeys(record[key])
    }
    return sorted
  }
  return value
}
