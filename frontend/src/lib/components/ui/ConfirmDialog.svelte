<script lang="ts">
  import { tick } from 'svelte'
  import { AlertTriangle, Info } from 'lucide-svelte'
  import { dialog, type PromptOptions } from '../../stores/dialog.svelte'
  import Button from './Button.svelte'
  import Modal from './Modal.svelte'

  let inputEl = $state<HTMLInputElement | null>(null)

  const request = $derived(dialog.request)
  const prompt: PromptOptions | null = $derived(
    request?.kind === 'prompt' ? request.options : null,
  )
  const tone = $derived(request?.options.tone ?? 'default')

  $effect(() => {
    if (request?.kind !== 'prompt') return
    void tick().then(() => inputEl?.focus())
  })

  function portal(node: HTMLElement): { destroy: () => void } {
    document.body.appendChild(node)
    return { destroy: () => node.remove() }
  }
</script>

{#snippet actions()}
  <span data-dialog-cancel>
    <Button variant="secondary" size="sm" onclick={() => dialog.cancel()}>
      {request?.options.cancelLabel ?? 'Batal'}
    </Button>
  </span>
  <span data-dialog-confirm>
    <Button
      variant={tone === 'danger' ? 'danger' : 'primary'}
      size="sm"
      disabled={prompt ? prompt.required !== false && !dialog.value.trim() : false}
      onclick={() => dialog.accept()}
    >
      {request?.options.confirmLabel ?? (prompt ? 'Simpan' : 'Lanjutkan')}
    </Button>
  </span>
{/snippet}

{#if request}
  <div use:portal>
    <Modal
      open={true}
      title={request.options.title}
      description={request.options.message}
      onclose={() => dialog.cancel()}
      width="max-w-md"
      footer={actions}
    >
      {#if prompt}
        <label class="block">
          <span class="label">{prompt.inputLabel ?? 'Nilai'}</span>
          <input
            bind:this={inputEl}
            class="input py-2 text-sm"
            type={prompt.inputType ?? 'text'}
            placeholder={prompt.inputPlaceholder ?? ''}
            value={dialog.value}
            data-dialog-input
            oninput={(event) => dialog.update((event.currentTarget as HTMLInputElement).value)}
            onkeydown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                dialog.accept()
              }
            }}
          />
        </label>
        {#if prompt.inputHint}
          <div class="mt-3 flex items-start gap-2 rounded-xl border border-line bg-stone-50 px-3 py-2.5 text-[11px] leading-relaxed text-ink-muted">
            <Info size={14} class="mt-0.5 shrink-0 text-forest-600" />
            <span>{prompt.inputHint}</span>
          </div>
        {/if}
      {:else if tone === 'danger'}
        <div class="flex items-start gap-3 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-xs leading-relaxed text-red-800">
          <AlertTriangle size={17} class="mt-0.5 shrink-0" />
          <span>Tindakan ini dapat mengubah atau menghapus data. Lanjutkan hanya jika Anda yakin.</span>
        </div>
      {/if}
    </Modal>
  </div>
{/if}
