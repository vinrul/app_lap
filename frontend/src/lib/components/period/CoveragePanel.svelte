<script lang="ts">
  import { onMount } from 'svelte'
  import {
    AlertCircle,
    Check,
    ChevronRight,
    Layers,
    MapPin,
    Rows3,
    ShieldCheck,
    Users,
  } from 'lucide-svelte'
  import { importsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type {
    CoverageProgram,
    CoverageResponse,
    CoverageUnit,
    QuickWinActivity,
    QuickWinDetail,
    RecordItem,
    UnitBreakdown,
  } from '../../types'
  import { formatDate, formatNumber } from '../../utils/format'
  import { qwGroupTitles } from '../../utils/constants'
  import { evidencePreview } from '../../stores/evidencePreview.svelte'
  import Button from '../ui/Button.svelte'
  import EvidenceList from '../ui/EvidenceList.svelte'
  import LocationMap from '../ui/LocationMap.svelte'
  import Modal from '../ui/Modal.svelte'

  type Props = { periodId: string }
  let { periodId }: Props = $props()

  type ActivityScope = 'polres' | 'satker' | 'masyarakat'

  let data = $state<CoverageResponse | null>(null)
  let loading = $state(true)
  let error = $state('')
  let expandedGroup = $state<string | null>(null)
  let activeGroup = $state('QW1')
  let activeCode = $state('')
  let polresFilter = $state('')
  let polresLoading = $state(false)
  let polresError = $state('')
  let polresRequest = 0
  let groupDetail = $state<QuickWinDetail | null>(null)
  let groupDetailCode = $state('')
  let groupDetailLoading = $state(false)
  let groupDetailError = $state('')
  let modalOpen = $state(false)
  let modalCode = $state('')
  let modalDetail = $state<QuickWinDetail | null>(null)
  let modalLoading = $state(false)
  let modalError = $state('')
  let activityOpen = $state(false)
  let activityTarget = $state<QuickWinActivity | null>(null)
  let activityRecord = $state<RecordItem | null>(null)
  let activityLoading = $state(false)
  let activityError = $state('')
  let activityFilter = $state<{ scope: ActivityScope; label: string } | null>(null)
  let filteredActivities = $state<QuickWinActivity[] | null>(null)
  let filteredTotal = $state(0)
  let filterLoading = $state(false)
  let filterError = $state('')

  onMount(load)

  const codeGroups = ['QW1', 'QW2', 'QW3', 'QW4', 'QW5', 'QW6', 'QW7', 'QW8']

  function pickGroup(value: string) {
    activeGroup = value
    const first = activeGroupCodes.find((item) => item.code.startsWith(`${value}.`))
    if (first) activeCode = first.code
  }

  async function load() {
    loading = true
    error = ''
    try {
      data = await importsApi.coverage(periodId, polresFilter || undefined)
      if (!data.codes.some((item) => item.code === activeCode)) {
        activeCode = data.codes[0]?.code ?? ''
        activeGroup = data.codes[0]?.code.split('.')[0] ?? 'QW1'
      }
    } catch (caught) {
      data = null
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  const groupsPerformed = $derived(
    (data?.groups ?? []).filter((group) => group.rows > 0).length,
  )
  const totalCodes = $derived(
    (data?.groups ?? []).reduce((total, group) => total + group.codes.length, 0),
  )
  const codesPerformed = $derived(
    (data?.groups ?? []).reduce(
      (total, group) => total + group.codes.filter((code) => code.rows > 0).length,
      0,
    ),
  )
  const activeGroupCodes = $derived(
    data?.codes ?? [],
  )
  const polresOptions = $derived(data?.polres_options ?? [])
  const activePolres = $derived(data?.filter_polres ?? '')
  const primaryLabel = $derived(activePolres ? 'Polsek' : 'Polres')
  const secondaryLabel = $derived(activePolres ? 'Satker Polres' : 'Satker Polda')
  const primaryLower = $derived(activePolres ? 'polsek' : 'polres')
  const secondaryLower = $derived(activePolres ? 'satker polres' : 'satker')
  const groupCodeCards = $derived(
    activeGroupCodes.filter((item) => item.code.startsWith(`${activeGroup}.`)),
  )
  const activeCodeCard = $derived(
    activeGroupCodes.find((item) => item.code === activeCode) ?? groupCodeCards[0] ?? null,
  )
  const modalTitle = $derived(
    modalDetail ? `${modalDetail.code} · ${modalDetail.title}` : modalCode || 'Detail sub-kode',
  )
  const modalDescription = $derived(
    modalDetail
      ? `${modalDetail.group} ${modalDetail.group_title} · ${formatNumber(modalDetail.total_rows)} kegiatan`
      : '',
  )
  const modalMissingPolres = $derived(modalDetail?.polres_missing ?? [])
  const modalMissingSatker = $derived(modalDetail?.satker_missing ?? [])
  const activityTitle = $derived(
    activityRecord
      ? `${activityRecord.kode} · ${activityRecord.qw_title}`
      : activityTarget
        ? `${activityTarget.kode} · ${activityTarget.title}`
        : 'Detail kegiatan',
  )
  const activityDescription = $derived(
    activityRecord
      ? `${formatDate(activityRecord.tanggal)} · ${activityRecord.polres || activityRecord.satuan || '—'}${activityRecord.polsek ? ` · ${activityRecord.polsek}` : ''}`
      : activityTarget
        ? `${formatDate(activityTarget.tanggal)} · ${activityTarget.polres || activityTarget.satker || '—'}`
        : '',
  )
  const activityImages = $derived(
    (activityRecord?.evidence ?? []).filter(
      (item) => item.status === 'ready' && item.content_type.startsWith('image/'),
    ),
  )
  const activityReadyEvidence = $derived(
    (activityRecord?.evidence ?? []).filter((item) => item.status === 'ready'),
  )
  const visibleActivities: QuickWinActivity[] = $derived(
    activityFilter ? (filteredActivities ?? []) : (modalDetail?.activities ?? []),
  )
  const activitySummary = $derived.by(() => {
    if (!modalDetail) return ''
    if (filterLoading) return 'Memuat…'
    if (activityFilter) {
      return `${formatNumber(filteredActivities?.length ?? 0)} dari ${formatNumber(filteredTotal)} kegiatan`
    }
    return modalDetail.activities_total > modalDetail.activities.length
      ? `${formatNumber(modalDetail.activities.length)} dari ${formatNumber(modalDetail.activities_total)} kegiatan`
      : `${formatNumber(modalDetail.activities.length)} kegiatan`
  })
  const activityEmptyText = $derived(
    activityFilter
      ? `Belum ada kegiatan dari ${activityFilter.label} pada kode ini.`
      : 'Belum ada kegiatan pada kode ini.',
  )

  function percent(value: number, total: number): number {
    if (total <= 0) return 0
    return Math.max(0, Math.min(100, Math.round((value / total) * 100)))
  }

  function formatValue(value: number, unit: string): string {
    return unit ? `${formatNumber(value)} ${unit}` : formatNumber(value)
  }

  async function changePolres(value: string) {
    polresFilter = value
    polresError = ''
    expandedGroup = null
    groupDetail = null
    groupDetailCode = ''
    const current = ++polresRequest
    polresLoading = true
    try {
      const result = await importsApi.coverage(periodId, value || undefined)
      if (current !== polresRequest) return
      data = result
      if (!result.codes.some((item) => item.code === activeCode)) {
        activeCode = result.codes[0]?.code ?? ''
        activeGroup = result.codes[0]?.code.split('.')[0] ?? 'QW1'
      }
    } catch (caught) {
      if (current !== polresRequest) return
      polresError = errorMessage(caught)
      polresFilter = data?.filter_polres ?? ''
    } finally {
      if (current === polresRequest) polresLoading = false
    }
  }

  async function toggleGroup(code: string) {
    if (expandedGroup === code) {
      expandedGroup = null
      return
    }
    expandedGroup = code
    if (groupDetailCode === code && groupDetail) return
    await loadGroupDetail(code)
  }

  async function loadGroupDetail(code: string) {
    const target = code
    groupDetailCode = target
    groupDetail = null
    groupDetailError = ''
    groupDetailLoading = true
    try {
      const result = await importsApi.quickWinDetail(
        periodId,
        target,
        undefined,
        activePolres || undefined,
      )
      if (groupDetailCode !== target) return
      groupDetail = result
    } catch (caught) {
      if (groupDetailCode !== target) return
      groupDetailError = errorMessage(caught)
    } finally {
      if (groupDetailCode === target) groupDetailLoading = false
    }
  }

  async function openCodeDetail(code: string) {
    clearActivityFilter()
    modalOpen = true
    modalCode = code
    modalDetail = null
    modalError = ''
    modalLoading = true
    try {
      const result = await importsApi.quickWinDetail(
        periodId,
        code,
        undefined,
        activePolres || undefined,
      )
      if (modalCode !== code) return
      modalDetail = result
    } catch (caught) {
      if (modalCode !== code) return
      modalError = errorMessage(caught)
    } finally {
      if (modalCode === code) modalLoading = false
    }
  }

  function closeCodeDetail() {
    modalOpen = false
    modalCode = ''
    modalDetail = null
    modalError = ''
    clearActivityFilter()
  }

  async function loadActivityFilter(code: string, scope: ActivityScope, label: string) {
    filteredActivities = null
    filteredTotal = 0
    filterError = ''
    filterLoading = true
    try {
      const result = await importsApi.quickWinDetail(periodId, code, { scope, label }, activePolres || undefined)
      if (modalCode !== code) return
      filteredActivities = result.activities
      filteredTotal = result.activities_total
    } catch (caught) {
      if (modalCode !== code) return
      filterError = errorMessage(caught)
    } finally {
      if (modalCode === code) filterLoading = false
    }
  }

  function filterByUnit(scope: 'polres' | 'satker', label: string) {
    if (activityFilter?.scope === scope && activityFilter?.label === label) {
      clearActivityFilter()
      return
    }
    activityFilter = { scope, label }
    void loadActivityFilter(modalCode, scope, label)
  }

  function filterByCommunity(label: string) {
    if (activityFilter?.scope === 'masyarakat' && activityFilter?.label === label) {
      clearActivityFilter()
      return
    }
    activityFilter = { scope: 'masyarakat', label }
    void loadActivityFilter(modalCode, 'masyarakat', label)
  }

  function clearActivityFilter() {
    activityFilter = null
    filteredActivities = null
    filteredTotal = 0
    filterError = ''
    filterLoading = false
  }

  function retryActivityFilter() {
    if (!activityFilter) return
    void loadActivityFilter(modalCode, activityFilter.scope, activityFilter.label)
  }

  async function openActivityDetail(activity: QuickWinActivity) {
    activityOpen = true
    activityTarget = activity
    activityRecord = null
    activityError = ''
    activityLoading = true
    try {
      const record = await importsApi.recordDetail(periodId, activity.record_id)
      if (activityTarget?.record_id !== activity.record_id) return
      activityRecord = record
    } catch (caught) {
      if (activityTarget?.record_id !== activity.record_id) return
      activityError = errorMessage(caught)
    } finally {
      if (activityTarget?.record_id === activity.record_id) activityLoading = false
    }
  }

  function closeActivityDetail() {
    activityOpen = false
    activityTarget = null
    activityRecord = null
    activityError = ''
  }

  function retryActivityDetail() {
    if (activityTarget) void openActivityDetail(activityTarget)
  }
</script>

{#snippet coverageStat(label: string, value: number, total: number)}
  <div class="min-w-[96px]">
    <div class="flex items-baseline justify-between gap-2">
      <span class="text-[10px] font-semibold tracking-wide text-ink-muted uppercase">{label}</span>
      <span class="text-[11px] font-semibold text-ink">{value}<span class="text-ink-muted">/{total}</span></span>
    </div>
    <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-stone-200">
      <div class="h-full rounded-full bg-forest-600" style={`width:${percent(value, total)}%`}></div>
    </div>
  </div>
{/snippet}

{#snippet chips(items: string[])}
  {#if items.length}
    <div class="flex flex-wrap gap-1.5">
      {#each items as item (item)}
        <span class="rounded-md border border-line bg-white px-2 py-1 text-[10px] text-ink-soft">{item}</span>
      {/each}
    </div>
  {:else}
    <p class="text-[11px] text-forest-600">Semua unit sudah tercatat.</p>
  {/if}
{/snippet}

{#snippet unitTable(items: UnitBreakdown[], scope: 'polres' | 'satker', onpick: ((label: string) => void) | null)}
  {#if items.length}
    <div class="max-h-80 overflow-auto rounded-xl border border-line">
      <table class="min-w-full text-left text-[11px]">
        <thead class="sticky top-0 bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
          <tr>
            <th class="whitespace-nowrap px-3 py-2">Unit</th>
            <th class="whitespace-nowrap px-3 py-2 text-right">Kegiatan</th>
            {#if scope === 'polres' && !activePolres}<th class="whitespace-nowrap px-3 py-2 text-right">Polsek</th>{/if}
            <th class="whitespace-nowrap px-3 py-2 text-right">Personel</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-line">
          {#each items as item (item.label)}
            {@const selected =
              activityFilter?.scope === scope && activityFilter?.label === item.label}
            <tr class="hover:bg-stone-50/60 {selected ? 'bg-forest-50/70' : ''}">
              <td class="whitespace-nowrap px-3 py-2 font-medium text-ink">
                {#if onpick}
                  <button
                    type="button"
                    class="text-left {selected
                      ? 'font-bold text-forest-900'
                      : 'text-forest-700 hover:text-forest-900'} underline decoration-dotted underline-offset-2 hover:decoration-solid"
                    aria-pressed={selected}
                    title="Tampilkan kegiatan {item.label} saja"
                    onclick={() => onpick(item.label)}
                  >
                    {item.label}
                  </button>
                {:else}
                  {item.label}
                {/if}
              </td>
              <td class="px-3 py-2 text-right">{formatNumber(item.rows)}</td>
              {#if scope === 'polres' && !activePolres}<td class="px-3 py-2 text-right text-ink-muted">{formatNumber(item.polsek)}</td>{/if}
              <td class="px-3 py-2 text-right">{formatNumber(item.personel)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="rounded-xl border border-dashed border-line px-3 py-4 text-center text-[11px] text-ink-muted">Belum ada unit pelaksana.</p>
  {/if}
{/snippet}

{#snippet activityList(items: QuickWinActivity[], emptyText: string)}
  {#if items.length}
    <div class="max-h-96 space-y-2 overflow-y-auto pr-1">
      {#each items as activity, index (index)}
        <button
          type="button"
          class="w-full rounded-xl border border-line bg-white p-3 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
          onclick={() => openActivityDetail(activity)}
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span class="rounded-md bg-forest-50 px-1.5 py-0.5 text-[10px] font-bold text-forest-700">{activity.kode}</span>
              <span class="text-[10px] text-ink-muted">{formatDate(activity.tanggal)}</span>
              <span class="text-[10px] text-ink-muted">{activity.polres || activity.satker}{#if activity.polsek} · {activity.polsek}{/if}</span>
            </div>
          </div>
          <p class="mt-1.5 text-[11px] leading-relaxed text-ink">{activity.kegiatan || activity.title || 'Kegiatan'}</p>
          <div class="mt-1.5 flex flex-wrap items-center justify-between gap-2">
            <div class="flex flex-wrap gap-2 text-[10px] text-ink-muted">
              {#if activity.personel}<span>{formatNumber(activity.personel)} personel</span>{/if}
              {#if activity.pelaksana}<span>{activity.pelaksana}</span>{/if}
            </div>
            <span class="shrink-0 text-[10px] font-semibold text-forest-700">Lihat detail →</span>
          </div>
        </button>
      {/each}
    </div>
  {:else}
    <p class="text-[11px] text-ink-muted">{emptyText}</p>
  {/if}
{/snippet}

{#snippet info(label: string, value: string)}
  <div class="rounded-xl border border-line bg-stone-50/60 px-3 py-2.5">
    <p class="text-[9px] font-bold tracking-[0.1em] text-ink-muted uppercase">{label}</p>
    <p class="mt-0.5 text-[11px] font-semibold text-ink">{value || '—'}</p>
  </div>
{/snippet}

{#if loading}
  <div class="space-y-4">
    <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
      {#each Array(5) as _}<div class="skeleton h-28"></div>{/each}
    </div>
    <div class="skeleton h-80 w-full"></div>
    <div class="skeleton h-96 w-full"></div>
  </div>
{:else if error}
  <div class="flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{:else if data}
  <div class="space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-line bg-white px-4 py-3">
      <div>
        <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Filter cakupan</p>
        <p class="mt-0.5 text-[11px] text-ink-muted">
          {polresFilter
            ? `Menampilkan ${polresFilter} saja: rincian per ${primaryLower} dan ${secondaryLower}.`
            : 'Menampilkan seluruh Polda. Pilih polres untuk melihat rincian polsek dan satker polres.'}
        </p>
      </div>
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
    </div>

    {#if polresError}
      <div class="flex flex-wrap items-center gap-2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
        <AlertCircle size={17} /> {polresError}
      </div>
    {/if}

    <section class="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
      {#each [
        { label: 'Kelompok QW', value: `${groupsPerformed}/${data.groups.length}`, hint: 'kelompok dengan kegiatan', icon: Layers },
        { label: 'Sub-kode', value: `${codesPerformed}/${totalCodes}`, hint: 'sub-kode dengan kegiatan', icon: Check },
        { label: `${primaryLabel} aktif`, value: `${data.active_polres}/${data.known_polres.length}`, hint: 'melapor pada periode', icon: Users },
        { label: `${secondaryLabel} aktif`, value: `${data.active_satker}/${data.known_satker.length}`, hint: 'melapor pada periode', icon: ShieldCheck },
        { label: 'Baris dianalisis', value: formatNumber(data.total_rows), hint: `${formatDate(data.date_start)} – ${formatDate(data.date_end)}`, icon: Rows3 },
      ] as card}
        <article class="card p-4">
          <card.icon size={17} class="text-forest-600" />
          <p class="mt-3 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{card.label}</p>
          <strong class="font-display text-2xl text-forest-900">{card.value}</strong>
          <p class="text-[10px] text-ink-muted">{card.hint}</p>
        </article>
      {/each}
    </section>

    <section class="card overflow-hidden">
      <div class="panel-heading">
        <div>
          <span class="eyebrow">CAKUPAN QUICK WINS</span>
          <h2 class="panel-title mt-0.5">Pelaksanaan per kelompok & sub-kode</h2>
          <p class="mt-1 text-[10px] text-ink-muted">
            Dihitung dari {secondaryLower}/{primaryLower} yang memiliki baris pada periode ini.
          </p>
        </div>
        <span class="badge badge-neutral">{data.groups.length} kelompok</span>
      </div>
      <div class="divide-y divide-line">
        {#each data.groups as group (group.code)}
          {@const open = expandedGroup === group.code}
          <article>
            <button
              type="button"
              class="flex w-full flex-wrap items-center justify-between gap-4 px-5 py-4 text-left transition-colors hover:bg-stone-50/70"
              onclick={() => toggleGroup(group.code)}
            >
              <div class="flex min-w-0 items-start gap-3">
                <span class="rounded-lg bg-forest-50 px-2 py-1 text-[10px] font-bold text-forest-700">{group.code}</span>
                <div class="min-w-0">
                  <p class="truncate text-sm font-semibold text-ink">{group.title}</p>
                  <p class="mt-0.5 text-[11px] text-ink-muted">{formatNumber(group.rows)} kegiatan · {group.codes.length} sub-kode</p>
                </div>
              </div>
              <div class="flex flex-wrap items-center gap-5">
                {@render coverageStat(primaryLabel, group.polres, group.polres_total)}
                {@render coverageStat(secondaryLabel, group.satker, group.satker_total)}
                <ChevronRight size={16} class="shrink-0 text-ink-muted transition-transform {open ? 'rotate-90' : ''}" />
              </div>
            </button>
            {#if open}
              <div class="space-y-4 border-t border-line bg-stone-50/40 px-5 py-4">
                {#if groupDetailLoading && groupDetailCode === group.code}
                  <div class="grid gap-3 xl:grid-cols-2">
                    <div class="skeleton h-44"></div>
                    <div class="skeleton h-44"></div>
                  </div>
                {:else if groupDetailError && groupDetailCode === group.code}
                  <div class="flex flex-wrap items-center gap-2 rounded-xl border border-red-200 bg-red-50 px-3 py-2 text-[11px] text-red-800">
                    <AlertCircle size={14} /> {groupDetailError}
                    <Button variant="secondary" size="sm" onclick={() => loadGroupDetail(group.code)}>Coba lagi</Button>
                  </div>
                {:else if groupDetail && groupDetail.code === group.code}
                  <div class="grid gap-4 xl:grid-cols-2">
                    <div>
                      <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan per {primaryLower}</p>
                      {@render unitTable(groupDetail.by_polres, 'polres', null)}
                    </div>
                    <div>
                      <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan per {secondaryLower}</p>
                      {@render unitTable(groupDetail.by_satker, 'satker', null)}
                    </div>
                  </div>
                {/if}

                <div>
                  <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
                    <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Sub-kode {group.code}</p>
                    <span class="text-[10px] text-ink-muted">Klik sub-kode untuk rincian per {primaryLower}.</span>
                  </div>
                  <div class="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
                    {#each group.codes as code (code.code)}
                      <button
                        type="button"
                        class="w-full rounded-xl border border-line bg-white p-3 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
                        onclick={() => openCodeDetail(code.code)}
                      >
                        <div class="flex items-start justify-between gap-2">
                          <span class="rounded-md bg-forest-50 px-1.5 py-0.5 text-[10px] font-bold text-forest-700">{code.code}</span>
                          <span class="text-[10px] text-ink-muted">{formatNumber(code.rows)} kegiatan</span>
                        </div>
                        <p class="mt-1.5 line-clamp-2 text-[11px] font-medium leading-snug text-ink">{code.title}</p>
                        <p class="mt-1.5 text-[10px] text-ink-muted">
                          {code.polres} {primaryLower} · {code.satker} {secondaryLower}
                          {#if code.polres_missing || code.satker_missing}
                            <span class="text-red-700"> · belum {code.polres_missing} {primaryLower} / {code.satker_missing} {secondaryLower}</span>
                          {/if}
                        </p>
                        <p class="mt-1.5 text-[10px] font-semibold text-forest-700">Lihat rincian →</p>
                      </button>
                    {/each}
                  </div>
                </div>

                <div class="grid gap-4 xl:grid-cols-2">
                  <div>
                    <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Polres belum melaksanakan</p>
                    <div class="mt-2">{@render chips(group.polres_missing)}</div>
                  </div>
                  <div>
                    <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Satker Polda belum melaksanakan</p>
                    <div class="mt-2">{@render chips(group.satker_missing)}</div>
                  </div>
                </div>
              </div>
            {/if}
          </article>
        {/each}
      </div>
    </section>

    <section class="card overflow-hidden">
      <div class="panel-heading">
        <div>
          <span class="eyebrow">CAKUPAN PER SUB-KODE</span>
          <h2 class="panel-title mt-0.5">Kartu cakupan Quick Wins</h2>
          <p class="mt-1 text-[10px] text-ink-muted">
            Dihitung otomatis dari isian laporan: angka dijumlahkan, pilihan jawaban diringkas per kategori.
          </p>
        </div>
        <span class="badge badge-neutral">{(data.codes ?? []).length} sub-kode</span>
      </div>

      <div class="flex flex-wrap items-end gap-2 border-b border-line px-5 py-3">
        <label class="flex flex-col gap-1">
          <span class="text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">Kelompok</span>
          <select class="input w-56 py-1.5 text-xs" value={activeGroup} onchange={(event) => pickGroup(event.currentTarget.value)}>
            {#each codeGroups as group (group)}
              <option value={group}>{group} · {qwGroupTitles[group] ?? group}</option>
            {/each}
          </select>
        </label>
        <label class="flex flex-col gap-1">
          <span class="text-[10px] font-bold tracking-[0.1em] text-ink-muted uppercase">Sub-kode</span>
          <select class="input w-72 py-1.5 text-xs" value={activeCode} onchange={(event) => (activeCode = event.currentTarget.value)}>
            {#each groupCodeCards as item (item.code)}
              <option value={item.code}>{item.code} · {item.title}</option>
            {/each}
          </select>
        </label>
      </div>

      {#if activeCodeCard}
        <div class="space-y-5 p-5">
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p class="text-sm font-semibold text-ink">
                {activeCodeCard.code}
                <span class="ml-2 text-[11px] font-normal text-ink-muted">{activeCodeCard.title}</span>
              </p>
              <p class="mt-1 text-[11px] text-ink-muted">
                {formatNumber(activeCodeCard.rows)} laporan · {activeCodeCard.polres}/{activeCodeCard.polres_total} {primaryLower} · {activeCodeCard.satker}/{activeCodeCard.satker_total} {secondaryLower}
              </p>
            </div>
            <div class="flex flex-wrap gap-4">
              {@render coverageStat(primaryLabel, activeCodeCard.polres, activeCodeCard.polres_total)}
              {@render coverageStat(secondaryLabel, activeCodeCard.satker, activeCodeCard.satker_total)}
            </div>
          </div>

          {#if activeCodeCard.metrics.length}
            <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
              {#each activeCodeCard.metrics as metric (metric.key)}
                <article class="rounded-xl border border-line bg-stone-50/60 p-3.5">
                  <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{metric.label}</p>
                  <p class="mt-1 font-display text-xl text-forest-900">{formatValue(metric.total, metric.unit)}</p>
                  <p class="mt-1 text-[10px] text-ink-muted">
                    {formatNumber(metric.filled_rows)} dari {formatNumber(activeCodeCard.rows)} laporan mengisi
                  </p>
                </article>
              {/each}
            </div>
          {:else if activeCodeCard.rows === 0}
            <p class="rounded-xl border border-dashed border-line px-3 py-6 text-center text-[11px] text-ink-muted">
              Belum ada laporan untuk {activeCodeCard.code} pada periode ini.
            </p>
          {/if}

          {#if activeCodeCard.breakdowns.length}
            <div class="grid gap-3 sm:grid-cols-2">
              {#each activeCodeCard.breakdowns as item (item.key)}
                {@const max = Math.max(1, ...item.values.map((value) => value.count))}
                <article class="rounded-xl border border-line p-3.5">
                  <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{item.label}</p>
                  <ul class="mt-2 space-y-1.5">
                    {#each item.values as value (value.label)}
                      <li>
                        <div class="flex items-center justify-between gap-2 text-[11px]">
                          <span class="truncate text-ink" title={value.label}>{value.label}</span>
                          <span class="shrink-0 font-semibold text-forest-800">{formatNumber(value.count)}</span>
                        </div>
                        <div class="mt-0.5 h-1.5 overflow-hidden rounded-full bg-stone-200">
                          <div class="h-full rounded-full bg-forest-600" style={`width:${Math.round((value.count / max) * 100)}%`}></div>
                        </div>
                      </li>
                    {/each}
                  </ul>
                  <p class="mt-2 text-[10px] text-ink-muted">
                    {formatNumber(item.filled_rows)} dari {formatNumber(activeCodeCard.rows)} laporan mengisi
                  </p>
                </article>
              {/each}
            </div>
          {/if}

          <div class="grid gap-4 xl:grid-cols-2">
            <div class="rounded-xl border border-line p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{primaryLabel} belum melaksanakan</p>
              <div class="mt-2">{@render chips(activeCodeCard.polres_missing)}</div>
            </div>
            <div class="rounded-xl border border-line p-3.5">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{secondaryLabel} belum melaksanakan</p>
              <div class="mt-2">{@render chips(activeCodeCard.satker_missing)}</div>
            </div>
          </div>

          <div class="grid gap-4 xl:grid-cols-2">
            {#each [{ label: primaryLabel, units: activeCodeCard.by_polres }, { label: secondaryLabel, units: activeCodeCard.by_satker }] as scope (scope.label)}
              <div>
                <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
                  <p class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">{scope.label} pelaksana</p>
                  <span class="text-[10px] text-ink-muted">{scope.units.length} unit</span>
                </div>
                {#if scope.units.length}
                  <div class="max-h-72 overflow-auto rounded-xl border border-line">
                    <table class="min-w-full text-left text-[11px]">
                      <thead class="sticky top-0 bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
                        <tr>
                          <th class="whitespace-nowrap px-3 py-2">Unit</th>
                          <th class="whitespace-nowrap px-3 py-2 text-right">Laporan</th>
                        </tr>
                      </thead>
                      <tbody class="divide-y divide-line">
                        {#each scope.units as unit (unit.label)}
                          <tr class="hover:bg-stone-50/60">
                            <td class="whitespace-nowrap px-3 py-2 font-medium text-ink">{unit.label}</td>
                            <td class="px-3 py-2 text-right">{formatNumber(unit.rows)}</td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>
                {:else}
                  <p class="rounded-xl border border-dashed border-line px-3 py-4 text-center text-[11px] text-ink-muted">
                    Belum ada unit yang melaporkan kode ini.
                  </p>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </section>
  </div>
{/if}

<Modal
  open={modalOpen}
  title={modalTitle}
  description={modalDescription}
  onclose={closeCodeDetail}
  width="max-w-6xl"
>
  {#if modalLoading}
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {#each Array(4) as _}<div class="skeleton h-20"></div>{/each}
      </div>
      <div class="grid gap-4 xl:grid-cols-2">
        <div class="skeleton h-52"></div>
        <div class="skeleton h-52"></div>
      </div>
      <div class="skeleton h-64"></div>
    </div>
  {:else if modalError}
    <div class="flex flex-wrap items-center gap-2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertCircle size={17} /> {modalError}
      <Button variant="secondary" size="sm" onclick={() => openCodeDetail(modalCode)}>Coba lagi</Button>
    </div>
  {:else if modalDetail}
    <div class="space-y-5">
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {#each [
          { label: 'Kegiatan', value: formatNumber(modalDetail.total_rows) },
          { label: primaryLabel, value: formatNumber(modalDetail.polres_count) },
          { label: secondaryLabel, value: formatNumber(modalDetail.satker_count) },
          { label: 'Personel', value: formatNumber(modalDetail.personel) },
        ] as card}
          <div class="rounded-xl border border-line bg-stone-50/60 px-3 py-2.5">
            <p class="text-[9px] font-bold tracking-[0.1em] text-ink-muted uppercase">{card.label}</p>
            <strong class="mt-0.5 block text-sm font-semibold text-forest-900">{card.value}</strong>
          </div>
        {/each}
      </div>

      <div class="grid gap-4 xl:grid-cols-2">
        <section>
          <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan per {primaryLower}</p>
            <span class="text-[10px] text-ink-muted">Klik unit untuk filter</span>
          </div>
          {@render unitTable(modalDetail.by_polres, 'polres', (label) => filterByUnit('polres', label))}
        </section>
        <section>
          <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan per {secondaryLower}</p>
            <span class="text-[10px] text-ink-muted">Klik unit untuk filter</span>
          </div>
          {@render unitTable(modalDetail.by_satker, 'satker', (label) => filterByUnit('satker', label))}
        </section>
      </div>

      {#if modalDetail.communities.length}
        <section class="rounded-xl border border-line p-3.5">
          <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Masyarakat yang terlibat</p>
            <span class="text-[10px] text-ink-muted">Klik kategori untuk filter kegiatan</span>
          </div>
          <div class="flex flex-wrap gap-1.5">
            {#each modalDetail.communities as item (item.label)}
              {@const selected = activityFilter?.scope === 'masyarakat' && activityFilter?.label === item.label}
              <button
                type="button"
                class="rounded-md border px-2 py-1 text-[10px] transition-colors {selected
                  ? 'border-forest-700 bg-forest-50 font-semibold text-forest-900'
                  : 'border-line bg-white text-ink hover:border-gold-300 hover:bg-gold-50/40'}"
                aria-pressed={selected}
                title="Tampilkan kegiatan yang melibatkan {item.label} saja"
                onclick={() => filterByCommunity(item.label)}
              >
                {item.label}
                <span class="ml-1 font-semibold text-forest-700">{formatNumber(item.count)}</span>
              </button>
            {/each}
          </div>
        </section>
      {/if}

      <div class="grid gap-4 xl:grid-cols-2">
        <div class="rounded-xl border border-line p-3.5">
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{primaryLabel} belum melaksanakan</p>
          <div class="mt-2">{@render chips(modalMissingPolres)}</div>
        </div>
        <div class="rounded-xl border border-line p-3.5">
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{secondaryLabel} belum melaksanakan</p>
          <div class="mt-2">{@render chips(modalMissingSatker)}</div>
        </div>
      </div>

      <section>
        <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
          <div>
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">
              Kegiatan{activityFilter ? ` · ${activityFilter.label}` : ''}
            </p>
            {#if activityFilter}
              <p class="mt-0.5 flex flex-wrap items-center gap-2 text-[10px] text-ink-muted">
                <span class="rounded-md bg-forest-50 px-1.5 py-0.5 font-semibold text-forest-700">
                  {activityFilter.scope === 'polres'
                    ? primaryLabel
                    : activityFilter.scope === 'satker'
                      ? secondaryLabel
                      : 'Masyarakat'}: {activityFilter.label}
                </span>
                <button
                  type="button"
                  class="font-semibold text-forest-700 underline underline-offset-2 hover:text-forest-900"
                  onclick={clearActivityFilter}
                >
                  Tampilkan semua
                </button>
              </p>
            {:else}
              <p class="mt-0.5 text-[10px] text-ink-muted">
                Klik nama {primaryLower}, {secondaryLower}, atau kategori masyarakat di atas untuk memfilter kegiatan.
              </p>
            {/if}
          </div>
          <span class="text-[10px] text-ink-muted">{activitySummary}</span>
        </div>
        {#if filterLoading}
          <div class="space-y-2">
            {#each Array(3) as _}<div class="skeleton h-16"></div>{/each}
          </div>
        {:else if filterError}
          <div class="flex flex-wrap items-center gap-2 rounded-xl border border-red-200 bg-red-50 px-3 py-2 text-[11px] text-red-800">
            <AlertCircle size={14} /> {filterError}
            <Button variant="secondary" size="sm" onclick={retryActivityFilter}>Coba lagi</Button>
          </div>
        {:else}
          {@render activityList(visibleActivities, activityEmptyText)}
        {/if}
      </section>
    </div>
  {/if}
</Modal>

<Modal
  open={activityOpen}
  title={activityTitle}
  description={activityDescription}
  onclose={closeActivityDetail}
  width="max-w-5xl"
>
  {#if activityLoading}
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {#each Array(4) as _}<div class="skeleton h-20"></div>{/each}
      </div>
      <div class="skeleton h-40"></div>
      <div class="skeleton h-40"></div>
    </div>
  {:else if activityError}
    <div class="flex flex-wrap items-center gap-2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertCircle size={17} /> {activityError}
      {#if activityTarget}
        <Button variant="secondary" size="sm" onclick={retryActivityDetail}>Coba lagi</Button>
      {/if}
    </div>
  {:else if activityRecord}
    <div class="space-y-5">
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {@render info('Tanggal', formatDate(activityRecord.tanggal))}
        {@render info('Unit', [activityRecord.polres, activityRecord.polsek].filter(Boolean).join(' · ') || activityRecord.satuan)}
        {@render info('Status penerimaan', activityRecord.status_penerimaan)}
        {@render info('Personel', formatNumber(activityRecord.personel_count))}
      </div>

      <section class="space-y-3">
        <div>
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan</p>
          <p class="mt-1 text-sm font-semibold text-ink">{activityRecord.kegiatan || activityRecord.qw_title || '—'}</p>
        </div>
        <div>
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Detail laporan</p>
          <p class="mt-1 whitespace-pre-line break-words rounded-xl border border-line bg-stone-50/60 p-3 text-[11px] leading-relaxed text-ink-soft">
            {activityRecord.isian_laporan || 'Tidak ada isian laporan.'}
          </p>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          {@render info('Pelaksana', activityRecord.pelaksana)}
          {@render info('Pejabat pelaksana', activityRecord.pejabat_pelaksana)}
          {@render info('Jabatan', activityRecord.jabatan)}
          {@render info('Pelapor', activityRecord.pelapor)}
        </div>
        {#if activityRecord.bukti_dukung}
          <div>
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Bukti dukung</p>
            <p class="mt-1 break-words whitespace-pre-line text-[11px] text-ink-soft">{activityRecord.bukti_dukung}</p>
          </div>
        {/if}
        {#if activityRecord.links.length}
          <div class="flex flex-wrap gap-2">
            {#each activityRecord.links as link (link)}
              <a class="btn btn-secondary btn-sm" href={link} target="_blank" rel="noreferrer">Buka tautan</a>
            {/each}
          </div>
        {/if}
      </section>

      <section>
        <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
          <div>
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Dokumentasi</p>
            <p class="mt-0.5 text-[10px] text-ink-muted">Klik foto untuk pratinjau. Berkas PDF dapat dipratinjau lewat daftar.</p>
          </div>
          <span class="text-[10px] text-ink-muted">{activityRecord.evidence.length} berkas</span>
        </div>
        {#if activityImages.length}
          <div class="mb-3 grid grid-cols-2 gap-2 sm:grid-cols-3">
            {#each activityImages as item (item.id)}
              <button
                type="button"
                class="group overflow-hidden rounded-xl border border-line bg-stone-100 text-left"
                onclick={() => evidencePreview.open(item, activityReadyEvidence)}
              >
                <img
                  src={item.url}
                  alt={item.filename || 'Dokumentasi kegiatan'}
                  class="h-32 w-full object-cover transition-transform group-hover:scale-105"
                  loading="lazy"
                />
                <span class="block truncate px-2 py-1.5 text-[10px] text-ink-muted">{item.filename || 'Foto kegiatan'}</span>
              </button>
            {/each}
          </div>
        {/if}
        {#if activityRecord.evidence.length}
          <EvidenceList items={activityRecord.evidence} />
        {:else}
          <p class="rounded-xl border border-dashed border-line px-3 py-4 text-center text-[11px] text-ink-muted">Tidak ada dokumentasi terunduh.</p>
        {/if}
      </section>

      <section>
        <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Lokasi</p>
        {#if activityRecord.latitude !== null && activityRecord.longitude !== null}
          <LocationMap
            latitude={activityRecord.latitude}
            longitude={activityRecord.longitude}
            label={activityRecord.kegiatan || 'Lokasi kegiatan'}
            detail={[activityRecord.polres, activityRecord.polsek].filter(Boolean).join(' · ')}
          />
        {:else if activityRecord.map_url}
          <a class="btn btn-secondary btn-sm" href={activityRecord.map_url} target="_blank" rel="noreferrer">
            <MapPin size={14} /> Buka peta
          </a>
        {:else}
          <p class="rounded-xl border border-dashed border-line px-3 py-4 text-center text-[11px] text-ink-muted">Tidak ada lokasi peta pada kegiatan ini.</p>
        {/if}
      </section>
    </div>
  {/if}
</Modal>
