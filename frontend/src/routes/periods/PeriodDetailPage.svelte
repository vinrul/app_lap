<script lang="ts">
  import { onMount } from 'svelte'
  import {
    AlertCircle,
    Archive,
    ArchiveRestore,
    CalendarDays,
    CloudUpload,
    FileText,
    LoaderCircle,
    PenLine,
    Pencil,
  } from 'lucide-svelte'
  import { periodsApi, reportsApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { PeriodDetail, ReportMetrics } from '../../lib/types'
  import { formatRange, statusOfPeriod } from '../../lib/utils/format'
  import { navigate } from '../../lib/stores/router.svelte'
  import { dialog } from '../../lib/stores/dialog.svelte'
  import { toast } from '../../lib/stores/toast.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import Tabs from '../../lib/components/ui/Tabs.svelte'
  import PeriodFormModal from '../../lib/components/period/PeriodFormModal.svelte'
  import PeriodOverview from '../../lib/components/period/PeriodOverview.svelte'
  import AnalysisPanel from '../../lib/components/period/AnalysisPanel.svelte'
  import CoveragePanel from '../../lib/components/period/CoveragePanel.svelte'
  import RecordsPanel from '../../lib/components/period/RecordsPanel.svelte'
  import DesignPanel from '../../lib/components/period/DesignPanel.svelte'
  import DocumentsPanel from '../../lib/components/period/DocumentsPanel.svelte'

  type Props = { periodId: string; tab?: string }

  let { periodId, tab = 'overview' }: Props = $props()

  let detail = $state<PeriodDetail | null>(null)
  let metrics = $state<ReportMetrics | null>(null)
  let loading = $state(true)
  let metricsLoading = $state(false)
  let error = $state('')
  let editOpen = $state(false)
  let busy = $state(false)

  // Tab yang disembunyikan sementara. Hapus id dari daftar ini untuk menampilkan lagi.
  const hiddenTabs = new Set(['overview', 'analysis'])

  const activeTab = $derived(
    ['overview', 'analysis', 'coverage', 'records', 'design', 'documents'].includes(tab) &&
      !hiddenTabs.has(tab)
      ? tab
      : 'coverage',
  )
  const status = $derived(detail ? statusOfPeriod(detail.period) : null)
  const typeLabel = $derived(
    detail?.period.period_type === 'daily'
      ? 'Harian'
      : detail?.period.period_type === 'monthly'
        ? 'Bulanan'
        : 'Mingguan / Rentang',
  )

  onMount(() => {
    void load()
  })

  async function load() {
    loading = true
    error = ''
    try {
      detail = await periodsApi.detail(periodId)
      await loadMetrics()
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  async function loadMetrics() {
    metricsLoading = true
    try {
      metrics = await periodsApi.metrics(periodId)
    } catch (caught) {
      metrics = null
      toast.error(errorMessage(caught))
    } finally {
      metricsLoading = false
    }
  }

  async function handleGenerated() {
    if (detail) {
      detail.reports = await reportsApi.list(periodId)
      detail.period.reports = detail.reports.length
    }
  }

  async function archivePeriod() {
    if (!detail) return
    const confirmed = await dialog.confirm({
      title: 'Arsipkan periode?',
      message: `Periode "${detail.period.label}" akan dipindahkan ke arsip dan tidak tampil di daftar aktif maupun ringkasan. Data impor, desain, dan dokumen tetap tersimpan, dan periode dapat dipulihkan kapan saja.`,
      confirmLabel: 'Arsipkan',
      cancelLabel: 'Batal',
    })
    if (!confirmed) return

    busy = true
    try {
      await periodsApi.archive(periodId)
      toast.success('Periode laporan diarsipkan.')
      navigate('/periods')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      busy = false
    }
  }

  async function restorePeriod() {
    busy = true
    try {
      await periodsApi.restore(periodId)
      toast.success('Periode laporan dipulihkan.')
      await load()
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      busy = false
    }
  }

  const tabs = $derived([
    { id: 'overview', label: 'Ringkasan' },
    { id: 'analysis', label: 'Analisis' },
    { id: 'coverage', label: 'Cakupan' },
    { id: 'records', label: 'Data sumber' },
    { id: 'design', label: 'Desain' },
    { id: 'documents', label: 'Dokumen', count: detail?.reports.length ?? 0 },
  ].filter((item) => !hiddenTabs.has(item.id)))
</script>

<svelte:head><title>{detail?.period.label ?? 'Periode'} · Laporan Mingguan PBM</title></svelte:head>

{#if loading && !detail}
  <div class="space-y-4">
    <div class="skeleton h-28 w-full"></div>
    <div class="grid gap-4 sm:grid-cols-3">{#each Array(3) as _}<div class="skeleton h-32"></div>{/each}</div>
  </div>
{:else if error && !detail}
  <div class="flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{:else if detail}
  <section class="card mb-5 overflow-hidden">
    <div class="flex flex-wrap items-start justify-between gap-5 border-b border-line px-6 py-5">
      <div class="min-w-0">
        <button class="text-[10px] font-bold tracking-[0.18em] text-gold-600 uppercase hover:underline" onclick={() => navigate('/periods')}>
          PERIODE LAPORAN
        </button>
        <h1 class="mt-2 font-display text-2xl font-semibold text-forest-900 sm:text-3xl">{detail.period.label}</h1>
        <div class="mt-2 flex flex-wrap items-center gap-3 text-xs text-ink-muted">
          <span class="flex items-center gap-1.5"><CalendarDays size={14} />{formatRange(detail.period.start_date, detail.period.end_date)}</span>
          <span class="text-stone-300">·</span>
          <span>{typeLabel}</span>
          <span class="text-stone-300">·</span>
          <span>{detail.period.imports} berkas impor</span>
          <span class="text-stone-300">·</span>
          <span>{detail.reports.length} PDF</span>
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        {#if status}<Badge tone={status.tone}>{status.label}</Badge>{/if}
        <Button variant="secondary" size="sm" onclick={() => (editOpen = true)}>
          <Pencil size={14} /> Edit
        </Button>
        {#if detail.period.archived_at}
          <Button variant="secondary" size="sm" loading={busy} onclick={restorePeriod}>
            <ArchiveRestore size={14} /> Pulihkan
          </Button>
        {:else}
          <Button variant="ghost" size="sm" loading={busy} onclick={archivePeriod}>
            <Archive size={14} /> Arsipkan
          </Button>
        {/if}
        <Button variant="secondary" size="sm" onclick={() => navigate('/imports')}>
          <CloudUpload size={14} /> Import data
        </Button>
        <Button size="sm" onclick={() => navigate(`/periods/${periodId}/design`)}>
          <PenLine size={14} /> Susun desain
        </Button>
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-3 bg-stone-50/60 px-6 py-3">
      <span class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Sumber data</span>
      {#if detail.imports.length}
        <span class="text-[11px] font-medium text-ink-soft">{detail.imports.length} berkas · {metrics?.total_rows ?? detail.period.rows} baris unik dalam rentang</span>
        {#if detail.imports.length > 1}
          <span class="text-[10px] text-ink-muted">termasuk {detail.imports.slice(0, 2).map((item) => item.filename).join(', ')} dan lainnya</span>
        {/if}
      {:else}
        <span class="text-[11px] text-ink-muted">Belum ada berkas import yang mencakup rentang ini.</span>
      {/if}
      {#if metricsLoading}<LoaderCircle size={15} class="animate-spin text-forest-600" />{/if}
      {#if metrics?.duplicate_rows}
        <span class="text-[11px] text-gold-700">{metrics.duplicate_rows} duplikat dilewati</span>
      {/if}
      <button class="btn btn-ghost btn-sm ml-auto" onclick={() => navigate(`/periods/${periodId}/documents`)}>
        <FileText size={14} /> Dokumen
      </button>
    </div>
  </section>

  <div class="mb-5">
    <Tabs
      items={tabs}
      active={activeTab}
      onchange={(id: string) => navigate(`/periods/${periodId}/${id}`)}
    />
  </div>

  {#if activeTab === 'overview'}
    <PeriodOverview {detail} {metrics} {metricsLoading} onnavigate={(path: string) => navigate(`/periods/${periodId}/${path}`)} />
  {:else if activeTab === 'analysis'}
    <AnalysisPanel {periodId} />
  {:else if activeTab === 'coverage'}
    <CoveragePanel {periodId} />
  {:else if activeTab === 'records'}
    <RecordsPanel {periodId} />
  {:else if activeTab === 'design'}
    <DesignPanel {periodId} {detail} ongenerated={handleGenerated} />
  {:else if activeTab === 'documents'}
    <DocumentsPanel reports={detail.reports} onrefresh={() => load()} />
  {/if}
{/if}

{#if detail && editOpen}
  <PeriodFormModal
    open={true}
    period={detail.period}
    onclose={() => (editOpen = false)}
    onsaved={() => {
      void load()
    }}
  />
{/if}
