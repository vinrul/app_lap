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
    UnitBreakdown,
  } from '../../types'
  import { formatDate, formatNumber } from '../../utils/format'
  import Button from '../ui/Button.svelte'
  import Modal from '../ui/Modal.svelte'

  type Props = { periodId: string }
  let { periodId }: Props = $props()

  let data = $state<CoverageResponse | null>(null)
  let loading = $state(true)
  let error = $state('')
  let expandedGroup = $state<string | null>(null)
  let activeProgram = $state('')
  let groupDetail = $state<QuickWinDetail | null>(null)
  let groupDetailCode = $state('')
  let groupDetailLoading = $state(false)
  let groupDetailError = $state('')
  let modalOpen = $state(false)
  let modalCode = $state('')
  let modalDetail = $state<QuickWinDetail | null>(null)
  let modalLoading = $state(false)
  let modalError = $state('')

  onMount(load)

  async function load() {
    loading = true
    error = ''
    try {
      data = await importsApi.coverage(periodId)
      if (!data.programs.some((program) => program.key === activeProgram)) {
        activeProgram = data.programs[0]?.key ?? ''
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
  const active = $derived(
    data?.programs.find((program) => program.key === activeProgram) ??
      data?.programs[0] ??
      null,
  )
  const unitScopes = $derived(
    active
      ? [
          { label: 'Satker Polda', units: active.by_satker },
          { label: 'Polres', units: active.by_polres },
        ]
      : [],
  )
  const modalTitle = $derived(
    modalDetail ? `${modalDetail.code} · ${modalDetail.title}` : modalCode || 'Detail sub-kode',
  )
  const modalDescription = $derived(
    modalDetail
      ? `${modalDetail.group} ${modalDetail.group_title} · ${formatNumber(modalDetail.total_rows)} kegiatan`
      : '',
  )
  const modalMissingPolres = $derived.by(() => {
    if (!modalDetail || !data) return []
    const performed = new Set(
      modalDetail.by_polres.map((item) => item.label.trim().toLowerCase()),
    )
    return data.known_polres.filter((label) => !performed.has(label.trim().toLowerCase()))
  })
  const modalMissingSatker = $derived.by(() => {
    if (!modalDetail || !data) return []
    const performed = new Set(
      modalDetail.by_satker.map((item) => item.label.trim().toLowerCase()),
    )
    return data.known_satker.filter((label) => !performed.has(label.trim().toLowerCase()))
  })

  function percent(value: number, total: number): number {
    if (total <= 0) return 0
    return Math.max(0, Math.min(100, Math.round((value / total) * 100)))
  }

  function formatValue(value: number, unit: string): string {
    return unit ? `${formatNumber(value)} ${unit}` : formatNumber(value)
  }

  function metricLabel(program: CoverageProgram, key: string): string {
    return program.metrics.find((metric) => metric.key === key)?.label ?? key
  }

  function unitMetric(unit: CoverageUnit, key: string) {
    return unit.values.find((value) => value.key === key)
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
      const result = await importsApi.quickWinDetail(periodId, target)
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
    modalOpen = true
    modalCode = code
    modalDetail = null
    modalError = ''
    modalLoading = true
    try {
      const result = await importsApi.quickWinDetail(periodId, code)
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

{#snippet unitTable(items: UnitBreakdown[], scope: 'polres' | 'satker')}
  {#if items.length}
    <div class="max-h-80 overflow-auto rounded-xl border border-line">
      <table class="min-w-full text-left text-[11px]">
        <thead class="sticky top-0 bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
          <tr>
            <th class="whitespace-nowrap px-3 py-2">Unit</th>
            <th class="whitespace-nowrap px-3 py-2 text-right">Kegiatan</th>
            {#if scope === 'polres'}<th class="whitespace-nowrap px-3 py-2 text-right">Polsek</th>{/if}
            <th class="whitespace-nowrap px-3 py-2 text-right">Personel</th>
            <th class="whitespace-nowrap px-3 py-2 text-right">Bibit</th>
            <th class="whitespace-nowrap px-3 py-2 text-right">Luas</th>
            <th class="whitespace-nowrap px-3 py-2 text-right">Jemaah</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-line">
          {#each items as item (item.label)}
            <tr class="hover:bg-stone-50/60">
              <td class="whitespace-nowrap px-3 py-2 font-medium text-ink">{item.label}</td>
              <td class="px-3 py-2 text-right">{formatNumber(item.rows)}</td>
              {#if scope === 'polres'}<td class="px-3 py-2 text-right text-ink-muted">{formatNumber(item.polsek)}</td>{/if}
              <td class="px-3 py-2 text-right">{formatNumber(item.personel)}</td>
              <td class="px-3 py-2 text-right">{formatNumber(item.bibit)}</td>
              <td class="px-3 py-2 text-right">{formatValue(item.area_m2, 'm²')}</td>
              <td class="px-3 py-2 text-right">{formatNumber(item.jamaah)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="rounded-xl border border-dashed border-line px-3 py-4 text-center text-[11px] text-ink-muted">Belum ada unit pelaksana.</p>
  {/if}
{/snippet}

{#snippet activityList(items: QuickWinActivity[])}
  {#if items.length}
    <div class="max-h-96 space-y-2 overflow-y-auto pr-1">
      {#each items as activity, index (index)}
        <article class="rounded-xl border border-line bg-white p-3">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span class="rounded-md bg-forest-50 px-1.5 py-0.5 text-[10px] font-bold text-forest-700">{activity.kode}</span>
              <span class="text-[10px] text-ink-muted">{formatDate(activity.tanggal)}</span>
              <span class="text-[10px] text-ink-muted">{activity.polres || activity.satker}{#if activity.polsek} · {activity.polsek}{/if}</span>
            </div>
            {#if activity.map_url}
              <a class="inline-flex items-center gap-1 text-[10px] font-semibold text-forest-700 hover:underline" href={activity.map_url} target="_blank" rel="noreferrer">
                <MapPin size={12} /> Peta
              </a>
            {/if}
          </div>
          <p class="mt-1.5 text-[11px] leading-relaxed text-ink">{activity.kegiatan || activity.title || 'Kegiatan'}</p>
          <div class="mt-1.5 flex flex-wrap gap-2 text-[10px] text-ink-muted">
            {#if activity.personel}<span>{formatNumber(activity.personel)} personel</span>{/if}
            {#if activity.bibit}<span>{formatNumber(activity.bibit)} bibit</span>{/if}
            {#if activity.area_m2}<span>{formatValue(activity.area_m2, 'm²')}</span>{/if}
            {#if activity.jamaah}<span>{formatNumber(activity.jamaah)} jemaah</span>{/if}
            {#if activity.pelaksana}<span>{activity.pelaksana}</span>{/if}
          </div>
        </article>
      {/each}
    </div>
  {:else}
    <p class="text-[11px] text-ink-muted">Belum ada kegiatan pada kode ini.</p>
  {/if}
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
    <section class="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
      {#each [
        { label: 'Kelompok QW', value: `${groupsPerformed}/${data.groups.length}`, hint: 'kelompok dengan kegiatan', icon: Layers },
        { label: 'Sub-kode', value: `${codesPerformed}/${totalCodes}`, hint: 'sub-kode dengan kegiatan', icon: Check },
        { label: 'Polres aktif', value: `${data.active_polres}/${data.known_polres.length}`, hint: 'melapor pada periode', icon: Users },
        { label: 'Satker Polda aktif', value: `${data.active_satker}/${data.known_satker.length}`, hint: 'melapor pada periode', icon: ShieldCheck },
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
          <p class="mt-1 text-[10px] text-ink-muted">Dihitung dari satker/polres yang memiliki baris pada periode ini.</p>
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
                {@render coverageStat('Polres', group.polres, data.known_polres.length)}
                {@render coverageStat('Satker', group.satker, data.known_satker.length)}
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
                      <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan per polres</p>
                      {@render unitTable(groupDetail.by_polres, 'polres')}
                    </div>
                    <div>
                      <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan per satker Polda</p>
                      {@render unitTable(groupDetail.by_satker, 'satker')}
                    </div>
                  </div>
                {/if}

                <div>
                  <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
                    <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Sub-kode {group.code}</p>
                    <span class="text-[10px] text-ink-muted">Klik sub-kode untuk rincian per polres.</span>
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
                          {code.polres} polres · {code.satker} satker
                          {#if code.polres_missing || code.satker_missing}
                            <span class="text-red-700"> · belum {code.polres_missing} polres / {code.satker_missing} satker</span>
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
          <span class="eyebrow">KELENGKAPAN DATA</span>
          <h2 class="panel-title mt-0.5">Cakupan program prioritas</h2>
        </div>
        <span class="badge badge-neutral">{data.programs.length} program</span>
      </div>
      <div class="flex flex-wrap gap-2 border-b border-line px-5 py-3">
        {#each data.programs as program (program.key)}
          <button
            type="button"
            class="btn btn-sm {active?.key === program.key ? 'btn-primary' : 'btn-secondary'}"
            onclick={() => (activeProgram = program.key)}
          >
            {program.title}
          </button>
        {/each}
      </div>

      {#if active}
        <div class="space-y-5 p-5">
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p class="text-sm font-semibold text-ink">
                {active.title}
                {#if active.code}<span class="ml-2 text-[11px] font-normal text-ink-muted">{active.code}</span>{/if}
              </p>
              <p class="mt-1 text-[11px] text-ink-muted">
                {formatNumber(active.rows)} baris · {active.polres}/{active.polres_total} polres · {active.satker}/{active.satker_total} satker
              </p>
            </div>
            {#if active.key !== 'media'}
              <div class="flex flex-wrap gap-4">
                {@render coverageStat('Polres', active.polres, active.polres_total)}
                {@render coverageStat('Satker', active.satker, active.satker_total)}
              </div>
            {/if}
          </div>

          <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            {#each active.metrics as metric (metric.key)}
              <article class="rounded-xl border border-line bg-stone-50/60 p-3.5">
                <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{metric.label}</p>
                <p class="mt-1 font-display text-xl text-forest-900">{formatValue(metric.total, metric.unit)}</p>
                <p class="mt-1 text-[10px] text-ink-muted">
                  {metric.filled_polres}/{active.polres} polres · {metric.filled_satker}/{active.satker} satker mengisi
                </p>
                <p class="mt-0.5 text-[10px] {active.rows === 0 ? 'text-ink-muted' : metric.missing_rows > 0 ? 'text-red-700' : 'text-forest-600'}">
                  {#if active.rows === 0}
                    Belum ada baris program
                  {:else if metric.missing_rows > 0}
                    {formatNumber(metric.missing_rows)} baris belum diisi
                  {:else}
                    Semua baris terisi
                  {/if}
                </p>
              </article>
            {/each}
          </div>

          {#if active.key !== 'media'}
            <div class="grid gap-4 xl:grid-cols-2">
              <div class="rounded-xl border border-line p-3.5">
                <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Polres belum melaksanakan</p>
                <div class="mt-2">{@render chips(active.polres_missing)}</div>
              </div>
              <div class="rounded-xl border border-line p-3.5">
                <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Satker Polda belum melaksanakan</p>
                <div class="mt-2">{@render chips(active.satker_missing)}</div>
              </div>
            </div>
          {/if}

          <div class="grid gap-4 xl:grid-cols-2">
            {#each unitScopes as scope (scope.label)}
              {@const incomplete = scope.units.filter((unit) => unit.missing.length > 0)}
              <div class="rounded-xl border border-line p-3.5">
                <div class="flex items-center justify-between gap-2">
                  <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{scope.label} belum lengkap</p>
                  <span class="badge {incomplete.length ? 'badge-danger' : 'badge-success'}">{incomplete.length} unit</span>
                </div>
                {#if !scope.units.length}
                  <p class="mt-2 text-[11px] text-ink-muted">Belum ada unit yang melaksanakan program ini.</p>
                {:else if incomplete.length}
                  <ul class="mt-2 max-h-64 space-y-2 overflow-y-auto pr-1">
                    {#each incomplete as unit (unit.label)}
                      <li class="rounded-lg bg-stone-50 px-2.5 py-2">
                        <p class="text-[11px] font-semibold text-ink">{unit.label}</p>
                        <p class="mt-0.5 text-[10px] text-red-700">
                          Belum: {unit.missing.map((key) => metricLabel(active, key)).join(', ')}
                        </p>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="mt-2 text-[11px] text-forest-600">Semua unit pada program ini sudah mengisi seluruh metrik.</p>
                {/if}
              </div>
            {/each}
          </div>

          {#each unitScopes as scope (scope.label)}
            <div>
              <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
                <div>
                  <p class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">{scope.label}</p>
                  <p class="mt-0.5 text-[10px] text-ink-muted">Nilai metrik dan penanda isian.</p>
                </div>
                <span class="text-[10px] text-ink-muted">{scope.units.length} unit</span>
              </div>
              {#if scope.units.length}
                <div class="overflow-x-auto rounded-xl border border-line">
                  <table class="min-w-full text-left text-[11px]">
                    <thead class="bg-stone-50 text-[9px] tracking-[0.08em] text-ink-muted uppercase">
                      <tr>
                        <th class="whitespace-nowrap px-3 py-2">Unit</th>
                        <th class="whitespace-nowrap px-3 py-2 text-right">Baris</th>
                        {#each active.metrics as metric (metric.key)}
                          <th class="whitespace-nowrap px-3 py-2 text-right">{metric.label}</th>
                        {/each}
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-line">
                      {#each scope.units as unit (unit.label)}
                        <tr class="hover:bg-stone-50/60">
                          <td class="whitespace-nowrap px-3 py-2 font-medium text-ink">{unit.label}</td>
                          <td class="px-3 py-2 text-right text-ink-muted">{formatNumber(unit.rows)}</td>
                          {#each active.metrics as metric (metric.key)}
                            <td class="px-3 py-2 text-right">
                              {#if unitMetric(unit, metric.key)?.filled}
                                <span class="font-medium text-ink">{formatValue(unitMetric(unit, metric.key)?.value ?? 0, metric.unit)}</span>
                              {:else}
                                <span class="rounded-md bg-red-50 px-1.5 py-0.5 text-[9px] font-bold text-red-700">Belum</span>
                              {/if}
                            </td>
                          {/each}
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {:else}
                <p class="rounded-xl border border-dashed border-line px-3 py-4 text-center text-[11px] text-ink-muted">
                  Belum ada unit yang melaksanakan program ini.
                </p>
              {/if}
            </div>
          {/each}
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
      <div class="grid gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-7">
        {#each Array(7) as _}<div class="skeleton h-20"></div>{/each}
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
      <div class="grid gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-7">
        {#each [
          { label: 'Kegiatan', value: formatNumber(modalDetail.total_rows) },
          { label: 'Polres', value: formatNumber(modalDetail.polres_count) },
          { label: 'Satker Polda', value: formatNumber(modalDetail.satker_count) },
          { label: 'Personel', value: formatNumber(modalDetail.personel) },
          { label: 'Bibit', value: formatNumber(modalDetail.bibit) },
          { label: 'Luas', value: formatValue(modalDetail.area_m2, 'm²') },
          { label: 'Jemaah', value: formatNumber(modalDetail.jamaah) },
        ] as card}
          <div class="rounded-xl border border-line bg-stone-50/60 px-3 py-2.5">
            <p class="text-[9px] font-bold tracking-[0.1em] text-ink-muted uppercase">{card.label}</p>
            <strong class="mt-0.5 block text-sm font-semibold text-forest-900">{card.value}</strong>
          </div>
        {/each}
      </div>

      <div class="grid gap-4 xl:grid-cols-2">
        <section>
          <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan per polres</p>
          {@render unitTable(modalDetail.by_polres, 'polres')}
        </section>
        <section>
          <p class="mb-2 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan per satker Polda</p>
          {@render unitTable(modalDetail.by_satker, 'satker')}
        </section>
      </div>

      <div class="grid gap-4 xl:grid-cols-2">
        <div class="rounded-xl border border-line p-3.5">
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Polres belum melaksanakan</p>
          <div class="mt-2">{@render chips(modalMissingPolres)}</div>
        </div>
        <div class="rounded-xl border border-line p-3.5">
          <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Satker Polda belum melaksanakan</p>
          <div class="mt-2">{@render chips(modalMissingSatker)}</div>
        </div>
      </div>

      <section>
        <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
          <div>
            <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Kegiatan</p>
            <p class="mt-0.5 text-[10px] text-ink-muted">Menampilkan maksimal 100 kegiatan terbaru.</p>
          </div>
          <span class="text-[10px] text-ink-muted">{modalDetail.activities.length} kegiatan</span>
        </div>
        {@render activityList(modalDetail.activities)}
      </section>
    </div>
  {/if}
</Modal>
