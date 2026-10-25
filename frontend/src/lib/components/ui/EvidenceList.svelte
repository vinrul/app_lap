<script lang="ts">
  import { AlertCircle, Clock3, Download } from 'lucide-svelte'
  import type { EvidenceItem } from '../../types'
  import { formatBytes } from '../../utils/format'

  type Props = { items: EvidenceItem[] }
  let { items }: Props = $props()
</script>

<div class="space-y-1.5">
  {#each items as item}
    {#if item.status === 'ready'}
      <a
        class="flex items-center gap-1.5 truncate text-[11px] font-medium text-forest-700 hover:underline"
        href={item.url}
        download
      >
        <Download size={12} class="shrink-0" />
        <span class="truncate">{item.filename || item.source_url}</span>
        {#if item.size_bytes}<span class="shrink-0 text-[10px] font-normal text-ink-muted">({formatBytes(item.size_bytes)})</span>{/if}
      </a>
    {:else if item.status === 'pending'}
      <p class="flex items-center gap-1.5 truncate text-[11px] text-ink-muted">
        <Clock3 size={12} class="shrink-0" />
        <span class="truncate">{item.source_url}</span>
        <span class="shrink-0 text-[10px]">· menunggu unduhan</span>
      </p>
    {:else}
      <div class="flex items-start gap-1.5 text-[11px] text-gold-700">
        <AlertCircle size={12} class="mt-0.5 shrink-0" />
        <span class="min-w-0">
          <a class="truncate font-medium hover:underline" href={item.source_url} target="_blank" rel="noopener noreferrer">{item.source_url}</a>
          {#if item.error}<span class="block text-[10px] text-ink-muted">{item.error}</span>{/if}
        </span>
      </div>
    {/if}
  {/each}
</div>
