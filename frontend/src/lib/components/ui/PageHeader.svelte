<script lang="ts">
  import type { Snippet } from 'svelte'

  type Props = {
    eyebrow?: string
    title: string
    description?: string
    actions?: Snippet
    compact?: boolean
  }

  let {
    eyebrow = '',
    title,
    description = '',
    actions,
    compact = false,
  }: Props = $props()
</script>

<div class="mb-6 flex flex-wrap items-end justify-between gap-4">
  <div class="min-w-0">
    {#if eyebrow}<span class="eyebrow">{eyebrow}</span>{/if}
    <h1 class="mt-1.5 font-display text-2xl font-semibold tracking-tight text-forest-900 sm:text-3xl {compact ? 'sm:text-2xl' : ''}">
      {title}
    </h1>
    {#if description}<p class="mt-2 max-w-2xl text-sm leading-relaxed text-ink-muted">{description}</p>{/if}
  </div>
  {#if actions}<div class="flex flex-wrap items-center gap-2">{@render actions()}</div>{/if}
</div>
