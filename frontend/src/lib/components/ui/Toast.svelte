<script lang="ts">
  import { CheckCircle2, Info, XCircle, X } from 'lucide-svelte'
  import { toast } from '../../stores/toast.svelte'

  const icons = {
    success: CheckCircle2,
    error: XCircle,
    info: Info,
  }

  const tones = {
    success: 'border-forest-200 bg-white text-forest-800',
    error: 'border-red-200 bg-white text-red-800',
    info: 'border-stone-200 bg-white text-ink',
  }
</script>

{#if toast.message}
  {@const Icon = icons[toast.tone]}
  <div class="animate-fade-up fixed right-4 bottom-4 z-[60] max-w-sm" role="status">
    <div class="flex items-start gap-3 rounded-2xl border px-4 py-3.5 shadow-lift {tones[toast.tone]}">
      <Icon size={18} class="mt-0.5 shrink-0" />
      <p class="text-sm leading-relaxed">{toast.message}</p>
      <button class="btn-icon ml-auto h-7 w-7 shrink-0" aria-label="Tutup notifikasi" onclick={() => toast.dismiss()}>
        <X size={14} />
      </button>
    </div>
  </div>
{/if}
