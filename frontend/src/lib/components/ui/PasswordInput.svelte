<script lang="ts">
  import { Eye, EyeOff } from 'lucide-svelte'

  type Props = {
    id?: string
    value?: string
    autocomplete?: 'current-password' | 'new-password' | 'off'
    placeholder?: string
    required?: boolean
    disabled?: boolean
    maxlength?: number
    ariaLabel?: string
    class?: string
  }

  let {
    id = '',
    value = $bindable(''),
    autocomplete = 'current-password',
    placeholder = '',
    required = false,
    disabled = false,
    maxlength = undefined,
    ariaLabel = 'Kata sandi',
    class: className = '',
  }: Props = $props()

  let visible = $state(false)
</script>

<div class="relative">
  <input
    id={id || undefined}
    class="input pr-11 {className}"
    type={visible ? 'text' : 'password'}
    bind:value
    {autocomplete}
    {placeholder}
    {required}
    {disabled}
    {maxlength}
    aria-label={ariaLabel}
  />
  <button
    type="button"
    class="absolute inset-y-0 right-0 flex w-11 items-center justify-center rounded-r-xl text-ink-muted transition-colors hover:text-forest-800 focus-visible:ring-2 focus-visible:ring-forest-500/40 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-40"
    aria-label={visible ? 'Sembunyikan kata sandi' : 'Tampilkan kata sandi'}
    aria-pressed={visible}
    title={visible ? 'Sembunyikan kata sandi' : 'Tampilkan kata sandi'}
    {disabled}
    onclick={() => (visible = !visible)}
  >
    {#if visible}
      <EyeOff size={17} />
    {:else}
      <Eye size={17} />
    {/if}
  </button>
</div>
