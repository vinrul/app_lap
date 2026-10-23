<script lang="ts">
  import type { Snippet } from 'svelte'
  import FolderOpen from 'lucide-svelte/icons/folder-open'

  type Props = {
    title: string
    description?: string
    icon?: Snippet
    children?: Snippet
  }

  let {
    title,
    description = '',
    icon,
    children,
  }: Props = $props()
</script>

<div class="flex flex-col items-center justify-center px-6 py-14 text-center">
  <span class="mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-forest-50 text-forest-600">
    {#if icon}{@render icon()}{:else}<FolderOpen size={24} />{/if}
  </span>
  <h3 class="font-display text-lg font-semibold text-forest-900">{title}</h3>
  {#if description}<p class="mt-1.5 max-w-md text-sm leading-relaxed text-ink-muted">{description}</p>{/if}
  {#if children}<div class="mt-5 flex flex-wrap items-center justify-center gap-2">{@render children()}</div>{/if}
</div>
