<script lang="ts">
  import { onMount } from 'svelte'
  import { Activity, ShieldAlert } from 'lucide-svelte'
  import { auth } from './lib/stores/auth.svelte'
  import { meta } from './lib/stores/meta.svelte'
  import { router, navigate } from './lib/stores/router.svelte'
  import { toast } from './lib/stores/toast.svelte'
  import LoginPage from './routes/login/LoginPage.svelte'
  import DashboardPage from './routes/dashboard/DashboardPage.svelte'
  import PeriodsPage from './routes/periods/PeriodsPage.svelte'
  import PeriodDetailPage from './routes/periods/PeriodDetailPage.svelte'
  import UsersPage from './routes/users/UsersPage.svelte'
  import AppShell from './lib/components/layout/AppShell.svelte'
  import Toast from './lib/components/ui/Toast.svelte'
  import ConfirmDialog from './lib/components/ui/ConfirmDialog.svelte'
  import Button from './lib/components/ui/Button.svelte'
  import EmptyState from './lib/components/ui/EmptyState.svelte'

  let bootError = $state('')

  onMount(() => {
    window.addEventListener('hashchange', router.sync)
    window.addEventListener('api:unauthorized', handleUnauthorized)
    void meta.load()
    void auth.bootstrap().catch(() => {
      bootError = 'Aplikasi tidak dapat memuat sesi.'
    })
    return () => {
      window.removeEventListener('hashchange', router.sync)
      window.removeEventListener('api:unauthorized', handleUnauthorized)
    }
  })

  function handleUnauthorized() {
    if (auth.user) {
      auth.clear()
      toast.error('Sesi berakhir. Silakan masuk kembali.')
    }
  }

  const title = $derived(
    router.route.name === 'dashboard'
      ? 'Ringkasan'
      : router.route.name === 'periods'
        ? 'Periode laporan'
        : router.route.name === 'period'
          ? 'Detail periode'
          : 'Pengguna & akses',
  )
</script>

<svelte:head>
  <title>{meta.data?.app_name ?? 'Laporan Mingguan PBM'} · {meta.data?.polda_name ?? 'Polda Bali'}</title>
  <meta name="description" content="Kelola impor data dan laporan mingguan PBM Polda Bali." />
</svelte:head>

{#if auth.loading}
  <main class="flex min-h-screen flex-col items-center justify-center gap-4 bg-forest-950 text-white">
    <span class="flex h-14 w-14 items-center justify-center rounded-2xl border border-gold-400/40 bg-white/5 text-gold-200">
      <Activity size={24} />
    </span>
    <div class="text-center">
      <p class="text-sm font-semibold tracking-[0.16em] text-gold-200">LAPORAN MINGGUAN PBM</p>
      <p class="mt-1 text-xs text-forest-200/60">Menyiapkan ruang kerja laporan…</p>
    </div>
  </main>
{:else if !auth.user}
  <LoginPage />
{:else}
  {#if router.route.name === 'users' && auth.user.role !== 'admin'}
    <AppShell user={auth.user} routeName={router.route.name} title="Akses ditolak">
      <div class="card">
        <EmptyState
          title="Akses administrator diperlukan"
          description="Halaman pengguna hanya dapat dibuka oleh administrator aktif."
        >
          <Button onclick={() => navigate('/dashboard')}>Kembali ke ringkasan</Button>
        </EmptyState>
      </div>
    </AppShell>
  {:else}
    <AppShell user={auth.user} routeName={router.route.name} {title}>
      {#if router.route.name === 'dashboard'}
        <DashboardPage user={auth.user} />
      {:else if router.route.name === 'periods'}
        <PeriodsPage />
      {:else if router.route.name === 'period' && router.route.id}
        {#key router.route.id}
          <PeriodDetailPage periodId={router.route.id} tab={router.route.tab} />
        {/key}
      {:else if router.route.name === 'users'}
        <UsersPage user={auth.user} />
      {:else}
        <div class="card">
          <EmptyState title="Halaman tidak ditemukan" description="Navigasi kembali ke ringkasan untuk melanjutkan.">
            <Button onclick={() => navigate('/dashboard')}>Kembali ke ringkasan</Button>
          </EmptyState>
        </div>
      {/if}
    </AppShell>
  {/if}
{/if}

{#if bootError}
  <div class="fixed bottom-4 left-4 z-[60] flex items-center gap-2 rounded-xl border border-red-200 bg-white px-4 py-3 text-sm text-red-800 shadow-lift">
    <ShieldAlert size={17} /> {bootError}
  </div>
{/if}

<Toast />
<ConfirmDialog />
