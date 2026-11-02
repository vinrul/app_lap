import type {
  LayoutBlockOffset,
  LayoutElement,
  LayoutPage,
  LayoutPagePatch,
  LayoutPatch,
  ReportLayout,
} from '../types'

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function same(left: unknown, right: unknown): boolean {
  return JSON.stringify(left ?? null) === JSON.stringify(right ?? null)
}

/**
 * Membandingkan layout yang terakhir diketahui dari server dengan layout lokal.
 * Hanya elemen/blok yang berubah dikirim, sehingga perubahan pengguna lain pada
 * elemen berbeda tidak ikut tertimpa.
 */
export function diffLayout(
  base: ReportLayout,
  current: ReportLayout,
  baseRevision: number,
): LayoutPatch | null {
  if (
    base.mode !== current.mode ||
    base.orientation !== current.orientation ||
    base.pages.length !== current.pages.length ||
    base.pages.some((page, index) => page.page !== current.pages[index]?.page)
  ) {
    return { base_revision: baseRevision, replace: clone(current) }
  }

  const pages: LayoutPagePatch[] = []
  for (const page of current.pages) {
    const previous = base.pages.find((item) => item.page === page.page)
    if (!previous) {
      return { base_revision: baseRevision, replace: clone(current) }
    }
    const patch: LayoutPagePatch = { page: page.page }
    let changed = false

    if (!same(previous.background, page.background)) {
      patch.background = page.background
      changed = true
    }
    if (!same(previous.background_mode, page.background_mode)) {
      patch.background_mode = page.background_mode
      changed = true
    }
    if (!same(previous.gradient_angle, page.gradient_angle)) {
      patch.gradient_angle = page.gradient_angle
      changed = true
    }
    if (!same(previous.gradient_colors, page.gradient_colors)) {
      patch.gradient_colors = page.gradient_colors
      changed = true
    }

    const previousElements = new Map(previous.elements.map((item) => [item.id, item]))
    const currentIds = new Set(page.elements.map((item) => item.id))
    const elements = page.elements.filter((item) => {
      const before = previousElements.get(item.id)
      return !before || !same(before, item)
    })
    if (elements.length) {
      patch.elements_upsert = clone(elements)
      changed = true
    }
    const removedElements = [...previousElements.keys()].filter((id) => !currentIds.has(id))
    if (removedElements.length) {
      patch.elements_remove = removedElements
      changed = true
    }

    const previousBlocks = previous.blocks ?? {}
    const currentBlocks = page.blocks ?? {}
    const blocks: { id: string; block: LayoutBlockOffset }[] = Object.entries(
      currentBlocks,
    )
      .filter(([id, block]) => !same(previousBlocks[id], block))
      .map(([id, block]) => ({ id, block: clone(block) }))
    if (blocks.length) {
      patch.blocks_upsert = blocks
      changed = true
    }
    const removedBlocks = Object.keys(previousBlocks).filter((id) => !(id in currentBlocks))
    if (removedBlocks.length) {
      patch.blocks_remove = removedBlocks
      changed = true
    }

    if (changed) pages.push(patch)
  }

  if (!pages.length) return null
  return { base_revision: baseRevision, pages }
}

/** Menerapkan patch ke salinan layout tanpa mengubah objek asli. */
export function applyLayoutPatch(layout: ReportLayout, patch: LayoutPatch): ReportLayout {
  if (patch.replace) return clone(patch.replace)
  const next = clone(layout)
  if (patch.mode) next.mode = patch.mode as ReportLayout['mode']
  if (patch.orientation) next.orientation = patch.orientation as ReportLayout['orientation']

  for (const pagePatch of patch.pages ?? []) {
    const page = next.pages.find((item) => item.page === pagePatch.page)
    if (!page) continue
    if (pagePatch.background !== undefined) page.background = pagePatch.background
    if (pagePatch.background_mode !== undefined) {
      page.background_mode = pagePatch.background_mode as LayoutPage['background_mode']
    }
    if (pagePatch.gradient_angle !== undefined) page.gradient_angle = pagePatch.gradient_angle
    if (pagePatch.gradient_colors !== undefined) {
      page.gradient_colors = [...pagePatch.gradient_colors]
    }

    const removedElements = new Set(pagePatch.elements_remove ?? [])
    if (removedElements.size) {
      page.elements = page.elements.filter((item) => !removedElements.has(item.id))
    }
    for (const element of pagePatch.elements_upsert ?? []) {
      const index = page.elements.findIndex((item) => item.id === element.id)
      if (index >= 0) page.elements[index] = clone(element)
      else page.elements.push(clone(element))
    }

    for (const id of pagePatch.blocks_remove ?? []) delete page.blocks[id]
    for (const entry of pagePatch.blocks_upsert ?? []) {
      page.blocks[entry.id] = clone(entry.block)
    }
  }
  return next
}

/**
 * Menggabungkan perubahan lokal yang belum tersimpan ke layout server terbaru.
 * Perubahan lokal menang pada elemen yang sama; elemen lain mengikuti server.
 */
export function mergeLocalIntoRemote(
  base: ReportLayout,
  local: ReportLayout,
  remote: ReportLayout,
): ReportLayout {
  const localPatch = diffLayout(base, local, 0)
  if (!localPatch) return remote
  // Perubahan struktur lokal (tambah/hapus halaman) belum bisa digabung per
  // elemen; pertahankan versi lokal dan kirim sebagai replace saat autosave.
  if (localPatch.replace) return local
  return applyLayoutPatch(remote, localPatch)
}

export type { LayoutElement, LayoutPagePatch, LayoutPatch }
