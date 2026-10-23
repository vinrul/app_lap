<script lang="ts">
  import { AlertCircle, ArrowRight, BarChart3, FileSpreadsheet, LockKeyhole, ShieldCheck } from 'lucide-svelte'
  import { auth } from '../../lib/stores/auth.svelte'
  import { navigate } from '../../lib/stores/router.svelte'
  import { meta } from '../../lib/stores/meta.svelte'
  import Button from '../../lib/components/ui/Button.svelte'

  let username = $state('')
  let password = $state('')
  let errorMessage = $state('')

  async function submit(event: SubmitEvent) {
    event.preventDefault()
    errorMessage = ''
    try {
      await auth.login(username, password)
      navigate('/dashboard')
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : auth.error
    }
  }
</script>

<svelte:head>
  <title>Masuk · Laporan Mingguan PBM</title>
</svelte:head>

<main class="grid min-h-screen bg-canvas lg:grid-cols-[1.05fr_1fr]">
  <section class="relative hidden overflow-hidden bg-forest-950 p-12 text-white lg:flex lg:flex-col">
    <div class="absolute -top-32 -right-32 h-96 w-96 rounded-full bg-forest-700/40 blur-3xl"></div>
    <div class="absolute -bottom-40 -left-24 h-96 w-96 rounded-full bg-gold-500/10 blur-3xl"></div>

    <div class="relative flex items-center gap-3">
      <img
        src="/logo_polda_bali.png"
        alt="Logo Polda Bali"
        class="h-11 w-11 shrink-0 rounded-xl border border-gold-400/40 bg-white object-contain p-1"
      />
      <div>
        <strong class="block text-sm font-bold tracking-[0.16em]">{meta.data?.polda_name ?? 'POLDA BALI'}</strong>
        <small class="block text-[9px] tracking-[0.24em] text-forest-200/65">LAPORAN MINGGUAN PBM</small>
      </div>
    </div>

    <div class="relative mt-auto max-w-xl">
      <span class="text-[10px] font-bold tracking-[0.24em] text-gold-300 uppercase">Ruang kerja anev mingguan</span>
      <h1 class="mt-4 font-display text-5xl leading-[1.06] font-medium text-balance">
        Dari rekap Excel<br />menjadi laporan<br /><em class="text-gold-200">yang siap dibaca.</em>
      </h1>
      <p class="mt-5 max-w-md text-sm leading-relaxed text-forest-100/75">
        Impor workbook Quick Wins, tinjau angka dan narasi, lalu terbitkan laporan PDF resmi dalam satu alur kerja.
      </p>

      <div class="mt-9 grid max-w-lg gap-3">
        <div class="flex items-start gap-3 rounded-xl border border-white/10 bg-white/5 p-3.5">
          <FileSpreadsheet size={18} class="mt-0.5 text-gold-300" />
          <div><strong class="text-xs">Validasi 18 kolom sumber</strong><p class="mt-0.5 text-[11px] leading-relaxed text-forest-200/65">Header dipetakan berdasarkan nama, bukan posisi kolom.</p></div>
        </div>
        <div class="flex items-start gap-3 rounded-xl border border-white/10 bg-white/5 p-3.5">
          <BarChart3 size={18} class="mt-0.5 text-gold-300" />
          <div><strong class="text-xs">Metrik dihitung dari isian laporan</strong><p class="mt-0.5 text-[11px] leading-relaxed text-forest-200/65">Bibit, jemaah, lokasi, aspirasi, dan publikasi direkap otomatis.</p></div>
        </div>
        <div class="flex items-start gap-3 rounded-xl border border-white/10 bg-white/5 p-3.5">
          <ShieldCheck size={18} class="mt-0.5 text-gold-300" />
          <div><strong class="text-xs">Data tetap di perangkat</strong><p class="mt-0.5 text-[11px] leading-relaxed text-forest-200/65">Workbook, draft, dan PDF disimpan pada penyimpanan lokal.</p></div>
        </div>
      </div>
    </div>

    <div class="relative mt-auto flex items-center gap-2 pt-10 text-[10px] tracking-[0.14em] text-forest-200/50 uppercase">
      <LockKeyhole size={12} /> Akses internal · koneksi loopback
    </div>
  </section>

  <section class="flex items-center justify-center p-6 sm:p-10">
    <div class="w-full max-w-sm">
      <div class="mb-8 lg:hidden">
        <img
          src="/logo_polda_bali.png"
          alt="Logo Polda Bali"
          class="h-11 w-11 rounded-xl bg-white object-contain p-1 ring-1 ring-stone-200"
        />
        <h1 class="mt-4 font-display text-2xl font-semibold text-forest-900">Laporan Mingguan PBM</h1>
        <p class="mt-1 text-sm text-ink-muted">{meta.data?.polda_name ?? 'Polda Bali'}</p>
      </div>

      <span class="eyebrow">MASUK KE APLIKASI</span>
      <h2 class="mt-2 font-display text-3xl font-semibold text-forest-900">Selamat datang</h2>
      <p class="mt-2 text-sm text-ink-muted">Gunakan akun internal untuk melanjutkan ke ruang kerja laporan.</p>

      {#if errorMessage}
        <div class="mt-6 flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800" role="alert">
          <AlertCircle size={17} class="mt-0.5 shrink-0" />
          <span>{errorMessage}</span>
        </div>
      {/if}

      <form class="mt-7 space-y-4" onsubmit={submit}>
        <div>
          <label class="label" for="username">Nama pengguna</label>
          <input id="username" class="input" bind:value={username} autocomplete="username" required />
        </div>
        <div>
          <label class="label" for="password">Kata sandi</label>
          <input id="password" class="input" type="password" bind:value={password} autocomplete="current-password" required />
        </div>
        <Button type="submit" loading={auth.loading} class="w-full">
          Masuk ke ruang kerja <ArrowRight size={16} />
        </Button>
      </form>

      <div class="mt-8 rounded-xl border border-line bg-white px-4 py-3 text-[11px] leading-relaxed text-ink-muted">
        <strong class="text-ink-soft">Belum ada akun?</strong> Hubungi administrator untuk account

      </div>
    </div>
  </section>
</main>
