<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertCircle, CalendarDays, CloudUpload, FileText, LoaderCircle, PenLine } from 'lucide-svelte'
  import { periodsApi, reportsApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { ImportView, PeriodDetail, ReportMetrics } from '../../lib/types'
  import { formatRange, statusOfPeriod } from '../../lib/utils/format'
  import { navigate } from '../../lib/stores/router.svelte'
  import { toast } from '../../lib/stores/toast.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import Tabs from '../../lib/components/ui/Tabs.svelte'
  import PeriodOverview from '../../lib/components/period/PeriodOverview.svelte'
  import ImportPanel from '../../lib/components/period/ImportPanel.svelte'
  import RecordsPanel from '../../lib/components/period/RecordsPanel.svelte'
  import EditorialPanel from '../../lib/components/period/EditorialPanel.svelte'
  import DesignPanel from '../../lib/components/period/DesignPanel.svelte'
  import DocumentsPanel from '../../lib/components/period/DocumentsPanel.svelte'

  type Props = { periodId: string; tab?: string }

  let { periodId, tab = 'overview' }: Props = $props()

  let detail = $state<PeriodDetail | null>(null)
  let metrics = $state<ReportMetrics | null>(null)
  let selectedImportId = $state('')
  let loading = $state(true)
  let metricsLoading = $state(false)
  let error = $state('')

  const activeTab = $derived(
    ['overview', 'imports', 'records', 'editorial', 'design', 'documents'].includes(tab) ? tab : 'overview',
  )
  const status = $derived(detail ? statusOfPeriod(detail.period) : null)

  onMount(() => {
    void load()
  })

  async function load(preferredImportId?: string) {
    loading = true
    error = ''
    try {
      detail = await periodsApi.detail(periodId)
      const nextImportId = preferredImportId || detail.latest_import_id || detail.imports[0]?.id || ''
      selectedImportId = nextImportId
      await loadMetrics(nextImportId)
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  async function loadMetrics(importId: string) {
    if (!importId) {
      metrics = null
      return
    }
    metricsLoading = true
    try {
      metrics = await periodsApi.metrics(periodId, importId)
    } catch (caught) {
      metrics = null
      toast.error(errorMessage(caught))
    } finally {
      metricsLoading = false
    }
  }

  async function selectImport(event: Event) {
    const importId = (event.currentTarget as HTMLSelectElement).value
    selectedImportId = importId
    await loadMetrics(importId)
  }

  async function handleImported(importId: string) {
    await load(importId)
    toast.success('Versi impor baru tersimpan. Riwayat sebelumnya tetap aman.')
  }

  async function handleGenerated() {
    if (detail) {
      detail.reports = await reportsApi.list(periodId)
    }
  }

  const tabs = $derived([
    { id: 'overview', label: 'Ringkasan' },
    { id: 'imports', label: 'Impor', count: detail?.imports.length ?? 0 },
    { id: 'records', label: 'Data sumber' },
    { id: 'editorial', label: 'Editorial' },
    { id: 'design', label: 'Desain' },
    { id: 'documents', label: 'Dokumen', count: detail?.reports.length ?? 0 },
  ])
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
          <span>{detail.imports.length} versi impor</span>
          <span class="text-stone-300">·</span>
          <span>{detail.reports.length} PDF</span>
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        {#if status}<Badge tone={status.tone}>{status.label}</Badge>{/if}
        <Button variant="secondary" size="sm" onclick={() => navigate(`/periods/${periodId}/imports`)}>
          <CloudUpload size={14} /> Impor
        </Button>
        <Button size="sm" onclick={() => navigate(`/periods/${periodId}/editorial`)}>
          <PenLine size={14} /> Susun laporan
        </Button>
      </div>
    </div>

    {#if detail.imports.length > 0}
      <div class="flex flex-wrap items-center gap-3 bg-stone-50/60 px-6 py-3">
        <span class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Versi data</span>
        <select class="input max-w-md py-2 text-xs" value={selectedImportId} onchange={selectImport}>
          {#each detail.imports as item}
            <option value={item.id}>{item.filename} · {item.row_count} baris</option>
          {/each}
        </select>
        {#if metricsLoading}<LoaderCircle size={15} class="animate-spin text-forest-600" />{/if}
        {#if metrics}
          <span class="text-[11px] text-ink-muted">{metrics.total_rows} baris dalam rentang · {metrics.outside_period} di luar rentang</span>
        {/if}
        <button class="btn btn-ghost btn-sm ml-auto" onclick={() => navigate(`/periods/${periodId}/documents`)}>
          <FileText size={14} /> Dokumen
        </button>
      </div>
    {/if}
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
  {:else if activeTab === 'imports'}
    <ImportPanel {detail} onimported={handleImported} />
  {:else if activeTab === 'records'}
    <RecordsPanel {periodId} {detail} {selectedImportId} onimportchange={selectImport} />
  {:else if activeTab === 'editorial'}
    <EditorialPanel
      {periodId}
      {detail}
      {metrics}
      {selectedImportId}
      onchangeimport={selectImport}
      ongenerated={handleGenerated}
    />
  {:else if activeTab === 'design'}
    <DesignPanel
      {periodId}
      {detail}
      {selectedImportId}
      onchangeimport={selectImport}
      ongenerated={handleGenerated}
    />
  {:else if activeTab === 'documents'}
    <DocumentsPanel reports={detail.reports} onrefresh={() => load(selectedImportId)} />
  {/if}
{/if}
