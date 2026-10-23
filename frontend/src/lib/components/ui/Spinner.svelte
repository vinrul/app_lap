<script lang="ts">
  import LoaderCircle from 'lucide-svelte/icons/loader-circle'
  type Props = { size?: number; label?: string }
  let { size = 18, label = 'Memuat' }: Props = $props()
</script>

<span class="inline-flex items-center gap-2 text-sm text-ink-muted" role="status">
  <LoaderCircle {size} class="animate-spin" />
  {label}
</span>
