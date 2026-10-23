<script lang="ts">
  import type { Snippet } from 'svelte'

  type Props = {
    items: { id: string; label: string; count?: number }[]
    active: string
    onchange: (id: string) => void
  }

  let {
    items,
    active,
    onchange,
  }: Props = $props()
</script>

<div class="flex gap-1 overflow-x-auto rounded-xl border border-line bg-white p-1">
  {#each items as item}
    <button
      class="flex cursor-pointer items-center gap-2 rounded-lg px-3.5 py-2 text-xs font-semibold whitespace-nowrap transition-colors {active ===
      item.id
        ? 'bg-forest-800 text-white'
        : 'text-ink-soft hover:bg-stone-100'}"
      onclick={() => onchange(item.id)}
    >
      {item.label}
      {#if item.count !== undefined}
        <span class="rounded-full bg-black/10 px-1.5 py-0.5 text-[10px]">{item.count}</span>
      {/if}
    </button>
  {/each}
</div>
