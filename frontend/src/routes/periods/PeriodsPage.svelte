<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertCircle, ArrowRight, CalendarDays, CloudUpload, FileText, Plus, Search } from 'lucide-svelte'
  import { periodsApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { Period } from '../../lib/types'
  import { formatDate, formatRange, statusOfPeriod } from '../../lib/utils/format'
  import { navigate } from '../../lib/stores/router.svelte'
  import PageHeader from '../../lib/components/ui/PageHeader.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import EmptyState from '../../lib/components/ui/EmptyState.svelte'
  import PeriodFormModal from '../../lib/components/period/PeriodFormModal.svelte'

  let periods = $state<Period[]>([])
  let loading = $state(true)
  let error = $state('')
  let search = $state('')
  let showForm = $state(false)

  onMount(load)

  async function load() {
    loading = true
    error = ''
    try {
      periods = await periodsApi.list()
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  const filtered = $derived(
    periods.filter((period) => {
      const term = search.trim().toLowerCase()
      if (!term) return true
      return (
        period.label.toLowerCase().includes(term) ||
        period.start_date.includes(term) ||
        period.end_date.includes(term)
      )
    }),
  )

  const totalImports = $derived(periods.reduce((total, period) => total + period.imports, 0))
  const totalReports = $derived(periods.reduce((total, period) => total + period.reports, 0))
</script>

<svelte:head><title>Periode laporan · Laporan Mingguan PBM</title></svelte:head>

<PageHeader
  eyebrow="ARSIP & PELAPORAN"
  title="Periode laporan"
  description="Kelola rentang laporan mingguan, unggah versi data, dan lanjutkan penyusunan dokumen."
>
  {#snippet actions()}
    <Button onclick={() => (showForm = true)}><Plus size={16} /> Buat periode</Button>
  {/snippet}
</PageHeader>

{#if error}
  <div class="mb-5 flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{/if}

<div class="mb-5 grid gap-3 sm:grid-cols-3">
  <div class="card flex items-center gap-3 px-5 py-4">
    <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><CalendarDays size={18} /></span>
    <div><p class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Periode</p><strong class="font-display text-xl text-forest-900">{periods.length}</strong></div>
  </div>
  <div class="card flex items-center gap-3 px-5 py-4">
    <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-gold-100 text-gold-700"><CloudUpload size={18} /></span>
    <div><p class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Total impor</p><strong class="font-display text-xl text-forest-900">{totalImports}</strong></div>
  </div>
  <div class="card flex items-center gap-3 px-5 py-4">
    <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><FileText size={18} /></span>
    <div><p class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">PDF dihasilkan</p><strong class="font-display text-xl text-forest-900">{totalReports}</strong></div>
  </div>
</div>

<section class="card overflow-hidden">
  <div class="panel-heading items-center">
    <div>
      <span class="eyebrow">SEMUA PERIODE</span>
      <h2 class="panel-title mt-1">Arsip laporan mingguan</h2>
    </div>
    <label class="relative block w-full sm:w-64">
      <Search size={15} class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-ink-muted" />
      <input class="input py-2 pl-9 text-xs" bind:value={search} placeholder="Cari periode atau tanggal…" />
    </label>
  </div>

  {#if loading}
    <div class="space-y-3 p-5">
      {#each Array(5) as _}<div class="skeleton h-12 w-full"></div>{/each}
    </div>
  {:else if filtered.length}
    <div class="overflow-x-auto">
      <table class="table">
        <thead>
          <tr>
            <th>Nama periode</th>
            <th>Rentang</th>
            <th class="text-right">Impor</th>
            <th class="text-right">PDF</th>
            <th>Status</th>
            <th class="text-right">Aksi</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as period}
            {@const status = statusOfPeriod(period)}
            <tr>
              <td>
                <button class="table-link" onclick={() => navigate(`/periods/${period.id}/overview`)}>{period.label}</button>
                <p class="mt-0.5 text-[10px] text-ink-muted">Dibuat {formatDate(period.created_at.slice(0, 10))} · {period.created_by}</p>
              </td>
              <td class="text-xs whitespace-nowrap text-ink-muted">{formatRange(period.start_date, period.end_date)}</td>
              <td class="text-right text-xs font-semibold text-ink-soft">{period.imports}</td>
              <td class="text-right text-xs font-semibold text-ink-soft">{period.reports}</td>
              <td><Badge tone={status.tone}>{status.label}</Badge></td>
              <td class="text-right">
                <button class="btn btn-ghost btn-sm" onclick={() => navigate(`/periods/${period.id}/overview`)}>
                  Buka <ArrowRight size={14} />
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <EmptyState
      title={search ? 'Periode tidak ditemukan' : 'Belum ada periode laporan'}
      description={search ? 'Coba kata kunci lain atau hapus filter pencarian.' : 'Buat rentang tanggal laporan untuk mulai mengelola data mingguan.'}
    >
      {#if search}
        <Button variant="secondary" onclick={() => (search = '')}>Hapus pencarian</Button>
      {:else}
        <Button onclick={() => (showForm = true)}><Plus size={15} /> Buat periode pertama</Button>
      {/if}
    </EmptyState>
  {/if}
</section>

<PeriodFormModal open={showForm} onclose={() => (showForm = false)} />
