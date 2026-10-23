<script lang="ts">
  import type { Snippet } from 'svelte'
  import { formatNumber } from '../../utils/format'

  type Props = {
    label: string
    value: number
    unit?: string
    detail?: string
    icon?: Snippet
    tone?: 'forest' | 'gold' | 'slate'
  }

  let {
    label,
    value,
    unit = '',
    detail = '',
    icon,
    tone = 'forest',
  }: Props = $props()

  const tones = {
    forest: 'bg-forest-50 text-forest-700',
    gold: 'bg-gold-100 text-gold-700',
    slate: 'bg-stone-100 text-stone-600',
  }
</script>

<div class="stat-tile">
  <div class="flex items-start justify-between gap-3">
    <span class="text-xs font-semibold tracking-wide text-ink-muted uppercase">{label}</span>
    {#if icon}
      <span class="flex h-9 w-9 items-center justify-center rounded-xl {tones[tone]}">{@render icon()}</span>
    {/if}
  </div>
  <div class="mt-4 flex items-end gap-1.5">
    <strong class="stat-value">{formatNumber(value)}</strong>
    {#if unit}<span class="pb-1 text-xs font-medium text-ink-muted">{unit}</span>{/if}
  </div>
  {#if detail}<p class="mt-2 text-xs leading-relaxed text-ink-muted">{detail}</p>{/if}
</div>
