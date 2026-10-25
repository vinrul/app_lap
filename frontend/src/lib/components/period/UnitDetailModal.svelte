<script lang="ts">
  import { untrack } from 'svelte'
  import { AlertCircle, ArrowLeft, ChevronRight, ListTree, LoaderCircle } from 'lucide-svelte'
  import { importsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type { QuickWinDetail, UnitDetail } from '../../types'
  import { qwGroupTitles } from '../../utils/constants'
  import { formatDate, formatNumber, formatRange } from '../../utils/format'
  import Modal from '../ui/Modal.svelte'
  import UnitBreakdownTable from './UnitBreakdownTable.svelte'

  type Props = {
    open?: boolean
    periodId: string
    scope: 'polres' | 'satker'
    label: string
    onclose: () => void
    onopenunit?: (scope: 'polres' | 'satker', label: string) => void
  }

  let { open = false, periodId, scope, label, onclose, onopenunit }: Props = $props()

  let data = $state<UnitDetail | null>(null)
  let loading = $state(false)
  let error = $state('')
  let activeCode = $state<string | null>(null)
  let quickWin = $state<QuickWinDetail | null>(null)
  let quickWinLoading = $state(false)
  let quickWinError = $state('')
  let loadedKey = ''

  const scopeLabel = $derived(scope === 'polres' ? 'Polres' : 'Satker Polda')
  const title = $derived(
    activeCode
      ? quickWin && quickWin.code === activeCode
        ? `${quickWin.code} · ${quickWin.title}`
        : activeCode
      : data?.label || label || 'Detail unit',
  )
  const description = $derived.by(() => {
    if (activeCode && quickWin && quickWin.code === activeCode) {
      return `${quickWin.group} ${quickWin.group_title} · ${formatNumber(quickWin.total_rows)} kegiatan`
    }
    if (activeCode) return 'Memuat detail Quick Wins…'
    if (data) {
      return `${scopeLabel} · ${formatNumber(data.metrics.total_rows)} baris · ${formatRange(data.date_start, data.date_end)}`
    }
    return ''
  })

  $effect(() => {
    const key = open ? `${periodId}|${scope}|${label}` : ''
    if (!open) {
      loadedKey = ''
      data = null
      error = ''
      activeCode = null
      quickWin = null
      quickWinError = ''
      return
    }
    if (!label || key === loadedKey) return
    loadedKey = key
    activeCode = null
    quickWin = null
    quickWinError = ''
    untrack(() => void loadUnit())
  })

  async function loadUnit() {
    loading = true
    error = ''
    try {
      data = await importsApi.unitDetail(periodId, scope, label)
    } catch (caught) {
      data = null
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  async function openQuickWin(code: string) {
    activeCode = code
    quickWinLoading = true
    quickWinError = ''
    try {
      quickWin = await importsApi.quickWinDetail(periodId, code)
    } catch (caught) {
      quickWin = null
      quickWinError = errorMessage(caught)
    } finally {
      quickWinLoading = false
    }
  }

  function goBack() {
    if (!quickWin) {
      activeCode = null
      return
    }
    if (quickWin.code.includes('.')) {
      void openQuickWin(quickWin.group)
    } else {
      activeCode = null
      quickWin = null
    }
  }

  const quickWinGroups = $derived(
    (data?.metrics.quick_wins ?? [])
      .filter((item) => Boolean(qwGroupTitles[item.label]))
      .map((item) => ({
        ...item,
        title: qwGroupTitles[item.label] ?? item.label,
      })),
  )
  const knownCodes = $derived(
    (data?.metrics.codes ?? []).filter((item) => /^QW\d+\.\d+$/.test(item.label)),
  )
</script>

<Modal {open} onclose={onclose} {title} {description} width="max-w-6xl">
  {#if loading}
    <div class="space-y-3">
      {#each Array(6) as _}<div class="skeleton h-24 w-full"></div>{/each}
    </div>
  {:else if error}
    <div class="flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertCircle size={17} /> {error}
    </div>
  {:else if data}
    {#if activeCode}
      <div class="space-y-5">
        <button class="btn btn-ghost btn-sm" onclick={goBack}>
          <ArrowLeft size={14} /> {quickWin?.code.includes('.') ? `Kembali ke ${quickWin?.group}` : `Kembali ke ${data.label}`}
        </button>

        {#if quickWinLoading}
          <div class="space-y-3">
            {#each Array(5) as _}<div class="skeleton h-20 w-full"></div>{/each}
          </div>
        {:else if quickWinError}
          <div class="flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
            <AlertCircle size={17} /> {quickWinError}
          </div>
        {:else if quickWin}
          <div class="grid gap-3 sm:grid-cols-4">
            {#each [
              ['Kegiatan', formatNumber(quickWin.total_rows)],
              ['Polres', formatNumber(quickWin.polres_count)],
              ['Satker Polda', formatNumber(quickWin.satker_count)],
              ['Rentang', formatRange(quickWin.date_start, quickWin.date_end)],
            ] as [label, value]}
              <div class="rounded-xl border border-line bg-stone-50/60 px-4 py-3">
                <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{label}</p>
                <strong class="mt-1 block text-sm font-semibold text-forest-900">{value}</strong>
              </div>
            {/each}
          </div>

          <section>
            <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
              <div>
                <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Sub-kode {quickWin.group}</h3>
                <p class="mt-0.5 text-[11px] text-ink-muted">Klik sub-kode untuk melihat cakupan dan kegiatannya.</p>
              </div>
              <span class="text-[11px] font-medium text-ink-muted">{quickWin.codes.length} sub-kode</span>
            </div>
            <div class="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
              {#each quickWin.codes as item}
                <button
                  class="rounded-xl border px-3.5 py-3 text-left transition-colors {activeCode === item.code
                    ? 'border-forest-700 bg-forest-50'
                    : 'border-line bg-white hover:border-gold-300 hover:bg-gold-50/40'}"
                  onclick={() => openQuickWin(item.code)}
                >
                  <div class="flex items-start justify-between gap-2">
                    <span class="rounded-md bg-forest-50 px-1.5 py-0.5 text-[10px] font-bold text-forest-700">{item.code}</span>
                    <ChevronRight size={13} class="mt-0.5 shrink-0 text-ink-muted" />
                  </div>
                  <p class="mt-1.5 line-clamp-2 text-[11px] font-medium leading-snug text-ink">{item.title}</p>
                  <p class="mt-1.5 text-[10px] text-ink-muted">
                    {formatNumber(item.rows)} kegiatan · {formatNumber(item.polres)} polres · {formatNumber(item.satker)} satker
                  </p>
                </button>
              {/each}
            </div>
          </section>

          <div class="grid gap-4 xl:grid-cols-2">
            <section class="card overflow-hidden">
              <div class="panel-heading">
                <div>
                  <span class="eyebrow">CAKUPAN</span>
                  <h3 class="panel-title mt-0.5">Polres pelaksana</h3>
                </div>
                <span class="text-xs text-ink-muted">{formatNumber(quickWin.by_polres.length)} polres</span>
              </div>
              <UnitBreakdownTable
                items={quickWin.by_polres}
                showPolsek
                unitHeading="Polres"
                empty="Belum ada polres yang melaksanakan kode ini."
                onselect={onopenunit ? (item) => onopenunit('polres', item.label) : undefined}
              />
            </section>
            <section class="card overflow-hidden">
              <div class="panel-heading">
                <div>
                  <span class="eyebrow">CAKUPAN</span>
                  <h3 class="panel-title mt-0.5">Satker Polda pelaksana</h3>
                </div>
                <span class="text-xs text-ink-muted">{formatNumber(quickWin.by_satker.length)} satker</span>
              </div>
              <UnitBreakdownTable
                items={quickWin.by_satker}
                unitHeading="Satker / pejabat"
                empty="Belum ada satker Polda yang melaksanakan kode ini."
                onselect={onopenunit ? (item) => onopenunit('satker', item.label) : undefined}
              />
            </section>
          </div>

          <section class="card overflow-hidden">
            <div class="panel-heading">
              <div>
                <span class="eyebrow">RINCIAN KEGIATAN</span>
                <h3 class="panel-title mt-0.5">Kegiatan terbaru</h3>
              </div>
              <span class="text-xs text-ink-muted">maks. 100 baris</span>
            </div>
            {#if quickWin.activities.length}
              <div class="max-h-[24rem] overflow-auto">
                <table class="table text-xs">
                  <thead class="sticky top-0 z-10 bg-stone-50">
                    <tr>
                      <th>Tanggal</th>
                      <th>Kode</th>
                      <th>Unit pelaksana</th>
                      <th>Kegiatan</th>
                      <th class="text-right">Analisis</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each quickWin.activities as activity}
                      <tr>
                        <td class="whitespace-nowrap">{formatDate(activity.tanggal)}</td>
                        <td class="whitespace-nowrap">
                          <span class="rounded-md bg-forest-50 px-1.5 py-0.5 text-[10px] font-bold text-forest-700">{activity.kode}</span>
                          <span class="mt-1 block max-w-[10rem] text-[10px] leading-snug text-ink-muted">{activity.title}</span>
                        </td>
                        <td class="max-w-[12rem]">
                          <span class="block truncate font-medium text-ink">{activity.polres || activity.satker || '—'}</span>
                          {#if activity.polsek}<span class="text-[10px] text-ink-muted">{activity.polsek}</span>{/if}
                        </td>
                        <td class="max-w-[18rem]">
                          <span class="block truncate font-medium text-ink" title={activity.kegiatan}>{activity.kegiatan || '—'}</span>
                          <span class="text-[10px] text-ink-muted">{activity.pelaksana || '—'}</span>
                        </td>
                        <td class="whitespace-nowrap text-right text-[10px] text-ink-muted">
                          {#if activity.personel}{formatNumber(activity.personel)} personel · {/if}
                          {#if activity.bibit}{formatNumber(activity.bibit)} bibit · {/if}
                          {#if activity.area_m2}{formatNumber(activity.area_m2)} m² · {/if}
                          {#if activity.jamaah}{formatNumber(activity.jamaah)} jemaah{/if}
                          {#if !activity.personel && !activity.bibit && !activity.area_m2 && !activity.jamaah}—{/if}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else}
              <p class="px-5 py-6 text-xs text-ink-muted">Belum ada kegiatan untuk kode ini.</p>
            {/if}
          </section>
        {/if}
      </div>
    {:else}
      <div class="space-y-5">
        <div class="flex flex-wrap items-center gap-2">
          <span class="badge badge-neutral">{scopeLabel}</span>
          <span class="text-xs text-ink-muted">{formatNumber(data.metrics.total_rows)} baris · {formatRange(data.date_start, data.date_end)}</span>
          {#if scope === 'polres'}<span class="text-xs text-ink-muted">· {formatNumber(data.polsek_count)} polsek</span>{/if}
        </div>

        <section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {#each data.metrics.cards as card}
            <article class="rounded-xl border border-line bg-white p-4">
              <p class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{card.label}</p>
              <div class="mt-2 flex items-end gap-1.5">
                <strong class="font-display text-2xl leading-none font-semibold text-forest-900">{formatNumber(card.value)}</strong>
                <span class="pb-0.5 text-[10px] text-ink-muted">{card.unit}</span>
              </div>
              <p class="mt-1.5 text-[11px] leading-relaxed text-ink-muted">{card.detail}</p>
            </article>
          {/each}
        </section>

        <div class="grid gap-4 xl:grid-cols-2">
          <section class="card overflow-hidden">
            <div class="panel-heading">
              <div>
                <span class="eyebrow">QUICK WINS</span>
                <h3 class="panel-title mt-0.5">Sebaran kelompok</h3>
              </div>
            </div>
            <div class="p-3">
              {#if quickWinGroups.length}
                <div class="grid gap-2 sm:grid-cols-2">
                  {#each quickWinGroups as item}
                    <button class="flex items-center justify-between gap-2 rounded-xl border border-line px-3 py-2.5 text-left hover:border-gold-300 hover:bg-gold-50/40" onclick={() => openQuickWin(item.label)}>
                      <span class="min-w-0">
                        <span class="text-[10px] font-bold text-forest-700">{item.label}</span>
                        <span class="mt-0.5 block truncate text-[11px] font-medium text-ink">{item.title}</span>
                      </span>
                      <span class="flex shrink-0 items-center gap-1 text-xs font-bold text-forest-900">{formatNumber(item.count)} <ChevronRight size={13} class="text-ink-muted" /></span>
                    </button>
                  {/each}
                </div>
              {:else}
                <p class="px-2 py-4 text-xs text-ink-muted">Belum ada data Quick Wins.</p>
              {/if}
            </div>
          </section>

          <section class="card overflow-hidden">
            <div class="panel-heading">
              <div>
                <span class="eyebrow">KODE</span>
                <h3 class="panel-title mt-0.5">Sebaran sub-kode</h3>
              </div>
              <span class="text-xs text-ink-muted">{knownCodes.length} kode</span>
            </div>
            <div class="max-h-[22rem] overflow-auto p-3">
              {#if knownCodes.length}
                <div class="space-y-1.5">
                  {#each knownCodes as item}
                    <button class="flex w-full items-center justify-between gap-3 rounded-lg border border-line px-3 py-2 text-left hover:border-gold-300 hover:bg-gold-50/40" onclick={() => openQuickWin(item.label)}>
                      <span class="rounded-md bg-forest-50 px-1.5 py-0.5 text-[10px] font-bold text-forest-700">{item.label}</span>
                      <span class="flex items-center gap-1 text-xs font-semibold text-forest-900">{formatNumber(item.count)} <ChevronRight size={12} class="text-ink-muted" /></span>
                    </button>
                  {/each}
                </div>
              {:else}
                <p class="px-2 py-4 text-xs text-ink-muted">Belum ada data kode.</p>
              {/if}
            </div>
          </section>
        </div>

        {#if scope === 'polres'}
          <section class="card overflow-hidden">
            <div class="panel-heading">
              <div>
                <span class="eyebrow">RINCIAN</span>
                <h3 class="panel-title mt-0.5">Polsek pelaksana</h3>
                <p class="mt-0.5 text-[11px] text-ink-muted">Termasuk kegiatan tingkat polres.</p>
              </div>
              <span class="text-xs text-ink-muted">{formatNumber(data.by_polsek.length)} unit</span>
            </div>
            <UnitBreakdownTable
              items={data.by_polsek}
              unitHeading="Polsek / tingkat polres"
              empty="Belum ada rincian polsek."
            />
          </section>
        {/if}

        <section class="card overflow-hidden">
          <div class="panel-heading">
            <div>
              <span class="eyebrow">SOROTAN</span>
              <h3 class="panel-title mt-0.5">Kegiatan terpilih</h3>
            </div>
            <ListTree size={17} class="text-ink-muted" />
          </div>
          {#if data.metrics.highlights.length}
            <div class="divide-y divide-stone-100">
              {#each data.metrics.highlights.slice(0, 8) as row}
                <div class="flex flex-wrap items-start justify-between gap-2 px-5 py-3">
                  <div class="min-w-0">
                    <p class="flex flex-wrap items-center gap-2">
                      <span class="rounded-md bg-forest-50 px-1.5 py-0.5 text-[10px] font-bold text-forest-700">{row.kode || '—'}</span>
                      <span class="text-[10px] text-ink-muted">{formatDate(row.tanggal)}</span>
                    </p>
                    <p class="mt-1 text-xs font-medium text-ink">{row.kegiatan || '—'}</p>
                    <p class="text-[10px] text-ink-muted">{row.location || row.satuan}</p>
                  </div>
                  <ChevronRight size={14} class="mt-1 shrink-0 text-ink-muted" />
                </div>
              {/each}
            </div>
          {:else}
            <p class="px-5 py-6 text-xs text-ink-muted">Belum ada sorotan kegiatan.</p>
          {/if}
        </section>
      </div>
    {/if}
  {:else if !loading}
    <div class="flex items-center gap-2 text-sm text-ink-muted"><LoaderCircle size={16} class="animate-spin" /> Memuat detail unit…</div>
  {/if}
</Modal>
