<script lang="ts">
  import { Download, ExternalLink, FileText, X } from 'lucide-svelte'
  import { evidencePreview } from '../../stores/evidencePreview.svelte'
  import { formatBytes } from '../../utils/format'

  const item = $derived(evidencePreview.item)
  const kind = $derived.by(() => {
    if (!item) return 'other'
    const type = item.content_type.toLowerCase()
    const filename = item.filename.toLowerCase()
    if (type.startsWith('image/') || /\.(png|jpe?g|webp|gif|bmp|avif)$/.test(filename)) {
      return 'image'
    }
    if (type.startsWith('application/pdf') || filename.endsWith('.pdf')) return 'pdf'
    return 'other'
  })
  const typeLabel = $derived.by(() => {
    if (!item) return ''
    if (kind === 'pdf') return 'PDF'
    if (kind === 'image') {
      const subtype = item.content_type.split('/')[1]?.toUpperCase()
      return subtype ? `Gambar ${subtype}` : 'Gambar'
    }
    return item.content_type || 'Berkas'
  })

  $effect(() => {
    if (!item) return
    const handler = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.preventDefault()
      event.stopImmediatePropagation()
      evidencePreview.close()
    }
    window.addEventListener('keydown', handler, true)
    return () => window.removeEventListener('keydown', handler, true)
  })
</script>

{#if item}
  <div
    class="fixed inset-0 z-[80] flex items-center justify-center p-2 sm:p-5"
    role="dialog"
    aria-modal="true"
    aria-label={`Pratinjau ${item.filename || 'bukti dukung'}`}
  >
    <button
      type="button"
      class="absolute inset-0 h-full w-full cursor-default bg-forest-950/85 backdrop-blur-sm"
      aria-label="Tutup pratinjau"
      tabindex="-1"
      onclick={() => evidencePreview.close()}
    ></button>

    <div class="relative z-10 flex h-full max-h-[94vh] w-full max-w-6xl flex-col overflow-hidden rounded-2xl bg-white shadow-2xl">
      <header class="flex flex-wrap items-center justify-between gap-3 border-b border-line px-4 py-3 sm:px-5">
        <div class="min-w-0">
          <p class="truncate text-sm font-semibold text-ink">{item.filename || 'Bukti dukung'}</p>
          <p class="mt-0.5 text-[10px] text-ink-muted">
            {typeLabel}{#if item.size_bytes} · {formatBytes(item.size_bytes)}{/if}
          </p>
        </div>
        <div class="flex shrink-0 flex-wrap items-center gap-2">
          <a class="btn btn-secondary btn-sm" href={item.url} target="_blank" rel="noopener">
            <ExternalLink size={14} /> Buka tab baru
          </a>
          <a class="btn btn-secondary btn-sm" href={item.url} download>
            <Download size={14} /> Unduh
          </a>
          <button type="button" class="btn-icon" aria-label="Tutup pratinjau" onclick={() => evidencePreview.close()}>
            <X size={18} />
          </button>
        </div>
      </header>

      <div class="min-h-0 flex-1 bg-stone-900">
        {#if kind === 'image'}
          <div class="flex h-full w-full items-center justify-center p-2 sm:p-4">
            <img
              class="max-h-full max-w-full object-contain"
              src={item.url}
              alt={item.filename || 'Bukti dukung'}
            />
          </div>
        {:else if kind === 'pdf'}
          <iframe
            class="h-full w-full border-0 bg-white"
            src={item.url}
            title={`Pratinjau ${item.filename || 'PDF bukti dukung'}`}
          ></iframe>
        {:else}
          <div class="flex h-full flex-col items-center justify-center gap-3 px-6 text-center text-white">
            <span class="flex h-16 w-16 items-center justify-center rounded-2xl bg-white/10">
              <FileText size={28} />
            </span>
            <div>
              <p class="font-semibold">Pratinjau tidak tersedia</p>
              <p class="mt-1 text-sm text-white/60">Berkas ini tidak dapat ditampilkan langsung di aplikasi.</p>
            </div>
            <a class="btn btn-secondary btn-sm" href={item.url} target="_blank" rel="noopener">
              <ExternalLink size={14} /> Buka berkas
            </a>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
