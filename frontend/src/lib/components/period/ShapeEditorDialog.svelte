<script lang="ts">
  import { untrack } from 'svelte'
  import { Check, Minus, Palette, Plus, RotateCcw, Shapes, SlidersHorizontal } from 'lucide-svelte'
  import { shapeCatalog, shapeDefinition } from '../../shapes/catalog'
  import type { LayoutElement } from '../../types'
  import Modal from '../ui/Modal.svelte'
  import Button from '../ui/Button.svelte'

  type Props = {
    element: LayoutElement
    onclose: () => void
    onsave: (patch: Partial<LayoutElement>) => void
  }

  type FillMode = 'solid' | 'linear' | 'radial'

  let { element, onclose, onsave }: Props = $props()

  const gradientPalette = ['#143d30', '#b8894b', '#3f765d', '#5b8fa8', '#a4743d', '#d9b87e', '#7a857f', '#2f5d7c']

  function pickerColor(value: string | undefined, fallback = '#143d30'): string {
    return value && /^#[0-9a-fA-F]{6}$/.test(value) ? value : fallback
  }

  function isTransparent(value: string | undefined | null): boolean {
    const normalized = value?.trim().toLowerCase() ?? ''
    return !normalized || normalized === 'transparent' || normalized === 'none'
  }

  function clampNumber(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, Number.isFinite(value) ? value : min))
  }

  function roundedSize(value: number): number {
    return Math.round(value * 10) / 10
  }

  function fillModeOf(value: LayoutElement): FillMode {
    return value.fill_mode === 'linear' || value.fill_mode === 'radial' ? value.fill_mode : 'solid'
  }

  function colorsOf(value: LayoutElement): string[] {
    const colors = value.gradient_colors ?? []
    if (colors.length >= 2) return colors
    return [pickerColor(value.fill, '#ffffff'), '#b8894b']
  }

  function gradientCss(value: LayoutElement): string {
    const colors = colorsOf(value)
    const mode = fillModeOf(value)
    if (mode === 'solid' || colors.length < 2) return value.fill ?? '#ffffff'
    const last = colors.length - 1
    const stops = colors
      .map((color, index) => `${color} ${((index / last) * 100).toFixed(1)}%`)
      .join(', ')
    return mode === 'radial'
      ? `radial-gradient(circle, ${stops})`
      : `linear-gradient(${value.gradient_angle ?? 90}deg, ${stops})`
  }

  function linearGradientCoords(
    angle: number,
    width: number,
    height: number,
  ): [number, number, number, number] {
    const radians = (angle * Math.PI) / 180
    let dx = Math.sin(radians) / Math.max(width, 0.001)
    let dy = -Math.cos(radians) / Math.max(height, 0.001)
    const length = Math.hypot(dx, dy)
    if (length > 0) {
      dx /= length
      dy /= length
    }
    const span = (Math.abs(dx) + Math.abs(dy)) * 0.5
    return [0.5 - dx * span, 0.5 - dy * span, 0.5 + dx * span, 0.5 + dy * span]
  }

  const initial = untrack(() => {
    const copy: LayoutElement = { ...element }
    copy.shape = shapeDefinition(copy.shape).kind
    copy.fill_mode = fillModeOf(copy)
    copy.gradient_colors = colorsOf(copy)
    copy.gradient_angle = copy.gradient_angle ?? 90
    copy.radius = copy.radius ?? 1
    copy.border_width = copy.border_width ?? 0.5
    return copy
  })
  let draft = $state<LayoutElement>(initial)
  let lastFill = $state(pickerColor(initial.fill, '#ffffff'))
  let lastBorder = $state(pickerColor(initial.border_color, '#143d30'))

  const shape = $derived(shapeDefinition(draft.shape))
  const mode = $derived(fillModeOf(draft))
  const colors = $derived(colorsOf(draft))
  const noFill = $derived(isTransparent(draft.fill))
  const noStroke = $derived(isTransparent(draft.border_color))
  const useGradient = $derived(!noFill && mode !== 'solid' && colors.length >= 2)
  const gradientId = 'shape-editor-gradient'
  const gradientCoords = $derived(
    linearGradientCoords(draft.gradient_angle ?? 90, draft.width || 60, draft.height || 40),
  )
  const fillValue = $derived(
    noFill ? 'none' : useGradient ? `url(#${gradientId})` : (draft.fill ?? '#ffffff'),
  )
  const strokeWidth = $derived(
    (draft.border_width ?? 0.5) > 0 && !noStroke ? (draft.border_width ?? 0.5) : 0,
  )
  const strokeColor = $derived(noStroke ? 'none' : (draft.border_color ?? '#143d30'))
  const previewRatio = $derived(
    `${Math.max(20, draft.width || 80)} / ${Math.max(12, draft.height || 48)}`,
  )
  const rectStyle = $derived(
    [
      `background:${noFill ? 'transparent' : useGradient ? gradientCss(draft) : (draft.fill ?? '#ffffff')}`,
      `border:${strokeWidth > 0 ? `${strokeWidth}px solid ${strokeColor}` : 'none'}`,
      `border-radius:${Math.min(60, Math.max(0, (draft.radius ?? 1) * 6))}px`,
    ].join(';'),
  )

  function setShape(kind: string): void {
    draft.shape = kind
  }

  function toggleFill(transparent: boolean): void {
    if (transparent) {
      if (!isTransparent(draft.fill)) lastFill = pickerColor(draft.fill, '#ffffff')
      draft.fill = 'transparent'
    } else {
      draft.fill = lastFill
    }
  }

  function toggleStroke(transparent: boolean): void {
    if (transparent) {
      if (!isTransparent(draft.border_color)) lastBorder = pickerColor(draft.border_color, '#143d30')
      draft.border_color = 'transparent'
    } else {
      draft.border_color = lastBorder
      draft.border_width = (draft.border_width ?? 0) > 0 ? draft.border_width : 0.5
    }
  }

  function setFillMode(value: string): void {
    draft.fill_mode = value === 'linear' || value === 'radial' ? value : 'solid'
    if (draft.fill_mode !== 'solid') draft.gradient_colors = colorsOf(draft)
  }

  function setFillColor(value: string): void {
    draft.fill = value
  }

  function setGradientColor(index: number, value: string): void {
    const next = [...colorsOf(draft)]
    next[index] = value
    draft.gradient_colors = next
  }

  function addGradientColor(): void {
    const next = colorsOf(draft)
    if (next.length >= 8) return
    draft.gradient_colors = [...next, gradientPalette[next.length % gradientPalette.length]]
  }

  function removeGradientColor(index: number): void {
    const next = colorsOf(draft)
    if (next.length <= 2) return
    draft.gradient_colors = next.filter((_, itemIndex) => itemIndex !== index)
  }

  function setBorderColor(value: string): void {
    draft.border_color = value
  }

  function setBorderWidth(value: number): void {
    if (!Number.isFinite(value)) return
    draft.border_width = roundedSize(clampNumber(value, 0, 24))
  }

  function setRadius(value: number): void {
    if (!Number.isFinite(value)) return
    draft.radius = roundedSize(clampNumber(value, 0, 200))
  }

  function setGradientAngle(value: number): void {
    if (!Number.isFinite(value)) return
    draft.gradient_angle = Math.round(clampNumber(value, 0, 360))
  }

  function resetStyle(): void {
    draft.fill = '#ffffff'
    draft.fill_mode = 'solid'
    draft.gradient_colors = []
    draft.gradient_angle = 90
    draft.border_color = '#143d30'
    draft.border_width = 0.5
    draft.radius = 1
    lastFill = '#ffffff'
    lastBorder = '#143d30'
  }

  function save(): void {
    onsave({
      shape: draft.shape,
      fill: isTransparent(draft.fill) ? 'transparent' : draft.fill,
      fill_mode: fillModeOf(draft),
      gradient_colors: colorsOf(draft),
      gradient_angle: draft.gradient_angle ?? 90,
      border_color: isTransparent(draft.border_color) ? 'transparent' : draft.border_color,
      border_width: draft.border_width ?? 0.5,
      radius: draft.radius ?? 1,
    })
  }
