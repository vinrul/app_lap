<script lang="ts">
  import {
    CalendarDays,
    LayoutDashboard,
    LogOut,
    ShieldCheck,
    Users,
    X,
  } from 'lucide-svelte'
  import type { User } from '../../types'
  import { navigate } from '../../stores/router.svelte'
  import { auth } from '../../stores/auth.svelte'
  import { meta } from '../../stores/meta.svelte'

  type Props = {
    user: User
    current: string
    mobileOpen?: boolean
    onclose: () => void
  }

  let {
    user,
    current,
    mobileOpen = false,
    onclose,
  }: Props = $props()

  const icons = {
    dashboard: LayoutDashboard,
    periods: CalendarDays,
    users: Users,
  }

  const items = $derived([
    { id: 'dashboard', label: 'Ringkasan', path: '/dashboard' },
    { id: 'periods', label: 'Periode laporan', path: '/periods' },
    ...(user.role === 'admin' ? [{ id: 'users', label: 'Pengguna & akses', path: '/users' }] : []),
  ])

  const isActive = (id: string) =>
    current === id || (id === 'periods' && current === 'period')

  async function signOut() {
    await auth.logout()
    navigate('/dashboard')
  }
</script>

<aside
  class="fixed inset-y-0 left-0 z-40 flex w-72 flex-col bg-forest-950 text-white transition-transform duration-200 lg:w-64 lg:translate-x-0 {mobileOpen
    ? 'translate-x-0'
    : '-translate-x-full'}"
  aria-label="Navigasi utama"
>
  <div class="flex items-center justify-between px-5 py-5">
    <button class="flex items-center gap-3 text-left" onclick={() => { navigate('/dashboard'); onclose() }}>
      <img
        src="/logo_polda_bali.png"
        alt="Logo Polda Bali"
        class="h-10 w-10 shrink-0 rounded-xl border border-gold-400/40 bg-white object-contain p-1"
      />
      <span>
        <strong class="block text-sm font-bold tracking-[0.12em]">{meta.data?.polda_name ?? 'POLDA BALI'}</strong>
        <small class="block text-[9px] tracking-[0.2em] text-forest-200/70">LAPORAN MINGGUAN PBM</small>
      </span>
    </button>
    <button class="btn-icon text-forest-100 hover:bg-white/10 lg:hidden" aria-label="Tutup navigasi" onclick={onclose}>
      <X size={18} />
    </button>
  </div>

  <div class="px-5 pt-3 pb-2 text-[9px] font-bold tracking-[0.22em] text-forest-200/50">RUANG KERJA</div>

  <nav class="flex-1 space-y-1 px-3">
    {#each items as item}
      {@const Icon = icons[item.id as keyof typeof icons]}
      <button
        class="nav-item {isActive(item.id) ? 'nav-item-active' : ''}"
        onclick={() => { navigate(item.path); onclose() }}
      >
        <Icon size={18} strokeWidth={1.8} />
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>

  <div class="mx-4 mb-4 rounded-xl border border-white/10 bg-white/5 p-3.5">
    <div class="flex items-center gap-2 text-[11px] font-semibold text-forest-100">
      <span class="relative flex h-2 w-2">
        <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-300 opacity-60"></span>
        <span class="relative inline-flex h-2 w-2 rounded-full bg-emerald-300"></span>
      </span>
      Mode lokal aktif
    </div>
    <p class="mt-1 text-[10px] leading-relaxed text-forest-200/60">Data tersimpan di perangkat ini dan tidak dikirim ke layanan eksternal.</p>
  </div>

  <div class="flex items-center gap-3 border-t border-white/10 px-4 py-4">
    <span class="flex h-9 w-9 items-center justify-center rounded-full bg-gold-400 font-display text-sm font-bold text-forest-950">
      {user.username.slice(0, 1).toUpperCase()}
    </span>
    <span class="min-w-0 flex-1">
      <strong class="block truncate text-xs font-semibold">{user.username}</strong>
      <small class="block text-[10px] text-forest-200/65">{user.role === 'admin' ? 'Administrator' : 'Operator'}</small>
    </span>
    <button class="btn-icon text-forest-100 hover:bg-white/10" aria-label="Keluar" title="Keluar" onclick={signOut}>
      <LogOut size={16} />
    </button>
  </div>
</aside>
