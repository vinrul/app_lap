<script lang="ts">
  import { untrack } from 'svelte'
  import { AlignLeft, Check, Palette, RotateCcw, Table2, Type } from 'lucide-svelte'
  import { fontOptions } from '../../fonts/catalog'
  import type { LayoutElement } from '../../types'
  import { createTableNode } from '../../utils/tableNode'
  import Modal from '../ui/Modal.svelte'
  import Button from '../ui/Button.svelte'

  type Props = {
    element: LayoutElement
    dataLocked?: boolean
    sourceLabel?: string
    onclose: () => void
    onsave: (patch: Partial<LayoutElement>) => void
  }

  type Align = 'left' | 'center' | 'right' | ''

  let {
    element,
    dataLocked = false,
    sourceLabel = '',
    onclose,
    onsave,
  }: Props = $props()

  const systemFonts = fontOptions.filter((font) => font.category === 'Sistem')
  const sansFonts = fontOptions.filter((font) => font.category === 'Sans')
  const serifFonts = fontOptions.filter((font) => font.category === 'Serif')

  function clampNumber(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, Number.isFinite(value) ? value : min))
  }

  function roundedSize(value: number): number {
    return Math.round(value * 10) / 10
  }

  function resizeStrings(values: string[] | undefined, count: number): string[] {
    return Array.from({ length: count }, (_, index) => values?.[index] ?? '')
  }

  function resizeNumbers(values: number[] | undefined, count: number): number[] {
    return Array.from({ length: count }, (_, index) => values?.[index] ?? 0)
  }

  function resizeAligns(values: Align[] | undefined, count: number): Align[] {
    return Array.from({ length: count }, (_, index) => {
      const value = values?.[index]
      return value === 'left' || value === 'center' || value === 'right' ? value : ''
    })
  }

  function columnCountOf(rows: string[][]): number {
    return Math.max(1, ...rows.map((row) => row.length))
  }

  function prepare(input: LayoutElement): LayoutElement {
    const copy: LayoutElement = {
      ...input,
      table_rows: (input.table_rows ?? []).map((row) => [...row]),
    }
    if (!copy.table_rows?.length) {
      copy.table_rows = dataLocked
        ? []
        : [
            ['Kolom 1', 'Kolom 2', 'Kolom 3'],
            ['', '', ''],
            ['', '', ''],
          ]
    }
    const columns = columnCountOf(copy.table_rows ?? [])
    copy.table_rows = (copy.table_rows ?? []).map((row) =>
      Array.from({ length: columns }, (_, index) => row[index] ?? ''),
    )
    copy.font_family = copy.font_family || 'sans'
    copy.table_header_font_family = copy.table_header_font_family || copy.font_family || 'sans'
    copy.table_column_backgrounds = resizeStrings(copy.table_column_backgrounds, columns)
    copy.table_column_colors = resizeStrings(copy.table_column_colors, columns)
    copy.table_column_font_families = resizeStrings(copy.table_column_font_families, columns)
    copy.table_column_font_sizes = resizeNumbers(copy.table_column_font_sizes, columns)
    copy.table_column_aligns = resizeAligns(copy.table_column_aligns, columns)
    return copy
  }

  const initial = untrack(() => prepare(element))
  let draft = $state<LayoutElement>(initial)
  let previewEl = $state<HTMLDivElement | null>(null)

  const rows = $derived(draft.table_rows ?? [])
  const columns = $derived(columnCountOf(rows))
  const baseSize = $derived(draft.font_size ?? 9)
  const stripeEnabled = $derived(
    Boolean(draft.table_stripe) && draft.table_stripe !== 'transparent',
  )
  const stripeColor = $derived(
    stripeEnabled ? (draft.table_stripe ?? '#f6f8f5') : '#f6f8f5',
  )
  const previewRatio = $derived(
    `${Math.max(20, draft.width || 120)} / ${Math.max(12, draft.height || 60)}`,
  )

  $effect(() => {
    const node = previewEl
    if (!node) return
    const table = createTableNode(document, draft)
    table.style.width = '100%'
    table.style.height = '100%'
    node.replaceChildren(table)
  })

  function swatchStyle(value: string | undefined): string {
    return `background:${value || '#ffffff'}`
  }

  function columnName(index: number): string {
    const label = rows[0]?.[index]?.trim()
    return label || `Kolom ${index + 1}`
  }

  function setCell(rowIndex: number, columnIndex: number, value: string): void {
    if (dataLocked) return
    draft.table_rows = rows.map((row, currentRow) => {
      if (currentRow !== rowIndex) return row
      const next = Array.from({ length: columns }, (_, index) => row[index] ?? '')
      next[columnIndex] = value
      return next
    })
  }

  function syncColumnArrays(count: number): void {
    draft.table_column_backgrounds = resizeStrings(draft.table_column_backgrounds, count)
    draft.table_column_colors = resizeStrings(draft.table_column_colors, count)
    draft.table_column_font_families = resizeStrings(
      draft.table_column_font_families,
      count,
    )
    draft.table_column_font_sizes = resizeNumbers(draft.table_column_font_sizes, count)
    draft.table_column_aligns = resizeAligns(draft.table_column_aligns, count)
  }

  function addRow(): void {
    if (dataLocked) return
    draft.table_rows = [...rows, Array.from({ length: columns }, () => '')]
  }

  function removeRow(): void {
    if (dataLocked || rows.length <= 1) return
    draft.table_rows = rows.slice(0, -1)
  }

  function addColumn(): void {
    if (dataLocked || columns >= 12) return
    draft.table_rows = rows.map((row) => [...row, ''])
    syncColumnArrays(columns + 1)
  }

  function removeColumn(): void {
    if (dataLocked || columns <= 1) return
    draft.table_rows = rows.map((row) => row.slice(0, -1))
    syncColumnArrays(columns - 1)
  }

  function setColumnValue(
    key: 'table_column_backgrounds' | 'table_column_colors' | 'table_column_font_families',
    index: number,
    value: string,
  ): void {
    const list = resizeStrings(draft[key], columns)
    list[index] = value
    draft[key] = list
  }

  function setColumnSize(index: number, value: number): void {
    const list = resizeNumbers(draft.table_column_font_sizes, columns)
    list[index] = Number.isFinite(value) ? roundedSize(clampNumber(value, 5, 72)) : 0
    draft.table_column_font_sizes = list
  }

  function clearColumnSize(index: number): void {
    const list = resizeNumbers(draft.table_column_font_sizes, columns)
    list[index] = 0
    draft.table_column_font_sizes = list
  }

  function setColumnAlign(index: number, value: string): void {
    const list = resizeAligns(draft.table_column_aligns, columns)
    list[index] = (value === 'left' || value === 'center' || value === 'right'
      ? value
      : '') as Align
    draft.table_column_aligns = list
  }

  function toggleColumnBackground(index: number, enabled: boolean): void {
    const preset =
      draft.table_stripe && draft.table_stripe !== 'transparent'
        ? draft.table_stripe
        : '#eef1ec'
    setColumnValue('table_column_backgrounds', index, enabled ? preset : '')
  }

  function toggleColumnColor(index: number, enabled: boolean): void {
    setColumnValue(
      'table_column_colors',
      index,
      enabled ? (draft.color || '#143d30') : '',
    )
  }

  function setBaseSize(value: number): void {
    if (!Number.isFinite(value)) return
    draft.font_size = roundedSize(clampNumber(value, 5, 72))
  }

  function setHeaderSize(value: number): void {
    if (!Number.isFinite(value)) return
    draft.table_header_font_size = roundedSize(clampNumber(value, 5, 72))
  }

  function setStripeEnabled(enabled: boolean): void {
    draft.table_stripe = enabled ? '#f6f8f5' : 'transparent'
  }

  function resetStyle(): void {
    draft.font_family = 'sans'
    draft.font_size = 9
    draft.color = '#143d30'
    draft.table_header_bg = '#143d30'
    draft.table_header_color = '#ffffff'
    draft.table_header_font_family = 'sans'
    draft.table_header_font_size = 0
    draft.table_stripe = '#f6f8f5'
    draft.fill = '#ffffff'
    draft.border_color = '#d7ddd6'
    draft.border_width = 0.5
    draft.table_column_backgrounds = Array.from({ length: columns }, () => '')
    draft.table_column_colors = Array.from({ length: columns }, () => '')
    draft.table_column_font_families = Array.from({ length: columns }, () => '')
    draft.table_column_font_sizes = Array.from({ length: columns }, () => 0)
    draft.table_column_aligns = Array.from({ length: columns }, () => '')
  }

  function save(): void {
    const count = columns
    const patch: Partial<LayoutElement> = {
      table_header: draft.table_header !== false,
      table_header_bg: draft.table_header_bg,
      table_header_color: draft.table_header_color,
      table_header_font_family: draft.table_header_font_family,
      table_header_font_size: draft.table_header_font_size,
      font_family: draft.font_family,
      font_size: draft.font_size,
      color: draft.color,
      table_stripe: stripeEnabled ? stripeColor : 'transparent',
      table_column_backgrounds: resizeStrings(draft.table_column_backgrounds, count),
      table_column_colors: resizeStrings(draft.table_column_colors, count),
      table_column_font_families: resizeStrings(
        draft.table_column_font_families,
        count,
      ),
      table_column_font_sizes: resizeNumbers(draft.table_column_font_sizes, count),
      table_column_aligns: resizeAligns(draft.table_column_aligns, count),
      fill: draft.fill,
      border_color: draft.border_color,
      border_width: draft.border_width,
    }
    if (!dataLocked) {
      patch.table_rows = rows.map((row) => [...row])
    }
    onsave(patch)
  }
