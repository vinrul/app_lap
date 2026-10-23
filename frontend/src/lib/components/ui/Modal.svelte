<script lang="ts">
  import type { Snippet } from 'svelte'
  import X from 'lucide-svelte/icons/x'

  type Props = {
    open?: boolean
    title: string
    description?: string
    onclose: () => void
    children?: Snippet
    footer?: Snippet
    width?: string
  }

  let {
    open = false,
    title,
    description = '',
    onclose,
    children,
    footer,
    width = 'max-w-xl',
  }: Props = $props()

  $effect(() => {
    if (!open) return
    const handler = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onclose()
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  })
</script>

{#if open}
  <div class="fixed inset-0 z-50 flex items-end justify-center p-0 sm:items-center sm:p-6">
    <button
      type="button"
      class="absolute inset-0 h-full w-full cursor-default bg-forest-950/50 backdrop-blur-[2px]"
      aria-label="Tutup dialog"
      tabindex="-1"
      onclick={onclose}
    ></button>
    <div
      class="relative animate-fade-up max-h-[92vh] w-full {width} overflow-y-auto rounded-t-3xl bg-white shadow-lift sm:rounded-3xl"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
    >
      <div class="flex items-start justify-between gap-4 border-b border-line px-6 py-5">
        <div>
          <h2 class="font-display text-xl font-semibold text-forest-900">{title}</h2>
          {#if description}<p class="mt-1 text-sm text-ink-muted">{description}</p>{/if}
        </div>
        <button class="btn-icon shrink-0" aria-label="Tutup" onclick={onclose}><X size={18} /></button>
      </div>
      <div class="px-6 py-5">{@render children?.()}</div>
      {#if footer}
        <div class="flex flex-wrap items-center justify-end gap-2 border-t border-line bg-stone-50/60 px-6 py-4">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}
