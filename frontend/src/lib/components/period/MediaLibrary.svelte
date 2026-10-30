<script lang="ts">
  import { untrack } from 'svelte'
  import { AlertCircle, Check, Images, Search, Upload } from 'lucide-svelte'
  import { designApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type { MediaFilters, MediaItem, UploadedDesignImage } from '../../types'
  import { formatBytes, formatDateTime } from '../../utils/format'
  import { toast } from '../../stores/toast.svelte'
  import Button from '../ui/Button.svelte'
  import Modal from '../ui/Modal.svelte'

  type Props = {
    open?: boolean
    periodId: string
    onclose: () => void
    onuse: (image: UploadedDesignImage, filename: string) => void
  }

  let { open = false, periodId, onclose, onuse }: Props = $props()

  let items = $state<MediaItem[]>([])
  let loading = $state(false)
  let error = $state('')
  let filter = $state<'all' | 'design' | 'evidence'>('all')
  let search = $state('')
  let selectedKey = $state('')
  let busy = $state(false)
  let uploading = $state(false)
  let fileInput = $state<HTMLInputElement | null>(null)
  let requestId = 0
  let loadedKey = ''
  let loadedFilter = ''
  let options = $state<MediaFilters>({ codes: [], polres: [], satker: [] })
  let codeFilter = $state('')
  let subFilter = $state('')
  let polresFilter = $state('')
  let satkerFilter = $state('')

  const activeCode = $derived(subFilter || codeFilter)
  const hasServerFilter = $derived(
    activeCode !== '' || polresFilter !== '' || satkerFilter !== '',
  )
  const filterKey = $derived(`${activeCode}|${polresFilter}|${satkerFilter}`)
  const groupCodes = $derived.by(() => {
    const totals = new Map<string, number>()
    for (const option of options.codes) {
      const group = option.label.split('.')[0]
      totals.set(group, (totals.get(group) ?? 0) + option.count)
    }
    return [...totals.entries()].map(([label, count]) => ({ label, count }))
  })
  const subCodes = $derived(
    options.codes.filter((option) => option.label.startsWith(`${codeFilter}.`)),
  )
  const filtered = $derived.by(() => {
    const query = search.trim().toLowerCase()
    return items.filter((item) => {
      if (filter !== 'all' && item.source !== filter) return false
      if (!query) return true
      return (
        item.filename.toLowerCase().includes(query) ||
        item.context.toLowerCase().includes(query) ||
        item.kode.toLowerCase().includes(query) ||
        item.import_filename.toLowerCase().includes(query)
      )
    })
  })
  const designCount = $derived(items.filter((item) => item.source === 'design').length)
  const evidenceCount = $derived(items.filter((item) => item.source === 'evidence').length)
  const selected = $derived(
    items.find((item) => `${item.source}:${item.id}` === selectedKey) ?? null,
  )

  $effect(() => {
    const key = open ? periodId : ''
    if (!open) {
      loadedKey = ''
      loadedFilter = ''
      return
    }
    if (!periodId || key === loadedKey) return
    loadedKey = key
    filter = 'all'
    search = ''
    selectedKey = ''
    codeFilter = ''
    subFilter = ''
    polresFilter = ''
    satkerFilter = ''
    loadedFilter = filterKey
    untrack(() => {
      void loadOptions()
      void load()
    })
  })

  $effect(() => {
    const key = filterKey
    if (!open || !loadedKey) return
    if (key === loadedFilter) return
    loadedFilter = key
    selectedKey = ''
    untrack(() => void load())
  })

  async function loadOptions() {
    try {
      options = await designApi.mediaFilters(periodId)
    } catch {
      options = { codes: [], polres: [], satker: [] }
    }
  }

  function pickCode(value: string) {
    codeFilter = value
    subFilter = ''
    if (value) filter = 'evidence'
  }

  function pickSubCode(value: string) {
    subFilter = value
    if (value) filter = 'evidence'
  }

  function pickPolres(value: string) {
    polresFilter = value
    if (value) {
      satkerFilter = ''
      filter = 'evidence'
    }
  }

  function pickSatker(value: string) {
    satkerFilter = value
    if (value) {
      polresFilter = ''
      filter = 'evidence'
    }
  }

  function clearFilters() {
    codeFilter = ''
    subFilter = ''
    polresFilter = ''
    satkerFilter = ''
  }

  async function load() {
    const current = ++requestId
    loading = true
    error = ''
    try {
      const result = await designApi.media(periodId, {
        kode: activeCode || undefined,
        polres: polresFilter || undefined,
        satker: satkerFilter || undefined,
      })
      if (current !== requestId) return
      items = result
    } catch (caught) {
      if (current === requestId) error = errorMessage(caught)
    } finally {
      if (current === requestId) loading = false
    }
  }

  function pickUpload() {
    fileInput?.click()
  }

  async function handleUpload(event: Event) {
    const input = event.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    if (!['image/png', 'image/jpeg'].includes(file.type)) {
      toast.error('Pilih gambar PNG atau JPEG.')
      return
    }
    if (file.size > 8 * 1024 * 1024) {
      toast.error('Ukuran gambar maksimal 8 MB.')
      return
    }
    uploading = true
    try {
      const uploaded = await designApi.uploadImage(periodId, file)
      toast.success('Gambar ditambahkan ke media library.')
      clearFilters()
      await loadOptions()
      await load()
      selectedKey = `design:${uploaded.id}`
      filter = 'all'
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      uploading = false
    }
  }

  async function useSelected() {
    if (!selected || busy) return
    busy = true
    error = ''
    try {
      const uploaded = await designApi.importMedia(periodId, selected.source, selected.id)
      onuse(uploaded, selected.filename || 'gambar')
      onclose()
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      busy = false
    }
  }
</script>

<Modal
  {open}
  onclose={onclose}
  title="Media library"
  description="Pilih gambar yang pernah diunggah atau bukti dukung gambar dari periode ini."
  width="max-w-6xl"
>
  <div class="flex flex-wrap items-center justify-between gap-2">
    <div class="flex flex-wrap gap-1.5">
      {#each [
        ['all', 'Semua', items.length],
        ['design', 'Gambar desain', designCount],
        ['evidence', 'Bukti dukung', evidenceCount],
      ] as [value, label, count]}
        <button
          type="button"
          class="rounded-lg border px-3 py-1.5 text-[11px] font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40 {filter === value
            ? 'border-forest-700 bg-forest-50 text-forest-800'
            : 'border-line bg-white text-ink-muted hover:border-gold-300'}"
          disabled={value === 'design' && hasServerFilter}
          title={value === 'design' && hasServerFilter
            ? 'Gambar desain tidak terikat kode maupun unit'
            : ''}
          onclick={() => (filter = value as 'all' | 'design' | 'evidence')}
        >
          {label} · {count}
        </button>
      {/each}
    </div>
    <Button variant="secondary" size="sm" loading={uploading} onclick={pickUpload}>
      <Upload size={14} /> Unggah gambar baru
    </Button>
  </div>

  <label class="relative mt-3 block">
    <Search size={15} class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-ink-muted" />
    <input class="input py-2 pl-9 text-xs" bind:value={search} placeholder="Cari nama gambar, kegiatan, kode, atau berkas impor…" />
  </label>

  <div class="mt-2 flex flex-wrap items-end gap-2">
    <label class="flex flex-col gap-1">
      <span class="text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">Kode</span>
      <select
        class="input w-40 py-1.5 text-xs"
        value={codeFilter}
        onchange={(event) => pickCode(event.currentTarget.value)}
      >
        <option value="">Semua kode</option>
        {#each groupCodes as option (option.label)}
          <option value={option.label}>{option.label} · {option.count} foto</option>
        {/each}
      </select>
    </label>
    <label class="flex flex-col gap-1">
      <span class="text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">Sub-kode</span>
      <select
        class="input w-48 py-1.5 text-xs"
        value={subFilter}
        disabled={!codeFilter}
        onchange={(event) => pickSubCode(event.currentTarget.value)}
      >
        <option value="">Semua sub-kode</option>
        {#each subCodes as option (option.label)}
          <option value={option.label}>{option.label} · {option.count} foto</option>
        {/each}
      </select>
    </label>
    <label class="flex flex-col gap-1">
      <span class="text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">Polres</span>
      <select
        class="input w-48 py-1.5 text-xs"
        value={polresFilter}
        onchange={(event) => pickPolres(event.currentTarget.value)}
      >
        <option value="">Semua polres</option>
        {#each options.polres as option (option.label)}
          <option value={option.label}>{option.label} · {option.count} foto</option>
        {/each}
      </select>
    </label>
    <label class="flex flex-col gap-1">
      <span class="text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">Satker Polda</span>
      <select
        class="input w-48 py-1.5 text-xs"
        value={satkerFilter}
        onchange={(event) => pickSatker(event.currentTarget.value)}
      >
        <option value="">Semua satker</option>
        {#each options.satker as option (option.label)}
          <option value={option.label}>{option.label} · {option.count} foto</option>
        {/each}
      </select>
    </label>
    {#if hasServerFilter}
      <button
        type="button"
        class="pb-1.5 text-[11px] font-semibold text-forest-700 underline underline-offset-2 hover:text-forest-900"
        onclick={clearFilters}
      >
        Hapus semua filter
      </button>
    {/if}
  </div>

  {#if !loading && items.length >= 300}
    <p class="mt-2 text-[10px] text-ink-muted">
      Menampilkan 300 foto terbaru. Persempit filter kode, polres, atau satker untuk melihat
      sisanya.
    </p>
  {/if}

  {#if error}
    <div class="mt-3 flex items-start gap-2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertCircle size={17} class="mt-0.5 shrink-0" /> <span>{error}</span>
    </div>
  {/if}

  {#if loading}
    <div class="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
      {#each Array(8) as _}<div class="skeleton aspect-square rounded-xl"></div>{/each}
    </div>
  {:else if filtered.length}
    <div class="mt-4 grid max-h-[52vh] grid-cols-2 gap-3 overflow-y-auto pr-1 sm:grid-cols-4 lg:grid-cols-5">
      {#each filtered as item (`${item.source}:${item.id}`)}
        {@const key = `${item.source}:${item.id}`}
        <button
          type="button"
          class="group overflow-hidden rounded-xl border bg-white text-left transition-colors {selectedKey === key
            ? 'border-forest-700 ring-2 ring-forest-700/20'
            : 'border-line hover:border-gold-300'}"
          onclick={() => (selectedKey = key)}
        >
          <div class="relative aspect-square bg-stone-100">
            <img
              src={item.preview_url}
              alt={item.filename || 'Media'}
              class="h-full w-full object-contain"
              loading="lazy"
            />
            {#if selectedKey === key}
              <span class="absolute top-1.5 right-1.5 flex h-5 w-5 items-center justify-center rounded-full bg-forest-700 text-white">
                <Check size={12} />
              </span>
            {/if}
          </div>
          <div class="p-2.5">
            <div class="flex flex-wrap items-center gap-1">
              <span class="badge {item.source === 'evidence' ? 'badge-progress' : 'badge-neutral'}">
                {item.source === 'evidence' ? 'Bukti dukung' : 'Gambar desain'}
              </span>
              {#if item.kode}<span class="badge badge-neutral">{item.kode}</span>{/if}
            </div>
            <p class="mt-1.5 truncate text-[11px] font-semibold text-ink" title={item.filename}>{item.filename || 'Tanpa nama'}</p>
            <p class="mt-0.5 line-clamp-2 text-[10px] leading-snug text-ink-muted">{item.context || '—'}</p>
            <p class="mt-1 text-[9px] text-ink-muted">{formatDateTime(item.created_at)} · {formatBytes(item.size_bytes)}</p>
          </div>
        </button>
      {/each}
    </div>
  {:else}
    <div class="mt-4 flex flex-col items-center rounded-2xl border border-dashed border-line px-5 py-10 text-center">
      <Images size={26} class="text-ink-muted" />
      <p class="mt-2 text-sm font-semibold text-ink">
        {hasServerFilter ? 'Belum ada bukti dukung untuk filter ini' : 'Belum ada gambar'}
      </p>
      <p class="mt-1 max-w-md text-xs text-ink-muted">
        {#if hasServerFilter}
          Tidak ada bukti dukung gambar yang cocok dengan kode atau unit terpilih pada periode
          berjalan. Pilih filter lain atau hapus filter.
        {:else}
          Unggah gambar PNG/JPEG pertama, atau pastikan bukti dukung periode ini sudah berhasil diunduh.
        {/if}
      </p>
      <Button class="mt-4" variant="secondary" size="sm" loading={uploading} onclick={pickUpload}>
        <Upload size={14} /> Unggah gambar
      </Button>
    </div>
  {/if}

  <div class="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-line pt-4">
    <div class="min-w-0">
      {#if selected}
        <p class="truncate text-xs font-semibold text-ink">{selected.filename || 'Gambar terpilih'}</p>
        <p class="text-[10px] text-ink-muted">
          {selected.source === 'evidence' ? selected.import_filename || 'Bukti dukung periode' : selected.period_label || 'Gambar desain'}
        </p>
      {:else}
        <p class="text-[11px] text-ink-muted">Pilih gambar untuk dipakai di kanvas.</p>
      {/if}
    </div>
    <div class="flex gap-2">
      <Button variant="ghost" onclick={onclose}>Batal</Button>
      <Button loading={busy} disabled={!selected} onclick={useSelected}>Gunakan gambar</Button>
    </div>
  </div>

  <input bind:this={fileInput} class="hidden" type="file" accept="image/png,image/jpeg" onchange={handleUpload} />
</Modal>
