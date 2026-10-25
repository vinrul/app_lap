<script lang="ts">
  import type { UnitBreakdown } from '../../types'
  import { formatNumber } from '../../utils/format'
  import UnitBreakdownTable from './UnitBreakdownTable.svelte'

  type Props = {
    satker?: UnitBreakdown[]
    polres?: UnitBreakdown[]
    onselect?: (scope: 'polres' | 'satker', item: UnitBreakdown) => void
  }

  let { satker = [], polres = [], onselect }: Props = $props()
</script>

<section class="card overflow-hidden">
  <div class="panel-heading">
    <div>
      <span class="eyebrow">BREAKDOWN PELAKSANA</span>
      <h2 class="panel-title mt-1">Satker Polda &amp; Polres</h2>
      <p class="mt-1 text-xs text-ink-muted">
        Rincian kontribusi kegiatan, personel, bibit, luas area, dan jemaah menurut unit pelaksana.
      </p>
    </div>
    <span class="badge badge-neutral">{formatNumber(satker.length + polres.length)} unit</span>
  </div>

  <div class="border-b border-line px-5 py-4">
    <div class="mb-3 flex flex-wrap items-end justify-between gap-2">
      <div>
        <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Satker Polda</h3>
        <p class="mt-0.5 text-[11px] text-ink-muted">Satker/pejabat pelaksana tingkat Polda tanpa polres/polsek.</p>
      </div>
      <span class="text-[11px] font-medium text-ink-muted">{satker.length} satker</span>
    </div>
    <div class="overflow-hidden rounded-xl border border-line">
      <UnitBreakdownTable
        items={satker}
        unitHeading="Satker / pejabat"
        empty="Belum ada kegiatan yang dilaksanakan satker tingkat Polda."
        onselect={onselect ? (item) => onselect('satker', item) : undefined}
      />
    </div>
  </div>

  <div class="px-5 py-4">
    <div class="mb-3 flex flex-wrap items-end justify-between gap-2">
      <div>
        <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Per Polres</h3>
        <p class="mt-0.5 text-[11px] text-ink-muted">Agregat kegiatan polres dan jajaran polsek di bawahnya.</p>
      </div>
      <span class="text-[11px] font-medium text-ink-muted">{polres.length} polres</span>
    </div>
    <div class="overflow-hidden rounded-xl border border-line">
      <UnitBreakdownTable
        items={polres}
        showPolsek
        unitHeading="Polres"
        empty="Belum ada kegiatan yang dilaporkan pada tingkat polres/polsek."
        onselect={onselect ? (item) => onselect('polres', item) : undefined}
      />
    </div>
  </div>
</section>
