<script lang="ts">
  import { onMount } from 'svelte'
  import {
    AlertCircle,
    Droplets,
    Link2,
    MapPin,
    Rows3,
    Sprout,
    Users,
    Wheat,
  } from 'lucide-svelte'
  import { importsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type { AnalysisResponse } from '../../types'
  import { formatDate, formatNumber } from '../../utils/format'
  import BarList from '../charts/BarList.svelte'
  import DailyBars from '../charts/DailyBars.svelte'

  type Props = { periodId: string }
  let { periodId }: Props = $props()

  let data = $state<AnalysisResponse | null>(null)
  let loading = $state(true)
  let error = $state('')

  onMount(load)

  async function load() {
    loading = true
    error = ''
    try {
      data = await importsApi.analysis(periodId)
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  const daily = $derived(
    (data?.by_date ?? []).map((item) => ({ date: item.label, count: item.count })),
  )
</script>

{#if loading}
  <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
    {#each Array(8) as _}<div class="skeleton h-28"></div>{/each}
  </div>
{:else if error}
  <div class="flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{:else if data}
  <div class="space-y-4">
    <section class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      {#each [
        { label: 'Baris unik', value: formatNumber(data.total_rows), hint: `${data.source_count} berkas · ${data.duplicate_rows} duplikat`, icon: Rows3 },
        { label: 'Personel', value: formatNumber(data.personel_total), hint: 'total personel tercatat', icon: Users },
        { label: 'Bibit ditanam', value: formatNumber(data.bibit_total), hint: 'dari kolom isian', icon: Sprout },
        { label: 'Luas area', value: `${formatNumber(data.area_total)} m²`, hint: 'area yang ditangani', icon: Wheat },
        { label: 'Jemaah', value: formatNumber(data.jamaah_total), hint: 'jemaah / jemaat / umat', icon: Users },
        { label: 'Titik koordinat', value: formatNumber(data.rows_with_coordinates), hint: 'baris dengan lat/long', icon: MapPin },
        { label: 'Tautan bukti', value: formatNumber(data.evidence_links), hint: 'tautan pada baris sumber', icon: Link2 },
        { label: 'Periode', value: formatDate(data.date_start), hint: `s.d. ${formatDate(data.date_end)}`, icon: Droplets },
      ] as card}
        <article class="card p-4">
          <card.icon size={17} class="text-forest-600" />
          <p class="mt-3 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">{card.label}</p>
          <strong class="font-display text-2xl text-forest-900">{card.value}</strong>
          <p class="text-[10px] text-ink-muted">{card.hint}</p>
        </article>
      {/each}
    </section>

    <section class="card p-5">
      <div class="mb-4 flex items-end justify-between gap-3">
        <div><span class="eyebrow">TREN HARIAN</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Baris per tanggal</h2></div>
        <span class="text-xs text-ink-muted">{formatDate(data.date_start)} – {formatDate(data.date_end)}</span>
      </div>
      <DailyBars items={daily} />
    </section>

    <div class="grid gap-4 xl:grid-cols-2">
      <section class="card p-5">
        <div class="mb-4"><span class="eyebrow">KELOMPOK</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Quick Wins</h2></div>
        <BarList items={data.by_group} limit={12} />
      </section>
      <section class="card p-5">
        <div class="mb-4"><span class="eyebrow">SEBARAN</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Tingkat pelaksana</h2></div>
        <BarList items={data.by_level} limit={10} barTone="gold" />
      </section>
      <section class="card p-5">
        <div class="mb-4"><span class="eyebrow">DETAIL KODE</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Kode kegiatan</h2></div>
        <BarList items={data.by_code} limit={20} />
      </section>
      <section class="card p-5">
        <div class="mb-4"><span class="eyebrow">SATUAN</span><h2 class="mt-1 font-display text-lg font-semibold text-forest-900">Unit / wilayah</h2></div>
        <BarList items={data.by_unit} limit={15} barTone="gold" />
      </section>
    </div>
  </div>
{/if}
