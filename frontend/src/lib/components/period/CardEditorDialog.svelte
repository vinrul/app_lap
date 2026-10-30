<script lang="ts">
  import { untrack } from 'svelte'
  import { Check, LayoutTemplate, Palette, RotateCcw, Type } from 'lucide-svelte'
  import { fontOptions } from '../../fonts/catalog'
  import type { LayoutElement } from '../../types'
  import { cardTemplate } from '../../utils/cardTemplates'
  import { createCardNode } from '../../utils/cardNode'
  import Modal from '../ui/Modal.svelte'
  import Button from '../ui/Button.svelte'

  type Props = {
    element: LayoutElement
    dataLocked?: boolean
    sourceLabel?: string
    onclose: () => void
    onsave: (patch: Partial<LayoutElement>) => void
  }

  type CardFontKey =
    | 'card_label_font_family'
    | 'card_value_font_family'
    | 'card_unit_font_family'
    | 'card_detail_font_family'
  type CardSizeKey =
    | 'card_label_font_size'
    | 'card_value_font_size'
    | 'card_unit_font_size'
    | 'card_detail_font_size'
  type CardColorKey =
    | 'card_label_color'
    | 'card_value_color'
    | 'card_unit_color'
    | 'card_detail_color'

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

  function prepare(input: LayoutElement): LayoutElement {
    const copy: LayoutElement = { ...input }
    const template = cardTemplate(copy.card_template)
    const base = copy.font_size ?? 9
    copy.card_label_font_family = copy.card_label_font_family || copy.font_family || 'sans'
    copy.card_value_font_family = copy.card_value_font_family || copy.font_family || 'sans'
    copy.card_unit_font_family = copy.card_unit_font_family || copy.font_family || 'sans'
    copy.card_detail_font_family = copy.card_detail_font_family || copy.font_family || 'sans'
    copy.card_label_font_size = clampNumber(
      copy.card_label_font_size && copy.card_label_font_size > 0
        ? copy.card_label_font_size
        : roundedSize(base),
      5,
      72,
    )
    copy.card_value_font_size = clampNumber(
      copy.card_value_font_size && copy.card_value_font_size > 0
        ? copy.card_value_font_size
        : roundedSize(Math.min(40, Math.max(14, base * 2.35))),
      5,
      72,
    )
    copy.card_unit_font_size = clampNumber(
      copy.card_unit_font_size && copy.card_unit_font_size > 0
        ? copy.card_unit_font_size
        : roundedSize(base * 0.82),
      5,
      72,
    )
    copy.card_detail_font_size = clampNumber(
      copy.card_detail_font_size && copy.card_detail_font_size > 0
        ? copy.card_detail_font_size
        : roundedSize(base * 0.78),
      5,
      72,
    )
    const baseColor = copy.color || template.color
    copy.card_label_color = copy.card_label_color || baseColor
    copy.card_value_color = copy.card_value_color || baseColor
    copy.card_unit_color = copy.card_unit_color || baseColor
    copy.card_detail_color = copy.card_detail_color || baseColor
    copy.card_accent = copy.card_accent || template.accent
    copy.fill = copy.fill || template.fill
    copy.card_icon_color = copy.card_icon_color || template.iconColor
    copy.card_icon_bg = copy.card_icon_bg || template.iconBg
    copy.card_icon_scale = copy.card_icon_scale ?? 100
    return copy
  }

  const initial = untrack(() => prepare(element))
  let draft = $state<LayoutElement>(initial)
  let previewEl = $state<HTMLDivElement | null>(null)

  const template = $derived(cardTemplate(draft.card_template))
  const previewRatio = $derived(
    `${Math.max(20, draft.width || 48)} / ${Math.max(12, draft.height || 33)}`,
  )

  $effect(() => {
    const node = previewEl
    if (!node) return
    const card = createCardNode(document, draft)
    card.style.width = '100%'
    card.style.height = '100%'
    node.replaceChildren(card)
  })

  function swatchStyle(value: string | undefined): string {
    return `background:${value || '#ffffff'}`
  }

  function setText(
    key: 'card_label' | 'card_value' | 'card_unit' | 'card_detail',
    value: string,
  ): void {
    draft[key] = value
  }

  function setFont(key: CardFontKey, value: string): void {
    draft[key] = value
  }

  function setSize(key: CardSizeKey, value: number): void {
    if (!Number.isFinite(value)) return
    draft[key] = roundedSize(clampNumber(value, 5, 72))
  }

  function setPartColor(key: CardColorKey, value: string): void {
    draft[key] = value
  }

  function setColor(key: 'card_accent' | 'fill' | 'card_icon_color' | 'card_icon_bg', value: string): void {
    draft[key] = value
  }

  function resetStyle(): void {
    const preset = template
    const base = draft.font_size ?? 9
    const baseColor = draft.color || preset.color
    draft.card_label_font_family = 'sans'
    draft.card_value_font_family = 'sans'
    draft.card_unit_font_family = 'sans'
    draft.card_detail_font_family = 'sans'
    draft.card_label_font_size = roundedSize(base)
    draft.card_value_font_size = roundedSize(Math.min(40, Math.max(14, base * 2.35)))
    draft.card_unit_font_size = roundedSize(base * 0.82)
    draft.card_detail_font_size = roundedSize(base * 0.78)
    draft.card_label_color = baseColor
    draft.card_value_color = baseColor
    draft.card_unit_color = baseColor
    draft.card_detail_color = baseColor
    draft.card_accent = preset.accent
    draft.fill = preset.fill
    draft.card_icon_color = preset.iconColor
    draft.card_icon_bg = preset.iconBg
    draft.card_icon_scale = 100
  }

  function save(): void {
    const patch: Partial<LayoutElement> = {
      card_label_font_family: draft.card_label_font_family,
      card_label_font_size: draft.card_label_font_size,
      card_label_color: draft.card_label_color,
      card_value_font_family: draft.card_value_font_family,
      card_value_font_size: draft.card_value_font_size,
      card_value_color: draft.card_value_color,
      card_unit_font_family: draft.card_unit_font_family,
      card_unit_font_size: draft.card_unit_font_size,
      card_unit_color: draft.card_unit_color,
      card_detail_font_family: draft.card_detail_font_family,
      card_detail_font_size: draft.card_detail_font_size,
      card_detail_color: draft.card_detail_color,
      card_accent: draft.card_accent,
      fill: draft.fill,
      card_icon_color: draft.card_icon_color,
      card_icon_bg: draft.card_icon_bg,
      card_icon_scale: draft.card_icon_scale,
    }
    if (!dataLocked) {
      patch.card_label = (draft.card_label ?? '').trim()
      patch.card_value = (draft.card_value ?? '').trim()
      patch.card_unit = (draft.card_unit ?? '').trim()
      patch.card_detail = (draft.card_detail ?? '').trim()
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

  {#snippet typographyRow(
    title: string,
    familyKey: CardFontKey,
    sizeKey: CardSizeKey,
    colorKey: CardColorKey,
  )}
    <span class="text-[11px] font-medium text-ink-soft">{title}</span>
    <select
      class="input min-w-0 py-1.5 text-[10px]"
      value={draft[familyKey] ?? 'sans'}
      onchange={(event) => setFont(familyKey, (event.currentTarget as HTMLSelectElement).value)}
    >
      {@render fontOptionsMarkup()}
    </select>
    <input
      class="input py-1.5 text-right text-[10px]"
      type="number"
      min="5"
      max="72"
      step="0.5"
      value={draft[sizeKey] ?? 9}
      oninput={(event) => setSize(sizeKey, (event.currentTarget as HTMLInputElement).valueAsNumber)}
    />
    <label
      class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line"
      title={draft[colorKey] ?? draft.color ?? '#143d30'}
    >
      <span class="h-full w-full" style={swatchStyle(draft[colorKey] ?? draft.color ?? '#143d30')}></span>
      <input
        class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
        type="color"
        value={draft[colorKey] ?? draft.color ?? '#143d30'}
        oninput={(event) => setPartColor(colorKey, (event.currentTarget as HTMLInputElement).value)}
      />
    </label>
  {/snippet}

  {#snippet actions()}
    <div class="flex w-full flex-wrap items-center justify-between gap-2">
      <Button variant="ghost" size="sm" onclick={resetStyle}>
        <RotateCcw size={14} /> Reset gaya
      </Button>
      <div class="flex flex-wrap items-center gap-2">
        <Button variant="ghost" onclick={onclose}>Batal</Button>
        <Button onclick={save}><Check size={15} /> Simpan kartu</Button>
      </div>
    </div>
  {/snippet}

<Modal
  open={true}
  title="Editor kartu"
  description="Atur isi kartu, warna, font, dan ukuran setiap bagian. Pratinjau di kiri mengikuti perubahan secara langsung."
  onclose={onclose}
  width="max-w-6xl"
  footer={actions}
>
  <div class="grid gap-5 lg:grid-cols-[minmax(0,1fr)_420px]">
    <div class="self-start lg:sticky lg:top-0">
      <div class="overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
        <div class="flex items-center justify-between gap-3 border-b border-line bg-stone-50/70 px-4 py-3">
          <span class="eyebrow">Pratinjau kartu</span>
          <span class="rounded-full bg-forest-800 px-2.5 py-1 text-[10px] font-semibold text-white">{template.label}</span>
        </div>
        <div class="bg-[radial-gradient(circle_at_1px_1px,#e7ece8_1px,transparent_0)] bg-size-[14px_14px] p-4">
          <div
            class="mx-auto min-h-55 w-full max-w-105"
            style={`aspect-ratio:${previewRatio}`}
            bind:this={previewEl}
          ></div>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-2 border-t border-line px-4 py-3 text-[10px] text-ink-muted">
          <span>{dataLocked ? 'Konten mengikuti sumber data' : 'Konten manual'}</span>
          <span>{Math.round(draft.width || 48)} × {Math.round(draft.height || 33)} mm</span>
        </div>
      </div>
      {#if dataLocked}
        <div class="mt-3 rounded-xl border border-forest-200 bg-forest-50 px-4 py-3 text-[11px] leading-relaxed text-forest-800">
          Label, nilai, unit, dan detail mengikuti sumber <strong>{sourceLabel || 'data terhubung'}</strong>. Perubahan di sini hanya mengatur tampilan kartu.
        </div>
      {:else}
        <p class="mt-3 text-[10px] leading-relaxed text-ink-muted">
          Ukuran font pada dasbor mengikuti ukuran kartu. Gunakan nilai 5–72 pt agar teks tetap proporsional.
        </p>
      {/if}
    </div>

    <div class="space-y-4 lg:max-h-[64vh] lg:overflow-y-auto lg:pr-1">
      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><LayoutTemplate size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Konten kartu</p>
            <p class="text-[10px] text-ink-muted">Teks yang tampil pada kartu.</p>
          </div>
        </div>

        {#if dataLocked}
          <div class="rounded-xl border border-line bg-stone-50 px-3.5 py-3">
            <p class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">{draft.card_label}</p>
            <p class="mt-1 font-display text-xl font-semibold" style={`color:${draft.card_value_color ?? draft.color ?? '#143d30'}`}>
              {draft.card_value}
              <span class="text-xs font-normal">{draft.card_unit}</span>
            </p>
            {#if draft.card_detail}<p class="mt-1 text-[11px] text-ink-muted">{draft.card_detail}</p>{/if}
          </div>
        {:else}
          <div class="grid gap-3 sm:grid-cols-2">
            <label class="block sm:col-span-2">
              <span class="label">Label</span>
              <input class="input py-1.5 text-xs" maxlength="80" value={draft.card_label ?? ''}
                oninput={(event) => setText('card_label', (event.currentTarget as HTMLInputElement).value)} />
            </label>
            <label class="block">
              <span class="label">Nilai</span>
              <input class="input py-1.5 text-xs" maxlength="40" value={draft.card_value ?? ''}
                oninput={(event) => setText('card_value', (event.currentTarget as HTMLInputElement).value)} />
            </label>
            <label class="block">
              <span class="label">Unit</span>
              <input class="input py-1.5 text-xs" maxlength="40" value={draft.card_unit ?? ''}
                oninput={(event) => setText('card_unit', (event.currentTarget as HTMLInputElement).value)} />
            </label>
            <label class="block sm:col-span-2">
              <span class="label">Detail</span>
              <input class="input py-1.5 text-xs" maxlength="160" value={draft.card_detail ?? ''}
                oninput={(event) => setText('card_detail', (event.currentTarget as HTMLInputElement).value)} />
            </label>
          </div>
        {/if}
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Type size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Tipografi per bagian</p>
            <p class="text-[10px] text-ink-muted">Font, ukuran, dan warna diatur terpisah.</p>
          </div>
        </div>

        <div class="grid grid-cols-[56px_minmax(0,1fr)_64px_36px] items-center gap-2">
          <span class="text-[9px] font-bold tracking-[0.12em] text-ink-muted uppercase">Bagian</span>
          <span class="text-[9px] font-bold tracking-[0.12em] text-ink-muted uppercase">Font</span>
          <span class="text-center text-[9px] font-bold tracking-[0.12em] text-ink-muted uppercase">Ukuran</span>
          <span class="text-[9px] font-bold tracking-[0.12em] text-ink-muted uppercase">Warna</span>

          {@render typographyRow('Label', 'card_label_font_family', 'card_label_font_size', 'card_label_color')}
          {@render typographyRow('Nilai', 'card_value_font_family', 'card_value_font_size', 'card_value_color')}
          {@render typographyRow('Unit', 'card_unit_font_family', 'card_unit_font_size', 'card_unit_color')}
          {@render typographyRow('Detail', 'card_detail_font_family', 'card_detail_font_size', 'card_detail_color')}
        </div>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Palette size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Warna kartu</p>
            <p class="text-[10px] text-ink-muted">Aksen, latar, dan warna ikon.</p>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <span class="label">Aksen</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.card_accent ?? template.accent}>
                <span class="h-full w-full" style={swatchStyle(draft.card_accent ?? template.accent)}></span>
                <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                  value={draft.card_accent ?? template.accent}
                  oninput={(event) => setColor('card_accent', (event.currentTarget as HTMLInputElement).value)} />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.card_accent ?? template.accent}</span>
            </span>
          </div>
          <div>
            <span class="label">Latar</span>
            <span class="flex items-center gap-2">
              <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.fill ?? template.fill}>
                <span class="h-full w-full" style={swatchStyle(draft.fill ?? template.fill)}></span>
                <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                  value={draft.fill ?? template.fill}
                  oninput={(event) => setColor('fill', (event.currentTarget as HTMLInputElement).value)} />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.fill ?? template.fill}</span>
            </span>
          </div>
        </div>

        {#if draft.card_icon}
          <div class="mt-3 grid grid-cols-2 gap-3 border-t border-line pt-3">
            <div>
              <span class="label">Warna ikon</span>
              <span class="flex items-center gap-2">
                <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.card_icon_color ?? template.iconColor}>
                  <span class="h-full w-full" style={swatchStyle(draft.card_icon_color ?? template.iconColor)}></span>
                  <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                    value={draft.card_icon_color ?? template.iconColor}
                    oninput={(event) => setColor('card_icon_color', (event.currentTarget as HTMLInputElement).value)} />
                </label>
                <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.card_icon_color ?? template.iconColor}</span>
              </span>
            </div>
            <div>
              <span class="label">Latar ikon</span>
              <span class="flex items-center gap-2">
                <label class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line" title={draft.card_icon_bg ?? template.iconBg}>
                  <span class="h-full w-full" style={swatchStyle(draft.card_icon_bg ?? template.iconBg)}></span>
                  <input class="absolute inset-0 h-full w-full cursor-pointer opacity-0" type="color"
                    value={draft.card_icon_bg ?? template.iconBg}
                    oninput={(event) => setColor('card_icon_bg', (event.currentTarget as HTMLInputElement).value)} />
                </label>
                <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.card_icon_bg ?? template.iconBg}</span>
              </span>
            </div>
            <label class="block sm:col-span-2">
              <span class="label">Skala ikon (%)</span>
              <input class="input py-1.5 text-xs" type="number" min="30" max="300" step="5"
                value={draft.card_icon_scale ?? 100}
                oninput={(event) => {
                  const value = (event.currentTarget as HTMLInputElement).valueAsNumber
                  if (Number.isFinite(value)) draft.card_icon_scale = clampNumber(value, 30, 300)
                }} />
            </label>
          </div>
        {/if}
      </section>
    </div>
  </div>
</Modal>
