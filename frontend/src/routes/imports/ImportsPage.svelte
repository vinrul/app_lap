<script lang="ts">
  import { onMount } from 'svelte'
  import {
    AlertCircle,
    CheckCircle2,
    CloudUpload,
    ExternalLink,
    FileSpreadsheet,
    History,
    Link2,
    MapPin,
    Rows3,
    Search,
    Upload,
    X,
  } from 'lucide-svelte'
  import { importsApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { ImportPreview, ImportView, RecordItem, RecordsResponse } from '../../lib/types'
  import { formatBytes, formatDate, formatDateTime, formatNumber } from '../../lib/utils/format'
  import { meta } from '../../lib/stores/meta.svelte'
  import { toast } from '../../lib/stores/toast.svelte'
  import PageHeader from '../../lib/components/ui/PageHeader.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import EmptyState from '../../lib/components/ui/EmptyState.svelte'
  import EvidenceList from '../../lib/components/ui/EvidenceList.svelte'
  import LocationMap from '../../lib/components/ui/LocationMap.svelte'
  import Modal from '../../lib/components/ui/Modal.svelte'

  let fileInput: HTMLInputElement
  let file = $state<File | null>(null)
  let preview = $state<ImportPreview | null>(null)
  let dragActive = $state(false)
  let checking = $state(false)
  let committing = $state(false)
  let error = $state('')
  let imports = $state<ImportView[]>([])
  let loading = $state(true)

  let recordsOpen = $state(false)
  let recordsLoading = $state(false)
  let recordsData = $state<RecordsResponse | null>(null)
  let selectedImport = $state<ImportView | null>(null)
  let selectedRecord = $state<RecordItem | null>(null)
  let recordsError = $state('')
  let retrying = $state<string | null>(null)

  const maxBytes = $derived(meta.data?.max_upload_bytes ?? 32 * 1024 * 1024)
  const totalRows = $derived(imports.reduce((total, item) => total + item.row_count, 0))
  const coverage = $derived.by(() => {
    if (!imports.length) return 'Belum ada data'
    const starts = imports.map((item) => item.date_start).filter(Boolean).sort()
    const ends = imports.map((item) => item.date_end).filter(Boolean).sort()
    if (!starts.length || !ends.length) return 'Rentang belum diketahui'
    return `${formatDate(starts[0])} – ${formatDate(ends[ends.length - 1])}`
  })

  onMount(load)

  async function load() {
    loading = true
    try {
      imports = await importsApi.list()
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  function choose(next: File | null | undefined) {
    if (!next) return
    if (next.size > maxBytes) {
      error = `Ukuran file melebihi batas ${formatBytes(maxBytes)}.`
      return
    }
    if (!next.name.toLowerCase().endsWith('.xlsx')) {
      error = 'Pilih workbook Excel berformat .xlsx.'
      return
    }
    file = next
    preview = null
    error = ''
  }

  function drop(event: DragEvent) {
    event.preventDefault()
    dragActive = false
    choose(event.dataTransfer?.files?.[0])
  }

  async function check() {
    if (!file) return
    checking = true
    error = ''
    try {
      preview = await importsApi.preview(file)
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      checking = false
    }
  }

  async function commit() {
    if (!file || !preview) return
    committing = true
    error = ''
    try {
      const saved = await importsApi.commit(file)
      toast.success(`${saved.row_count} baris dari ${saved.filename} berhasil diimpor.`)
      reset()
      await load()
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      committing = false
    }
  }

  function reset() {
    file = null
    preview = null
    error = ''
    if (fileInput) fileInput.value = ''
  }

  async function openRecords(item: ImportView) {
    selectedImport = item
    recordsOpen = true
    recordsLoading = true
    recordsError = ''
    recordsData = null
    try {
      recordsData = await importsApi.records(item.id, { page: 1, pageSize: 100 })
    } catch (caught) {
      recordsError = errorMessage(caught)
    } finally {
      recordsLoading = false
    }
  }

  function closeRecords() {
    recordsOpen = false
    selectedImport = null
    recordsData = null
    selectedRecord = null
  }

  async function retryEvidence(item: ImportView) {
    retrying = item.id
    try {
      const result = await importsApi.retryEvidence(item.id)
      if (result.queued) {
        toast.success(`${result.queued} tautan bukti dukung dijadwalkan ulang.`)
      } else {
        toast.show('Tidak ada tautan bukti dukung yang perlu diunduh ulang.', 'info')
      }
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      retrying = null
    }
  }
</script>

<svelte:head><title>Import data · Laporan Mingguan PBM</title></svelte:head>

<PageHeader
  eyebrow="PANGKALAN DATA"
  title="Import data"
  description="Unggah workbook Quick Wins di sini. Periode laporan akan mengambil data otomatis berdasarkan tanggal."
>
  {#snippet actions()}
    <Button onclick={() => fileInput?.click()}><CloudUpload size={16} /> Pilih workbook</Button>
  {/snippet}
</PageHeader>

{#if error}
  <div class="mb-5 flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{/if}

<div class="mb-5 grid gap-3 sm:grid-cols-3">
  <div class="card flex items-center gap-3 px-5 py-4">
    <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><FileSpreadsheet size={18} /></span>
    <div><p class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Berkas impor</p><strong class="font-display text-xl text-forest-900">{imports.length}</strong></div>
  </div>
  <div class="card flex items-center gap-3 px-5 py-4">
    <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-gold-100 text-gold-700"><Rows3 size={18} /></span>
    <div><p class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Baris tersimpan</p><strong class="font-display text-xl text-forest-900">{formatNumber(totalRows)}</strong></div>
  </div>
  <div class="card flex items-center gap-3 px-5 py-4">
    <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><History size={18} /></span>
    <div><p class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Cakupan data</p><strong class="text-sm font-semibold text-forest-900">{coverage}</strong></div>
  </div>
</div>

<div class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(280px,0.7fr)]">
  <section class="card p-5 sm:p-6">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <span class="eyebrow">UNGGAH WORKBOOK</span>
        <h2 class="mt-1 font-display text-xl font-semibold text-forest-900">Pilih file sumber</h2>
        <p class="mt-1 text-sm text-ink-muted">Sheet <code class="rounded bg-stone-100 px-1.5 py-0.5 text-xs text-forest-800">Data</code>. Wajib: Tanggal, Waktu kirim (WIB), Satuan, Tingkat, Polres, Polsek, dan Kode. Kolom lain opsional.</p>
      </div>
      <span class="badge badge-neutral">LANGKAH 01</span>
    </div>

    <input bind:this={fileInput} id="import-file" class="sr-only" type="file" accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      onchange={(event) => choose((event.currentTarget as HTMLInputElement).files?.[0])} />

    <label
      for="import-file"
      class="mt-5 flex cursor-pointer flex-col items-center justify-center rounded-2xl border-2 border-dashed px-6 py-10 text-center transition-colors {dragActive
        ? 'border-forest-500 bg-forest-50'
        : 'border-stone-300 bg-stone-50/50 hover:border-forest-300 hover:bg-forest-50/40'}"
      ondragover={(event) => { event.preventDefault(); dragActive = true }}
      ondragleave={() => (dragActive = false)}
      ondrop={drop}
    >
      <span class="flex h-12 w-12 items-center justify-center rounded-2xl bg-white text-forest-700 shadow-sm"><Upload size={22} /></span>
      <strong class="mt-3 text-sm text-ink">{file ? file.name : 'Tarik file Excel ke sini'}</strong>
      <span class="mt-1 text-xs text-ink-muted">
        {file ? `${formatBytes(file.size)} · siap diperiksa` : 'atau pilih file dari perangkat'}
      </span>
      <span class="btn btn-secondary btn-sm mt-4">Pilih workbook</span>
      <span class="mt-2 text-[10px] text-ink-muted">Format .xlsx · Maksimum {formatBytes(maxBytes)}</span>
    </label>

    <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
      <span class="text-[11px] text-ink-muted">File diproses lokal dan tidak dikirim ke layanan lain.</span>
      <div class="flex gap-2">
        {#if file}<Button variant="ghost" size="sm" onclick={reset}><X size={14} /> Hapus</Button>{/if}
        <Button disabled={!file} loading={checking} onclick={check}>
          {#if !checking}<Search size={15} />{/if} Periksa isi file
        </Button>
        {#if preview}
          <Button loading={committing} disabled={preview.errors.length > 0} onclick={commit}><CheckCircle2 size={15} /> Simpan import</Button>
        {/if}
      </div>
    </div>
  </section>

  <aside class="card p-5">
    <div class="flex items-center gap-2">
      <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-gold-100 text-gold-700"><FileSpreadsheet size={18} /></span>
      <div><span class="eyebrow">ATURAN DATA</span><h2 class="font-display text-base font-semibold text-forest-900">Deduplikasi & analisis</h2></div>
    </div>
    <ul class="mt-3 space-y-2 text-[11px] leading-relaxed text-ink-muted">
      <li>• Duplikat dikenali dari tanggal, waktu kirim, satuan, tingkat, polres, polsek, dan kode.</li>
      <li>• Polres dan polsek kosong dianggap kegiatan Polda/satker Polda.</li>
      <li>• Polsek kosong dianggap kegiatan tingkat polres/satker polres.</li>
      <li>• Jumlah personel, bibit, luas, jemaah, dan koordinat diekstrak otomatis.</li>
      <li>• Tautan bukti dukung disimpan pada baris sumber.</li>
    </ul>
  </aside>
</div>

{#if preview}
  <section class="card mt-5 overflow-hidden">
    <div class="panel-heading">
      <div>
        <span class="eyebrow">HASIL PEMERIKSAAN</span>
        <h2 class="panel-title mt-1">{preview.sheet} · {formatNumber(preview.valid_rows)} baris valid</h2>
        <p class="mt-1 text-xs text-ink-muted">
          {preview.file_date_start ? `${formatDate(preview.file_date_start)} – ${formatDate(preview.file_date_end)}` : 'Rentang tanggal tidak terdeteksi'}
        </p>
      </div>
      {#if preview.errors.length}
        <span class="badge badge-danger"><AlertCircle size={12} /> PERIKSA HEADER</span>
      {:else}
        <span class="badge badge-success"><CheckCircle2 size={12} /> SIAP DISIMPAN</span>
      {/if}
    </div>

    {#if preview.errors.length}
      <div class="border-b border-red-200 bg-red-50 px-5 py-4 text-red-800">
        <p class="text-[10px] font-bold tracking-[0.12em] uppercase">Kesalahan</p>
        <ul class="mt-1.5 space-y-1 text-[11px] leading-relaxed">
          {#each preview.errors as message}<li>• {message}</li>{/each}
        </ul>
      </div>
    {/if}

    <div class="grid gap-3 border-b border-line p-5 sm:grid-cols-2 lg:grid-cols-4">
      {#each [
        ['Baris valid', formatNumber(preview.valid_rows)],
        ['Duplikat dilewati', formatNumber(preview.duplicate_rows)],
        ['Baris bermasalah', formatNumber(preview.skipped_rows)],
        ['Tautan bukti', formatNumber(preview.evidence_links)],
      ] as [label, value]}
        <div class="rounded-xl border border-line bg-stone-50/60 px-4 py-3">
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{label}</p>
          <strong class="font-display text-xl text-forest-900">{value}</strong>
        </div>
      {/each}
    </div>

    <div class="grid gap-3 border-b border-line p-5 sm:grid-cols-2 lg:grid-cols-4">
      {#each [
        ['Personel', formatNumber(preview.personel_total)],
        ['Bibit ditanam', formatNumber(preview.bibit_total)],
        ['Luas area', `${formatNumber(preview.area_total)} m²`],
        ['Jemaah dijangkau', formatNumber(preview.jamaah_total)],
      ] as [label, value]}
        <div class="rounded-xl border border-line px-4 py-3">
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{label}</p>
          <strong class="font-display text-xl text-forest-900">{value}</strong>
        </div>
      {/each}
    </div>

    {#if preview.warnings.length}
      <div class="border-b border-line bg-gold-50/50 px-5 py-4">
        <p class="text-[10px] font-bold tracking-[0.12em] text-gold-700 uppercase">Catatan</p>
        <ul class="mt-1.5 space-y-1 text-[11px] leading-relaxed text-ink-soft">
          {#each preview.warnings.slice(0, 8) as warning}<li>• {warning}</li>{/each}
        </ul>
      </div>
    {/if}

    <div class="overflow-x-auto">
      <table class="table">
        <thead><tr><th>#</th><th>Tanggal</th><th>Kode</th><th>Satuan / unit</th><th>Kegiatan</th><th>Status</th></tr></thead>
        <tbody>
          {#each preview.preview_rows as row}
            <tr>
              <td class="text-xs text-ink-muted">{row.source_row}</td>
              <td class="text-xs whitespace-nowrap">{formatDate(row.tanggal)}</td>
              <td><span class="rounded-md bg-forest-50 px-2 py-1 text-[10px] font-bold text-forest-700">{row.kode || '—'}</span></td>
              <td class="max-w-[220px] truncate text-xs">{row.satuan || '—'}</td>
              <td class="max-w-md text-xs">{row.kegiatan || '—'}</td>
              <td class="text-xs whitespace-nowrap">{row.status_penerimaan || 'Belum diisi'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>
{/if}

<section class="card mt-5 overflow-hidden">
  <div class="panel-heading">
    <div>
      <span class="eyebrow">RIWAYAT IMPOR</span>
      <h2 class="panel-title mt-1">Berkas tersimpan</h2>
    </div>
  </div>
  {#if loading}
    <div class="space-y-3 p-5">{#each Array(4) as _}<div class="skeleton h-12 w-full"></div>{/each}</div>
  {:else if imports.length}
    <div class="overflow-x-auto">
      <table class="table">
        <thead><tr><th>Berkas</th><th>Cakupan tanggal</th><th class="text-right">Baris</th><th class="text-right">Duplikat</th><th>Diimpor</th><th class="text-right">Aksi</th></tr></thead>
        <tbody>
          {#each imports as item}
            <tr>
              <td>
                <strong class="block text-xs font-semibold text-ink">{item.filename}</strong>
                <small class="text-[10px] text-ink-muted">{item.imported_by} · parser v{item.parser_version}</small>
              </td>
              <td class="text-xs whitespace-nowrap text-ink-muted">
                {item.date_start ? `${formatDate(item.date_start)} – ${formatDate(item.date_end)}` : '—'}
              </td>
              <td class="text-right text-xs font-semibold text-ink-soft">{formatNumber(item.row_count)}</td>
              <td class="text-right text-xs text-ink-muted">{formatNumber(item.duplicate_rows)}</td>
              <td class="text-xs whitespace-nowrap text-ink-muted">{formatDateTime(item.created_at)}</td>
              <td class="text-right">
                <div class="flex items-center justify-end gap-1">
                  <button class="btn btn-ghost btn-sm" onclick={() => openRecords(item)}>Lihat baris</button>
                  <button class="btn btn-ghost btn-sm" disabled={retrying === item.id} onclick={() => retryEvidence(item)}>
                    {retrying === item.id ? 'Menjadwalkan…' : 'Unduh bukti'}
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <EmptyState title="Belum ada import" description="Unggah workbook Excel pertama untuk mulai mengisi pangkalan data." />
  {/if}
</section>

<Modal open={recordsOpen} onclose={closeRecords} title={selectedImport?.filename || 'Baris impor'}
  description={selectedImport ? `${formatNumber(selectedImport.row_count)} baris · ${selectedImport.date_start ? `${formatDate(selectedImport.date_start)} – ${formatDate(selectedImport.date_end)}` : 'tanpa tanggal'}` : ''}
  width="max-w-5xl">
  {#if recordsLoading}
    <div class="space-y-3">{#each Array(6) as _}<div class="skeleton h-10 w-full"></div>{/each}</div>
  {:else if recordsError}
    <div class="flex items-center gap-2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800"><AlertCircle size={17} /> {recordsError}</div>
  {:else if recordsData}
    <div class="max-h-[60vh] overflow-auto rounded-xl border border-line">
      <table class="table">
        <thead><tr><th>Tanggal</th><th>Kode</th><th>Unit</th><th>Kegiatan</th><th>Analisis</th><th></th></tr></thead>
        <tbody>
          {#each recordsData.records as row}
            <tr>
              <td class="text-xs whitespace-nowrap">{formatDate(row.tanggal)}</td>
              <td><span class="rounded-md bg-forest-50 px-2 py-1 text-[10px] font-bold whitespace-nowrap text-forest-700">{row.kode || '—'}</span></td>
              <td class="max-w-[200px] truncate text-xs">{row.unit || row.satuan || '—'}</td>
              <td class="max-w-xs text-xs">{row.kegiatan || '—'}</td>
              <td class="text-[10px] whitespace-nowrap text-ink-muted">
                {row.personel_count ? `${row.personel_count} personel · ` : ''}
                {row.bibit_count ? `${row.bibit_count} bibit · ` : ''}
                {row.area_m2 ? `${row.area_m2} m² · ` : ''}
                {row.jamaah_count ? `${row.jamaah_count} jemaah` : ''}
                {row.latitude !== null && row.longitude !== null ? ' · koordinat ✓' : ''}
              </td>
              <td class="text-right"><button class="btn btn-ghost btn-sm" onclick={() => (selectedRecord = row)}>Detail</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <p class="mt-2 text-[10px] text-ink-muted">Menampilkan maksimal 100 baris pertama.</p>
  {/if}
</Modal>

<Modal open={selectedRecord !== null} onclose={() => (selectedRecord = null)} title={selectedRecord?.kegiatan || 'Detail baris'}
  description={selectedRecord ? `${selectedRecord.kode} · ${selectedRecord.unit || selectedRecord.satuan}` : ''} width="max-w-3xl">
  {#if selectedRecord}
    <div class="space-y-4 text-xs">
      <div class="grid gap-2 sm:grid-cols-2">
        <p><span class="text-ink-muted">Tanggal:</span> {formatDate(selectedRecord.tanggal)} {selectedRecord.waktu_kirim_wib}</p>
        <p><span class="text-ink-muted">Tingkat:</span> {selectedRecord.level || selectedRecord.tingkat || '—'}</p>
        <p><span class="text-ink-muted">Polda:</span> {selectedRecord.polda || '—'}</p>
        <p><span class="text-ink-muted">Polres:</span> {selectedRecord.polres || '—'}</p>
        <p><span class="text-ink-muted">Polsek:</span> {selectedRecord.polsek || '—'}</p>
        <p><span class="text-ink-muted">Kode:</span> {selectedRecord.kode} {selectedRecord.qw_title ? `· ${selectedRecord.qw_title}` : ''}</p>
      </div>
      <div class="grid gap-2 sm:grid-cols-4">
        {#each [
          ['Personel', formatNumber(selectedRecord.personel_count)],
          ['Bibit', formatNumber(selectedRecord.bibit_count)],
          ['Luas', `${formatNumber(selectedRecord.area_m2)} m²`],
          ['Jemaah', formatNumber(selectedRecord.jamaah_count)],
        ] as [label, value]}
          <div class="rounded-lg border border-line bg-stone-50 px-3 py-2"><p class="text-[9px] font-bold tracking-[0.1em] text-ink-muted uppercase">{label}</p><strong class="text-sm text-forest-900">{value}</strong></div>
        {/each}
      </div>
      {#if selectedRecord.latitude !== null && selectedRecord.longitude !== null}
        <div>
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Lokasi kegiatan</p>
          <div class="mt-1.5">
            <LocationMap
              latitude={selectedRecord.latitude}
              longitude={selectedRecord.longitude}
              label={selectedRecord.kegiatan || 'Lokasi kegiatan'}
              detail={[selectedRecord.kode, selectedRecord.satuan].filter(Boolean).join(' · ')}
            />
          </div>
          {#if selectedRecord.map_url}
            <a class="mt-2 inline-flex items-center gap-1.5 text-[11px] font-medium text-forest-700 hover:underline" href={selectedRecord.map_url} target="_blank" rel="noopener noreferrer">
              <MapPin size={13} /> Buka tautan peta dari laporan
            </a>
          {/if}
        </div>
      {:else if selectedRecord.map_url}
        <a class="inline-flex items-center gap-1.5 text-[11px] font-medium text-forest-700 hover:underline" href={selectedRecord.map_url} target="_blank" rel="noopener noreferrer">
          <MapPin size={13} /> {selectedRecord.map_url}
        </a>
      {/if}
      <div>
        <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Isian laporan</p>
        <pre class="mt-1.5 max-h-64 overflow-auto rounded-xl border border-line bg-stone-50 p-3 text-[11px] leading-relaxed whitespace-pre-wrap">{selectedRecord.isian_laporan || '—'}</pre>
      </div>
      {#if selectedRecord.links.length}
        <div>
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Tautan bukti</p>
          <div class="mt-1.5 space-y-1">
            {#each selectedRecord.links as link}
              <a class="flex items-center gap-1.5 truncate text-[11px] text-forest-700 hover:underline" href={link} target="_blank" rel="noopener noreferrer"><Link2 size={12} /> {link}</a>
            {/each}
          </div>
        </div>
      {/if}
      {#if selectedRecord.evidence.length}
        <div>
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Bukti terunduh</p>
          <div class="mt-1.5">
            <EvidenceList items={selectedRecord.evidence} />
          </div>
        </div>
      {/if}
    </div>
  {/if}
</Modal>
