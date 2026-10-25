<script lang="ts">
  import { untrack } from 'svelte'
  import {
    AlertCircle,
    ChevronLeft,
    ChevronRight,
    ExternalLink,
    Filter,
    Search,
    SlidersHorizontal,
  } from 'lucide-svelte'
  import { recordsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type { RecordItem, RecordsResponse } from '../../types'
  import { formatDate, formatNumber } from '../../utils/format'
  import Button from '../ui/Button.svelte'
  import EvidenceList from '../ui/EvidenceList.svelte'
  import Modal from '../ui/Modal.svelte'

  type Props = {
    periodId: string
  }

  let { periodId }: Props = $props()

  let data = $state<RecordsResponse | null>(null)
  let loading = $state(true)
  let error = $state('')
  let search = $state('')
  let quickWins = $state('')
  let status = $state('')
  let polres = $state('')
  let page = $state(1)
  let pageSize = $state(25)
  let selected = $state<RecordItem | null>(null)
  let lastKey = ''

  $effect(() => {
    const key = periodId
    if (key === lastKey) return
    lastKey = key
    untrack(() => {
      page = 1
      void load()
    })
  })

  async function load() {
    loading = true
    error = ''
    try {
      data = await recordsApi.list(periodId, {
        search,
        quickWins,
        status,
        polres,
        page,
        pageSize,
      })
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  function submitSearch(event: SubmitEvent) {
    event.preventDefault()
    page = 1
    void load()
  }

  function applyFilter() {
    page = 1
    void load()
  }

  function resetFilters() {
    search = ''
    quickWins = ''
    status = ''
    polres = ''
    page = 1
    void load()
  }

  function go(next: number) {
    page = next
    void load()
  }

  const totalPages = $derived(Math.max(1, Math.ceil((data?.total ?? 0) / pageSize)))
  const hasFilters = $derived(Boolean(search || quickWins || status || polres))
</script>

<section class="card overflow-hidden">
  <div class="panel-heading">
    <div>
      <span class="eyebrow">BARIS SUMBER</span>
      <h2 class="panel-title mt-1">Data periode</h2>
      <p class="mt-1 text-xs text-ink-muted">Klik satu baris untuk melihat isian laporan lengkap dan tautan bukti.</p>
    </div>
    <div class="flex items-center gap-2">
      <span class="badge badge-neutral">{formatNumber(data?.total ?? 0)} baris</span>
      {#if data}<span class="text-[10px] text-ink-muted">{data.source_count} berkas · {data.duplicate_rows} duplikat</span>{/if}
    </div>
  </div>

  <form class="flex flex-wrap items-end gap-2 border-b border-line px-5 py-4" onsubmit={submitSearch}>
    <label class="relative min-w-52 flex-1">
      <Search size={15} class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-ink-muted" />
      <input class="input py-2 pl-9 text-xs" bind:value={search} placeholder="Cari satuan, kegiatan, kode, atau isian…" />
    </label>
    <select class="input w-auto py-2 text-xs" bind:value={quickWins} onchange={applyFilter} aria-label="Filter Quick Wins">
      <option value="">Semua Quick Wins</option>
      {#each data?.quick_wins ?? [] as item}<option value={item.label}>{item.label} · {item.count}</option>{/each}
    </select>
    <select class="input w-auto py-2 text-xs" bind:value={status} onchange={applyFilter} aria-label="Filter status">
      <option value="">Semua status</option>
      {#each data?.statuses ?? [] as item}<option value={item.label}>{item.label} · {item.count}</option>{/each}
    </select>
    <select class="input w-auto py-2 text-xs" bind:value={polres} onchange={applyFilter} aria-label="Filter polres">
      <option value="">Semua polres</option>
      {#each data?.polres ?? [] as item}<option value={item.label}>{item.label} · {item.count}</option>{/each}
    </select>
    <Button type="submit" size="sm" variant="secondary"><Filter size={14} /> Terapkan</Button>
    {#if hasFilters}
      <Button size="sm" variant="ghost" onclick={resetFilters}>Reset</Button>
    {/if}
  </form>

  {#if error}
    <div class="m-5 flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertCircle size={17} /> {error}
    </div>
  {/if}

  {#if loading}
    <div class="space-y-3 p-5">{#each Array(8) as _}<div class="skeleton h-10 w-full"></div>{/each}</div>
  {:else if data?.records.length}
    <div class="overflow-x-auto">
      <table class="table">
        <thead>
          <tr><th>Tanggal</th><th>Satuan / wilayah</th><th>Kode</th><th>Kegiatan</th><th>Status</th><th></th></tr>
        </thead>
        <tbody>
          {#each data.records as row}
            <tr class="cursor-pointer" onclick={() => (selected = row)}>
              <td class="text-xs whitespace-nowrap">{formatDate(row.tanggal)}</td>
              <td>
                <strong class="block text-xs font-semibold text-ink">{row.unit || row.satuan || row.polsek || row.polres || '—'}</strong>
                <small class="block text-[10px] text-ink-muted">{row.level || row.tingkat}{row.polres && row.unit !== row.polres ? ` · ${row.polres}` : ''}</small>
              </td>
              <td><span class="rounded-md bg-forest-50 px-2 py-1 text-[10px] font-bold whitespace-nowrap text-forest-700">{row.kode || '—'}</span></td>
              <td class="max-w-md text-xs">{row.kegiatan || '—'}</td>
              <td class="text-xs whitespace-nowrap">{row.status_penerimaan || 'Belum diisi'}</td>
              <td class="text-right"><button class="btn btn-ghost btn-sm" onclick={(event) => { event.stopPropagation(); selected = row }}>Detail</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="flex flex-wrap items-center justify-between gap-3 border-t border-line px-5 py-3.5">
      <span class="text-xs text-ink-muted">
        Menampilkan {formatNumber((page - 1) * pageSize + 1)}–{formatNumber(Math.min(page * pageSize, data.total))} dari {formatNumber(data.total)} baris
      </span>
      <div class="flex items-center gap-2">
        <select class="input w-auto py-1.5 text-[11px]" bind:value={pageSize} onchange={() => { page = 1; void load() }} aria-label="Baris per halaman">
          <option value={10}>10 / halaman</option>
          <option value={25}>25 / halaman</option>
          <option value={50}>50 / halaman</option>
          <option value={100}>100 / halaman</option>
        </select>
        <button class="btn-icon" aria-label="Halaman sebelumnya" disabled={page <= 1} onclick={() => go(page - 1)}><ChevronLeft size={17} /></button>
        <span class="text-xs font-semibold text-ink-soft">{page} / {totalPages}</span>
        <button class="btn-icon" aria-label="Halaman berikutnya" disabled={page >= totalPages} onclick={() => go(page + 1)}><ChevronRight size={17} /></button>
      </div>
    </div>
  {:else}
    <div class="px-5 py-10 text-center">
      <SlidersHorizontal size={22} class="mx-auto text-ink-muted" />
      <p class="mt-2 text-sm font-semibold text-ink">Tidak ada baris yang cocok</p>
      <p class="mt-1 text-xs text-ink-muted">Ubah kata kunci atau reset filter untuk melihat seluruh data.</p>
      {#if hasFilters}<Button class="mt-4" size="sm" variant="secondary" onclick={resetFilters}>Reset filter</Button>{/if}
    </div>
  {/if}
</section>

<Modal
  open={selected !== null}
  onclose={() => (selected = null)}
  title={selected?.kegiatan || 'Detail baris sumber'}
  description={selected ? `${selected.satuan} · Baris #${selected.source_row}` : ''}
  width="max-w-3xl"
>
  {#if selected}
    <div class="space-y-5">
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {#each [
          ['Tanggal', selected.tanggal],
          ['Kode', selected.kode],
          ['Quick Wins', selected.quick_wins],
          ['Status', selected.status_penerimaan || 'Belum diisi'],
          ['Tingkat', selected.level || selected.tingkat],
          ['Unit', selected.unit || selected.satuan],
          ['Polres', selected.polres],
          ['Polsek', selected.polsek],
          ['Pelapor', selected.pelapor],
          ['Sumber', selected.import_filename],
        ] as [label, value]}
          <div class="rounded-lg border border-line bg-stone-50/60 px-3 py-2">
            <p class="text-[9px] font-bold tracking-[0.12em] text-ink-muted uppercase">{label}</p>
            <p class="mt-0.5 text-xs font-medium break-words text-ink">{value || '—'}</p>
          </div>
        {/each}
      </div>

      <div>
        <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Pejabat & pelaksana</h3>
        <div class="mt-2 grid gap-2 text-xs sm:grid-cols-2">
          <p><span class="text-ink-muted">Pejabat:</span> {selected.pejabat_pelaksana || '—'}</p>
          <p><span class="text-ink-muted">Jabatan:</span> {selected.jabatan || '—'}</p>
          <p><span class="text-ink-muted">Pelaksana:</span> {selected.pelaksana || '—'}</p>
          <p><span class="text-ink-muted">Waktu kirim:</span> {selected.waktu_kirim_wib || '—'}</p>
        </div>
      </div>

      <div>
        <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Analisis otomatis</h3>
        <div class="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
          {#each [
            ['Personel', formatNumber(selected.personel_count)],
            ['Bibit', formatNumber(selected.bibit_count)],
            ['Luas area', `${formatNumber(selected.area_m2)} m²`],
            ['Jemaah', formatNumber(selected.jamaah_count)],
          ] as [label, value]}
            <div class="rounded-lg border border-line bg-stone-50/60 px-3 py-2">
              <p class="text-[9px] font-bold tracking-[0.12em] text-ink-muted uppercase">{label}</p>
              <p class="mt-0.5 text-xs font-semibold text-forest-900">{value}</p>
            </div>
          {/each}
        </div>
        {#if selected.latitude !== null && selected.longitude !== null}
          <p class="mt-2 text-xs text-ink-soft">Koordinat: {selected.latitude}, {selected.longitude}</p>
        {/if}
        {#if selected.map_url}
          <a class="mt-1 inline-flex items-center gap-1.5 text-[11px] font-medium text-forest-700 hover:underline" href={selected.map_url} target="_blank" rel="noopener noreferrer">
            <ExternalLink size={12} /> Buka tautan peta
          </a>
        {/if}
      </div>

      {#if Object.keys(selected.fields).length}
        <div>
          <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Isian terstruktur</h3>
          <div class="mt-2 max-h-72 overflow-auto rounded-xl border border-line">
            <table class="table text-xs">
              <tbody>
                {#each Object.entries(selected.fields) as [key, value]}
                  <tr><td class="w-2/5 font-medium text-ink-soft">{key}</td><td class="whitespace-pre-wrap text-ink">{value || '—'}</td></tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <div>
        <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Isian laporan asli</h3>
        <pre class="mt-2 max-h-72 overflow-auto rounded-xl border border-line bg-stone-50 p-4 text-[11px] leading-relaxed whitespace-pre-wrap text-ink-soft">{selected.isian_laporan || 'Tidak ada isian.'}</pre>
      </div>

      {#if selected.links.length}
        <div>
          <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Tautan</h3>
          <div class="mt-2 space-y-1.5">
            {#each selected.links as link}
              <a class="flex items-center gap-1.5 truncate text-[11px] font-medium text-forest-700 hover:underline" href={link} target="_blank" rel="noopener noreferrer">
                <ExternalLink size={12} class="shrink-0" /> {link}
              </a>
            {/each}
          </div>
        </div>
      {/if}

      {#if selected.evidence.length}
        <div>
          <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Bukti terunduh</h3>
          <div class="mt-2">
            <EvidenceList items={selected.evidence} />
          </div>
        </div>
      {/if}

      <div>
        <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Keluhan personel</h3>
        <p class="mt-1.5 text-xs whitespace-pre-wrap text-ink-soft">{selected.keluhan_personel_polri || 'Tidak ada.'}</p>
      </div>
    </div>
  {/if}
</Modal>
