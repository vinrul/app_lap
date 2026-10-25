<script lang="ts">
  import type { UnitBreakdown } from '../../types'
  import { formatNumber } from '../../utils/format'

  type Props = {
    items: UnitBreakdown[]
    showPolsek?: boolean
    unitHeading?: string
    empty?: string
    onselect?: (item: UnitBreakdown) => void
  }

  let {
    items,
    showPolsek = false,
    unitHeading = 'Unit',
    empty = 'Belum ada data untuk breakdown ini.',
    onselect,
  }: Props = $props()
</script>

{#if items.length === 0}
  <p class="px-4 py-6 text-xs text-ink-muted">{empty}</p>
{:else}
  <div class="max-h-[26rem] overflow-auto">
    <table class="table text-xs">
      <thead class="sticky top-0 z-10 bg-stone-50">
        <tr>
          <th>{unitHeading}</th>
          {#if showPolsek}<th class="text-right">Polsek</th>{/if}
          <th class="text-right">Baris</th>
          <th class="text-right">Personel</th>
          <th class="text-right">Bibit</th>
          <th class="text-right">Luas (m²)</th>
          <th class="text-right">Jemaah</th>
          {#if onselect}<th></th>{/if}
        </tr>
      </thead>
      <tbody>
        {#each items as item}
          <tr>
            <td class="max-w-[16rem]">
              {#if onselect}
                <button class="block max-w-full truncate text-left font-semibold text-forest-800 hover:underline" title={item.label} onclick={() => onselect?.(item)}>{item.label}</button>
              {:else}
                <span class="block truncate font-semibold text-ink" title={item.label}>{item.label}</span>
              {/if}
            </td>
            {#if showPolsek}<td class="text-right text-ink-muted">{item.polsek || '—'}</td>{/if}
            <td class="text-right font-semibold text-forest-900">{formatNumber(item.rows)}</td>
            <td class="text-right text-ink-soft">{formatNumber(item.personel)}</td>
            <td class="text-right text-ink-soft">{formatNumber(item.bibit)}</td>
            <td class="text-right whitespace-nowrap text-ink-soft">{formatNumber(item.area_m2)}</td>
            <td class="text-right text-ink-soft">{formatNumber(item.jamaah)}</td>
            {#if onselect}
              <td class="text-right">
                <button class="btn btn-ghost btn-sm" onclick={() => onselect?.(item)}>Detail</button>
              </td>
            {/if}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
