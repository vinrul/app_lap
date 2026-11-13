<script lang="ts">
  import type { RepeatedMaterialSuspect } from '../../types'
  import Modal from './Modal.svelte'
  import Button from './Button.svelte'

  type Props = {
    suspect?: RepeatedMaterialSuspect | null
    onclose: () => void
  }

  let { suspect = null, onclose }: Props = $props()

  const open = $derived(Boolean(suspect))
  const title = $derived(
    suspect ? `Materi identik · ${suspect.activity}` : 'Materi identik',
  )
  const description = $derived.by(() => {
    if (!suspect) return ''
    return `${suspect.report_count} baris dari ${suspect.unit_count} satuan · ${suspect.rows.length} baris ditampilkan`
  })
</script>

<Modal {open} {title} {description} {onclose} width="max-w-6xl">
  {#if suspect}
    <div class="space-y-4">
      <div class="grid gap-2 sm:grid-cols-3">
        <div class="rounded-xl border border-red-200 bg-red-50/50 px-3 py-2">
          <p class="text-[10px] font-bold tracking-[0.1em] text-red-700 uppercase">Baris identik</p>
          <strong class="font-display text-xl text-forest-900">{suspect.report_count}</strong>
        </div>
        <div class="rounded-xl border border-red-200 bg-red-50/50 px-3 py-2">
          <p class="text-[10px] font-bold tracking-[0.1em] text-red-700 uppercase">Satuan</p>
          <strong class="font-display text-xl text-forest-900">{suspect.unit_count}</strong>
        </div>
        <div class="rounded-xl border border-line bg-stone-50/60 px-3 py-2">
          <p class="text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">Ditampilkan</p>
          <strong class="font-display text-xl text-forest-900">{suspect.rows.length}</strong>
        </div>
      </div>

      <div>
        <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">
          Satuan yang memakai materi ini · {suspect.unit_count}
        </p>
        {#if suspect.unit_counts?.length}
          <div class="mt-1 max-h-52 overflow-auto rounded-xl border border-line">
            <table class="min-w-full text-left text-[11px]">
              <thead class="sticky top-0 z-10 bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
                <tr>
                  <th class="px-3 py-2">Satuan</th>
                  <th class="w-24 px-3 py-2 text-right">Baris</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-line">
                {#each suspect.unit_counts as unit (unit.satuan)}
                  <tr class="hover:bg-stone-50/60">
                    <td class="px-3 py-1.5 break-words text-ink">{unit.satuan}</td>
                    <td class="px-3 py-1.5 text-right font-semibold text-ink">{unit.rows}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <p class="mt-1 max-h-28 overflow-auto rounded-xl border border-line bg-stone-50/60 p-3 text-[11px] leading-relaxed break-words text-ink-soft">
            {suspect.units.join(', ')}
          </p>
        {/if}
      </div>

      <div>
        <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Contoh materi</p>
        <p class="mt-1 whitespace-pre-wrap break-words rounded-xl border border-line bg-stone-50/60 p-3 text-[11px] leading-relaxed text-ink-soft">
          {suspect.sample || '—'}
        </p>
      </div>

      <div class="max-h-[50vh] overflow-auto rounded-xl border border-line">
        <table class="min-w-[1000px] text-left text-[11px]">
          <thead class="sticky top-0 z-10 bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
            <tr>
              <th class="whitespace-nowrap px-3 py-2">Baris</th>
              <th class="whitespace-nowrap px-3 py-2">Waktu kirim</th>
              <th class="whitespace-nowrap px-3 py-2">Satuan</th>
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
                <td class="max-w-[180px] px-3 py-2 text-ink">{row.satuan || '—'}</td>
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
                <td class="px-3 py-4 text-ink-muted" colspan="7">Tidak ada rincian baris untuk indikator ini.</td>
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
    <p class="text-sm text-ink-muted">Pilih indikator materi identik untuk melihat rincian baris.</p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onclose}>Tutup</Button>
  {/snippet}
</Modal>
