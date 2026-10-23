<script lang="ts">
  import {
    AlertTriangle,
    ArrowRight,
    CalendarDays,
    Check,
    CloudUpload,
    FileText,
    MapPin,
    Megaphone,
    MessageSquareQuote,
    Rows3,
    Trees,
  } from 'lucide-svelte'
  import type { ImportView, PeriodDetail, ReportMetrics } from '../../types'
  import { formatDateTime, formatNumber, formatRange } from '../../utils/format'
  import { quickWinsLabels } from '../../utils/constants'
  import Button from '../ui/Button.svelte'
  import EmptyState from '../ui/EmptyState.svelte'
  import BarList from '../charts/BarList.svelte'
  import DailyBars from '../charts/DailyBars.svelte'

  type Props = {
    detail: PeriodDetail
    metrics: ReportMetrics | null
    metricsLoading?: boolean
    onnavigate: (tab: string) => void
  }

  let {
    detail,
    metrics,
    metricsLoading = false,
    onnavigate,
  }: Props = $props()

  const latestImport = $derived<ImportView | undefined>(detail.imports[0])
</script>

{#if !metrics}
  <section class="card">
    <EmptyState
      title="Belum ada data impor"
      description="Unggah workbook Excel untuk periode ini. Setelah tersimpan, metrik dan pratinjau laporan akan muncul otomatis."
    >
      <Button onclick={() => onnavigate('imports')}><CloudUpload size={15} /> Impor workbook</Button>
    </EmptyState>
  </section>
{:else}
  <div class="grid gap-4 xl:grid-cols-[minmax(0,1.7fr)_minmax(300px,0.8fr)]">
    <div class="space-y-4">
      <section>
        <div class="mb-3 flex items-end justify-between gap-3">
          <div>
            <span class="eyebrow">METRIK PERIODE</span>
            <h2 class="mt-1 font-display text-xl font-semibold text-forest-900">Enam sorotan utama</h2>
          </div>
          {#if metricsLoading}<span class="text-xs text-ink-muted">Memperbarui…</span>{/if}
        </div>
        <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {#each metrics.cards as card}
            <article class="card p-5">
              <span class="text-[11px] font-semibold tracking-wide text-ink-muted uppercase">{card.label}</span>
              <div class="mt-3 flex items-end gap-1.5">
                <strong class="font-display text-3xl leading-none font-semibold text-forest-900">{formatNumber(card.value)}</strong>
                <span class="pb-0.5 text-[11px] text-ink-muted">{card.unit}</span>
              </div>
              <p class="mt-2 text-xs leading-relaxed text-ink-muted">{card.detail}</p>
            </article>
          {/each}
        </div>
      </section>

      <section class="card p-5">
        <div class="mb-4 flex items-end justify-between gap-3">
          <div><span class="eyebrow">TREN HARIAN</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Laporan masuk</h2></div>
          <span class="text-xs text-ink-muted">{formatRange(metrics.period_start, metrics.period_end)}</span>
        </div>
        <DailyBars items={metrics.daily} />
      </section>

      <div class="grid gap-4 lg:grid-cols-2">
        <section class="card p-5">
          <div class="mb-4"><span class="eyebrow">DISTRIBUSI</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Quick Wins</h2></div>
          <BarList items={metrics.quick_wins} limit={8} formatLabel={(label) => quickWinsLabels[label] ?? label} />
        </section>
        <section class="card p-5">
          <div class="mb-4"><span class="eyebrow">SEBARAN</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Laporan per polres</h2></div>
          <BarList items={metrics.by_polres} limit={10} barTone="gold" />
        </section>
      </div>

      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div class="card p-4">
          <Trees size={17} class="text-forest-600" />
          <p class="mt-3 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Bibit ditanam</p>
          <strong class="font-display text-2xl text-forest-900">{formatNumber(metrics.green_policing.bibit)}</strong>
          <p class="text-[11px] text-ink-muted">{metrics.green_policing.rows} laporan hijau</p>
        </div>
        <div class="card p-4">
          <MapPin size={17} class="text-gold-600" />
          <p class="mt-3 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Lokasi ibadah</p>
          <strong class="font-display text-2xl text-forest-900">{formatNumber(metrics.minggu_kasih.locations)}</strong>
          <p class="text-[11px] text-ink-muted">{formatNumber(metrics.minggu_kasih.people)} jemaat</p>
        </div>
        <div class="card p-4">
          <MessageSquareQuote size={17} class="text-forest-600" />
          <p class="mt-3 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Aspirasi warga</p>
          <strong class="font-display text-2xl text-forest-900">{formatNumber(metrics.aspirations.total)}</strong>
          <p class="text-[11px] text-ink-muted">{metrics.aspirations.themes.length} tema utama</p>
        </div>
        <div class="card p-4">
          <Megaphone size={17} class="text-gold-600" />
          <p class="mt-3 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Publikasi media</p>
          <strong class="font-display text-2xl text-forest-900">{formatNumber(metrics.media.publications)}</strong>
          <p class="text-[11px] text-ink-muted">{metrics.media.rows_with_media} kegiatan teramplifikasi</p>
        </div>
      </div>
    </div>

    <aside class="space-y-4">
      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div><span class="eyebrow">KUALITAS DATA</span><h2 class="panel-title mt-1">Sumber terpilih</h2></div>
          <Rows3 size={17} class="text-ink-muted" />
        </div>
        <div class="space-y-3 px-5 py-4">
          <div class="flex items-center justify-between gap-3 text-sm">
            <span class="text-ink-muted">Baris dalam periode</span>
            <strong class="text-forest-900">{formatNumber(metrics.total_rows)}</strong>
          </div>
          <div class="flex items-center justify-between gap-3 text-sm">
            <span class="text-ink-muted">Di luar rentang</span>
            <strong class={metrics.outside_period > 0 ? 'text-gold-700' : 'text-forest-900'}>{formatNumber(metrics.outside_period)}</strong>
          </div>
          <div class="flex items-center justify-between gap-3 text-sm">
            <span class="text-ink-muted">Versi impor</span>
            <strong class="text-forest-900">{detail.imports.length}</strong>
          </div>
          <div class="flex items-center justify-between gap-3 text-sm">
            <span class="text-ink-muted">PDF dihasilkan</span>
            <strong class="text-forest-900">{detail.reports.length}</strong>
          </div>
          {#if latestImport}
            <div class="rounded-xl border border-line bg-stone-50 px-3.5 py-3">
              <p class="truncate text-xs font-semibold text-ink">{latestImport.filename}</p>
              <p class="mt-1 text-[10px] text-ink-muted">{formatNumber(latestImport.row_count)} baris · {formatDateTime(latestImport.created_at)}</p>
              <p class="text-[10px] text-ink-muted">Diunggah oleh {latestImport.imported_by}</p>
            </div>
          {/if}
          {#if metrics.outside_period > 0}
            <div class="flex items-start gap-2 rounded-lg border border-gold-200 bg-gold-100/60 px-3 py-2 text-[11px] text-gold-700">
              <AlertTriangle size={14} class="mt-0.5 shrink-0" />
              Baris di luar periode tidak masuk agregat. Periksa kembali rentang tanggal bila diperlukan.
            </div>
          {/if}
          <Button variant="secondary" size="sm" class="w-full" onclick={() => onnavigate('records')}>
            Periksa data sumber <ArrowRight size={14} />
          </Button>
        </div>
      </section>

      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div><span class="eyebrow">ALUR KERJA</span><h2 class="panel-title mt-1">Langkah laporan</h2></div>
          <span class="text-[10px] font-bold text-ink-muted">01—04</span>
        </div>
        <ol class="space-y-0 px-5 py-4">
          <li class="flex gap-3 pb-4">
            <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-forest-700 text-white"><Check size={13} /></span>
            <div><strong class="text-xs text-ink">Tentukan periode</strong><p class="text-[11px] text-ink-muted">{formatRange(detail.period.start_date, detail.period.end_date)}</p></div>
          </li>
          <li class="flex gap-3 border-t border-stone-100 pt-4 pb-4">
            <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-forest-700 text-white"><Check size={13} /></span>
            <div><strong class="text-xs text-ink">Impor dan validasi</strong><p class="text-[11px] text-ink-muted">{detail.imports.length} versi tersimpan</p></div>
          </li>
          <li class="flex gap-3 border-t border-stone-100 pt-4 pb-4">
            <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full {detail.reports.length ? 'bg-forest-700 text-white' : 'border border-stone-300 bg-white text-ink-muted'}">
              {#if detail.reports.length}<Check size={13} />{:else}03{/if}
            </span>
            <div><strong class="text-xs text-ink">Lengkapi narasi</strong><p class="text-[11px] text-ink-muted">Editorial dan penandatangan</p></div>
          </li>
          <li class="flex gap-3 border-t border-stone-100 pt-4">
            <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full {detail.reports.length ? 'bg-forest-700 text-white' : 'border border-stone-300 bg-white text-ink-muted'}">
              {#if detail.reports.length}<Check size={13} />{:else}04{/if}
            </span>
            <div><strong class="text-xs text-ink">Terbitkan PDF</strong><p class="text-[11px] text-ink-muted">{detail.reports.length ? `${detail.reports.length} dokumen tersedia` : 'Belum diterbitkan'}</p></div>
          </li>
        </ol>
        <div class="border-t border-line p-4">
          <Button class="w-full" onclick={() => onnavigate('editorial')}><FileText size={15} /> Buka editorial</Button>
        </div>
      </section>

      <section class="card p-5">
        <div class="flex items-center gap-2">
          <CalendarDays size={15} class="text-gold-600" />
          <span class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Catatan periode</span>
        </div>
        <p class="mt-2 text-xs leading-relaxed text-ink-muted">
          Angka pada laporan merupakan agregat baris sumber dan isian laporan. Tinjau kembali narasi editorial serta penandatangan sebelum PDF diterbitkan.
        </p>
      </section>
    </aside>
  </div>
{/if}
