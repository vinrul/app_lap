<script lang="ts">
  import type { LabeledCount } from '../../types'
  import { formatNumber } from '../../utils/format'

  type Props = {
    items?: LabeledCount[]
    limit?: number
    empty?: string
    formatLabel?: (label: string) => string
    formatDetail?: (label: string) => string
    barTone?: 'forest' | 'gold'
  }

  let {
    items = [],
    limit = 8,
    empty = 'Belum ada data.',
    formatLabel,
    formatDetail,
    barTone = 'forest',
  }: Props = $props()

  const visible = $derived(items.slice(0, limit))
  const max = $derived(Math.max(1, ...visible.map((item) => item.count)))
  const tones = {
    forest: 'bg-forest-500',
    gold: 'bg-gold-500',
  }
</script>

{#if visible.length === 0}
  <p class="text-sm text-ink-muted">{empty}</p>
{:else}
  <div class="space-y-2.5">
    {#each visible as item}
      {@const labelText = formatLabel ? formatLabel(item.label) : (item.label || 'Belum dikategorikan')}
      {@const detailText = formatDetail ? formatDetail(item.label) : ''}
      <div class="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-x-3 gap-y-1">
        <div class="min-w-0">
          <span class="block truncate text-xs font-medium text-ink-soft" title={detailText ? `${labelText} — ${detailText}` : labelText}>
            {labelText}
          </span>
          {#if detailText}
            <span class="mt-0.5 line-clamp-2 text-[10px] leading-snug text-ink-muted" title={detailText}>
              {detailText}
            </span>
          {/if}
        </div>
        <span class="pt-0.5 text-xs font-bold text-forest-900">{formatNumber(item.count)}</span>
        <span class="col-span-2 h-1.5 overflow-hidden rounded-full bg-stone-100">
          <span
            class="block h-full rounded-full {tones[barTone]}"
            style:width={`${Math.max(3, (item.count / max) * 100)}%`}
          ></span>
        </span>
      </div>
    {/each}
  </div>
{/if}
