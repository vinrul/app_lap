<script lang="ts">
  import { onMount } from 'svelte'
  import {
    AlertCircle,
    Copy,
    FileDown,
    Link2,
    RefreshCw,
    ShieldAlert,
  } from 'lucide-svelte'
  import {
    downloadDriveReport,
    downloadDuplicateReport,
    downloadRapidBotReport,
    downloadScheduleReport,
    importsApi,
  } from '../../api'
  import { errorMessage } from '../../api/client'
  import type {
    AnevIndicators,
    BotDetection,
    CoverageResponse,
    CoverageSchedule,
    CoverageScheduleChild,
    CoverageScheduleStatus,
    CoverageScheduleUnit,
    DuplicateGroup,
    DriveGroup,
    RapidInputSuspect,
    RepeatedMaterialSuspect,
  } from '../../types'
  import { formatDate, formatNumber } from '../../utils/format'
  import { auth } from '../../stores/auth.svelte'
  import { toast } from '../../stores/toast.svelte'
  import Button from '../ui/Button.svelte'
  import Tabs from '../ui/Tabs.svelte'
  import BotSuspectDialog from '../ui/BotSuspectDialog.svelte'
  import BotMaterialDialog from '../ui/BotMaterialDialog.svelte'

  type Props = { periodId: string }
  let { periodId }: Props = $props()

  type AnevTab = 'rapid' | 'schedule' | 'duplicate' | 'drive' | 'material'

  const reader = $derived(auth.user?.role === 'polres' || auth.user?.role === 'polda')

  let activeTab = $state<AnevTab>('rapid')
  let data = $state<CoverageResponse | null>(null)
  let indicators = $state<AnevIndicators | null>(null)
  let loading = $state(true)
  let error = $state('')
  let polresFilter = $state('')
  let polresLoading = $state(false)
  let polresError = $state('')
  let polresRequest = 0
  let scheduleStatus = $state<'' | CoverageScheduleStatus>('')
  let scheduleScope = $state<'' | 'polres' | 'satker'>('')
  let botSuspect = $state<RapidInputSuspect | null>(null)
  let materialSuspect = $state<RepeatedMaterialSuspect | null>(null)
  let rapidPdfBusy = $state(false)
  let schedulePdfBusy = $state(false)
  let duplicatePdfBusy = $state(false)
  let drivePdfBusy = $state(false)
  let rebuildBusy = $state(false)

  const emptyBotDetection: BotDetection = { rapid_input: [], repeated_material: [] }

  onMount(load)

  const botDetection = $derived(data?.bot_detection ?? emptyBotDetection)
  const schedules = $derived(data?.schedule ?? [])
  const scheduled = $derived(schedules.filter((item) => item.expected_dates.length))
  const polresOptions = $derived(data?.polres_options ?? [])
  const activePolres = $derived(data?.filter_polres ?? '')
  const primaryLabel = $derived(activePolres ? 'Polsek' : 'Polres')
  const secondaryLabel = $derived(activePolres ? 'Satker Polres' : 'Satker Polda')
  const primaryLower = $derived(activePolres ? 'polsek' : 'polres')
  const secondaryLower = $derived(activePolres ? 'satker polres' : 'satker')

  const relatedRapid = $derived.by(() => {
    const current = botSuspect
    if (!current) return []
    return botDetection.rapid_input.filter(
      (item) => item !== current && item.tanggal === current.tanggal,
    )
  })

  const duplicateUnits = $derived(
    (indicators?.duplicate.units ?? []).filter(
      (unit) => unit.exact_rows > 0 || unit.fuzzy_rows > 0,
    ),
  )
  const driveUnits = $derived(
    (indicators?.drive.units ?? []).filter(
      (unit) => unit.repeated_rows > 0 || unit.repeated_foto_rows > 0,
    ),
  )
  const duplicateExactTotal = $derived(
    (indicators?.duplicate.units ?? []).reduce((total, unit) => total + unit.exact_rows, 0),
  )
  const duplicateFuzzyTotal = $derived(
    (indicators?.duplicate.units ?? []).reduce((total, unit) => total + unit.fuzzy_rows, 0),
  )

  const tabs = $derived([
    { id: 'rapid', label: 'Input cepat', count: botDetection.rapid_input.length },
    { id: 'schedule', label: 'Jadwal pelaporan', count: scheduled.length },
    { id: 'duplicate', label: 'Duplikasi isian', count: duplicateUnits.length },
    { id: 'drive', label: 'Tautan Drive', count: driveUnits.length },
    {
      id: 'material',
      label: 'Materi identik lintas satuan',
      count: botDetection.repeated_material.length,
    },
  ])

  async function load() {
    loading = true
    error = ''
    try {
      const [coverageData, indicatorData] = await Promise.all([
        importsApi.coverage(periodId, polresFilter || undefined),
        importsApi.anevIndicators(periodId, polresFilter || undefined),
      ])
      data = coverageData
      indicators = indicatorData
    } catch (caught) {
      data = null
      indicators = null
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  async function changePolres(value: string) {
    polresFilter = value
    polresError = ''
    const current = ++polresRequest
    polresLoading = true
    try {
      const [coverageData, indicatorData] = await Promise.all([
        importsApi.coverage(periodId, value || undefined),
        importsApi.anevIndicators(periodId, value || undefined),
      ])
      if (current !== polresRequest) return
      data = coverageData
      indicators = indicatorData
    } catch (caught) {
      if (current !== polresRequest) return
      polresError = errorMessage(caught)
      polresFilter = data?.filter_polres ?? ''
    } finally {
      if (current === polresRequest) polresLoading = false
    }
  }

  async function downloadRapidPdf(): Promise<void> {
    if (rapidPdfBusy) return
    rapidPdfBusy = true
    try {
      await downloadRapidBotReport(periodId, polresFilter || undefined)
      toast.success('Laporan PDF input cepat berhasil diunduh.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      rapidPdfBusy = false
    }
  }

  async function downloadSchedulePdf(): Promise<void> {
    if (schedulePdfBusy) return
    schedulePdfBusy = true
    try {
      await downloadScheduleReport(periodId, polresFilter || undefined)
      toast.success('Laporan PDF jadwal pelaporan berhasil diunduh.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      schedulePdfBusy = false
    }
  }

  async function downloadDuplicatePdf(): Promise<void> {
    if (duplicatePdfBusy) return
    duplicatePdfBusy = true
    try {
      await downloadDuplicateReport(periodId, polresFilter || undefined)
      toast.success('Laporan PDF duplikasi isian berhasil diunduh.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      duplicatePdfBusy = false
    }
  }

  async function downloadDrivePdf(): Promise<void> {
    if (drivePdfBusy) return
    drivePdfBusy = true
    try {
      await downloadDriveReport(periodId, polresFilter || undefined)
      toast.success('Laporan PDF tautan Drive berhasil diunduh.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      drivePdfBusy = false
    }
  }

  async function rebuildDuplicates(): Promise<void> {
    if (rebuildBusy) return
    rebuildBusy = true
    try {
      const result = await importsApi.rebuildAnev(periodId)
      toast.success(`${formatNumber(result.duplicates)} baris duplikat disiapkan.`)
      await load()
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      rebuildBusy = false
    }
  }

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

  function scheduleUnitsFor(item: CoverageSchedule): CoverageScheduleUnit[] {
    const units: CoverageScheduleUnit[] = []
    if (scheduleScope !== 'satker') units.push(...item.polres)
    if (scheduleScope !== 'polres') units.push(...item.satker)
    return units
  }

  function scheduleCount(item: CoverageSchedule, status: CoverageScheduleStatus): number {
    return scheduleUnitsFor(item).filter((unit) => unit.status === status).length
  }

  function filterScheduleUnits(units: CoverageScheduleUnit[]): CoverageScheduleUnit[] {
    return scheduleStatus ? units.filter((unit) => unit.status === scheduleStatus) : units
  }

  function filterScheduleChildren(children: CoverageScheduleChild[]): CoverageScheduleChild[] {
    return scheduleStatus ? children.filter((child) => child.status === scheduleStatus) : children
  }

  function scheduleStatusLabel(status: CoverageScheduleStatus): string {
    switch (status) {
      case 'on_time':
        return 'Sesuai jadwal'
      case 'off_schedule':
        return 'Di luar jadwal'
      case 'missing':
        return 'Belum melapor'
      default:
        return 'Tidak ada jadwal'
    }
  }

  function scheduleUnitClass(status: CoverageScheduleStatus): string {
    const base =
      'inline-flex items-center rounded-full border px-1.5 py-0.5 text-[9px] font-semibold'
    switch (status) {
      case 'on_time':
        return `${base} border-forest-200 bg-forest-50 text-forest-700`
      case 'off_schedule':
        return `${base} border-gold-300 bg-gold-50 text-gold-700`
      case 'missing':
        return `${base} border-red-200 bg-red-50 text-red-700`
      default:
        return `${base} border-line bg-stone-100 text-ink-muted`
    }
  }
</script>

{#snippet scheduleUnits(items: CoverageScheduleUnit[])}
  <div class="space-y-1">
    {#each filterScheduleUnits(items) as unit (unit.label)}
      <div class="rounded-lg border border-line bg-white px-2 py-1.5">
        <div class="flex flex-wrap items-center justify-between gap-1">
          <span class="text-[10px] font-semibold text-ink">{unit.label}</span>
          <span class={scheduleUnitClass(unit.status)}>{scheduleStatusLabel(unit.status)}</span>
        </div>
        {#if unit.on_time_dates.length}
          <p class="mt-0.5 text-[9px] text-forest-700">Sesuai: {unit.on_time_dates.map(formatDate).join(', ')}</p>
        {/if}
        {#if unit.off_schedule_dates.length}
          <p class="mt-0.5 text-[9px] text-gold-700">Hari laporan: {unit.off_schedule_dates.map(formatDate).join(', ')}</p>
        {/if}
        {#if unit.polsek.length || unit.satker.length}
          <details class="mt-1">
            <summary class="cursor-pointer text-[9px] font-semibold text-forest-700">Rincian polsek / satker</summary>
            <div class="mt-1 space-y-1.5 border-t border-line pt-1.5">
              {#if unit.polsek.length}
                <div>
                  <p class="text-[8px] font-bold tracking-[0.1em] text-ink-muted uppercase">Polsek</p>
                  {#each filterScheduleChildren(unit.polsek) as child (child.label)}
                    <div class="mt-0.5 rounded-md border border-line bg-stone-50/60 px-1.5 py-1">
                      <div class="flex flex-wrap items-center justify-between gap-1">
                        <span class="text-[9px] font-medium text-ink">{child.label}</span>
                        <span class={scheduleUnitClass(child.status)}>{scheduleStatusLabel(child.status)}</span>
                      </div>
                      {#if child.on_time_dates.length}<p class="text-[8px] text-forest-700">Sesuai: {child.on_time_dates.map(formatDate).join(', ')}</p>{/if}
                      {#if child.off_schedule_dates.length}<p class="text-[8px] text-gold-700">Hari laporan: {child.off_schedule_dates.map(formatDate).join(', ')}</p>{/if}
                    </div>
                  {:else}
                    <p class="mt-0.5 text-[9px] text-ink-muted">Tidak ada polsek pada filter ini.</p>
                  {/each}
                </div>
              {/if}
              {#if unit.satker.length}
                <div>
                  <p class="text-[8px] font-bold tracking-[0.1em] text-ink-muted uppercase">Satker polres</p>
                  {#each filterScheduleChildren(unit.satker) as child (child.label)}
                    <div class="mt-0.5 rounded-md border border-line bg-stone-50/60 px-1.5 py-1">
                      <div class="flex flex-wrap items-center justify-between gap-1">
                        <span class="text-[9px] font-medium text-ink">{child.label}</span>
                        <span class={scheduleUnitClass(child.status)}>{scheduleStatusLabel(child.status)}</span>
                      </div>
                      {#if child.on_time_dates.length}<p class="text-[8px] text-forest-700">Sesuai: {child.on_time_dates.map(formatDate).join(', ')}</p>{/if}
                      {#if child.off_schedule_dates.length}<p class="text-[8px] text-gold-700">Hari laporan: {child.off_schedule_dates.map(formatDate).join(', ')}</p>{/if}
                    </div>
                  {:else}
                    <p class="mt-0.5 text-[9px] text-ink-muted">Tidak ada satker pada filter ini.</p>
                  {/each}
                </div>
              {/if}
            </div>
          </details>
        {/if}
      </div>
    {:else}
      <p class="text-[10px] text-ink-muted">Tidak ada unit pada filter ini.</p>
    {/each}
  </div>
{/snippet}

{#snippet duplicateGroupTable(groups: DuplicateGroup[], emptyText: string)}
  {#if groups.length}
    <div class="max-h-96 space-y-2 overflow-auto pr-1">
      {#each groups as group, index (index)}
        <details class="rounded-xl border border-line bg-white">
          <summary class="flex cursor-pointer flex-wrap items-center justify-between gap-2 px-3 py-2">
            <span class="min-w-0">
              <strong class="text-xs text-ink">{group.satuan}</strong>
              <span class="text-[10px] text-ink-muted"> · {group.kode} · {group.kegiatan}</span>
            </span>
            <span class="badge badge-danger shrink-0">{group.count} baris</span>
          </summary>
          <div class="space-y-2 border-t border-line p-3">
            <p class="text-[10px] text-ink-muted">
              Kemiripan: {group.similarity >= 0.999 ? 'persis' : `≥${Math.round(group.similarity * 100)}%`}
              · Baris {group.source_rows.join(', ')}
            </p>
            <p class="max-h-24 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-stone-50 p-2 text-[10px] text-ink-soft">{group.sample}</p>
            <div class="max-h-64 overflow-auto rounded-lg border border-line">
              <table class="min-w-full text-left text-[10px]">
                <thead class="sticky top-0 bg-stone-50 text-[9px] text-ink-muted uppercase">
                  <tr>
                    <th class="px-2 py-1.5">Baris</th>
                    <th class="px-2 py-1.5">Tanggal</th>
                    <th class="px-2 py-1.5">Kode</th>
                    <th class="px-2 py-1.5">Kegiatan</th>
                    <th class="px-2 py-1.5">Isian</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-line">
                  {#each group.rows as row (row.source_row)}
                    <tr class="align-top">
                      <td class="px-2 py-1.5 font-semibold">{row.source_row}</td>
                      <td class="whitespace-nowrap px-2 py-1.5 text-ink-muted">{formatDate(row.tanggal)}</td>
                      <td class="whitespace-nowrap px-2 py-1.5">{row.kode || '—'}</td>
                      <td class="max-w-[180px] px-2 py-1.5">{row.kegiatan || '—'}</td>
                      <td class="px-2 py-1.5">
                        <p class="max-w-[420px] whitespace-pre-wrap break-words text-[9px] text-ink-soft">{row.isian_laporan || '—'}</p>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        </details>
      {/each}
    </div>
  {:else}
    <p class="rounded-xl border border-dashed border-line px-3 py-5 text-center text-[11px] text-ink-muted">{emptyText}</p>
  {/if}
{/snippet}

{#snippet driveGroupTable(groups: DriveGroup[])}
  {#if groups.length}
    <div class="max-h-[28rem] space-y-2 overflow-auto pr-1">
      {#each groups as group, index (index)}
        <details class="rounded-xl border border-line bg-white">
          <summary class="flex cursor-pointer flex-wrap items-center justify-between gap-2 px-3 py-2">
            <span class="min-w-0">
              <strong class="text-xs text-ink">{group.count}× {group.drive_id}</strong>
              <span class="text-[10px] text-ink-muted">
                · {group.units.slice(0, 4).join(', ')}{#if group.units.length > 4} +{group.units.length - 4}{/if}
              </span>
            </span>
            <span class="badge badge-danger shrink-0">{group.count} baris</span>
          </summary>
          <div class="space-y-2 border-t border-line p-3">
            <a class="block break-all text-[10px] text-forest-700 underline" href={group.link} target="_blank" rel="noreferrer">{group.link || group.drive_id}</a>
            <p class="max-h-24 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-stone-50 p-2 text-[10px] text-ink-soft">{group.sample}</p>
            <div class="max-h-64 overflow-auto rounded-lg border border-line">
              <table class="min-w-full text-left text-[10px]">
                <thead class="sticky top-0 bg-stone-50 text-[9px] text-ink-muted uppercase">
                  <tr>
                    <th class="px-2 py-1.5">Baris</th>
                    <th class="px-2 py-1.5">Tanggal</th>
                    <th class="px-2 py-1.5">Satuan</th>
                    <th class="px-2 py-1.5">Kode</th>
                    <th class="px-2 py-1.5">Isian</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-line">
                  {#each group.rows as row (row.source_row)}
                    <tr class="align-top">
                      <td class="px-2 py-1.5 font-semibold">{row.source_row}</td>
                      <td class="whitespace-nowrap px-2 py-1.5 text-ink-muted">{formatDate(row.tanggal)}</td>
                      <td class="max-w-[180px] px-2 py-1.5">{row.satuan || '—'}</td>
                      <td class="whitespace-nowrap px-2 py-1.5">{row.kode || '—'}</td>
                      <td class="px-2 py-1.5">
                        <p class="max-w-[420px] whitespace-pre-wrap break-words text-[9px] text-ink-soft">{row.isian_laporan || '—'}</p>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        </details>
      {/each}
    </div>
  {:else}
    <p class="rounded-xl border border-dashed border-line px-3 py-5 text-center text-[11px] text-ink-muted">Tidak ada tautan Drive yang berulang.</p>
  {/if}
{/snippet}

{#if loading}
  <div class="space-y-4">
    <div class="skeleton h-20 w-full"></div>
    <div class="skeleton h-80 w-full"></div>
  </div>
{:else if error}
  <div class="flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{:else if data}
  <div class="space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-line bg-white px-4 py-3">
      <div>
        <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Filter anev</p>
        <p class="mt-0.5 text-[11px] text-ink-muted">
          {reader
            ? 'Data dibatasi otomatis sesuai unit akun Anda.'
            : polresFilter
              ? `Menampilkan ${polresFilter} saja: rincian per ${primaryLower} dan ${secondaryLower}.`
              : 'Menampilkan seluruh Polda. Pilih polres untuk mempersempit input cepat dan jadwal.'}
        </p>
      </div>
      {#if !reader}
        <label class="flex items-center gap-2 text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">
          Polres
          <select
            class="input w-60 py-1.5 text-xs font-normal tracking-normal normal-case"
            value={polresFilter}
            disabled={polresLoading}
            onchange={(event) => changePolres(event.currentTarget.value)}
          >
            <option value="">Semua polres</option>
            {#each polresOptions as option (option)}
              <option value={option}>{option}</option>
            {/each}
          </select>
        </label>
      {/if}
    </div>

    {#if polresError}
      <div class="flex flex-wrap items-center gap-2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
        <AlertCircle size={17} /> {polresError}
      </div>
    {/if}

    <Tabs
      items={tabs}
      active={activeTab}
      onchange={(id: string) => (activeTab = id as AnevTab)}
    />

    {#if activeTab === 'rapid'}
      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow flex items-center gap-1.5 text-red-700">
              <ShieldAlert size={13} /> INDIKASI LAPORAN OTOMATIS
            </span>
            <h2 class="panel-title mt-0.5">Input cepat berulang</h2>
            <p class="mt-1 max-w-3xl text-[10px] leading-relaxed text-ink-muted">
              Satuan yang mengirim banyak laporan dalam jendela waktu singkat. Indikasi untuk pemeriksaan,
              bukan penilaian otomatis. Klik kartu untuk melihat rincian baris.
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-1.5">
            <span class="badge badge-danger">{botDetection.rapid_input.length} satuan</span>
            {#if botDetection.rapid_input.length}
              <Button
                variant="secondary"
                size="sm"
                loading={rapidPdfBusy}
                onclick={() => void downloadRapidPdf()}
              >
                <FileDown size={13} /> PDF input cepat
              </Button>
            {/if}
          </div>
        </div>

        <div class="space-y-2 border-t border-line p-4">
          {#each botDetection.rapid_input as item}
            <button
              type="button"
              class="w-full rounded-xl border border-red-200 bg-red-50/40 px-3 py-2 text-left transition-colors hover:border-red-400 hover:bg-red-50"
              title="Lihat rincian baris {item.satuan}"
              onclick={() => (botSuspect = item)}
            >
              <div class="flex items-start justify-between gap-2">
                <strong class="text-xs text-ink">{item.satuan}</strong>
                <span class="badge badge-danger shrink-0">{item.report_count} laporan</span>
              </div>
              <p class="mt-1 text-[10px] text-ink-muted">
                {formatDate(item.tanggal)} · {item.first_time}–{item.last_time} · {formatWindow(item.window_seconds)} · {formatPace(item.report_count, item.window_seconds)}
              </p>
              {#if item.same_minute_count > 1}
                <p class="mt-0.5 text-[10px] font-medium text-red-700">
                  {item.same_minute_count} laporan pada menit yang sama
                </p>
              {/if}
              <p class="mt-1 line-clamp-2 text-[10px] text-ink-soft">{item.activities.join(' · ')}</p>
              <p class="mt-1 text-[9px] text-ink-muted">
                Baris {item.source_rows.join(', ')}{#if item.report_count > item.source_rows.length} · +{item.report_count - item.source_rows.length} baris lain{/if}
              </p>
              <span class="mt-1 inline-block text-[10px] font-semibold text-red-700">Lihat rincian →</span>
            </button>
          {:else}
            <p class="rounded-xl border border-dashed border-line px-3 py-6 text-center text-[11px] text-ink-muted">
              Tidak ada indikasi input cepat berulang pada periode ini.
            </p>
          {/each}
        </div>
      </section>
    {:else if activeTab === 'schedule'}
      <section class="card overflow-hidden">
        <div class="px-5 py-4">
          <div class="flex flex-wrap items-end justify-between gap-3 border-b border-line pb-3">
            <div>
              <span class="eyebrow">KEPATUHAN JADWAL</span>
              <h2 class="panel-title mt-0.5">Jadwal pelaporan QW1.1–QW1.8</h2>
              <p class="mt-1 max-w-3xl text-[11px] leading-relaxed text-ink-muted">
                Senin: QW1.1 & QW1.2 · Rabu: QW1.4 · Kamis: QW1.3 · Jumat: QW1.5 & QW1.6 · Sabtu: QW1.7 · Minggu: QW1.8.
                Laporan pada hari lain dihitung di luar jadwal. Sub-kode lain dapat dilaporkan setiap hari.
              </p>
            </div>
            <div class="flex flex-wrap items-end gap-2">
              <label class="flex flex-col gap-1">
                <span class="text-[9px] font-bold tracking-[0.1em] text-ink-muted uppercase">Status</span>
                <select class="input w-44 py-1.5 text-xs" bind:value={scheduleStatus}>
                  <option value="">Semua status</option>
                  <option value="on_time">Sesuai jadwal</option>
                  <option value="off_schedule">Di luar jadwal</option>
                  <option value="missing">Belum melapor</option>
                </select>
              </label>
              <label class="flex flex-col gap-1">
                <span class="text-[9px] font-bold tracking-[0.1em] text-ink-muted uppercase">Unit</span>
                <select class="input w-48 py-1.5 text-xs" bind:value={scheduleScope}>
                  <option value="">{primaryLabel} & {secondaryLower}</option>
                  <option value="polres">{primaryLabel}</option>
                  <option value="satker">{secondaryLabel}</option>
                </select>
              </label>
              <Button
                variant="secondary"
                size="sm"
                loading={schedulePdfBusy}
                disabled={!scheduled.length}
                onclick={() => void downloadSchedulePdf()}
              >
                <FileDown size={13} /> PDF jadwal
              </Button>
            </div>
          </div>

          <div class="mt-3 grid gap-3 xl:grid-cols-2">
            {#each scheduled as item (item.code)}
              <article class="rounded-xl border border-line bg-white p-3.5">
                <div class="flex flex-wrap items-start justify-between gap-2">
                  <div class="min-w-0">
                    <p class="text-sm font-semibold text-ink">
                      <span class="rounded-md bg-forest-50 px-1.5 py-0.5 text-[10px] font-bold text-forest-700">{item.code}</span>
                      <span class="ml-1.5 text-[11px] font-normal text-ink-muted">{item.title}</span>
                    </p>
                    <p class="mt-1 text-[10px] text-ink-muted">
                      Jadwal <strong class="text-ink">{item.day}</strong>
                      {#if item.expected_dates.length}· {item.expected_dates.map(formatDate).join(', ')}{/if}
                    </p>
                  </div>
                  <span class="badge badge-neutral">{item.day}</span>
                </div>

                <div class="mt-2 flex flex-wrap gap-1.5">
                  <span class="inline-flex items-center rounded-full border border-forest-200 bg-forest-50 px-1.5 py-0.5 text-[9px] font-semibold text-forest-700">
                    {scheduleCount(item, 'on_time')} sesuai
                  </span>
                  <span class="inline-flex items-center rounded-full border border-gold-300 bg-gold-50 px-1.5 py-0.5 text-[9px] font-semibold text-gold-700">
                    {scheduleCount(item, 'off_schedule')} di luar
                  </span>
                  <span class="inline-flex items-center rounded-full border border-red-200 bg-red-50 px-1.5 py-0.5 text-[9px] font-semibold text-red-700">
                    {scheduleCount(item, 'missing')} belum
                  </span>
                </div>

                <details class="mt-2" open={scheduleStatus !== ''}>
                  <summary class="cursor-pointer text-[10px] font-semibold text-forest-700">Lihat unit</summary>
                  <div class="mt-2 grid gap-2 sm:grid-cols-2">
                    {#if scheduleScope !== 'satker'}
                      <div>
                        <p class="text-[9px] font-bold tracking-[0.1em] text-ink-muted uppercase">{primaryLabel}</p>
                        <div class="mt-1">{@render scheduleUnits(item.polres)}</div>
                      </div>
                    {/if}
                    {#if scheduleScope !== 'polres'}
                      <div>
                        <p class="text-[9px] font-bold tracking-[0.1em] text-ink-muted uppercase">{secondaryLabel}</p>
                        <div class="mt-1">{@render scheduleUnits(item.satker)}</div>
                      </div>
                    {/if}
                  </div>
                </details>
              </article>
            {:else}
              <p class="rounded-xl border border-dashed border-line bg-white px-3 py-6 text-center text-[11px] text-ink-muted xl:col-span-2">
                Periode ini tidak memuat hari yang menjadi jadwal QW1.1–QW1.8.
              </p>
            {/each}
          </div>
        </div>
      </section>
    {:else if activeTab === 'duplicate'}
      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow flex items-center gap-1.5 text-red-700">
              <Copy size={13} /> DUPLIKASI ISIAN
            </span>
            <h2 class="panel-title mt-0.5">Kemiripan isian per satuan</h2>
            <p class="mt-1 max-w-3xl text-[10px] leading-relaxed text-ink-muted">
              Isian laporan yang sama atau mirip di dalam satuan. Duplikat persis memakai teks setelah
              normalisasi huruf, spasi, dan tautan; kemiripan ≥75% memakai Jaccard shingle lima kata.
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-1.5">
            <span class="badge badge-danger">{duplicateUnits.length} satuan</span>
            <Button
              variant="secondary"
              size="sm"
              loading={duplicatePdfBusy}
              onclick={() => void downloadDuplicatePdf()}
            >
              <FileDown size={13} /> PDF duplikasi
            </Button>
          </div>
        </div>

        {#if indicators}
          {#if !indicators.duplicate_ready}
            <div class="m-4 rounded-xl border border-gold-300 bg-gold-50/60 p-3 text-[11px] text-gold-800">
              <p>
                Analisis duplikat belum lengkap. Impor yang belum disiapkan:
                {indicators.pending_imports.join(', ')}.
              </p>
              {#if !reader}
                <Button
                  class="mt-2"
                  variant="secondary"
                  size="sm"
                  loading={rebuildBusy}
                  onclick={() => void rebuildDuplicates()}
                >
                  <RefreshCw size={13} /> Siapkan analisis duplikat
                </Button>
              {:else}
                <p class="mt-1 text-[10px]">Minta operator menyiapkan analisis duplikat.</p>
              {/if}
            </div>
          {/if}

          <div class="grid gap-3 border-t border-line p-4 sm:grid-cols-2 xl:grid-cols-4">
            <article class="rounded-xl border border-line bg-stone-50/60 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Baris diperiksa</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(indicators.total_rows)}</p>
            </article>
            <article class="rounded-xl border border-line bg-stone-50/60 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Satuan terindikasi</p>
              <p class="mt-1 font-display text-xl text-forest-900">{duplicateUnits.length}</p>
            </article>
            <article class="rounded-xl border border-red-200 bg-red-50/50 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-red-700 uppercase">Duplikat persis</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(duplicateExactTotal)}</p>
            </article>
            <article class="rounded-xl border border-red-200 bg-red-50/50 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-red-700 uppercase">Mirip ≥75%</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(duplicateFuzzyTotal)}</p>
            </article>
          </div>

          <div class="border-t border-line p-4">
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Ringkasan per satuan</p>
            <div class="mt-2 max-h-80 overflow-auto rounded-xl border border-line">
              <table class="min-w-full text-left text-[11px]">
                <thead class="sticky top-0 bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
                  <tr>
                    <th class="px-3 py-2">Satuan</th>
                    <th class="px-3 py-2 text-right">Baris</th>
                    <th class="px-3 py-2 text-right">Persis</th>
                    <th class="px-3 py-2 text-right">%</th>
                    <th class="px-3 py-2 text-right">Mirip</th>
                    <th class="px-3 py-2 text-right">%</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-line">
                  {#each duplicateUnits.slice(0, 50) as unit (unit.satuan)}
                    <tr class="hover:bg-stone-50/60">
                      <td class="px-3 py-1.5 font-medium text-ink">{unit.satuan}</td>
                      <td class="px-3 py-1.5 text-right">{formatNumber(unit.rows)}</td>
                      <td class="px-3 py-1.5 text-right">{formatNumber(unit.exact_rows)}</td>
                      <td class="px-3 py-1.5 text-right text-ink-muted">{unit.exact_percent}%</td>
                      <td class="px-3 py-1.5 text-right">{formatNumber(unit.fuzzy_rows)}</td>
                      <td class="px-3 py-1.5 text-right text-ink-muted">{unit.fuzzy_percent}%</td>
                    </tr>
                  {:else}
                    <tr><td colspan="6" class="px-3 py-4 text-center text-ink-muted">Tidak ada satuan dengan duplikasi isian.</td></tr>
                  {/each}
                </tbody>
              </table>
            </div>
            {#if duplicateUnits.length > 50}
              <p class="mt-1 text-[9px] text-ink-muted">Menampilkan 50 dari {duplicateUnits.length} satuan. Daftar lengkap ada di PDF.</p>
            {/if}
          </div>

          <div class="grid gap-4 border-t border-line p-4 xl:grid-cols-2">
            <div>
              <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Grup duplikat persis</p>
              {@render duplicateGroupTable(indicators.duplicate.exact_groups, 'Tidak ada duplikat persis.')}
            </div>
            <div>
              <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Grup mirip ≥75%</p>
              {@render duplicateGroupTable(indicators.duplicate.fuzzy_groups, 'Tidak ada isian mirip ≥75%.')}
            </div>
          </div>
        {:else}
          <p class="p-4 text-[11px] text-ink-muted">Data indikator duplikasi belum dimuat.</p>
        {/if}
      </section>
    {:else if activeTab === 'drive'}
      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow flex items-center gap-1.5 text-red-700">
              <Link2 size={13} /> TAUTAN BERULANG
            </span>
            <h2 class="panel-title mt-0.5">Foto kegiatan dan tautan Google Drive berulang</h2>
            <p class="mt-1 max-w-3xl text-[10px] leading-relaxed text-ink-muted">
              Tautan Drive pada isian laporan (termasuk baris Foto kegiatan) dan kolom Bukti dukung.
              Baris dianggap berulang jika ID Drive-nya dipakai minimal dua kali pada periode ini.
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-1.5">
            <span class="badge badge-danger">{driveUnits.length} satuan</span>
            <Button
              variant="secondary"
              size="sm"
              loading={drivePdfBusy}
              onclick={() => void downloadDrivePdf()}
            >
              <FileDown size={13} /> PDF tautan Drive
            </Button>
          </div>
        </div>

        {#if indicators}
          {#if !indicators.duplicate_ready}
            <div class="m-4 rounded-xl border border-gold-300 bg-gold-50/60 p-3 text-[11px] text-gold-800">
              Analisis duplikat belum lengkap; tautan Drive tetap dihitung dari baris yang tersedia.
              Impor yang belum disiapkan: {indicators.pending_imports.join(', ')}.
            </div>
          {/if}

          <div class="grid gap-3 border-t border-line p-4 sm:grid-cols-2 xl:grid-cols-6">
            <article class="rounded-xl border border-line bg-stone-50/60 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Baris Drive</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(indicators.drive.total_drive_rows)}</p>
            </article>
            <article class="rounded-xl border border-red-200 bg-red-50/50 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-red-700 uppercase">Drive berulang</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(indicators.drive.repeated_drive_rows)}</p>
            </article>
            <article class="rounded-xl border border-line bg-stone-50/60 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Foto Drive</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(indicators.drive.foto_drive_rows)}</p>
            </article>
            <article class="rounded-xl border border-red-200 bg-red-50/50 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-red-700 uppercase">Foto berulang</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(indicators.drive.repeated_foto_rows)}</p>
            </article>
            <article class="rounded-xl border border-line bg-stone-50/60 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">ID berulang</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(indicators.drive.repeated_ids)}</p>
            </article>
            <article class="rounded-xl border border-line bg-stone-50/60 p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Pemakaian terbanyak</p>
              <p class="mt-1 font-display text-xl text-forest-900">{formatNumber(indicators.drive.max_reuse)}×</p>
            </article>
          </div>

          <div class="border-t border-line p-4">
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Ringkasan per satuan</p>
            <div class="mt-2 max-h-80 overflow-auto rounded-xl border border-line">
              <table class="min-w-full text-left text-[11px]">
                <thead class="sticky top-0 bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
                  <tr>
                    <th class="px-3 py-2">Satuan</th>
                    <th class="px-3 py-2 text-right">Baris</th>
                    <th class="px-3 py-2 text-right">Drive</th>
                    <th class="px-3 py-2 text-right">Berulang</th>
                    <th class="px-3 py-2 text-right">Foto</th>
                    <th class="px-3 py-2 text-right">Foto berulang</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-line">
                  {#each driveUnits.slice(0, 50) as unit (unit.satuan)}
                    <tr class="hover:bg-stone-50/60">
                      <td class="px-3 py-1.5 font-medium text-ink">{unit.satuan}</td>
                      <td class="px-3 py-1.5 text-right">{formatNumber(unit.rows)}</td>
                      <td class="px-3 py-1.5 text-right">{formatNumber(unit.drive_rows)}</td>
                      <td class="px-3 py-1.5 text-right">{formatNumber(unit.repeated_rows)}</td>
                      <td class="px-3 py-1.5 text-right">{formatNumber(unit.foto_rows)}</td>
                      <td class="px-3 py-1.5 text-right">{formatNumber(unit.repeated_foto_rows)}</td>
                    </tr>
                  {:else}
                    <tr><td colspan="6" class="px-3 py-4 text-center text-ink-muted">Tidak ada tautan Drive berulang.</td></tr>
                  {/each}
                </tbody>
              </table>
            </div>
            {#if driveUnits.length > 50}
              <p class="mt-1 text-[9px] text-ink-muted">Menampilkan 50 dari {driveUnits.length} satuan. Daftar lengkap ada di PDF.</p>
            {/if}
          </div>

          <div class="border-t border-line p-4">
            <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Tautan Drive yang berulang</p>
            {@render driveGroupTable(indicators.drive.groups)}
          </div>
        {:else}
          <p class="p-4 text-[11px] text-ink-muted">Data indikator Drive belum dimuat.</p>
        {/if}
      </section>
    {:else if activeTab === 'material'}
      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow flex items-center gap-1.5 text-red-700">
              <ShieldAlert size={13} /> MATERI BERULANG
            </span>
            <h2 class="panel-title mt-0.5">Materi identik lintas satuan</h2>
            <p class="mt-1 max-w-3xl text-[10px] leading-relaxed text-ink-muted">
              Materi isian yang dipakai oleh beberapa satuan. Klik kartu untuk melihat semua satuan
              dan rincian baris yang memakai materi tersebut.
            </p>
          </div>
          <span class="badge badge-danger">{botDetection.repeated_material.length} materi</span>
        </div>

        <div class="space-y-2 border-t border-line p-4">
          {#each botDetection.repeated_material as item}
            <button
              type="button"
              class="w-full rounded-xl border border-red-200 bg-red-50/40 px-3 py-2 text-left transition-colors hover:border-red-400 hover:bg-red-50"
              title="Lihat rincian materi {item.activity}"
              onclick={() => (materialSuspect = item)}
            >
              <div class="flex items-start justify-between gap-2">
                <strong class="text-xs text-ink">{item.activity}</strong>
                <span class="badge badge-danger shrink-0">{item.report_count} baris</span>
              </div>
              <p class="mt-1 text-[10px] text-ink-muted">
                {item.unit_count} satuan: {item.units.slice(0, 5).join(', ')}{#if item.unit_count > 5} · +{item.unit_count - 5} satuan lain{/if}
              </p>
              <p class="mt-1 line-clamp-2 text-[10px] text-ink-soft">“{item.sample}”</p>
              <p class="mt-1 text-[9px] text-ink-muted">
                Baris {item.source_rows.join(', ')}{#if item.report_count > item.source_rows.length} · +{item.report_count - item.source_rows.length} baris lain{/if}
              </p>
              <span class="mt-1 inline-block text-[10px] font-semibold text-red-700">Lihat rincian →</span>
            </button>
          {:else}
            <p class="rounded-xl border border-dashed border-line px-3 py-6 text-center text-[11px] text-ink-muted">
              Tidak ada materi identik lintas satuan pada periode ini.
            </p>
          {/each}
        </div>
      </section>
    {/if}
  </div>
{/if}

<BotSuspectDialog
  suspect={botSuspect}
  related={relatedRapid}
  onpick={(item) => (botSuspect = item)}
  onclose={() => (botSuspect = null)}
/>
<BotMaterialDialog suspect={materialSuspect} onclose={() => (materialSuspect = null)} />
