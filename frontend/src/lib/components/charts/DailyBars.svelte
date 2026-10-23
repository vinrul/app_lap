<script lang="ts">
  import type { DailyCount } from '../../types'
  import { formatNumber, formatShortDate } from '../../utils/format'

  type Props = { items?: DailyCount[] }
  let { items = [] }: Props = $props()

  const max = $derived(Math.max(1, ...items.map((item) => item.count)))
</script>

{#if items.length === 0}
  <p class="text-sm text-ink-muted">Belum ada data harian.</p>
{:else}
  <div class="flex h-40 items-end gap-2">
    {#each items as item}
      <div class="group flex min-w-0 flex-1 flex-col items-center gap-2">
        <span class="text-[10px] font-semibold text-forest-800 opacity-0 transition group-hover:opacity-100">
          {formatNumber(item.count)}
        </span>
        <div
          class="w-full rounded-t-md bg-forest-200 transition-colors group-hover:bg-forest-500"
          style:height={`${Math.max(4, (item.count / max) * 110)}px`}
          title={`${formatShortDate(item.date)}: ${formatNumber(item.count)} laporan`}
        ></div>
        <span class="truncate text-[10px] text-ink-muted">{formatShortDate(item.date).replace(/ \d{4}$/, '')}</span>
      </div>
    {/each}
  </div>
{/if}