</script>

{#snippet actions()}
  <div class="flex w-full flex-wrap items-center justify-between gap-2">
    <Button variant="ghost" size="sm" onclick={resetStyle}>
      <RotateCcw size={14} /> Reset gaya
    </Button>
    <div class="flex flex-wrap items-center gap-2">
      <Button variant="ghost" onclick={onclose}>Batal</Button>
      <Button onclick={save}><Check size={15} /> Simpan bentuk</Button>
    </div>
  </div>
{/snippet}

<Modal
  open={true}
  title="Editor bentuk"
  description="Atur jenis bentuk, isi atau gradien, garis tepi, dan sudut. Pratinjau di kiri mengikuti ukuran objek di halaman."
  onclose={onclose}
  width="max-w-5xl"
  footer={actions}
>
  <div class="grid gap-5 lg:grid-cols-[minmax(0,1fr)_360px]">
    <div class="self-start lg:sticky lg:top-0">
      <div class="overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
        <div class="flex items-center justify-between gap-3 border-b border-line bg-stone-50/70 px-4 py-3">
          <span class="eyebrow">Pratinjau bentuk</span>
          <span class="rounded-full bg-forest-800 px-2.5 py-1 text-[10px] font-semibold text-white">{shape.label}</span>
        </div>
        <div class="bg-[radial-gradient(circle_at_1px_1px,#e7ece8_1px,transparent_0)] bg-size-[14px_14px] p-4">
          <div
            class="w-full overflow-hidden rounded-xl border border-line bg-white"
            style={`aspect-ratio:${previewRatio}`}
          >
            {#if shape.kind === 'rect'}
              <div class="h-full w-full" style={rectStyle}></div>
            {:else}
              <svg
                class="h-full w-full"
                viewBox="0 0 100 100"
                preserveAspectRatio="none"
                aria-hidden="true"
              >
                {#if useGradient}
                  <defs>
                    {#if mode === 'radial'}
                      <radialGradient id={gradientId} cx="50%" cy="50%" r="50%">
                        {#each colors as color, index (index)}
                          <stop
                            offset={`${((index / Math.max(1, colors.length - 1)) * 100).toFixed(1)}%`}
                            stop-color={color}
                          />
                        {/each}
                      </radialGradient>
                    {:else}
                      <linearGradient
                        id={gradientId}
                        x1={gradientCoords[0]}
                        y1={gradientCoords[1]}
                        x2={gradientCoords[2]}
                        y2={gradientCoords[3]}
                      >
                        {#each colors as color, index (index)}
                          <stop
                            offset={`${((index / Math.max(1, colors.length - 1)) * 100).toFixed(1)}%`}
                            stop-color={color}
                          />
                        {/each}
                      </linearGradient>
                    {/if}
                  </defs>
                {/if}
                {#if shape.ellipse}
                  <ellipse
                    cx="50"
                    cy="50"
                    rx="50"
                    ry="50"
                    fill={fillValue}
                    stroke={strokeColor}
                    stroke-width={strokeWidth}
                    vector-effect="non-scaling-stroke"
                    stroke-linejoin="round"
                  />
                {:else if shape.path}
                  <path
                    d={shape.path}
                    fill={fillValue}
                    stroke={strokeColor}
                    stroke-width={strokeWidth}
                    vector-effect="non-scaling-stroke"
                    stroke-linejoin="round"
                  />
                {:else}
                  <polygon
                    points={shape.points ?? ''}
                    fill={fillValue}
                    stroke={strokeColor}
                    stroke-width={strokeWidth}
                    vector-effect="non-scaling-stroke"
                    stroke-linejoin="round"
                  />
                {/if}
              </svg>
            {/if}
          </div>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-2 border-t border-line px-4 py-3 text-[10px] text-ink-muted">
          <span>{Math.round(draft.width || 0)} × {Math.round(draft.height || 0)} mm</span>
          <span>{mode === 'solid' ? 'Isi solid' : mode === 'linear' ? 'Gradien linear' : 'Gradien radial'}</span>
        </div>
      </div>
      <p class="mt-3 text-[10px] leading-relaxed text-ink-muted">
        Bentuk non-kotak dirender sebagai SVG sehingga gradien dan garis mengikuti kontur bentuk.
      </p>
    </div>

    <div class="space-y-4 lg:max-h-[64vh] lg:overflow-y-auto lg:pr-1">
      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><Shapes size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Jenis bentuk</p>
            <p class="text-[10px] text-ink-muted">Pilih kontur yang dipakai objek.</p>
          </div>
        </div>
        <div class="grid grid-cols-4 gap-1.5">
          {#each shapeCatalog as option (option.kind)}
            <button
              type="button"
              class="flex min-w-0 flex-col items-center gap-1 rounded-lg border px-1 py-1.5 hover:border-gold-300 hover:bg-gold-50 {shape.kind === option.kind ? 'border-gold-300 bg-gold-50 text-forest-800' : 'border-line text-ink-muted'}"
              title={option.label}
              onclick={() => setShape(option.kind)}
            >
              <svg class="h-4 w-4" viewBox="0 0 100 100" fill="none" stroke="currentColor"
                stroke-width="6" stroke-linejoin="round" aria-hidden="true">
                {#if option.kind === 'rect'}
                  <rect x="4" y="16" width="92" height="68" rx="5" />
                {:else if option.ellipse}
                  <ellipse cx="50" cy="50" rx="46" ry="46" />
                {:else if option.path}
                  <path d={option.path} />
                {:else}
                  <polygon points={option.points} />
                {/if}
              </svg>
              <span class="w-full truncate text-center text-[8px] leading-tight">{option.label}</span>
            </button>
          {/each}
        </div>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Palette size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Isi &amp; gradien</p>
            <p class="text-[10px] text-ink-muted">Warna solid atau campuran 2–8 warna.</p>
          </div>
        </div>
        <label class="flex items-center gap-2 text-xs font-medium text-ink">
          <input
            type="checkbox"
            checked={noFill}
            onchange={(event) => toggleFill((event.currentTarget as HTMLInputElement).checked)}
          />
          Tanpa isi
        </label>
        <label class="mt-3 block">
          <span class="label">Tipe isi</span>
          <select
            class="input py-1.5 text-xs disabled:opacity-40"
            value={mode}
            disabled={noFill}
            onchange={(event) => setFillMode((event.currentTarget as HTMLSelectElement).value)}
          >
            <option value="solid">Warna solid</option>
            <option value="linear">Gradien linear</option>
            <option value="radial">Gradien radial</option>
          </select>
        </label>
        {#if mode === 'solid'}
          <div class="mt-3">
            <span class="label">Warna isi</span>
            <span class="flex items-center gap-2">
              <label
                class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line"
                title={draft.fill}
              >
                <span class="h-full w-full" style={`background:${pickerColor(draft.fill, '#ffffff')}`}></span>
                <input
                  class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                  type="color"
                  value={pickerColor(draft.fill, '#ffffff')}
                  disabled={noFill}
                  oninput={(event) => setFillColor((event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.fill === 'transparent' ? 'transparan' : draft.fill}</span>
            </span>
          </div>
        {:else}
          <div class="mt-3 space-y-2 rounded-xl border border-line p-3">
            <span class="block h-7 rounded-md border border-line" style={`background:${gradientCss(draft)}`}></span>
            <div class="space-y-1.5">
              {#each colors as color, index (index)}
                <div class="flex items-center gap-2">
                  <input
                    class="input h-8 min-w-0 flex-1 py-1"
                    type="color"
                    value={pickerColor(color, '#ffffff')}
                    disabled={noFill}
                    oninput={(event) => setGradientColor(index, (event.currentTarget as HTMLInputElement).value)}
                  />
                  <button
                    type="button"
                    class="rounded p-1 text-ink-muted hover:bg-stone-100 disabled:opacity-30"
                    title="Hapus warna"
                    disabled={noFill || colors.length <= 2}
                    onclick={() => removeGradientColor(index)}
                  >
                    <Minus size={13} />
                  </button>
                </div>
              {/each}
            </div>
            <div class="flex flex-wrap items-center gap-2">
              <Button variant="secondary" size="sm" disabled={noFill || colors.length >= 8} onclick={addGradientColor}>
                <Plus size={13} /> Tambah warna
              </Button>
              {#if mode === 'linear'}
                <label class="flex items-center gap-1.5 text-[10px] text-ink-muted">Sudut
                  <input
                    class="input w-16 py-1 text-xs disabled:opacity-40"
                    type="number"
                    min="0"
                    max="360"
                    step="5"
                    value={draft.gradient_angle ?? 90}
                    disabled={noFill}
                    oninput={(event) => setGradientAngle((event.currentTarget as HTMLInputElement).valueAsNumber)}
                  />°
                </label>
              {/if}
            </div>
            <p class="text-[10px] leading-relaxed text-ink-muted">Warna tersebar merata dari 0% sampai 100%.</p>
          </div>
        {/if}
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><SlidersHorizontal size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Garis tepi &amp; sudut</p>
            <p class="text-[10px] text-ink-muted">Warna, ketebalan, dan lengkungan sudut.</p>
          </div>
        </div>
        <label class="flex items-center gap-2 text-xs font-medium text-ink">
          <input
            type="checkbox"
            checked={noStroke}
            onchange={(event) => toggleStroke((event.currentTarget as HTMLInputElement).checked)}
          />
          Tanpa garis
        </label>
        <div class="mt-3 grid grid-cols-2 gap-3">
          <div>
            <span class="label">Warna garis</span>
            <span class="flex items-center gap-2">
              <label
                class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line"
                title={draft.border_color}
              >
                <span class="h-full w-full" style={`background:${pickerColor(draft.border_color)}`}></span>
                <input
                  class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                  type="color"
                  value={pickerColor(draft.border_color)}
                  disabled={noStroke}
                  oninput={(event) => setBorderColor((event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.border_color === 'transparent' ? 'transparan' : draft.border_color}</span>
            </span>
          </div>
          <label class="block">
            <span class="label">Tebal (pt)</span>
            <input
              class="input py-1.5 text-xs disabled:opacity-40"
              type="number"
              min="0"
              max="24"
              step="0.25"
              value={draft.border_width ?? 0.5}
              disabled={noStroke}
              oninput={(event) => setBorderWidth((event.currentTarget as HTMLInputElement).valueAsNumber)}
            />
          </label>
          {#if shape.kind === 'rect'}
            <label class="block">
              <span class="label">Sudut (mm)</span>
              <input
                class="input py-1.5 text-xs"
                type="number"
                min="0"
                max="200"
                step="0.5"
                value={draft.radius ?? 1}
                oninput={(event) => setRadius((event.currentTarget as HTMLInputElement).valueAsNumber)}
              />
            </label>
          {/if}
        </div>
      </section>
    </div>
  </div>
</Modal>
