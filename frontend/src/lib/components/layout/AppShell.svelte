<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { User } from '../../types'
  import Sidebar from './Sidebar.svelte'
  import Topbar from './Topbar.svelte'
  import { meta } from '../../stores/meta.svelte'

  type Props = {
    user: User
    routeName: string
    title: string
    children?: Snippet
  }

  let {
    user,
    routeName,
    title,
    children,
  }: Props = $props()

  let mobileOpen = $state(false)
</script>

<div class="min-h-screen bg-canvas">
  <Sidebar {user} current={routeName} {mobileOpen} onclose={() => (mobileOpen = false)} />
  {#if mobileOpen}
    <button
      class="fixed inset-0 z-30 bg-forest-950/40 backdrop-blur-[1px] lg:hidden"
      aria-label="Tutup navigasi"
      onclick={() => (mobileOpen = false)}
    ></button>
  {/if}

  <div class="lg:pl-64">
    <Topbar {title} onmenu={() => (mobileOpen = true)} />
    <main class="mx-auto w-full max-w-[1440px] px-4 py-6 sm:px-6 lg:px-8 lg:py-8">
      <div class="animate-fade-in">
        {@render children?.()}
      </div>
      <footer class="mt-12 flex flex-wrap items-center justify-between gap-2 border-t border-line pt-5 text-[10px] font-semibold tracking-[0.14em] text-ink-muted uppercase">
        <span>Laporan Mingguan PBM · {meta.data?.polda_name ?? 'Polda Bali'}</span>
        <span>Data lokal · Diverifikasi operator</span>
      </footer>
    </main>
  </div>
</div>
