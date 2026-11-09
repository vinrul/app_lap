<script lang="ts">
  import { untrack } from 'svelte'
  import { Check, Palette, RotateCcw, Search, Shapes } from 'lucide-svelte'
  import { iconByName, iconCatalog } from '../../icons/catalog'
  import type { LayoutElement } from '../../types'
  import Modal from '../ui/Modal.svelte'
  import Button from '../ui/Button.svelte'

  type Props = {
    element: LayoutElement
    onclose: () => void
    onsave: (patch: Partial<LayoutElement>) => void
  }

  let { element, onclose, onsave }: Props = $props()

  function pickerColor(value: string | undefined, fallback = '#143d30'): string {
    return value && /^#[0-9a-fA-F]{6}$/.test(value) ? value : fallback
  }

  function clampNumber(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, Number.isFinite(value) ? value : min))
  }

  function roundedSize(value: number): number {
    return Math.round(value * 10) / 10
  }

  const categories = ['Semua', ...new Set(iconCatalog.map((icon) => icon.category))]

  const initial = untrack(() => {
    const copy: LayoutElement = { ...element }
    copy.icon_name = copy.icon_name || iconCatalog[0]?.name || ''
    copy.icon_color = copy.icon_color || '#143d30'
    copy.icon_stroke = copy.icon_stroke ?? 2
    return copy
  })
  let draft = $state<LayoutElement>(initial)
  let search = $state('')
  let category = $state('Semua')

  const filteredIcons = $derived.by(() => {
    const query = search.trim().toLowerCase()
    return iconCatalog.filter((icon) => {
      if (category !== 'Semua' && icon.category !== category) return false
      if (!query) return true
      return icon.label.toLowerCase().includes(query) || icon.name.includes(query)
    })
  })
  const currentIcon = $derived(iconByName.get(draft.icon_name ?? ''))
  const previewRatio = $derived(
    `${Math.max(12, draft.width || 14)} / ${Math.max(12, draft.height || 14)}`,
  )

  function chooseIcon(name: string): void {
    draft.icon_name = name
  }

  function setColor(value: string): void {
    draft.icon_color = value
  }

  function setStroke(value: number): void {
    if (!Number.isFinite(value)) return
    draft.icon_stroke = roundedSize(clampNumber(value, 1, 4))
  }

  function resetStyle(): void {
    draft.icon_color = '#143d30'
    draft.icon_stroke = 2
  }

  function save(): void {
    onsave({
      icon_name: draft.icon_name,
      icon_color: draft.icon_color,
      icon_stroke: draft.icon_stroke,
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
      <Button onclick={save}><Check size={15} /> Simpan ikon</Button>
    </div>
  </div>
{/snippet}

<Modal
  open={true}
  title="Editor ikon"
  description="Pilih ikon, warna, dan ketebalan garis. Pratinjau di kiri mengikuti ukuran objek di halaman."
  onclose={onclose}
  width="max-w-5xl"
  footer={actions}
>
  <div class="grid gap-5 lg:grid-cols-[minmax(0,1fr)_380px]">
    <div class="self-start lg:sticky lg:top-0">
      <div class="overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
        <div class="flex items-center justify-between gap-3 border-b border-line bg-stone-50/70 px-4 py-3">
          <span class="eyebrow">Pratinjau ikon</span>
          <span class="rounded-full bg-forest-800 px-2.5 py-1 text-[10px] font-semibold text-white">
            {currentIcon?.label ?? 'Ikon'}
          </span>
        </div>
        <div class="bg-[radial-gradient(circle_at_1px_1px,#e7ece8_1px,transparent_0)] bg-size-[14px_14px] p-4">
          <div
            class="mx-auto w-full max-w-[280px] overflow-hidden rounded-xl border border-line bg-white shadow-sm"
            style={`aspect-ratio:${previewRatio}`}
          >
            <div class="flex h-full w-full items-center justify-center p-4" style={`color:${draft.icon_color ?? '#143d30'}`}>
              {#if currentIcon}
                <svg
                  class="h-full w-full"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width={draft.icon_stroke ?? 2}
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >{@html currentIcon.svg}</svg>
              {:else}
                <span class="text-xs text-ink-muted">Ikon tidak ditemukan</span>
              {/if}
            </div>
          </div>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-2 border-t border-line px-4 py-3 text-[10px] text-ink-muted">
          <span class="truncate">{draft.icon_name || '—'}</span>
          <span>{Math.round(draft.width || 0)} × {Math.round(draft.height || 0)} mm</span>
        </div>
      </div>
      <p class="mt-3 text-[10px] leading-relaxed text-ink-muted">
        Ukuran ikon mengikuti kotak elemen. Ubah lebar dan tinggi objek lewat panel properti atau tarik handle di kanvas.
      </p>
    </div>

    <div class="space-y-4 lg:max-h-[64vh] lg:overflow-y-auto lg:pr-1">
      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><Shapes size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Pilih ikon</p>
            <p class="text-[10px] text-ink-muted">Cari berdasarkan nama atau kategori.</p>
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <label class="relative min-w-0 flex-1">
            <Search size={14} class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-muted" />
            <input
              class="input py-2 pl-8 text-sm"
              placeholder="Cari ikon…"
              bind:value={search}
            />
          </label>
          <select class="input max-w-[190px] py-2 text-sm" bind:value={category}>
            {#each categories as item}
              <option value={item}>{item}</option>
            {/each}
          </select>
        </div>
        <div class="mt-3 max-h-[42vh] overflow-auto pr-1">
          {#if filteredIcons.length}
            <div class="grid grid-cols-5 gap-1.5 sm:grid-cols-6">
              {#each filteredIcons as icon (icon.name)}
                <button
                  type="button"
                  class="flex min-w-0 flex-col items-center gap-1 rounded-xl border px-1.5 py-2 text-ink hover:border-gold-300 hover:bg-gold-50 {draft.icon_name === icon.name ? 'border-forest-600 bg-forest-50 text-forest-800' : 'border-line'}"
                  title={icon.label}
                  onclick={() => chooseIcon(icon.name)}
                >
                  <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                    stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{@html icon.svg}</svg>
                  <span class="w-full truncate text-center text-[8px] leading-tight">{icon.label}</span>
                </button>
              {/each}
            </div>
          {:else}
            <p class="py-8 text-center text-sm text-ink-muted">Tidak ada ikon yang cocok.</p>
          {/if}
        </div>
        <p class="mt-2 text-[10px] text-ink-muted">{filteredIcons.length} ikon ditampilkan dari {iconCatalog.length} ikon Lucide.</p>
      </section>

      <section class="rounded-2xl border border-line bg-white p-4">
        <div class="mb-3 flex items-center gap-2.5">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-gold-100 text-gold-700"><Palette size={15} /></span>
          <div>
            <p class="text-xs font-semibold text-ink">Gaya ikon</p>
            <p class="text-[10px] text-ink-muted">Warna dan ketebalan garis ikon.</p>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <span class="label">Warna ikon</span>
            <span class="flex items-center gap-2">
              <label
                class="relative inline-flex h-8 w-9 cursor-pointer overflow-hidden rounded-lg border border-line"
                title={draft.icon_color ?? '#143d30'}
              >
                <span class="h-full w-full" style={`background:${pickerColor(draft.icon_color)}`}></span>
                <input
                  class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                  type="color"
                  value={pickerColor(draft.icon_color)}
                  oninput={(event) => setColor((event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <span class="font-mono text-[10px] uppercase text-ink-muted">{draft.icon_color ?? '#143d30'}</span>
            </span>
          </div>
          <label class="block">
            <span class="label">Ketebalan garis</span>
            <input
              class="input py-1.5 text-xs"
              type="number"
              min="1"
              max="4"
              step="0.25"
              value={draft.icon_stroke ?? 2}
              oninput={(event) => setStroke((event.currentTarget as HTMLInputElement).valueAsNumber)}
            />
          </label>
        </div>
        <div class="mt-3">
          <div class="flex items-center justify-between gap-2">
            <span class="label mb-0">Ketebalan</span>
            <span class="text-[11px] font-semibold text-forest-800">{draft.icon_stroke ?? 2}</span>
          </div>
          <input
            class="mt-1 w-full accent-forest-700"
            type="range"
            min="1"
            max="4"
            step="0.25"
            value={draft.icon_stroke ?? 2}
            aria-label="Ketebalan garis ikon"
            oninput={(event) => setStroke((event.currentTarget as HTMLInputElement).valueAsNumber)}
          />
        </div>
      </section>
    </div>
  </div>
</Modal>
