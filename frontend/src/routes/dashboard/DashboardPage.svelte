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
    Layers,
    Plus,
    Rows3,
    ShieldCheck,
    Sparkles,
    Users,
  } from 'lucide-svelte'
  import { dashboardApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { Dashboard, User } from '../../lib/types'
  import { formatDateTime, formatNumber, formatRange, statusOfPeriod } from '../../lib/utils/format'
  import { navigate } from '../../lib/stores/router.svelte'
  import PageHeader from '../../lib/components/ui/PageHeader.svelte'
  import StatCard from '../../lib/components/ui/StatCard.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import EmptyState from '../../lib/components/ui/EmptyState.svelte'
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
  const reader = $derived(user.role === 'polres' || user.role === 'polda')

  // Ringkasan mengikuti halaman Cakupan periode terbaru agar angkanya sama.
  const coverage = $derived(data?.coverage ?? null)
  const coverageGroups = $derived(coverage?.groups ?? [])
  const groupsPerformed = $derived(coverageGroups.filter((group) => group.rows > 0).length)
  const totalCodes = $derived(
    coverageGroups.reduce((total, group) => total + group.codes.length, 0),
  )
  const codesPerformed = $derived(
    coverageGroups.reduce(
      (total, group) => total + group.codes.filter((code) => code.rows > 0).length,
      0,
    ),
  )

  function percent(value: number, total: number): number {
    if (total <= 0) return 0
    return Math.max(0, Math.min(100, Math.round((value / total) * 100)))
  }
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

{#snippet coverageStat(label: string, value: number, total: number)}
  <div>
    <div class="flex items-baseline justify-between gap-2">
      <span class="text-[10px] font-semibold tracking-wide text-ink-muted uppercase">{label}</span>
      <span class="text-[11px] font-semibold text-ink">{value}<span class="text-ink-muted">/{total}</span></span>
    </div>
    <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-stone-200">
      <div class="h-full rounded-full bg-forest-600" style={`width:${percent(value, total)}%`}></div>
    </div>
  </div>
{/snippet}

{#if error}
  <div class="mb-5 flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{/if}

{#if loading}
  <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
    {#each Array(5) as _}
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
          {#if !reader}
            <Button onclick={() => navigate(`/periods/${latest.id}/imports`)}>
              <CloudUpload size={15} /> Impor data
            </Button>
          {/if}
        {:else if !reader}
          <Button onclick={() => navigate('/periods')}><Plus size={15} /> Buat periode</Button>
        {/if}
      </div>
    </div>
  </section>

  {#if coverage}
    <section class="mb-5 grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
      {#each [
        { label: 'Kelompok QW', value: `${groupsPerformed}/${coverage.groups.length}`, hint: 'kelompok dengan kegiatan', icon: Layers },
        { label: 'Sub-kode', value: `${codesPerformed}/${totalCodes}`, hint: 'sub-kode dengan kegiatan', icon: CheckCircle2 },
        { label: 'Polres aktif', value: `${coverage.active_polres}/${coverage.known_polres.length}`, hint: 'melapor pada periode', icon: Users },
        { label: 'Satker aktif', value: `${coverage.active_satker}/${coverage.known_satker.length}`, hint: 'melapor pada periode', icon: ShieldCheck },
        { label: 'Baris dianalisis', value: formatNumber(coverage.total_rows), hint: formatRange(coverage.date_start, coverage.date_end), icon: Rows3 },
      ] as card}
        <article class="card p-4">
          <card.icon size={17} class="text-forest-600" />
          <p class="mt-3 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{card.label}</p>
          <strong class="font-display text-2xl text-forest-900">{card.value}</strong>
          <p class="mt-1 text-[10px] text-ink-muted">{card.hint}</p>
        </article>
      {/each}
    </section>
    <p class="mb-5 text-[11px] text-ink-muted">
      Arsip: {formatNumber(data.counts.periods)} periode · {formatNumber(data.counts.imports)} versi impor · {formatNumber(data.counts.reports)} PDF tersimpan.
    </p>
  {:else}
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
  {/if}

  {#if coverage && latest}
    <section class="mb-5">
      <div class="mb-3 flex items-end justify-between gap-3">
        <div>
          <span class="eyebrow">CAKUPAN PER KELOMPOK</span>
          <h2 class="mt-1 font-display text-xl font-semibold text-forest-900">Quick Wins periode terbaru</h2>
        </div>
        <Button variant="ghost" size="sm" onclick={() => navigate(`/periods/${latest?.id}/coverage`)}>
          Buka cakupan <ArrowRight size={14} />
        </Button>
      </div>
      <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {#each coverageGroups as group (group.code)}
          {@const codesDone = group.codes.filter((code) => code.rows > 0).length}
          <article class="card p-4">
            <div class="flex items-center justify-between gap-2">
              <span class="rounded-md bg-forest-50 px-2 py-0.5 text-[10px] font-bold text-forest-700">{group.code}</span>
              <span class="text-[10px] text-ink-muted">{formatNumber(group.rows)} laporan</span>
            </div>
            <p class="mt-2 min-h-8 text-xs font-semibold text-ink">{group.title}</p>
            <p class="mt-1 text-[10px] text-ink-muted">{codesDone}/{group.codes.length} sub-kode terisi</p>
            <div class="mt-3 space-y-2">
              {@render coverageStat('Polres', group.polres, group.polres_total)}
              {@render coverageStat('Satker', group.satker, group.satker_total)}
            </div>
          </article>
        {/each}
      </div>
    </section>
  {:else}
    <section class="card mb-5">
      <EmptyState
        title="Belum ada data cakupan"
        description="Unggah workbook Excel pada periode terbaru agar ringkasan cakupan muncul di sini."
      >
        <Button onclick={() => navigate(latest ? `/periods/${latest.id}/imports` : '/periods')}>
          <CloudUpload size={15} /> Impor workbook
        </Button>
      </EmptyState>
    </section>
  {/if}

  {#if metrics}
    <section class="card mb-5 p-5">
      <div class="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <span class="eyebrow">TREN HARIAN</span>
          <h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Laporan masuk per hari</h2>
        </div>
        <span class="text-xs text-ink-muted">{formatRange(metrics.period_start, metrics.period_end)} · {metrics.daily.length} hari</span>
      </div>
      <DailyBars items={metrics.daily} />
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