</script>

{#snippet fontOptionsMarkup()}
    <optgroup label="Sistem">
      {#each systemFonts as font}
        <option value={font.key}>{font.label}</option>
      {/each}
    </optgroup>
    <optgroup label="Google Fonts · Sans">
      {#each sansFonts as font}
        <option value={font.key}>{font.label}</option>
      {/each}
    </optgroup>
    <optgroup label="Google Fonts · Serif">
      {#each serifFonts as font}
        <option value={font.key}>{font.label}</option>
      {/each}
    </optgroup>
  {/snippet}

  {#snippet actions()}
    <div class="flex w-full flex-wrap items-center justify-between gap-2">
      <Button variant="ghost" size="sm" onclick={resetStyle}>
        <RotateCcw size={14} /> Reset gaya
      </Button>
      <div class="flex flex-wrap items-center gap-2">
        <Button variant="ghost" onclick={onclose}>Batal</Button>
        <Button onclick={save}><Check size={15} /> Simpan tabel</Button>
      </div>
    </div>
  {/snippet}

<Modal
  open={true}
  title="Editor tabel"
  description="Atur isi tabel, header, font, dan warna per kolom. Pratinjau di kiri mengikuti perubahan secara langsung."
  onclose={onclose}
  width="max-w-6xl"
  footer={actions}
>
  <div class="grid gap-5 lg:grid-cols-[minmax(0,1fr)_440px]">
    <div class="self-start lg:sticky lg:top-0">
      <div class="overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
        <div class="flex items-center justify-between gap-3 border-b border-line bg-stone-50/70 px-4 py-3">
          <span class="eyebrow">Pratinjau tabel</span>
          <span class="rounded-full bg-forest-800 px-2.5 py-1 text-[10px] font-semibold text-white">{columns} kolom · {rows.length} baris</span>
        </div>
        <div class="bg-[radial-gradient(circle_at_1px_1px,#e7ece8_1px,transparent_0)] bg-size-[14px_14px] p-4">
          <div
            class="mx-auto max-h-105 min-h-55 w-full max-w-105 overflow-hidden rounded-lg border border-line bg-white"
            style={`aspect-ratio:${previewRatio}`}
            bind:this={previewEl}
          ></div>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-2 border-t border-line px-4 py-3 text-[10px] text-ink-muted">
          <span>{dataLocked ? 'Data mengikuti sumber' : 'Data manual'}</span>
          <span>{Math.round(draft.width || 120)} × {Math.round(draft.height || 60)} mm</span>
        </div>
      </div>
      {#if dataLocked}
        <div class="mt-3 rounded-xl border border-forest-200 bg-forest-50 px-4 py-3 text-[11px] leading-relaxed text-forest-800">
          Isi tabel mengikuti sumber <strong>{sourceLabel || 'data terhubung'}</strong>. Perubahan di sini hanya mengatur tampilan.
        </div>
      {/if}
    </div>

    <div class="space-y-4 lg:max-h-[64vh] lg:overflow-y-auto lg:pr-1">
      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><Table2 size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Data tabel</p>
            <p class="text-[10px] text-ink-muted">Baris, kolom, dan header tabel.</p>
          </div>
        </div>

        <label class="flex items-center gap-2 text-xs text-ink-soft">
          <input
            type="checkbox"
            class="accent-forest-700"
            checked={draft.table_header !== false}
            onchange={(event) => (draft.table_header = (event.currentTarget as HTMLInputElement).checked)}
          />
          Baris pertama sebagai header
        </label>

        {#if dataLocked}
          <p class="mt-3 rounded-xl border border-line bg-stone-50 px-3 py-2.5 text-[11px] text-ink-muted">
            Isi sel tidak dapat diubah karena tabel terhubung ke sumber data.
          </p>
        {:else}
          <div class="mt-3 flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" onclick={addRow}>+ Baris</Button>
            <Button variant="secondary" size="sm" onclick={removeRow}>− Baris</Button>
            <Button variant="secondary" size="sm" onclick={addColumn} disabled={columns >= 12}>+ Kolom</Button>
            <Button variant="secondary" size="sm" onclick={removeColumn} disabled={columns <= 1}>− Kolom</Button>
          </div>
          <div class="mt-3 max-h-56 overflow-auto rounded-lg border border-line">
            <div class="min-w-max">
              {#each rows as row, rowIndex (rowIndex)}
                <div class="flex">
                  {#each Array(columns) as _, columnIndex (columnIndex)}
                    <input
                      class="input w-28 rounded-none border-0 border-r border-b border-line px-2 py-1 text-[11px]"
                      value={row[columnIndex] ?? ''}
                      placeholder={`Kolom ${columnIndex + 1}`}
                      oninput={(event) => setCell(rowIndex, columnIndex, (event.currentTarget as HTMLInputElement).value)}
                    />
                  {/each}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Type size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Header</p>
            <p class="text-[10px] text-ink-muted">Warna dan tipografi baris header.</p>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <span class="label">Latar header</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.table_header_bg ?? '#143d30'}>
                <span class="h-full w-full" style={swatchStyle(draft.table_header_bg ?? '#143d30')}></span>
                <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                  value={draft.table_header_bg ?? '#143d30'}
                  oninput={(event) => (draft.table_header_bg = (event.currentTarget as HTMLInputElement).value)} />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.table_header_bg ?? '#143d30'}</span>
            </span>
          </div>
          <div>
            <span class="label">Teks header</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.table_header_color ?? '#ffffff'}>
                <span class="h-full w-full" style={swatchStyle(draft.table_header_color ?? '#ffffff')}></span>
                <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                  value={draft.table_header_color ?? '#ffffff'}
                  oninput={(event) => (draft.table_header_color = (event.currentTarget as HTMLInputElement).value)} />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.table_header_color ?? '#ffffff'}</span>
            </span>
          </div>
        </div>

        <div class="mt-3 grid grid-cols-[minmax(0,1fr)_84px] gap-2">
          <label class="block">
            <span class="label">Font header</span>
            <select class="input py-1.5 text-xs" value={draft.table_header_font_family ?? 'sans'}
              onchange={(event) => (draft.table_header_font_family = (event.currentTarget as HTMLSelectElement).value)}>
              {@render fontOptionsMarkup()}
            </select>
          </label>
          <label class="block">
            <span class="label">Ukuran (pt)</span>
            <input class="input py-1.5 text-xs" type="number" min="5" max="72" step="0.5"
              value={draft.table_header_font_size && draft.table_header_font_size > 0 ? draft.table_header_font_size : baseSize}
              oninput={(event) => setHeaderSize((event.currentTarget as HTMLInputElement).valueAsNumber)} />
          </label>
        </div>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><AlignLeft size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Badan tabel</p>
            <p class="text-[10px] text-ink-muted">Tipografi dasar dan baris ganjil.</p>
          </div>
        </div>

        <div class="grid grid-cols-[minmax(0,1fr)_84px] gap-2">
          <label class="block">
            <span class="label">Font dasar</span>
            <select class="input py-1.5 text-xs" value={draft.font_family ?? 'sans'}
              onchange={(event) => (draft.font_family = (event.currentTarget as HTMLSelectElement).value)}>
              {@render fontOptionsMarkup()}
            </select>
          </label>
          <label class="block">
            <span class="label">Ukuran (pt)</span>
            <input class="input py-1.5 text-xs" type="number" min="5" max="72" step="0.5"
              value={draft.font_size ?? 9}
              oninput={(event) => setBaseSize((event.currentTarget as HTMLInputElement).valueAsNumber)} />
          </label>
        </div>

        <div class="mt-3 grid grid-cols-2 gap-3">
          <div>
            <span class="label">Warna teks</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.color ?? '#143d30'}>
                <span class="h-full w-full" style={swatchStyle(draft.color ?? '#143d30')}></span>
                <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                  value={draft.color ?? '#143d30'}
                  oninput={(event) => (draft.color = (event.currentTarget as HTMLInputElement).value)} />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.color ?? '#143d30'}</span>
            </span>
          </div>
          <div>
            <span class="label">Baris ganjil</span>
            <div class="flex items-center gap-2">
              <input type="checkbox" class="accent-forest-700" checked={stripeEnabled}
                onchange={(event) => setStripeEnabled((event.currentTarget as HTMLInputElement).checked)} />
              {#if stripeEnabled}
                <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={stripeColor}>
                  <span class="h-full w-full" style={swatchStyle(stripeColor)}></span>
                  <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                    value={stripeColor}
                    oninput={(event) => (draft.table_stripe = (event.currentTarget as HTMLInputElement).value)} />
                </label>
                <span class="font-mono text-[10px] uppercase text-ink-muted">{stripeColor}</span>
              {/if}
            </div>
          </div>
        </div>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Palette size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Tampilan tabel</p>
            <p class="text-[10px] text-ink-muted">Latar, border, dan ketebalan garis.</p>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <span class="label">Latar tabel</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.fill ?? '#ffffff'}>
                <span class="h-full w-full" style={swatchStyle(draft.fill ?? '#ffffff')}></span>
                <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                  value={draft.fill ?? '#ffffff'}
                  oninput={(event) => (draft.fill = (event.currentTarget as HTMLInputElement).value)} />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.fill ?? '#ffffff'}</span>
            </span>
          </div>
          <div>
            <span class="label">Warna border</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.border_color ?? '#d7ddd6'}>
                <span class="h-full w-full" style={swatchStyle(draft.border_color ?? '#d7ddd6')}></span>
                <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                  value={draft.border_color ?? '#d7ddd6'}
                  oninput={(event) => (draft.border_color = (event.currentTarget as HTMLInputElement).value)} />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.border_color ?? '#d7ddd6'}</span>
            </span>
          </div>
          <label class="block">
            <span class="label">Tebal border (pt)</span>
            <input class="input py-1.5 text-xs" type="number" min="0" max="10" step="0.25"
              value={draft.border_width ?? 0.5}
              oninput={(event) => {
                const value = (event.currentTarget as HTMLInputElement).valueAsNumber
                if (Number.isFinite(value)) draft.border_width = clampNumber(value, 0, 10)
              }} />
          </label>
        </div>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3">
          <p class="text-xs font-semibold text-ink">Warna & font per kolom</p>
          <p class="mt-1 text-[10px] leading-relaxed text-ink-muted">
            Latar kolom menimpa baris ganjil. Biarkan “Ikuti dasar” agar mengikuti pengaturan badan tabel.
          </p>
        </div>

        <div class="space-y-2">
          {#each Array(columns) as _, index (index)}
            {@const columnBackground = draft.table_column_backgrounds?.[index] ?? ''}
            {@const columnColor = draft.table_column_colors?.[index] ?? ''}
            {@const columnFamily = draft.table_column_font_families?.[index] ?? ''}
            {@const columnSize = draft.table_column_font_sizes?.[index] ?? 0}
            {@const columnAlign = draft.table_column_aligns?.[index] ?? ''}
            <div class="rounded-xl border border-line bg-stone-50/50 p-3">
              <div class="flex items-center justify-between gap-2">
                <div class="min-w-0">
                  <p class="truncate text-[11px] font-semibold text-ink">{columnName(index)}</p>
                  <p class="text-[9px] text-ink-muted">Kolom {index + 1}</p>
                </div>
                <select
                  class="input w-24 py-1 text-[10px]"
                  value={columnAlign}
                  onchange={(event) => setColumnAlign(index, (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="">Otomatis</option>
                  <option value="left">Kiri</option>
                  <option value="center">Tengah</option>
                  <option value="right">Kanan</option>
                </select>
              </div>

              <div class="mt-2 grid grid-cols-[minmax(0,1fr)_64px] gap-2">
                <select
                  class="input min-w-0 py-1.5 text-[10px]"
                  value={columnFamily}
                  onchange={(event) => setColumnValue('table_column_font_families', index, (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="">Ikuti dasar</option>
                  {@render fontOptionsMarkup()}
                </select>
                <input
                  class="input py-1.5 text-right text-[10px]"
                  type="number"
                  min="5"
                  max="72"
                  step="0.5"
                  value={columnSize > 0 ? columnSize : ''}
                  placeholder={String(baseSize)}
                  oninput={(event) => setColumnSize(index, (event.currentTarget as HTMLInputElement).valueAsNumber)}
                />
              </div>

              <div class="mt-2 flex flex-wrap items-center gap-4 text-[10px] text-ink-soft">
                <label class="flex items-center gap-1.5">
                  <input type="checkbox" class="accent-forest-700" checked={Boolean(columnBackground)}
                    onchange={(event) => toggleColumnBackground(index, (event.currentTarget as HTMLInputElement).checked)} />
                  Latar
                </label>
                {#if columnBackground}
                  <label class="relative inline-flex h-7 w-8 cursor-pointer overflow-hidden rounded-md border border-line" title={columnBackground}>
                    <span class="h-full w-full" style={swatchStyle(columnBackground)}></span>
                    <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                      value={columnBackground}
                      oninput={(event) => setColumnValue('table_column_backgrounds', index, (event.currentTarget as HTMLInputElement).value)} />
                  </label>
                {/if}
                <label class="flex items-center gap-1.5">
                  <input type="checkbox" class="accent-forest-700" checked={Boolean(columnColor)}
                    onchange={(event) => toggleColumnColor(index, (event.currentTarget as HTMLInputElement).checked)} />
                  Teks
                </label>
                {#if columnColor}
                  <label class="relative inline-flex h-7 w-8 cursor-pointer overflow-hidden rounded-md border border-line" title={columnColor}>
                    <span class="h-full w-full" style={swatchStyle(columnColor)}></span>
                    <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                      value={columnColor}
                      oninput={(event) => setColumnValue('table_column_colors', index, (event.currentTarget as HTMLInputElement).value)} />
                  </label>
                {/if}
                {#if columnSize > 0}
                  <button type="button" class="text-[10px] font-semibold text-forest-700 hover:underline" onclick={() => clearColumnSize(index)}>
                    Ukuran ikut dasar
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </section>
    </div>
  </div>
</Modal>
