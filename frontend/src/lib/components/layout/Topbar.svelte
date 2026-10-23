<script lang="ts">
  import { CalendarDays, Menu, ShieldCheck } from 'lucide-svelte'

  type Props = {
    title: string
    onmenu: () => void
  }

  let {
    title,
    onmenu,
  }: Props = $props()

  const today = new Intl.DateTimeFormat('id-ID', { dateStyle: 'full' }).format(new Date())
</script>

<header class="sticky top-0 z-30 border-b border-line/80 bg-canvas/85 backdrop-blur-md">
  <div class="flex h-16 items-center gap-3 px-4 sm:px-6 lg:px-8">
    <button class="btn-icon lg:hidden" aria-label="Buka navigasi" onclick={onmenu}>
      <Menu size={20} />
    </button>
    <div class="flex min-w-0 items-center gap-2 text-xs text-ink-muted">
      <span class="hidden sm:inline">Ruang kerja</span>
      <span class="hidden text-stone-300 sm:inline">/</span>
      <strong class="truncate font-semibold text-ink">{title}</strong>
    </div>
    <div class="ml-auto flex items-center gap-3">
      <span class="hidden items-center gap-1.5 text-xs text-ink-muted md:flex">
        <CalendarDays size={14} />
        {today}
      </span>
      <span class="hidden h-5 w-px bg-line md:block"></span>
      <span class="flex items-center gap-1.5 rounded-full border border-forest-200 bg-white px-2.5 py-1 text-[10px] font-semibold text-forest-700">
        <ShieldCheck size={12} />
        Lokal
      </span>
    </div>
  </div>
</header>
