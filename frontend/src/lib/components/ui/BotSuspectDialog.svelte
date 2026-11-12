<script lang="ts">
  import type { RapidInputSuspect } from '../../types'
  import { formatDate } from '../../utils/format'
  import Modal from './Modal.svelte'
  import Button from './Button.svelte'

  type Props = {
    suspect?: RapidInputSuspect | null
    onclose: () => void
  }

  let { suspect = null, onclose }: Props = $props()

  const open = $derived(Boolean(suspect))
  const title = $derived(suspect ? `Input cepat · ${suspect.satuan}` : 'Input cepat')

  const description = $derived.by(() => {
    if (!suspect) return ''
    return `${formatDate(suspect.tanggal)} · ${suspect.first_time}–${suspect.last_time} · ${suspect.report_count} laporan · ${formatWindow(suspect.window_seconds)} · ${formatPace(suspect.report_count, suspect.window_seconds)}`
  })

  function formatWindow(seconds: number): string {
    if (seconds <= 0) return 'kurang dari 1 menit'
    if (seconds < 60) return `${seconds} detik`
    const minutes = Math.floor(seconds / 60)
    const rest = seconds % 60
    return rest ? `${minutes} menit ${rest} detik` : `${minutes} menit`
  }

  function formatPace(count: number, seconds: number): string {
    if (count <= 1) return '—'
    return `±${Math.max(1, Math.round(seconds / (count - 1)))} detik/laporan`
  }
</script>

<Modal {open} {title} {description} {onclose} width="max-w-6xl">
  {#if suspect}
    <div class="space-y-4">
      <div class="grid gap-2 sm:grid-cols-3">
        <div class="rounded-xl border border-red-200 bg-red-50/50 px-3 py-2">
          <p class="text-[10px] font-bold tracking-[0.1em] text-red-700 uppercase">Laporan</p>
          <strong class="font-display text-xl text-forest-900">{suspect.report_count}</strong>
        </div>
        <div class="rounded-xl border border-red-200 bg-red-50/50 px-3 py-2">
          <p class="text-[10px] font-bold tracking-[0.1em] text-red-700 uppercase">Menit yang sama</p>
          <strong class="font-display text-xl text-forest-900">{suspect.same_minute_count}</strong>
        </div>
        <div class="rounded-xl border border-line bg-stone-50/60 px-3 py-2">
          <p class="text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">Kegiatan</p>
          <strong class="font-display text-xl text-forest-900">{suspect.activities.length}</strong>
        </div>
      </div>

      <p class="text-[11px] leading-relaxed text-ink-muted">
        {suspect.activities.join(' · ')}
      </p>

      <div class="max-h-[55vh] overflow-auto rounded-xl border border-line">
        <table class="min-w-[900px] text-left text-[11px]">
          <thead class="sticky top-0 z-10 bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
            <tr>
              <th class="whitespace-nowrap px-3 py-2">Baris</th>
              <th class="whitespace-nowrap px-3 py-2">Waktu kirim</th>
              <th class="whitespace-nowrap px-3 py-2">Kode</th>
              <th class="whitespace-nowrap px-3 py-2">Kegiatan</th>
              <th class="whitespace-nowrap px-3 py-2">Status</th>
              <th class="min-w-[320px] px-3 py-2">Isian laporan</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-line">
            {#each suspect.rows as row}
              <tr class="align-top hover:bg-stone-50/60">
                <td class="whitespace-nowrap px-3 py-2 font-semibold text-ink">{row.source_row}</td>
                <td class="whitespace-nowrap px-3 py-2 text-ink-muted">{row.waktu_kirim_wib || row.tanggal}</td>
                <td class="whitespace-nowrap px-3 py-2">
                  <span class="rounded-md bg-forest-50 px-2 py-0.5 text-[10px] font-bold text-forest-700">
                    {row.kode || '—'}
                  </span>
                </td>
                <td class="max-w-[220px] px-3 py-2 text-ink">{row.kegiatan || '—'}</td>
                <td class="whitespace-nowrap px-3 py-2 text-ink-muted">{row.status_penerimaan || 'Belum diisi'}</td>
                <td class="px-3 py-2">
                  <p class="max-w-[520px] whitespace-pre-wrap break-words text-[10px] leading-relaxed text-ink-soft">
                    {row.isian_laporan || '—'}
                  </p>
                </td>
              </tr>
            {:else}
              <tr>
                <td class="px-3 py-4 text-ink-muted" colspan="6">Tidak ada rincian baris untuk indikator ini.</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      {#if suspect.report_count > suspect.rows.length}
        <p class="text-[10px] text-ink-muted">
          Menampilkan {suspect.rows.length} dari {suspect.report_count} baris. Baris lainnya dapat dilihat di tab Data periode.
        </p>
      {/if}
    </div>
  {:else}
    <p class="text-sm text-ink-muted">Pilih indikator input cepat untuk melihat rincian baris.</p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onclose}>Tutup</Button>
  {/snippet}
</Modal>
