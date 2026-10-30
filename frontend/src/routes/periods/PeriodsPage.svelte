<script lang="ts">
  import { onMount } from 'svelte'
  import {
    AlertCircle,
    Archive,
    ArchiveRestore,
    ArrowRight,
    CalendarDays,
    CloudUpload,
    FileText,
    Pencil,
    Plus,
    Search,
  } from 'lucide-svelte'
  import { periodsApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { Period } from '../../lib/types'
  import { formatDate, formatRange, statusOfPeriod } from '../../lib/utils/format'
  import { navigate } from '../../lib/stores/router.svelte'
  import { dialog } from '../../lib/stores/dialog.svelte'
  import { toast } from '../../lib/stores/toast.svelte'
  import PageHeader from '../../lib/components/ui/PageHeader.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import EmptyState from '../../lib/components/ui/EmptyState.svelte'
  import Tabs from '../../lib/components/ui/Tabs.svelte'
  import PeriodFormModal from '../../lib/components/period/PeriodFormModal.svelte'

  let periods = $state<Period[]>([])
  let loading = $state(true)
  let error = $state('')
  let search = $state('')
  let scope = $state<'active' | 'archived'>('active')
  let formOpen = $state(false)
  let editing = $state<Period | null>(null)
  let busyId = $state('')

  onMount(load)

  async function load() {
    loading = true
    error = ''
    try {
      periods = await periodsApi.list(true)
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  const activePeriods = $derived(periods.filter((period) => !period.archived_at))
  const archivedPeriods = $derived(periods.filter((period) => period.archived_at))

  const filtered = $derived(
    (scope === 'archived' ? archivedPeriods : activePeriods).filter((period) => {
      const term = search.trim().toLowerCase()
      if (!term) return true
      return (
        period.label.toLowerCase().includes(term) ||
        period.start_date.includes(term) ||
        period.end_date.includes(term)
      )
    }),
  )

  const totalImports = $derived(activePeriods.reduce((total, period) => total + period.imports, 0))
  const totalReports = $derived(activePeriods.reduce((total, period) => total + period.reports, 0))

  function openCreate() {
    editing = null
    formOpen = true
  }

  function openEdit(period: Period) {
    editing = period
    formOpen = true
  }

  function closeForm() {
    formOpen = false
    editing = null
  }

  async function archivePeriod(period: Period) {
    const confirmed = await dialog.confirm({
      title: 'Arsipkan periode?',
      message: `Periode "${period.label}" akan dipindahkan ke arsip dan tidak tampil di daftar aktif maupun ringkasan. Data impor, desain, dan dokumen tetap tersimpan, dan periode dapat dipulihkan kapan saja.`,
      confirmLabel: 'Arsipkan',
      cancelLabel: 'Batal',
    })
    if (!confirmed) return

    busyId = period.id
    try {
      await periodsApi.archive(period.id)
      toast.success('Periode laporan diarsipkan.')
      await load()
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      busyId = ''
    }
  }

  async function restorePeriod(period: Period) {
    busyId = period.id
    try {
      await periodsApi.restore(period.id)
      toast.success('Periode laporan dipulihkan.')
      await load()
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      busyId = ''
    }
  }
</script>

<svelte:head><title>Periode laporan · Laporan Mingguan PBM</title></svelte:head>

<PageHeader
  eyebrow="ARSIP & PELAPORAN"
  title="Periode laporan"
  description="Kelola rentang laporan mingguan, unggah versi data, dan lanjutkan penyusunan dokumen."
>
  {#snippet actions()}
    <Button onclick={openCreate}><Plus size={16} /> Buat periode</Button>
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
    <div>
      <p class="text-[10px] font-bold tracking-[0.14em] text-ink-muted uppercase">Periode aktif</p>
      <strong class="font-display text-xl text-forest-900">{activePeriods.length}</strong>
      <p class="text-[10px] text-ink-muted">{archivedPeriods.length} diarsipkan</p>
    </div>
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
      <span class="eyebrow">{scope === 'archived' ? 'PERIODE DIARSIPKAN' : 'PERIODE AKTIF'}</span>
      <h2 class="panel-title mt-1">Daftar periode laporan</h2>
    </div>
    <div class="flex w-full flex-col gap-2 sm:w-auto sm:flex-row sm:items-center">
      <Tabs
        items={[
          { id: 'active', label: 'Aktif', count: activePeriods.length },
          { id: 'archived', label: 'Arsip', count: archivedPeriods.length },
        ]}
        active={scope}
        onchange={(id: string) => (scope = id as 'active' | 'archived')}
      />
      <label class="relative block w-full sm:w-64">
        <Search size={15} class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-ink-muted" />
        <input class="input py-2 pl-9 text-xs" bind:value={search} placeholder="Cari periode atau tanggal…" />
      </label>
    </div>
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
          {#each filtered as period (period.id)}
            {@const status = statusOfPeriod(period)}
            <tr>
              <td>
                <button class="table-link" onclick={() => navigate(`/periods/${period.id}/overview`)}>{period.label}</button>
                <p class="mt-0.5 text-[10px] text-ink-muted">
                  {#if period.archived_at}
                    Diarsipkan {formatDate(period.archived_at.slice(0, 10))} · {period.created_by}
                  {:else}
                    Dibuat {formatDate(period.created_at.slice(0, 10))} · {period.created_by}
                  {/if}
                </p>
              </td>
              <td class="text-xs whitespace-nowrap text-ink-muted">{formatRange(period.start_date, period.end_date)}</td>
              <td class="text-right text-xs font-semibold text-ink-soft">{period.imports}</td>
              <td class="text-right text-xs font-semibold text-ink-soft">{period.reports}</td>
              <td><Badge tone={status.tone}>{status.label}</Badge></td>
              <td class="text-right">
                <div class="flex flex-wrap justify-end gap-1">
                  <button class="btn btn-ghost btn-sm" onclick={() => openEdit(period)}>
                    <Pencil size={14} /> Edit
                  </button>
                  {#if period.archived_at}
                    <button
                      class="btn btn-ghost btn-sm"
                      disabled={busyId === period.id}
                      onclick={() => restorePeriod(period)}
                    >
                      <ArchiveRestore size={14} /> Pulihkan
                    </button>
                  {:else}
                    <button
                      class="btn btn-ghost btn-sm"
                      disabled={busyId === period.id}
                      onclick={() => archivePeriod(period)}
                    >
                      <Archive size={14} /> Arsip
                    </button>
                  {/if}
                  <button class="btn btn-ghost btn-sm" onclick={() => navigate(`/periods/${period.id}/overview`)}>
                    Buka <ArrowRight size={14} />
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <EmptyState
      title={search
        ? 'Periode tidak ditemukan'
        : scope === 'archived'
          ? 'Belum ada periode diarsipkan'
          : archivedPeriods.length
            ? 'Belum ada periode aktif'
            : 'Belum ada periode laporan'}
      description={search
        ? 'Coba kata kunci lain atau hapus filter pencarian.'
        : scope === 'archived'
          ? 'Periode yang diarsipkan akan muncul di sini dan dapat dipulihkan kapan saja.'
          : archivedPeriods.length
            ? 'Semua periode sudah diarsipkan. Buka tab Arsip untuk melihat atau memulihkannya.'
            : 'Buat rentang tanggal laporan untuk mulai mengelola data mingguan.'}
    >
      {#if search}
        <Button variant="secondary" onclick={() => (search = '')}>Hapus pencarian</Button>
      {:else if scope === 'active' && archivedPeriods.length}
        <Button variant="secondary" onclick={() => (scope = 'archived')}>
          <ArchiveRestore size={15} /> Lihat arsip
        </Button>
      {:else if scope === 'active'}
        <Button onclick={openCreate}><Plus size={15} /> Buat periode pertama</Button>
      {/if}
    </EmptyState>
  {/if}
</section>

{#if formOpen}
  <PeriodFormModal
    open={true}
    period={editing}
    onclose={closeForm}
    onsaved={() => {
      void load()
    }}
  />
{/if}
