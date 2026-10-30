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

  function portal(node: HTMLElement): { destroy: () => void } {
    document.body.appendChild(node)
    return { destroy: () => node.remove() }
  }
</script>

{#if open}
  <div use:portal class="fixed inset-0 z-50 flex items-end justify-center p-0 sm:items-center sm:p-6">
    <button
      type="button"
      class="absolute inset-0 h-full w-full cursor-default bg-forest-950/50 backdrop-blur-[2px]"
      aria-label="Tutup dialog"
      tabindex="-1"
      onclick={onclose}
    ></button>
    <div
      class="relative flex max-h-[92dvh] w-full {width} animate-fade-up flex-col overflow-hidden rounded-t-3xl bg-white shadow-lift sm:rounded-3xl"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
    >
      <div class="flex shrink-0 items-start justify-between gap-3 border-b border-line bg-white px-4 py-4 sm:gap-4 sm:px-6 sm:py-5">
        <div class="min-w-0">
          <h2 class="font-display text-lg font-semibold break-words text-forest-900 sm:text-xl">{title}</h2>
          {#if description}<p class="mt-1 text-xs leading-relaxed break-words text-ink-muted sm:text-sm">{description}</p>{/if}
        </div>
        <button
          type="button"
          class="btn-icon shrink-0 bg-stone-50"
          aria-label="Tutup dialog"
          title="Tutup (Esc)"
          onclick={onclose}
        >
          <X size={18} />
        </button>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 py-4 sm:px-6 sm:py-5">
        {@render children?.()}
      </div>
      {#if footer}
        <div class="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-line bg-stone-50/60 px-4 py-3 sm:px-6 sm:py-4">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}
