<script lang="ts">
  import type { Snippet } from 'svelte'

  type Props = {
    tone?: 'neutral' | 'progress' | 'success' | 'danger'
    children?: Snippet
  }

  let { tone = 'neutral', children }: Props = $props()

  const tones = {
    neutral: 'badge-neutral',
    progress: 'badge-progress',
    success: 'badge-success',
    danger: 'badge-danger',
  }
</script>

<span class="badge {tones[tone]}">
  <span class="h-1.5 w-1.5 rounded-full bg-current opacity-75"></span>
  {@render children?.()}
</span>
