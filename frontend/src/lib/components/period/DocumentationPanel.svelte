<script lang="ts">
  import { onMount } from 'svelte'
  import { FileSearch } from 'lucide-svelte'
  import { documentationApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type {
    DocumentationImage,
    DocumentationResponse,
    RealtimeServerMessage,
  } from '../../types'
  import { realtime } from '../../realtime/socket'
  import { toast } from '../../stores/toast.svelte'
  import { dialog } from '../../stores/dialog.svelte'
  import { formatDate, formatNumber } from '../../utils/format'
  import Button from '../ui/Button.svelte'

  type Props = { periodId: string }

  let { periodId }: Props = $props()

  let data = $state<DocumentationResponse | null>(null)
  let loading = $state(true)
  let scanning = $state(false)
  let done = $state(0)
  let total = $state(0)
  let filter = $state('')
  let expandedRow = $state<number | null>(null)
  let details = $state<Record<number, DocumentationImage[]>>({})
  let detailLoading = $state<number | null>(null)

  const job = $derived(`docs:${periodId}`)

  onMount(() => {
    const off = realtime.onMessage(handle)
    realtime.connect()
    void load()
    return off
  })

  async function load(): Promise<void> {
    loading = true
    try {
      data = await documentationApi.list(periodId)
      scanning = data.scanning
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      loading = false
    }
  }

  function handle(message: RealtimeServerMessage): void {
    if (message.type === 'job_state' && message.job === job) {
      scanning = true
      total = message.total
      done = message.ready
      return
    }
    if (message.type === 'job_finished' && message.job === job) {
      scanning = false
      total = message.total
      done = message.ready
      expandedRow = null
      details = {}
      void load()
    }
  }

  async function toggleRow(rowId: number): Promise<void> {
    if (expandedRow === rowId) {
      expandedRow = null
      return
    }
    expandedRow = rowId
    if (details[rowId]) {
      return
    }
    detailLoading = rowId
    try {
      details[rowId] = await documentationApi.images(periodId, rowId)
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      detailLoading = null
    }
  }

  async function scan(force = false): Promise<void> {
    try {
      const result = force
        ? await documentationApi.rescan(periodId)
        : await documentationApi.scan(periodId)
      if (result.started) {
        toast.success(force ? 'Pindai ulang dimulai.' : 'Analisis dokumentasi dimulai.')
      } else {
        toast.show('Analisis sedang berjalan.', 'info')
      }
      scanning = true
      realtime.send({ type: 'watch_job', job })
    } catch (caught) {
      toast.error(errorMessage(caught))
    }
  }

  async function rescan(): Promise<void> {
    const confirmed = await dialog.confirm({
      title: 'Pindai ulang semua gambar?',
      message:
        'Seluruh gambar yang sudah terunduh pada periode ini akan dianalisis ulang dengan aturan OCR terbaru, termasuk yang sebelumnya valid. Progres dapat dibatalkan.',
      confirmLabel: 'Pindai ulang',
    })
    if (!confirmed) return
    await scan(true)
  }

  async function cancel(): Promise<void> {
    try {
      await documentationApi.cancel(periodId)
      toast.show('Permintaan pembatalan dikirim.', 'info')
    } catch (caught) {
      toast.error(errorMessage(caught))
    }
  }

  function statusLabel(status: string): string {
    switch (status) {
      case 'valid':
        return 'Valid sesuai'
      case 'manual':
        return 'Cek manual'
      case 'mabes':
        return 'Valid, cek ke mabes'
      case 'invalid':
        return 'Tidak valid'
      case 'pending':
        return 'Belum dianalisis'
      case 'ready':
        return 'Siap dianalisis'
      case 'failed':
        return 'Gagal diunduh'
      case 'skipped':
        return 'Dilewati'
      default:
        return 'Belum dianalisis'
    }
  }

  function statusClass(status: string): string {
    const base = 'inline-flex items-center rounded-full border px-1.5 py-0.5 text-[9px] font-semibold'
    switch (status) {
      case 'valid':
        return `${base} border-forest-200 bg-forest-50 text-forest-700`
      case 'manual':
        return `${base} border-gold-300 bg-gold-50 text-gold-700`
      case 'mabes':
        return `${base} border-line bg-stone-50 text-ink-muted`
      case 'invalid':
        return `${base} border-red-200 bg-red-50 text-red-700`
      case 'failed':
        return `${base} border-red-200 bg-red-50 text-red-700`
      default:
        return `${base} border-line bg-white text-ink-muted`
    }
  }

  const rows = $derived((data?.rows ?? []).filter((row) => !filter || row.status === filter))
  const percent = $derived(total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0)
</script>

<section class="card mb-4 flex flex-wrap items-center justify-between gap-3 px-5 py-4">
  <div class="max-w-2xl">
    <span class="eyebrow">ANALISIS DOKUMENTASI</span>
    <p class="mt-1 text-[11px] text-ink-muted">
      Memeriksa watermark (alamat wajib, tanggal dan jam, koordinat opsional) serta EXIF pada
      bukti dukung gambar, lalu membandingkannya dengan tanggal dan lokasi pada baris laporan.
    </p>
  </div>
  <div class="flex items-center gap-2">
    {#if scanning}
      <Button variant="secondary" size="sm" onclick={cancel}>Batalkan</Button>
    {:else}
      <Button variant="secondary" size="sm" onclick={rescan}>Pindai ulang</Button>
      <Button size="sm" onclick={() => void scan()}><FileSearch size={14} /> Pindai sekarang</Button>
    {/if}
  </div>
</section>

{#if scanning}
  <div class="card mb-4 px-5 py-4">
    <div class="flex items-center justify-between text-[11px] text-ink-muted">
      <span>Memindai {formatNumber(done)}/{formatNumber(total)} gambar…</span>
      <span>{percent}%</span>
    </div>
    <div class="mt-2 h-2 overflow-hidden rounded-full bg-stone-200">
      <div class="h-full rounded-full bg-forest-600 transition-all" style={`width:${percent}%`}></div>
    </div>
  </div>
{/if}

{#if data}
  <div class="mb-4 grid gap-3 sm:grid-cols-3 lg:grid-cols-5">
    {#each ([
      ['Valid sesuai', data.summary.valid, 'text-forest-700'],
      ['Cek manual', data.summary.manual, 'text-gold-700'],
      ['Valid, cek mabes', data.summary.mabes, 'text-ink-soft'],
      ['Tidak valid', data.summary.invalid, 'text-red-700'],
      ['Belum dianalisis', data.summary.pending, 'text-ink-muted'],
    ] as [string, number, string][]) as [label, value, tone]}
      <div class="card px-4 py-3">
        <p class="text-[9px] font-bold tracking-[0.12em] text-ink-muted uppercase">{label}</p>
        <p class="mt-1 text-xl font-semibold {tone}">{formatNumber(value)}</p>
      </div>
    {/each}
  </div>
{/if}

<section class="card overflow-hidden">
  <div class="flex flex-wrap items-center justify-between gap-2 border-b border-line px-5 py-3">
    <div>
      <span class="eyebrow">BARIS LAPORAN</span>
      <p class="mt-0.5 text-[10px] text-ink-muted">
        {#if data}
          {formatNumber(data.summary.total_rows)} baris · {formatNumber(data.summary.images_done)}/{formatNumber(data.summary.images_total)} gambar dianalisis
        {:else}
          Memuat…
        {/if}
      </p>
    </div>
    <select class="input max-w-[180px] py-1.5 text-xs" bind:value={filter}>
      <option value="">Semua status</option>
      <option value="valid">Valid sesuai</option>
      <option value="manual">Cek manual</option>
      <option value="mabes">Valid, cek mabes</option>
      <option value="invalid">Tidak valid</option>
      <option value="pending">Belum dianalisis</option>
    </select>
  </div>

  {#if loading}
    <div class="space-y-2 p-5">
      {#each Array(4) as _}<div class="skeleton h-9 w-full"></div>{/each}
    </div>
  {:else if rows.length}
    <div class="overflow-x-auto">
      <table class="table">
        <thead>
          <tr>
            <th>Tanggal</th>
            <th>Unit</th>
            <th class="text-right">Tautan</th>
            <th class="text-right">Gambar</th>
            <th>Status</th>
            <th>Keterangan</th>
            <th class="text-right">Aksi</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.row_id)}
            <tr>
              <td class="text-xs whitespace-nowrap text-ink-muted">
                {row.tanggal ? formatDate(row.tanggal) : '—'}
              </td>
              <td class="text-xs font-medium text-ink">{row.unit || '—'}</td>
              <td class="text-right text-xs text-ink-muted">{row.links}</td>
              <td class="text-right text-xs text-ink-muted">{row.images}</td>
              <td><span class={statusClass(row.status)}>{statusLabel(row.status)}</span></td>
              <td class="max-w-[320px] text-[11px] text-ink-muted">{row.reason || '—'}</td>
              <td class="text-right">
                <button
                  type="button"
                  class="text-[10px] font-semibold text-forest-700 hover:underline"
                  onclick={() => void toggleRow(row.row_id)}
                >
                  {expandedRow === row.row_id ? 'Tutup' : 'Detail'}
                </button>
              </td>
            </tr>
            {#if expandedRow === row.row_id}
              <tr>
                <td colspan="7" class="bg-stone-50/60 p-0">
                  <div class="px-4 py-3">
                    {#if detailLoading === row.row_id}
                      <div class="skeleton h-8 w-full"></div>
                    {:else if (details[row.row_id] ?? []).length}
                      <div class="space-y-3">
                        {#each details[row.row_id] ?? [] as image (image.evidence_id)}
                          <div class="rounded border border-line bg-white p-3">
                            <div class="flex flex-wrap items-start justify-between gap-2">
                              <div class="min-w-0">
                                <p class="truncate text-[11px] font-medium text-ink">
                                  {image.filename || image.content_type || image.evidence_id}
                                </p>
                                <p class="text-[10px] text-ink-muted">
                                  {image.content_type || '—'} · {image.analyzed_at
                                    ? image.analyzed_at.slice(0, 16).replace('T', ' ')
                                    : 'belum dianalisis'}
                                </p>
                              </div>
                              <span class={statusClass(image.status)}
                                >{statusLabel(image.status)}</span
                              >
                            </div>
                            <p class="mt-1 text-[11px] text-ink-muted">{image.reason || '—'}</p>
                            {#if image.ocr_text}
                              <pre
                                class="mt-2 max-h-40 overflow-auto whitespace-pre-wrap rounded bg-stone-50 p-2 text-[10px] leading-relaxed text-ink-muted"
                                >{image.ocr_text}</pre
                              >
                            {:else}
                              <p class="mt-2 text-[10px] text-ink-muted italic">
                                Teks OCR tidak tersedia.
                              </p>
                            {/if}
                          </div>
                        {/each}
                      </div>
                    {:else}
                      <p class="text-[11px] text-ink-muted">
                        Tidak ada bukti dukung gambar pada baris ini.
                      </p>
                    {/if}
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="px-5 py-8 text-center text-[11px] text-ink-muted">
      Tidak ada baris pada filter ini.
    </p>
  {/if}
</section>
