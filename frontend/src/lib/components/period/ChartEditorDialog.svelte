<script lang="ts">
  import { untrack } from 'svelte'
  import {
    BarChart3,
    Check,
    Hash,
    ListTree,
    Minus,
    Palette,
    Plus,
    RotateCcw,
    Tags,
    Type,
  } from 'lucide-svelte'
  import { fontOptions } from '../../fonts/catalog'
  import type { ChartSeries, LayoutElement } from '../../types'
  import { CHART_PALETTE, chartColor, createChartSvg } from '../../utils/chartSvg'
  import Modal from '../ui/Modal.svelte'
  import Button from '../ui/Button.svelte'

  type Props = {
    element: LayoutElement
    dataLocked?: boolean
    sourceLabel?: string
    onclose: () => void
    onsave: (patch: Partial<LayoutElement>) => void
  }

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

  function normalizeChartSeries(input: ChartSeries[] | undefined, count: number): ChartSeries[] {
    return (input ?? [])
      .slice(0, 6)
      .filter((item) => item && Array.isArray(item.values))
      .map((item, index) => ({
        label: (item.label ?? '').trim() || `Seri ${index + 1}`,
        values: Array.from({ length: count }, (_, valueIndex) => {
          const value = Number(item.values?.[valueIndex])
          return Number.isFinite(value) ? Math.max(0, value) : 0
        }),
        color: item.color || CHART_PALETTE[index % CHART_PALETTE.length],
      }))
  }

  function prepare(input: LayoutElement): LayoutElement {
    const copy: LayoutElement = { ...input }
    const base = Math.min(22, Math.max(10, (copy.font_size ?? 9) * 1.6))
    copy.chart_title = copy.chart_title ?? ''
    copy.chart_title_font_family = copy.chart_title_font_family || 'sans'
    copy.chart_value_font_family = copy.chart_value_font_family || 'sans'
    copy.chart_title_font_size = clampNumber(
      copy.chart_title_font_size && copy.chart_title_font_size > 0
        ? copy.chart_title_font_size
        : roundedSize((base + 4) / 1.6),
      5,
      48,
    )
    copy.chart_label_font_size = clampNumber(
      copy.chart_label_font_size && copy.chart_label_font_size > 0
        ? copy.chart_label_font_size
        : roundedSize((base - 2) / 1.6),
      5,
      32,
    )
    copy.chart_value_font_size = clampNumber(
      copy.chart_value_font_size && copy.chart_value_font_size > 0
        ? copy.chart_value_font_size
        : roundedSize((base - 2) / 1.6),
      5,
      32,
    )
    copy.chart_legend_font_size = clampNumber(
      copy.chart_legend_font_size && copy.chart_legend_font_size > 0
        ? copy.chart_legend_font_size
        : roundedSize((base - 3) / 1.6),
      5,
      32,
    )
    copy.chart_label_rotation = clampNumber(copy.chart_label_rotation ?? 0, -90, 90)
    copy.chart_title_wrap = Boolean(copy.chart_title_wrap)
    const labels = copy.chart_labels?.length ? [...copy.chart_labels].slice(0, 20) : ['A', 'B', 'C']
    copy.chart_labels = labels
    copy.chart_values = labels.map((_, index) => copy.chart_values?.[index] ?? 0)
    copy.chart_colors = labels.map(
      (_, index) => copy.chart_colors?.[index] ?? CHART_PALETTE[index % CHART_PALETTE.length],
    )
    copy.chart_series = normalizeChartSeries(copy.chart_series, labels.length)
    return copy
  }

  const initial = untrack(() => prepare(element))
  let draft = $state<LayoutElement>(initial)
  let previewEl = $state<HTMLDivElement | null>(null)

  const labels = $derived(draft.chart_labels ?? [])
  const values = $derived(draft.chart_values ?? [])
  const series = $derived(draft.chart_series ?? [])
  const isMultiSeries = $derived(series.length > 0)
  const dataSummary = $derived(
    isMultiSeries
      ? `${labels.length} kategori · ${series.length} seri`
      : `${labels.length} kategori`,
  )
  const isPieLike = $derived(draft.chart_type === 'pie' || draft.chart_type === 'donut')
  const isSingleColor = $derived(draft.chart_type === 'hbar' || draft.chart_type === 'line')
  const chartTypeLabel = $derived(
    draft.chart_type === 'hbar'
      ? 'Bar ke samping'
      : draft.chart_type === 'line'
        ? 'Line'
        : draft.chart_type === 'pie'
          ? 'Pie'
          : draft.chart_type === 'donut'
            ? 'Donut'
            : 'Bar',
  )

  $effect(() => {
    const node = previewEl
    if (!node) return
    const svg = createChartSvg(document, draft)
    node.replaceChildren(svg)
  })

  function setPrompt(key: 'chart_title' | 'chart_title_font_family' | 'chart_value_font_family', value: string): void {
    draft[key] = value
  }

  function setSize(
    key: 'chart_title_font_size' | 'chart_label_font_size' | 'chart_value_font_size' | 'chart_legend_font_size',
    value: number,
    min: number,
    max: number,
  ): void {
    if (!Number.isFinite(value)) return
    draft[key] = roundedSize(clampNumber(value, min, max))
  }

  function setRotation(value: number): void {
    if (!Number.isFinite(value)) return
    draft.chart_label_rotation = Math.round(clampNumber(value, -90, 90))
  }

  function setLabel(index: number, value: string): void {
    draft.chart_labels = (draft.chart_labels ?? []).map((item, itemIndex) =>
      itemIndex === index ? value : item,
    )
  }

  function setValue(index: number, value: number): void {
    const next = Math.max(0, Number.isFinite(value) ? value : 0)
    draft.chart_values = (draft.chart_values ?? []).map((item, itemIndex) =>
      itemIndex === index ? next : item,
    )
  }

  function setColor(index: number, value: string): void {
    const colors = [...(draft.chart_colors ?? [])]
    colors[index] = value
    draft.chart_colors = colors
  }

  function setUniformColor(value: string): void {
    draft.chart_colors = (draft.chart_labels ?? []).map(() => value)
  }

  function syncAggregate(next = draft.chart_series ?? []): void {
    const count = draft.chart_labels?.length ?? 0
    draft.chart_values = Array.from({ length: count }, (_, index) =>
      next.reduce((sum, item) => sum + (item.values[index] ?? 0), 0),
    )
  }

  function addLabel(): void {
    if (dataLocked) return
    const nextLabels = [...(draft.chart_labels ?? [])]
    if (nextLabels.length >= 20) return
    nextLabels.push(`Kategori ${nextLabels.length + 1}`)
    draft.chart_labels = nextLabels
    draft.chart_colors = nextLabels.map(
      (_, index) => draft.chart_colors?.[index] ?? CHART_PALETTE[index % CHART_PALETTE.length],
    )
    if (isMultiSeries) {
      draft.chart_series = (draft.chart_series ?? []).map((item) => ({
        ...item,
        values: [...item.values, 0],
      }))
      syncAggregate()
      return
    }
    draft.chart_values = [...(draft.chart_values ?? []), 0]
  }

  function removeLabel(index: number): void {
    if (dataLocked || (draft.chart_labels?.length ?? 0) <= 1) return
    draft.chart_labels = (draft.chart_labels ?? []).filter((_, itemIndex) => itemIndex !== index)
    draft.chart_colors = (draft.chart_colors ?? []).filter((_, itemIndex) => itemIndex !== index)
    if (isMultiSeries) {
      draft.chart_series = (draft.chart_series ?? []).map((item) => ({
        ...item,
        values: item.values.filter((_, itemIndex) => itemIndex !== index),
      }))
      syncAggregate()
      return
    }
    draft.chart_values = (draft.chart_values ?? []).filter((_, itemIndex) => itemIndex !== index)
  }

  function setChartSeriesLabel(seriesIndex: number, value: string): void {
    draft.chart_series = (draft.chart_series ?? []).map((item, itemIndex) =>
      itemIndex === seriesIndex ? { ...item, label: value } : item,
    )
  }

  function setChartSeriesColor(seriesIndex: number, value: string): void {
    draft.chart_series = (draft.chart_series ?? []).map((item, itemIndex) =>
      itemIndex === seriesIndex ? { ...item, color: value } : item,
    )
  }

  function setChartSeriesValue(seriesIndex: number, valueIndex: number, value: number): void {
    const next = Math.max(0, Number.isFinite(value) ? value : 0)
    draft.chart_series = (draft.chart_series ?? []).map((item, itemIndex) => {
      if (itemIndex !== seriesIndex) return item
      const seriesValues = [...item.values]
      seriesValues[valueIndex] = next
      return { ...item, values: seriesValues }
    })
    syncAggregate()
  }

  function addChartSeries(): void {
    if (dataLocked) return
    const current = draft.chart_series ?? []
    if (current.length >= 6) return
    const labelCount = draft.chart_labels?.length ?? 0
    draft.chart_series = [
      ...current,
      {
        label: `Seri ${current.length + 1}`,
        values: Array.from({ length: labelCount }, () => 0),
        color: CHART_PALETTE[current.length % CHART_PALETTE.length],
      },
    ]
    syncAggregate()
  }

  function removeChartSeries(seriesIndex: number): void {
    if (dataLocked || (draft.chart_series?.length ?? 0) <= 1) return
    draft.chart_series = (draft.chart_series ?? []).filter(
      (_, itemIndex) => itemIndex !== seriesIndex,
    )
    syncAggregate()
  }

  function resetStyle(): void {
    const base = Math.min(22, Math.max(10, (draft.font_size ?? 9) * 1.6))
    draft.chart_title_font_family = 'sans'
    draft.chart_title_font_size = roundedSize((base + 4) / 1.6)
    draft.chart_title_wrap = false
    draft.chart_label_font_size = roundedSize((base - 2) / 1.6)
    draft.chart_label_rotation = 0
    draft.chart_value_font_family = 'sans'
    draft.chart_value_font_size = roundedSize((base - 2) / 1.6)
    draft.chart_legend_font_size = roundedSize((base - 3) / 1.6)
    draft.chart_show_values = true
    draft.chart_show_legend = true
  }

  function save(): void {
    const patch: Partial<LayoutElement> = {
      chart_type: draft.chart_type,
      chart_title: (draft.chart_title ?? '').trim(),
      chart_title_font_family: draft.chart_title_font_family,
      chart_title_font_size: draft.chart_title_font_size,
      chart_title_wrap: draft.chart_title_wrap,
      chart_label_font_size: draft.chart_label_font_size,
      chart_label_rotation: draft.chart_label_rotation,
      chart_value_font_family: draft.chart_value_font_family,
      chart_value_font_size: draft.chart_value_font_size,
      chart_legend_font_size: draft.chart_legend_font_size,
      chart_colors: [...(draft.chart_colors ?? [])],
      chart_show_values: draft.chart_show_values !== false,
      chart_show_legend: draft.chart_show_legend !== false,
      chart_axis_color: draft.chart_axis_color,
      fill: draft.fill,
      chart_series: (draft.chart_series ?? []).map((item, index) => ({
        label: item.label.trim() || `Seri ${index + 1}`,
        values: [...item.values],
        color: item.color || CHART_PALETTE[index % CHART_PALETTE.length],
      })),
    }
    if (!dataLocked) {
      patch.chart_labels = [...(draft.chart_labels ?? [])]
      patch.chart_values = [...(draft.chart_values ?? [])]
    }
    onsave(patch)
  }

  function swatchStyle(value: string | undefined): string {
    return `background:${value || '#ffffff'}`
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
      <Button onclick={save}><Check size={15} /> Simpan grafik</Button>
    </div>
  </div>
{/snippet}

<Modal
  open={true}
  title="Editor grafik"
  description="Atur judul, label, nilai, warna, dan legend. Pratinjau di kiri memakai data terbaru dari grafik ini."
  onclose={onclose}
  width="max-w-6xl"
  footer={actions}
>
  <div class="grid gap-5 lg:grid-cols-[minmax(0,1fr)_380px]">
    <div class="self-start lg:sticky lg:top-0">
      <div class="overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
        <div class="flex items-center justify-between gap-3 border-b border-line bg-stone-50/70 px-4 py-3">
          <span class="eyebrow">Pratinjau grafik</span>
          <span class="rounded-full bg-forest-800 px-2.5 py-1 text-[10px] font-semibold text-white">{chartTypeLabel}</span>
        </div>
        <div class="bg-[radial-gradient(circle_at_1px_1px,#e7ece8_1px,transparent_0)] bg-size-[14px_14px] p-4">
          <div
            class="aspect-5/3 w-full overflow-hidden rounded-xl border border-line bg-white"
            bind:this={previewEl}
          ></div>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-2 border-t border-line px-4 py-3 text-[10px] text-ink-muted">
          <span>{dataSummary} · {dataLocked ? 'data terhubung' : 'data manual'}</span>
          <span>Kanvas 600 × 360</span>
        </div>
      </div>
      {#if dataLocked}
        <div class="mt-3 rounded-xl border border-forest-200 bg-forest-50 px-4 py-3 text-[11px] leading-relaxed text-forest-800">
          Label dan nilai mengikuti sumber <strong>{sourceLabel || 'data terhubung'}</strong>. Perubahan di sini hanya mengatur tampilan grafik.
        </div>
      {:else}
        <p class="mt-3 text-[10px] leading-relaxed text-ink-muted">
          Data manual dapat diubah pada tabel di kanan. Kategori sumbu dipakai bersama; setiap seri punya label, warna, dan nilai sendiri per kategori.
        </p>
      {/if}
    </div>

    <div class="space-y-4 lg:max-h-[64vh] lg:overflow-y-auto lg:pr-1">
      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><BarChart3 size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Jenis & data</p>
            <p class="text-[10px] text-ink-muted">Tipe grafik, kategori sumbu, dan seri data.</p>
          </div>
        </div>

        <label class="block">
          <span class="label">Jenis grafik</span>
          <select class="input py-1.5 text-xs" value={draft.chart_type ?? 'bar'}
            onchange={(event) => (draft.chart_type = (event.currentTarget as HTMLSelectElement).value as LayoutElement['chart_type'])}>
            <option value="bar">Bar</option>
            <option value="hbar">Bar ke samping</option>
            <option value="line">Line</option>
            <option value="pie">Pie</option>
            <option value="donut">Donut</option>
          </select>
        </label>

        {#if isMultiSeries}
          <p class="mt-3 rounded-xl border border-line bg-stone-50 px-3 py-2 text-[10px] leading-relaxed text-ink-muted">
            Grafik ini memakai <strong class="text-ink-soft">{series.length} seri</strong>. Seri berkelompok dirender pada jenis grafik <strong class="text-ink-soft">Bar</strong>; jenis lain memakai total per kategori.
          </p>

          <div class="mt-4 flex items-center justify-between">
            <span class="label mb-0">Kategori sumbu</span>
            {#if !dataLocked}
              <button type="button" class="btn btn-ghost btn-sm" disabled={labels.length >= 20} onclick={addLabel}>
                <Plus size={13} /> Tambah
              </button>
            {/if}
          </div>

          <div class="mt-2 space-y-1.5">
            {#each labels as label, index (index)}
              <div class="flex items-center gap-1.5">
                <input
                  class="input min-w-0 flex-1 py-1.5 text-xs"
                  value={label}
                  placeholder={`Kategori ${index + 1}`}
                  disabled={dataLocked}
                  oninput={(event) => setLabel(index, (event.currentTarget as HTMLInputElement).value)}
                />
                {#if !dataLocked}
                  <button
                    type="button"
                    class="btn-icon h-8 w-8 shrink-0"
                    aria-label="Hapus kategori"
                    title="Hapus kategori"
                    disabled={labels.length <= 1}
                    onclick={() => removeLabel(index)}
                  >
                    <Minus size={14} />
                  </button>
                {/if}
              </div>
            {/each}
          </div>

          <div class="mt-4 flex items-center justify-between">
            <span class="label mb-0">Seri data</span>
            {#if !dataLocked}
              <button type="button" class="btn btn-ghost btn-sm" disabled={series.length >= 6} onclick={addChartSeries}>
                <Plus size={13} /> Tambah seri
              </button>
            {/if}
          </div>

          <div class="mt-2 space-y-2">
            {#each series as item, seriesIndex (seriesIndex)}
              <div class="rounded-xl border border-line bg-stone-50/60 p-2.5">
                <div class="flex items-center gap-1.5">
                  <input
                    class="input min-w-0 flex-1 py-1.5 text-xs"
                    value={item.label}
                    placeholder={`Seri ${seriesIndex + 1}`}
                    disabled={dataLocked}
                    oninput={(event) => setChartSeriesLabel(seriesIndex, (event.currentTarget as HTMLInputElement).value)}
                  />
                  <label class="relative inline-flex h-8 w-8 shrink-0 cursor-pointer overflow-hidden rounded-lg border border-line" title={item.color}>
                    <span class="h-full w-full" style={swatchStyle(item.color)}></span>
                    <input
                      class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                      type="color"
                      value={item.color || CHART_PALETTE[seriesIndex % CHART_PALETTE.length]}
                      oninput={(event) => setChartSeriesColor(seriesIndex, (event.currentTarget as HTMLInputElement).value)}
                    />
                  </label>
                  {#if !dataLocked}
                    <button
                      type="button"
                      class="btn-icon h-8 w-8 shrink-0"
                      aria-label="Hapus seri"
                      title="Hapus seri"
                      disabled={series.length <= 1}
                      onclick={() => removeChartSeries(seriesIndex)}
                    >
                      <Minus size={14} />
                    </button>
                  {/if}
                </div>
                <div class="mt-2 grid grid-cols-2 gap-1.5">
                  {#each labels as label, labelIndex (labelIndex)}
                    <label class="block">
                      <span class="mb-0.5 block truncate text-[9px] text-ink-muted" title={label}>{label || `Kategori ${labelIndex + 1}`}</span>
                      <input
                        class="input py-1 text-right text-xs"
                        type="number"
                        min="0"
                        step="any"
                        value={item.values[labelIndex] ?? 0}
                        disabled={dataLocked}
                        oninput={(event) => setChartSeriesValue(seriesIndex, labelIndex, (event.currentTarget as HTMLInputElement).valueAsNumber)}
                      />
                    </label>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="mt-4 flex items-center justify-between">
            <span class="label mb-0">Kategori data</span>
            {#if !dataLocked}
              <button type="button" class="btn btn-ghost btn-sm" disabled={labels.length >= 20} onclick={addLabel}>
                <Plus size={13} /> Tambah
              </button>
            {/if}
          </div>

          <div class="mt-2 space-y-1.5">
            {#each labels as label, index (index)}
              <div class="flex items-center gap-1.5">
                <input
                  class="input min-w-0 flex-1 py-1.5 text-xs"
                  value={label}
                  placeholder={`Kategori ${index + 1}`}
                  disabled={dataLocked}
                  oninput={(event) => setLabel(index, (event.currentTarget as HTMLInputElement).value)}
                />
                <input
                  class="input w-20 py-1.5 text-right text-xs"
                  type="number"
                  min="0"
                  step="any"
                  value={values[index] ?? 0}
                  disabled={dataLocked}
                  oninput={(event) => setValue(index, (event.currentTarget as HTMLInputElement).valueAsNumber)}
                />
                {#if !isSingleColor}
                  <label class="relative inline-flex h-8 w-8 shrink-0 cursor-pointer overflow-hidden rounded-lg border border-line" title={chartColor(draft, index)}>
                    <span class="h-full w-full" style={swatchStyle(chartColor(draft, index))}></span>
                    <input
                      class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                      type="color"
                      value={chartColor(draft, index)}
                      oninput={(event) => setColor(index, (event.currentTarget as HTMLInputElement).value)}
                    />
                  </label>
                {/if}
                {#if !dataLocked}
                  <button
                    type="button"
                    class="btn-icon h-8 w-8 shrink-0"
                    aria-label="Hapus kategori"
                    title="Hapus kategori"
                    disabled={labels.length <= 1}
                    onclick={() => removeLabel(index)}
                  >
                    <Minus size={14} />
                  </button>
                {/if}
              </div>
            {/each}
          </div>

          {#if isSingleColor}
            <div class="mt-3 flex items-center justify-between rounded-xl border border-line bg-stone-50 px-3 py-2.5">
              <span class="text-[11px] font-medium text-ink-soft">Warna {draft.chart_type === 'line' ? 'garis' : 'batang'}</span>
              <label class="relative inline-flex h-8 w-8 cursor-pointer overflow-hidden rounded-lg border border-line" title={chartColor(draft, 0)}>
                <span class="h-full w-full" style={swatchStyle(chartColor(draft, 0))}></span>
                <input
                  class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                  type="color"
                  value={chartColor(draft, 0)}
                  oninput={(event) => setUniformColor((event.currentTarget as HTMLInputElement).value)}
                />
              </label>
            </div>
          {/if}
        {/if}
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Type size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Judul</p>
            <p class="text-[10px] text-ink-muted">Font, ukuran, dan wrap judul grafik.</p>
          </div>
        </div>

        <label class="block">
          <span class="label">Teks judul</span>
          <textarea
            class="textarea text-xs"
            rows="2"
            maxlength="200"
            value={draft.chart_title ?? ''}
            oninput={(event) => setPrompt('chart_title', (event.currentTarget as HTMLTextAreaElement).value)}
          ></textarea>
        </label>

        <div class="mt-3 grid grid-cols-[minmax(0,1fr)_84px] gap-2">
          <label class="block">
            <span class="label">Font judul</span>
            <select
              class="input py-1.5 text-xs"
              value={draft.chart_title_font_family ?? 'sans'}
              onchange={(event) => setPrompt('chart_title_font_family', (event.currentTarget as HTMLSelectElement).value)}
            >
              {@render fontOptionsMarkup()}
            </select>
          </label>
          <label class="block">
            <span class="label">Ukuran (pt)</span>
            <input
              class="input py-1.5 text-xs"
              type="number"
              min="5"
              max="48"
              step="0.5"
              value={draft.chart_title_font_size ?? 11.5}
              oninput={(event) => setSize('chart_title_font_size', (event.currentTarget as HTMLInputElement).valueAsNumber, 5, 48)}
            />
          </label>
        </div>

        <label class="mt-3 flex items-center gap-2 text-xs text-ink-soft">
          <input
            type="checkbox"
            class="accent-forest-700"
            checked={Boolean(draft.chart_title_wrap)}
            onchange={(event) => (draft.chart_title_wrap = (event.currentTarget as HTMLInputElement).checked)}
          />
          Wrap judul otomatis (maksimal 3 baris)
        </label>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><Tags size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Label kategori</p>
            <p class="text-[10px] text-ink-muted">Ukuran dan sudut teks label sumbu.</p>
          </div>
        </div>

        {#if isPieLike}
          <p class="rounded-xl border border-line bg-stone-50 px-3 py-2.5 text-[11px] leading-relaxed text-ink-muted">
            Grafik pie/donut tidak memakai label sumbu. Ukuran teks label kategori diatur lewat <strong class="text-ink-soft">ukuran teks legend</strong> di bagian Legend.
          </p>
        {:else}
          <div class="grid grid-cols-[minmax(0,1fr)_84px] gap-2">
            <label class="block">
              <span class="label">Sudut label</span>
              <input
                class="w-full accent-forest-700"
                type="range"
                min="-90"
                max="90"
                step="5"
                value={draft.chart_label_rotation ?? 0}
                oninput={(event) => setRotation((event.currentTarget as HTMLInputElement).valueAsNumber)}
              />
            </label>
            <label class="block">
              <span class="label">Ukuran (pt)</span>
              <input
                class="input py-1.5 text-xs"
                type="number"
                min="5"
                max="32"
                step="0.5"
                value={draft.chart_label_font_size ?? 7.8}
                oninput={(event) => setSize('chart_label_font_size', (event.currentTarget as HTMLInputElement).valueAsNumber, 5, 32)}
              />
            </label>
          </div>
          <div class="mt-2 flex items-center justify-between text-[10px] text-ink-muted">
            <span>Putar label agar tidak saling bertumpuk.</span>
            <span class="font-mono">{Math.round(draft.chart_label_rotation ?? 0)}°</span>
          </div>
        {/if}
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Hash size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Nilai</p>
            <p class="text-[10px] text-ink-muted">Font dan ukuran angka pada grafik.</p>
          </div>
        </div>

        <label class="mb-3 flex items-center gap-2 text-xs text-ink-soft">
          <input
            type="checkbox"
            class="accent-forest-700"
            checked={draft.chart_show_values !== false}
            onchange={(event) => (draft.chart_show_values = (event.currentTarget as HTMLInputElement).checked)}
          />
          Tampilkan nilai pada grafik
        </label>

        {#if isMultiSeries && series.length > 2}
          <p class="mb-3 text-[10px] leading-relaxed text-ink-muted">
            Nilai per batang hanya dirender bila seri maksimal 2 agar tidak bertumpuk.
          </p>
        {/if}

        <div class="grid grid-cols-[minmax(0,1fr)_84px] gap-2">
          <label class="block">
            <span class="label">Font nilai</span>
            <select
              class="input py-1.5 text-xs"
              value={draft.chart_value_font_family ?? 'sans'}
              onchange={(event) => setPrompt('chart_value_font_family', (event.currentTarget as HTMLSelectElement).value)}
            >
              {@render fontOptionsMarkup()}
            </select>
          </label>
          <label class="block">
            <span class="label">Ukuran (pt)</span>
            <input
              class="input py-1.5 text-xs"
              type="number"
              min="5"
              max="32"
              step="0.5"
              value={draft.chart_value_font_size ?? 7.8}
              oninput={(event) => setSize('chart_value_font_size', (event.currentTarget as HTMLInputElement).valueAsNumber, 5, 32)}
            />
          </label>
        </div>
      </section>

      {#if isPieLike || isMultiSeries}
        <section class="rounded-2xl border border-line bg-white p-4">
          <div class="mb-3 flex items-center justify-between gap-3">
            <div class="flex items-center gap-2.5">
              <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><ListTree size={15} /></span>
              <div>
                <p class="text-xs font-semibold text-ink">Legend</p>
                <p class="text-[10px] text-ink-muted">
                  {isPieLike ? 'Tampilkan keterangan seri di samping grafik.' : 'Tampilkan keterangan seri di atas grafik.'}
                </p>
              </div>
            </div>
            <input
              type="checkbox"
              class="h-4 w-4 accent-forest-700"
              aria-label="Tampilkan legend"
              checked={draft.chart_show_legend !== false}
              onchange={(event) => (draft.chart_show_legend = (event.currentTarget as HTMLInputElement).checked)}
            />
          </div>
          {#if draft.chart_show_legend !== false}
            <label class="block">
              <span class="label">Ukuran teks legend (pt)</span>
              <input
                class="input py-1.5 text-xs"
                type="number"
                min="5"
                max="32"
                step="0.5"
                value={draft.chart_legend_font_size ?? 7.1}
                oninput={(event) => setSize('chart_legend_font_size', (event.currentTarget as HTMLInputElement).valueAsNumber, 5, 32)}
              />
            </label>
            <p class="mt-2 text-[10px] leading-relaxed text-ink-muted">Teks item legend mengikuti label {isMultiSeries ? 'seri' : 'kategori'} dan warnanya mengikuti warna {isMultiSeries ? 'seri' : 'kategori'}.</p>
          {/if}
        </section>
      {/if}

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Palette size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Tampilan</p>
            <p class="text-[10px] text-ink-muted">Warna sumbu dan latar grafik.</p>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div class="block">
            <span class="label">Warna sumbu</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.chart_axis_color ?? '#6d7872'}>
                <span class="h-full w-full" style={swatchStyle(draft.chart_axis_color ?? '#6d7872')}></span>
                <input
                  class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                  type="color"
                  value={draft.chart_axis_color ?? '#6d7872'}
                  oninput={(event) => (draft.chart_axis_color = (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.chart_axis_color ?? '#6d7872'}</span>
            </span>
          </div>
          <div class="block">
            <span class="label">Latar grafik</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.fill ?? '#ffffff'}>
                <span class="h-full w-full" style={swatchStyle(draft.fill ?? '#ffffff')}></span>
                <input
                  class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                  type="color"
                  value={draft.fill ?? '#ffffff'}
                  oninput={(event) => (draft.fill = (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.fill ?? '#ffffff'}</span>
            </span>
          </div>
        </div>
      </section>
    </div>
  </div>
</Modal>
