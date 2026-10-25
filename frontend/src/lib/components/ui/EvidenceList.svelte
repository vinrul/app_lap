<script lang="ts">
  import { AlertCircle, Clock3, Download, Eye, FileText } from 'lucide-svelte'
  import { evidencePreview } from '../../stores/evidencePreview.svelte'
  import type { EvidenceItem } from '../../types'
  import { formatBytes } from '../../utils/format'

  type Props = { items: EvidenceItem[] }
  let { items }: Props = $props()

  const readyItems = $derived(items.filter((item) => item.status === 'ready'))
</script>

<div class="space-y-1.5">
  {#if readyItems.length > 1}
    <button
      type="button"
      class="mb-1 inline-flex items-center gap-1.5 rounded-lg border border-forest-200 bg-forest-50 px-2.5 py-1.5 text-[10px] font-semibold text-forest-700 hover:bg-forest-100"
      onclick={() => evidencePreview.open(readyItems[0], readyItems)}
    >
      <Eye size={12} /> Pratinjau semua ({readyItems.length})
    </button>
  {/if}
  {#each items as item}
    {#if item.status === 'ready'}
      <div class="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-lg border border-line bg-white px-2.5 py-1.5" data-evidence-ready={item.id}>
        <span class="flex min-w-0 flex-1 items-center gap-1.5 text-[11px] font-medium text-ink">
          <FileText size={12} class="shrink-0 text-forest-600" />
          <span class="truncate">{item.filename || item.source_url}</span>
          {#if item.size_bytes}<span class="shrink-0 text-[10px] font-normal text-ink-muted">({formatBytes(item.size_bytes)})</span>{/if}
        </span>
        <div class="flex shrink-0 items-center gap-1">
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] font-semibold text-forest-700 hover:bg-forest-50"
            title="Pratinjau langsung"
            data-evidence-preview={item.id}
            onclick={() => evidencePreview.open(item, readyItems)}
          >
            <Eye size={12} /> Pratinjau
          </button>
          <a
            class="inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] font-semibold text-forest-700 hover:bg-forest-50"
            href={item.url}
            download
            title="Unduh berkas"
            data-evidence-download={item.id}
          >
            <Download size={12} /> Unduh
          </a>
        </div>
      </div>
    {:else if item.status === 'pending'}
      <p class="flex items-center gap-1.5 truncate text-[11px] text-ink-muted">
        <Clock3 size={12} class="shrink-0" />
        <span class="truncate">{item.filename || item.source_url}</span>
        <span class="shrink-0 text-[10px]">· menunggu unduhan</span>
      </p>
    {:else}
      <div class="flex items-start gap-1.5 text-[11px] text-gold-700">
        <AlertCircle size={12} class="mt-0.5 shrink-0" />
        <span class="min-w-0">
          <a class="truncate font-medium hover:underline" href={item.source_url} target="_blank" rel="noopener noreferrer">{item.filename || item.source_url}</a>
          {#if item.error}<span class="block text-[10px] text-ink-muted">{item.error}</span>{/if}
        </span>
      </div>
    {/if}
  {/each}
</div>
