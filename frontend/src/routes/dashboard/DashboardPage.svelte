<script lang="ts">
  import { onMount } from 'svelte'
  import {
    AlertCircle,
    ArrowRight,
    CalendarDays,
    CheckCircle2,
    ClipboardList,
    CloudUpload,
    FileText,
    Plus,
    Rows3,
    Sparkles,
  } from 'lucide-svelte'
  import { dashboardApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { Dashboard, User } from '../../lib/types'
  import { formatDateTime, formatNumber, formatRange, statusOfPeriod } from '../../lib/utils/format'
  import { quickWinsLabels } from '../../lib/utils/constants'
  import { navigate } from '../../lib/stores/router.svelte'
  import PageHeader from '../../lib/components/ui/PageHeader.svelte'
  import StatCard from '../../lib/components/ui/StatCard.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import EmptyState from '../../lib/components/ui/EmptyState.svelte'
  import BarList from '../../lib/components/charts/BarList.svelte'
  import DailyBars from '../../lib/components/charts/DailyBars.svelte'

  type Props = { user: User }

  let { user }: Props = $props()

  let data = $state<Dashboard | null>(null)
  let loading = $state(true)
  let error = $state('')

  onMount(load)

  async function load() {
    loading = true
    error = ''
    try {
      data = await dashboardApi.get()
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  const latest = $derived(data?.latest_period ?? null)
  const metrics = $derived(data?.metrics ?? null)
  const status = $derived(latest ? statusOfPeriod(latest) : null)
</script>

<svelte:head><title>Ringkasan · Laporan Mingguan PBM</title></svelte:head>

<PageHeader
  eyebrow="PUSAT KENDALI LAPORAN"
  title={`Selamat bekerja, ${user.username}.`}
  description="Pantau periode terbaru, kualitas data, dan kesiapan laporan mingguan Polda Bali."
>
  {#snippet actions()}
    <Button onclick={() => navigate('/periods')}><Plus size={16} /> Mulai laporan</Button>
  {/snippet}
</PageHeader>

{#if error}
  <div class="mb-5 flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{/if}

{#if loading}
  <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
    {#each Array(4) as _}
      <div class="card h-32 p-5"><div class="skeleton h-3 w-24"></div><div class="skeleton mt-6 h-8 w-32"></div></div>
    {/each}
  </div>
{:else if data}
  <section class="relative mb-5 overflow-hidden rounded-3xl bg-forest-950 px-6 py-7 text-white shadow-lift sm:px-8">
    <div class="pointer-events-none absolute -top-24 -right-16 h-72 w-72 rounded-full bg-forest-700/45 blur-3xl"></div>
    <div class="pointer-events-none absolute -bottom-28 left-1/3 h-56 w-56 rounded-full bg-gold-500/10 blur-3xl"></div>

    <div class="relative flex flex-wrap items-start justify-between gap-6">
      <div class="max-w-2xl">
        <span class="flex items-center gap-2 text-[10px] font-bold tracking-[0.2em] text-gold-300 uppercase">
          <Sparkles size={13} /> {latest ? 'PERIODE LAPORAN TERBARU' : 'BELUM ADA PERIODE'}
        </span>
        <h2 class="mt-3 font-display text-3xl leading-tight font-medium sm:text-4xl">
          {latest?.label ?? 'Mulai laporan minggu pertama'}
        </h2>
        <p class="mt-2 text-sm text-forest-100/75">
          {latest ? formatRange(latest.start_date, latest.end_date) : 'Tentukan rentang tanggal, unggah workbook, lalu susun laporan.'}
        </p>
        {#if latest}
          <div class="mt-4 flex flex-wrap items-center gap-2 text-[11px]">
            <span class="rounded-full border border-white/15 bg-white/5 px-2.5 py-1">{latest.imports} versi impor</span>
            <span class="rounded-full border border-white/15 bg-white/5 px-2.5 py-1">{latest.reports} PDF</span>
            {#if status}<span class="rounded-full border border-gold-300/30 bg-gold-300/10 px-2.5 py-1 text-gold-200">{status.label}</span>{/if}
          </div>
        {/if}
      </div>
      <div class="flex flex-wrap gap-2">
        {#if latest}
          <Button variant="secondary" onclick={() => navigate(`/periods/${latest.id}/overview`)}>
            Buka periode <ArrowRight size={15} />
          </Button>
          <Button onclick={() => navigate(`/periods/${latest.id}/imports`)}>
            <CloudUpload size={15} /> Impor data
          </Button>
        {:else}
          <Button onclick={() => navigate('/periods')}><Plus size={15} /> Buat periode</Button>
        {/if}
      </div>
    </div>
  </section>

  <section class="mb-5 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
    <StatCard label="Periode tersimpan" value={data.counts.periods} unit="periode" detail="Arsip laporan mingguan" tone="forest">
      {#snippet icon()}<CalendarDays size={17} />{/snippet}
    </StatCard>
    <StatCard label="Versi impor" value={data.counts.imports} unit="unggahan" detail="Workbook tersimpan aman" tone="gold">
      {#snippet icon()}<CloudUpload size={17} />{/snippet}
    </StatCard>
    <StatCard label="PDF dihasilkan" value={data.counts.reports} unit="dokumen" detail="Siap ditinjau atau diunduh" tone="forest">
      {#snippet icon()}<FileText size={17} />{/snippet}
    </StatCard>
    <StatCard label="Baris sumber" value={data.counts.rows} unit="baris" detail="Tersimpan pada basis data" tone="slate">
      {#snippet icon()}<Rows3 size={17} />{/snippet}
    </StatCard>
  </section>

  {#if metrics}
    <section class="mb-5">
      <div class="mb-3 flex items-end justify-between gap-3">
        <div>
          <span class="eyebrow">SOROTAN PERIODE</span>
          <h2 class="mt-1 font-display text-xl font-semibold text-forest-900">Enam metrik utama</h2>
        </div>
        <span class="text-xs text-ink-muted">{formatNumber(metrics.total_rows)} baris dalam periode</span>
      </div>
      <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {#each metrics.cards as card}
          <article class="card flex flex-col p-5">
            <span class="text-xs font-semibold tracking-wide text-ink-muted uppercase">{card.label}</span>
            <div class="mt-3 flex items-end gap-1.5">
              <strong class="font-display text-3xl leading-none font-semibold text-forest-900">{formatNumber(card.value)}</strong>
              <span class="pb-0.5 text-[11px] text-ink-muted">{card.unit}</span>
            </div>
            <p class="mt-2 text-xs leading-relaxed text-ink-muted">{card.detail}</p>
          </article>
        {/each}
      </div>
    </section>

    <section class="mb-5 grid gap-4 lg:grid-cols-3">
      <div class="card p-5 lg:col-span-2">
        <div class="mb-4 flex items-end justify-between">
          <div><span class="eyebrow">TREN HARIAN</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Laporan masuk per hari</h2></div>
          <span class="text-xs text-ink-muted">{metrics.daily.length} hari</span>
        </div>
        <DailyBars items={metrics.daily} />
      </div>
      <div class="card p-5">
        <div class="mb-4"><span class="eyebrow">DISTRIBUSI</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Quick Wins</h2></div>
        <BarList items={metrics.quick_wins} limit={8} formatLabel={(label) => quickWinsLabels[label] ?? label} />
      </div>
    </section>

    <section class="mb-5 grid gap-4 lg:grid-cols-2">
      <div class="card p-5">
        <div class="mb-4"><span class="eyebrow">SEBARAN WILAYAH</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Laporan per polres</h2></div>
        <BarList items={metrics.by_polres} limit={10} />
      </div>
      <div class="card p-5">
        <div class="mb-4"><span class="eyebrow">KEGIATAN MENONJOL</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Sorotan satuan</h2></div>
        <div class="space-y-3">
          {#each metrics.highlights.slice(0, 4) as item}
            <div class="flex gap-3 border-b border-stone-100 pb-3 last:border-0 last:pb-0">
              <span class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-forest-50 text-[10px] font-bold text-forest-700">{item.kode || '—'}</span>
              <div class="min-w-0">
                <p class="truncate text-xs font-semibold text-ink">{item.kegiatan || 'Kegiatan tidak disebutkan'}</p>
                <p class="mt-0.5 truncate text-[11px] text-ink-muted">{item.satuan} · {item.location || 'Lokasi belum diisi'}</p>
              </div>
            </div>
          {:else}
            <p class="text-sm text-ink-muted">Belum ada kegiatan menonjol.</p>
          {/each}
        </div>
      </div>
    </section>
  {:else}
    <section class="card mb-5">
      <EmptyState
        title="Belum ada data untuk dianalisis"
        description="Unggah workbook Excel pada periode terbaru agar metrik dan sorotan otomatis muncul di sini."
      >
        <Button onclick={() => navigate(latest ? `/periods/${latest.id}/imports` : '/periods')}>
          <CloudUpload size={15} /> Impor workbook
        </Button>
      </EmptyState>
    </section>
  {/if}

  <section class="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
    <div class="card overflow-hidden">
      <div class="panel-heading">
        <div><span class="eyebrow">RIWAYAT</span><h2 class="panel-title mt-1">Periode laporan</h2></div>
        <Button variant="ghost" size="sm" onclick={() => navigate('/periods')}>Lihat semua <ArrowRight size={14} /></Button>
      </div>
      {#if data.recent_periods.length}
        <div class="overflow-x-auto">
          <table class="table">
            <thead><tr><th>Periode</th><th>Rentang</th><th>PDF</th><th>Status</th></tr></thead>
            <tbody>
              {#each data.recent_periods as period}
                {@const periodStatus = statusOfPeriod(period)}
                <tr>
                  <td><button class="table-link" onclick={() => navigate(`/periods/${period.id}/overview`)}>{period.label}</button></td>
                  <td class="text-xs whitespace-nowrap text-ink-muted">{formatRange(period.start_date, period.end_date)}</td>
                  <td class="text-xs text-ink-muted">{period.reports}</td>
                  <td><Badge tone={periodStatus.tone}>{periodStatus.label}</Badge></td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <EmptyState title="Arsip masih kosong" description="Periode yang Anda buat akan muncul di sini.">
          <Button onclick={() => navigate('/periods')}><Plus size={15} /> Buat periode</Button>
        </EmptyState>
      {/if}
    </div>

    <div class="card overflow-hidden">
      <div class="panel-heading">
        <div><span class="eyebrow">DOKUMEN</span><h2 class="panel-title mt-1">PDF terbaru</h2></div>
        <ClipboardList size={17} class="text-ink-muted" />
      </div>
      {#if data.recent_reports.length}
        <div class="divide-y divide-stone-100">
          {#each data.recent_reports as report}
            <button
              class="flex w-full items-center gap-3 px-5 py-3.5 text-left transition-colors hover:bg-forest-50/50"
              onclick={() => navigate(`/periods/${report.period_id}/documents`)}
            >
              <span class="flex h-9 w-9 items-center justify-center rounded-lg bg-forest-50 text-forest-700"><FileText size={17} /></span>
              <span class="min-w-0 flex-1">
                <strong class="block truncate text-xs font-semibold text-ink">{report.period_label || 'Laporan mingguan'}</strong>
                <small class="block truncate text-[10px] text-ink-muted">{formatDateTime(report.created_at)} · {report.source_filename}</small>
              </span>
              <CheckCircle2 size={15} class="shrink-0 text-forest-500" />
            </button>
          {/each}
        </div>
      {:else}
        <EmptyState title="Belum ada PDF" description="PDF yang dihasilkan akan tercatat di sini." />
      {/if}
    </div>
  </section>
{/if}
