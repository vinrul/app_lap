<script lang="ts">
  import {
    AlertCircle,
    AlertTriangle,
    Check,
    CheckCircle2,
    CloudUpload,
    FileSpreadsheet,
    History,
    LockKeyhole,
    Search,
    Upload,
    X,
  } from 'lucide-svelte'
  import { importsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type { ImportPreview, ImportView, PeriodDetail } from '../../types'
  import { formatBytes, formatDateTime, formatNumber } from '../../utils/format'
  import { meta } from '../../stores/meta.svelte'
  import { toast } from '../../stores/toast.svelte'
  import Button from '../ui/Button.svelte'

  type Props = {
    detail: PeriodDetail
    onimported: (importId: string) => Promise<void>
  }

  let { detail, onimported }: Props = $props()

  let fileInput: HTMLInputElement
  let file = $state<File | null>(null)
  let preview = $state<ImportPreview | null>(null)
  let dragActive = $state(false)
  let checking = $state(false)
  let committing = $state(false)
  let error = $state('')

  const maxBytes = $derived(meta.data?.max_upload_bytes ?? 32 * 1024 * 1024)

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
      preview = await importsApi.preview(detail.period.id, file)
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
      const saved: ImportView = await importsApi.commit(detail.period.id, file)
      await onimported(saved.id)
      file = null
      preview = null
      if (fileInput) fileInput.value = ''
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
</script>

<div class="grid gap-4 lg:grid-cols-[minmax(0,1.5fr)_minmax(280px,0.7fr)]">
  <section class="card p-5 sm:p-6">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <span class="eyebrow">UNGGAH WORKBOOK</span>
        <h2 class="mt-1 font-display text-xl font-semibold text-forest-900">Pilih file sumber</h2>
        <p class="mt-1 text-sm text-ink-muted">Sheet <code class="rounded bg-stone-100 px-1.5 py-0.5 text-xs text-forest-800">Data</code> dengan 18 kolom sesuai template.</p>
      </div>
      <span class="badge badge-neutral">LANGKAH 01</span>
    </div>

    <input
      bind:this={fileInput}
      id="excel-file"
      class="sr-only"
      type="file"
      accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      onchange={(event) => choose((event.currentTarget as HTMLInputElement).files?.[0])}
    />

    <label
      for="excel-file"
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

    {#if error}
      <div class="mt-4 flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800" role="alert">
        <AlertCircle size={17} class="mt-0.5 shrink-0" /> <span>{error}</span>
      </div>
    {/if}

    <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
      <span class="flex items-center gap-1.5 text-[11px] text-ink-muted"><LockKeyhole size={13} /> File diproses lokal dan tidak diunggah ke internet.</span>
      <div class="flex gap-2">
        {#if file}
          <Button variant="ghost" size="sm" onclick={reset}><X size={14} /> Hapus</Button>
        {/if}
        <Button disabled={!file} loading={checking} onclick={check}>
          {#if !checking}<Search size={15} />{/if} Periksa isi file
        </Button>
      </div>
    </div>
  </section>

  <aside class="card p-5">
    <div class="flex items-center gap-2">
      <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-gold-100 text-gold-700"><FileSpreadsheet size={18} /></span>
      <div><span class="eyebrow">PEMETAAN DATA</span><h2 class="font-display text-base font-semibold text-forest-900">Kolom dikenali</h2></div>
    </div>
    <p class="mt-3 text-xs leading-relaxed text-ink-muted">Header dicocokkan berdasarkan nama, sehingga urutan kolom Excel tidak menjadi masalah.</p>
    <div class="mt-4 flex items-baseline gap-2">
      <strong class="font-display text-4xl text-forest-900">18</strong>
      <span class="text-xs text-ink-muted">kolom sumber</span>
    </div>
    <div class="mt-4 flex flex-wrap gap-1.5">
      {#each ['Tanggal', 'Satuan', 'Kode', 'Quick Wins', 'Status', 'Isian laporan'] as tag}
        <span class="rounded-md bg-stone-100 px-2 py-1 text-[10px] font-medium text-ink-soft">{tag}</span>
      {/each}
      <span class="rounded-md bg-stone-100 px-2 py-1 text-[10px] font-medium text-ink-muted">+12 lainnya</span>
    </div>
    <div class="mt-5 rounded-xl border border-line bg-stone-50 px-3.5 py-3">
      <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Versi tersimpan</p>
      <p class="mt-1 text-sm font-semibold text-forest-900">{detail.imports.length} versi impor</p>
      <p class="mt-1 text-[11px] leading-relaxed text-ink-muted">Unggahan baru tidak menghapus versi lama. Pilih versi data pada tab ringkasan.</p>
    </div>
  </aside>
</div>

{#if preview}
  <section class="card mt-4 overflow-hidden">
    <div class="panel-heading">
      <div>
        <span class="eyebrow">HASIL PEMERIKSAAN</span>
        <h2 class="panel-title mt-1">Pratinjau impor</h2>
      </div>
      <span class="badge badge-progress">LANGKAH 02</span>
    </div>

    <div class="grid gap-3 border-b border-line px-5 py-4 sm:grid-cols-2 lg:grid-cols-5">
      <div><p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Baris terbaca</p><strong class="font-display text-2xl text-forest-900">{formatNumber(preview.total_rows)}</strong></div>
      <div><p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Tanggal valid</p><strong class="font-display text-2xl text-forest-900">{formatNumber(preview.valid_rows)}</strong></div>
      <div><p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Dilewati</p><strong class="font-display text-2xl text-ink-soft">{formatNumber(preview.skipped_rows)}</strong></div>
      <div><p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Duplikat</p><strong class="font-display text-2xl text-ink-soft">{formatNumber(preview.duplicate_rows)}</strong></div>
      <div><p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Di luar periode</p><strong class="font-display text-2xl {preview.unmatched_dates ? 'text-gold-700' : 'text-forest-900'}">{formatNumber(preview.unmatched_dates)}</strong></div>
    </div>

    {#if preview.errors.length}
      <div class="m-5 flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
        <AlertCircle size={17} class="mt-0.5 shrink-0" />
        <div><strong>Header perlu diperiksa</strong><p class="mt-1 text-xs">{preview.errors.join(' ')}</p></div>
      </div>
    {/if}

    {#if preview.warnings.length}
      <div class="m-5 flex items-start gap-2.5 rounded-xl border border-gold-200 bg-gold-100/60 px-4 py-3 text-sm text-gold-800">
        <AlertTriangle size={17} class="mt-0.5 shrink-0" />
        <div>
          <strong>Perhatian sebelum menyimpan</strong>
          <ul class="mt-1 list-disc space-y-0.5 pl-4 text-xs">
            {#each preview.warnings.slice(0, 8) as warning}<li>{warning}</li>{/each}
          </ul>
          {#if preview.warnings.length > 8}<p class="mt-1 text-[11px]">… dan {preview.warnings.length - 8} peringatan lain.</p>{/if}
        </div>
      </div>
    {/if}

    <div class="grid gap-4 border-b border-line px-5 py-4 lg:grid-cols-3">
      <div>
        <span class="eyebrow">RENTANG FILE</span>
        <p class="mt-1 text-sm font-semibold text-forest-900">
          {preview.file_date_start && preview.file_date_end ? `${preview.file_date_start} → ${preview.file_date_end}` : 'Tidak terdeteksi'}
        </p>
      </div>
      <div class="lg:col-span-2">
        <span class="eyebrow">QUICK WINS TERBACA</span>
        <div class="mt-2 flex flex-wrap gap-1.5">
          {#each preview.quick_wins as item}
            <span class="badge badge-neutral">{item.label} · {formatNumber(item.count)}</span>
          {/each}
        </div>
      </div>
    </div>

    <div class="overflow-x-auto">
      <table class="table">
        <thead><tr><th>Baris</th><th>Tanggal</th><th>Satuan</th><th>Kode</th><th>Kegiatan</th><th>Status</th></tr></thead>
        <tbody>
          {#each preview.preview_rows as row}
            <tr>
              <td class="text-xs text-ink-muted">#{row.source_row}</td>
              <td class="text-xs whitespace-nowrap">{row.tanggal}</td>
              <td class="text-xs">{row.satuan || '—'}</td>
              <td><span class="rounded-md bg-forest-50 px-2 py-1 text-[10px] font-bold text-forest-700">{row.kode || '—'}</span></td>
              <td class="max-w-md text-xs">{row.kegiatan || '—'}</td>
              <td class="text-xs whitespace-nowrap">{row.status_penerimaan || '—'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="flex flex-wrap items-center justify-between gap-3 border-t border-line bg-stone-50/60 px-5 py-4">
      <span class="text-xs text-ink-muted">
        {preview.date_counts.length} tanggal terdeteksi · {formatNumber(preview.valid_rows - preview.unmatched_dates)} baris dalam rentang periode
      </span>
      <div class="flex gap-2">
        <Button variant="ghost" onclick={reset}>Batalkan</Button>
        <Button
          disabled={preview.errors.length > 0 || preview.valid_rows === 0 || preview.valid_rows === preview.unmatched_dates}
          loading={committing}
          onclick={commit}
        >
          <Check size={16} /> Simpan sebagai versi impor
        </Button>
      </div>
    </div>
  </section>
{/if}

<section class="card mt-4 overflow-hidden">
  <div class="panel-heading">
    <div class="flex items-center gap-2"><History size={17} class="text-ink-muted" /><div><span class="eyebrow">RIWAYAT</span><h2 class="panel-title mt-1">Versi impor</h2></div></div>
    <span class="badge badge-neutral">{detail.imports.length} versi</span>
  </div>
  {#if detail.imports.length}
    <div class="divide-y divide-stone-100">
      {#each detail.imports as item, index}
        <div class="flex flex-wrap items-center gap-3 px-5 py-3.5">
          <span class="flex h-9 w-9 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><FileSpreadsheet size={17} /></span>
          <div class="min-w-0 flex-1">
            <p class="truncate text-xs font-semibold text-ink">{item.filename}</p>
            <p class="mt-0.5 text-[10px] text-ink-muted">{formatNumber(item.row_count)} baris · {formatDateTime(item.created_at)} · {item.imported_by}</p>
          </div>
          {#if index === 0}
            <span class="badge badge-success"><CheckCircle2 size={12} /> TERBARU</span>
          {:else}
            <span class="badge badge-neutral">Versi {detail.imports.length - index}</span>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <div class="px-5 py-8 text-center text-sm text-ink-muted">Belum ada versi impor untuk periode ini.</div>
  {/if}
</section>
