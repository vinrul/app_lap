<script lang="ts">
  import { Bell, CalendarDays, Menu, ShieldCheck } from 'lucide-svelte'
  import { notifications } from '../../stores/notifications.svelte'
  import { navigate } from '../../stores/router.svelte'
  import { formatDateTime } from '../../utils/format'

  type Props = {
    title: string
    onmenu: () => void
  }

  let { title, onmenu }: Props = $props()

  let open = $state(false)
  const today = new Intl.DateTimeFormat('id-ID', { dateStyle: 'full' }).format(new Date())

  function openItem(item: (typeof notifications.items)[number]): void {
    open = false
    void notifications.markRead(item.id)
    if (item.url) navigate(item.url.replace(/^\/?#/, '/'))
  }
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
      <div class="relative">
        <button
          class="btn-icon relative"
          aria-label="Notifikasi"
          aria-expanded={open}
          onclick={() => (open = !open)}
        >
          <Bell size={18} />
          {#if notifications.unread > 0}
            <span
              class="absolute -top-0.5 -right-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-600 px-1 text-[9px] font-bold text-white"
            >
              {notifications.unread > 99 ? '99+' : notifications.unread}
            </span>
          {/if}
        </button>
        {#if open}
          <button
            class="fixed inset-0 z-40 cursor-default"
            aria-label="Tutup notifikasi"
            onclick={() => (open = false)}
          ></button>
          <div
            class="absolute top-11 right-0 z-50 w-80 overflow-hidden rounded-xl border border-line bg-white shadow-lift"
          >
            <div class="flex items-center justify-between border-b border-line px-3 py-2">
              <span class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">
                Notifikasi
              </span>
              <button
                class="text-[10px] font-semibold text-forest-700 hover:underline"
                onclick={() => notifications.markAll()}
              >
                Tandai dibaca
              </button>
            </div>
            <div class="max-h-96 overflow-y-auto">
              {#if notifications.items.length}
                {#each notifications.items as item (item.id)}
                  <button
                    class="block w-full border-b border-line/60 px-3 py-2 text-left hover:bg-stone-50"
                    onclick={() => openItem(item)}
                  >
                    <div class="flex items-start justify-between gap-2">
                      <span class="text-xs font-semibold text-ink">{item.title}</span>
                      {#if !item.read_at}
                        <span class="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-forest-600"></span>
                      {/if}
                    </div>
                    <p class="mt-0.5 text-[11px] text-ink-muted">{item.body}</p>
                    <p class="mt-0.5 text-[9px] text-ink-muted">
                      {formatDateTime(item.created_at)}
                    </p>
                  </button>
                {/each}
              {:else}
                <p class="px-3 py-6 text-center text-[11px] text-ink-muted">
                  Belum ada notifikasi.
                </p>
              {/if}
            </div>
          </div>
        {/if}
      </div>
      <span
        class="flex items-center gap-1.5 rounded-full border border-forest-200 bg-white px-2.5 py-1 text-[10px] font-semibold text-forest-700"
      >
        <ShieldCheck size={12} />
        Lokal
      </span>
    </div>
  </div>
</header>
