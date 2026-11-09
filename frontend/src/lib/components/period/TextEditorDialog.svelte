<script lang="ts">
  import { untrack } from 'svelte'
  import { AlignLeft, Check, Palette, RotateCcw, Type } from 'lucide-svelte'
  import { fontOptions, fontStack } from '../../fonts/catalog'
  import type { LayoutElement } from '../../types'
  import Modal from '../ui/Modal.svelte'
  import Button from '../ui/Button.svelte'

  type Props = {
    element: LayoutElement
    onclose: () => void
    onsave: (patch: Partial<LayoutElement>) => void
  }

  let { element, onclose, onsave }: Props = $props()

  const systemFonts = fontOptions.filter((font) => font.category === 'Sistem')
  const sansFonts = fontOptions.filter((font) => font.category === 'Sans')
  const serifFonts = fontOptions.filter((font) => font.category === 'Serif')

  function clampNumber(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, Number.isFinite(value) ? value : min))
  }

  function roundedSize(value: number): number {
    return Math.round(value * 10) / 10
  }

  function pickerColor(value: string | undefined, fallback = '#143d30'): string {
    return value && /^#[0-9a-fA-F]{6}$/.test(value) ? value : fallback
  }

  function isTransparent(value: string | undefined | null): boolean {
    const normalized = value?.trim().toLowerCase() ?? ''
    return !normalized || normalized === 'transparent' || normalized === 'none'
  }

  const initial = untrack(() => {
    const copy: LayoutElement = { ...element }
    copy.font_family = copy.font_family || 'sans'
    copy.font_size = clampNumber(copy.font_size && copy.font_size > 0 ? copy.font_size : 12, 4, 300)
    copy.color = copy.color || '#143d30'
    copy.align = copy.align ?? 'left'
    copy.valign = copy.valign ?? 'top'
    copy.background = copy.background ?? 'transparent'
    return copy
  })
  let draft = $state<LayoutElement>(initial)
  let lastBackground = $state(
    isTransparent(initial.background) ? '#ffffff' : pickerColor(initial.background, '#ffffff'),
  )

  const transparentBackground = $derived(isTransparent(draft.background))
  const previewRatio = $derived(
    `${Math.max(20, draft.width || 120)} / ${Math.max(10, draft.height || 24)}`,
  )
  const previewBoxStyle = $derived(
    [
      `background:${transparentBackground ? 'transparent' : draft.background}`,
      `justify-content:${draft.valign === 'middle' ? 'center' : draft.valign === 'bottom' ? 'flex-end' : 'flex-start'}`,
    ].join(';'),
  )
  const previewTextStyle = $derived(
    [
      `font-family:${fontStack(draft.font_family)}`,
      `font-size:${draft.font_size ?? 12}pt`,
      `color:${draft.color ?? '#143d30'}`,
      `font-weight:${draft.bold ? 700 : 400}`,
      `font-style:${draft.italic ? 'italic' : 'normal'}`,
      `text-align:${draft.align ?? 'left'}`,
    ].join(';'),
  )

  function setText(value: string): void {
    draft.text = value
  }

  function setFont(value: string): void {
    draft.font_family = value
  }

  function setSize(value: number): void {
    if (!Number.isFinite(value)) return
    draft.font_size = roundedSize(clampNumber(value, 4, 300))
  }

  function setColor(value: string): void {
    draft.color = value
  }

  function setAlign(value: string): void {
    draft.align =
      value === 'center' || value === 'right' ? value : 'left'
  }

  function setValign(value: string): void {
    draft.valign =
      value === 'middle' || value === 'bottom' ? value : 'top'
  }

  function toggleBackground(transparent: boolean): void {
    if (transparent) {
      if (!isTransparent(draft.background)) {
        lastBackground = pickerColor(draft.background, '#ffffff')
      }
      draft.background = 'transparent'
    } else {
      draft.background = lastBackground
    }
  }

  function setBackground(value: string): void {
    draft.background = value
  }

  function resetStyle(): void {
    draft.font_family = 'sans'
    draft.font_size = 12
    draft.color = '#143d30'
    draft.bold = false
    draft.italic = false
    draft.align = 'left'
    draft.valign = 'top'
    draft.background = 'transparent'
    lastBackground = '#ffffff'
  }

  function save(): void {
    onsave({
      text: draft.text ?? '',
      font_family: draft.font_family,
      font_size: draft.font_size,
      color: draft.color,
      bold: Boolean(draft.bold),
      italic: Boolean(draft.italic),
      align: draft.align,
      valign: draft.valign,
      background: isTransparent(draft.background) ? 'transparent' : draft.background,
    })
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
      <Button onclick={save}><Check size={15} /> Simpan teks</Button>
    </div>
  </div>
{/snippet}

<Modal
  open={true}
  title="Editor teks"
  description="Atur isi teks, tipografi, perataan, dan latar. Pratinjau di kiri mengikuti ukuran kotak teks di halaman."
  onclose={onclose}
  width="max-w-5xl"
  footer={actions}
>
  <div class="grid gap-5 lg:grid-cols-[minmax(0,1fr)_360px]">
    <div class="self-start lg:sticky lg:top-0">
      <div class="overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
        <div class="flex items-center justify-between gap-3 border-b border-line bg-stone-50/70 px-4 py-3">
          <span class="eyebrow">Pratinjau teks</span>
          <span class="rounded-full bg-forest-800 px-2.5 py-1 text-[10px] font-semibold text-white">Teks</span>
        </div>
        <div class="bg-[radial-gradient(circle_at_1px_1px,#e7ece8_1px,transparent_0)] bg-size-[14px_14px] p-4">
          <div
            class="w-full overflow-hidden rounded-xl border border-line bg-white shadow-sm"
            style={`aspect-ratio:${previewRatio}`}
          >
            <div class="flex h-full w-full flex-col p-2" style={previewBoxStyle}>
              <span
                class="block w-full break-words whitespace-pre-wrap"
                style={previewTextStyle}
              >{draft.text || 'Teks kosong'}</span>
            </div>
          </div>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-2 border-t border-line px-4 py-3 text-[10px] text-ink-muted">
          <span>{Math.round(draft.width || 0)} × {Math.round(draft.height || 0)} mm</span>
          <span>Kotak teks {Math.round(draft.width || 0)} mm</span>
        </div>
      </div>
      <p class="mt-3 text-[10px] leading-relaxed text-ink-muted">
        Pratinjau memakai skala kotak teks sebenarnya. Perubahan di sini juga dapat disunting langsung di kanvas dengan klik dua kali.
      </p>
    </div>

    <div class="space-y-4 lg:max-h-[64vh] lg:overflow-y-auto lg:pr-1">
      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><Type size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Isi teks</p>
            <p class="text-[10px] text-ink-muted">Teks yang tampil pada halaman.</p>
          </div>
        </div>
        <label class="block">
          <span class="label">Teks</span>
          <textarea
            class="textarea text-xs"
            rows="7"
            value={draft.text ?? ''}
            placeholder="Tulis teks di sini…"
            oninput={(event) => setText((event.currentTarget as HTMLTextAreaElement).value)}
          ></textarea>
        </label>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Type size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Tipografi</p>
            <p class="text-[10px] text-ink-muted">Font, ukuran, warna, dan gaya huruf.</p>
          </div>
        </div>
        <label class="block">
          <span class="label">Font</span>
          <select
            class="input py-1.5 text-xs"
            value={draft.font_family ?? 'sans'}
            onchange={(event) => setFont((event.currentTarget as HTMLSelectElement).value)}
          >
            {@render fontOptionsMarkup()}
          </select>
        </label>
        <div class="mt-3 grid grid-cols-2 gap-3">
          <label class="block">
            <span class="label">Ukuran (pt)</span>
            <input
              class="input py-1.5 text-xs"
              type="number"
              min="4"
              max="300"
              step="0.5"
              value={draft.font_size ?? 12}
              oninput={(event) => setSize((event.currentTarget as HTMLInputElement).valueAsNumber)}
            />
          </label>
          <div>
            <span class="label">Warna</span>
            <span class="flex items-center gap-2">
              <label
                class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line"
                title={draft.color ?? '#143d30'}
              >
                <span class="h-full w-full" style={`background:${pickerColor(draft.color)}`}></span>
                <input
                  class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                  type="color"
                  value={pickerColor(draft.color)}
                  oninput={(event) => setColor((event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.color ?? '#143d30'}</span>
            </span>
          </div>
        </div>
        <div class="mt-3 flex flex-wrap items-center gap-4 text-xs">
          <label class="flex items-center gap-2">
            <input
              type="checkbox"
              checked={draft.bold}
              onchange={(event) => (draft.bold = (event.currentTarget as HTMLInputElement).checked)}
            /> Tebal
          </label>
          <label class="flex items-center gap-2">
            <input
              type="checkbox"
              checked={draft.italic}
              onchange={(event) => (draft.italic = (event.currentTarget as HTMLInputElement).checked)}
            /> Miring
          </label>
        </div>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><AlignLeft size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Perataan &amp; latar</p>
            <p class="text-[10px] text-ink-muted">Posisi teks di dalam kotak dan warna latar.</p>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="label">Rata horizontal</span>
            <select
              class="input py-1.5 text-xs"
              value={draft.align ?? 'left'}
              onchange={(event) => setAlign((event.currentTarget as HTMLSelectElement).value)}
            >
              <option value="left">Kiri</option>
              <option value="center">Tengah</option>
              <option value="right">Kanan</option>
            </select>
          </label>
          <label class="block">
            <span class="label">Rata vertikal</span>
            <select
              class="input py-1.5 text-xs"
              value={draft.valign ?? 'top'}
              onchange={(event) => setValign((event.currentTarget as HTMLSelectElement).value)}
            >
              <option value="top">Atas</option>
              <option value="middle">Tengah</option>
              <option value="bottom">Bawah</option>
            </select>
          </label>
        </div>
        <div class="mt-3">
          <label class="flex items-center gap-2 text-xs font-medium text-ink">
            <input
              type="checkbox"
              checked={transparentBackground}
              onchange={(event) => toggleBackground((event.currentTarget as HTMLInputElement).checked)}
            />
            Latar transparan
          </label>
          {#if !transparentBackground}
            <span class="mt-2 flex items-center gap-2">
              <label
                class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line"
                title={draft.background}
              >
                <span class="h-full w-full" style={`background:${pickerColor(draft.background, '#ffffff')}`}></span>
                <input
                  class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                  type="color"
                  value={pickerColor(draft.background, '#ffffff')}
                  oninput={(event) => setBackground((event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.background}</span>
            </span>
          {/if}
        </div>
      </section>

      <div class="flex items-center gap-2.5 rounded-xl border border-line bg-stone-50 px-4 py-3 text-[10px] leading-relaxed text-ink-muted">
        <Palette size={14} class="shrink-0 text-ink-muted" />
        <span>Perataan vertikal mengatur posisi teks bila kotak lebih tinggi dari isi teks.</span>
      </div>
    </div>
  </div>
</Modal>
