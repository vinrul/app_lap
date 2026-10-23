<script lang="ts">
  import type { Snippet } from 'svelte'
  import LoaderCircle from 'lucide-svelte/icons/loader-circle'

  type Props = {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
    size?: 'md' | 'sm'
    type?: 'button' | 'submit' | 'reset'
    disabled?: boolean
    loading?: boolean
    onclick?: (event: MouseEvent) => void
    children?: Snippet
    class?: string
  }

  let {
    variant = 'primary',
    size = 'md',
    type = 'button',
    disabled = false,
    loading = false,
    onclick,
    children,
    class: className = '',
  }: Props = $props()

  const variants = {
    primary: 'btn-primary',
    secondary: 'btn-secondary',
    ghost: 'btn-ghost',
    danger: 'btn-danger',
  }
</script>

<button
  {type}
  class="btn {variants[variant]} {size === 'sm' ? 'btn-sm' : ''} {className}"
  disabled={disabled || loading}
  {onclick}
>
  {#if loading}<LoaderCircle size={16} class="animate-spin" />{/if}
  {@render children?.()}
</button>
