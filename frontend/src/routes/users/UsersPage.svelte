<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertCircle, Check, Plus, ShieldCheck, UserPlus, X } from 'lucide-svelte'
  import { usersApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { AdminSession, AdminUser, User, UserOptions } from '../../lib/types'
  import { formatDateTime } from '../../lib/utils/format'
  import { auth } from '../../lib/stores/auth.svelte'
  import { dialog } from '../../lib/stores/dialog.svelte'
  import { toast } from '../../lib/stores/toast.svelte'
  import PageHeader from '../../lib/components/ui/PageHeader.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import Modal from '../../lib/components/ui/Modal.svelte'
  import EmptyState from '../../lib/components/ui/EmptyState.svelte'
  import PasswordInput from '../../lib/components/ui/PasswordInput.svelte'

  type Props = { user: User }

  let { user }: Props = $props()

  let users = $state<AdminUser[]>([])
  let options = $state<UserOptions>({ polres: [], satker: [] })
  let loading = $state(true)
  let error = $state('')
  let showForm = $state(false)
  let busyId = $state('')

  let form = $state({ username: '', password: '', role: 'operator', polres: '', satker: '' })
  let formLoading = $state(false)
  let formError = $state('')

  let editing = $state<AdminUser | null>(null)
  let editForm = $state({ role: 'operator', polres: '', satker: '', password: '' })
  let editLoading = $state(false)
  let editError = $state('')

  let sessions = $state<AdminSession[]>([])
  let sessionsLoading = $state(false)
  let revokingSession = $state<number | null>(null)

  function roleLabel(role: string): string {
    switch (role) {
      case 'admin':
        return 'Administrator'
      case 'operator':
        return 'Operator'
      case 'polres':
        return 'Polres'
      case 'polda':
        return 'Polda'
      default:
        return role
    }
  }

  function openEdit(account: AdminUser) {
    editing = account
    editError = ''
    editForm = { role: account.role, polres: account.polres, satker: account.satker, password: '' }
  }

  onMount(load)

  async function load() {
    loading = true
    error = ''
    try {
      const [accounts, unitOptions] = await Promise.all([usersApi.list(), usersApi.options()])
      users = accounts
      options = unitOptions
      void loadSessions()
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  async function loadSessions(): Promise<void> {
    sessionsLoading = true
    try {
      sessions = await usersApi.sessions()
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      sessionsLoading = false
    }
  }

  function sessionOnline(value: string): boolean {
    if (!value) return false
    const seen = Date.parse(value)
    return Number.isFinite(seen) && Date.now() - seen < 120_000
  }

  async function revokeSession(entry: AdminSession): Promise<void> {
    const confirmed = await dialog.confirm({
      title: 'Cabut sesi?',
      message: `Sesi ${entry.username} akan diakhiri dan perangkat tersebut harus masuk kembali.`,
      confirmLabel: 'Cabut sesi',
      tone: 'danger',
    })
    if (!confirmed) return
    revokingSession = entry.id
    try {
      await usersApi.revokeSession(entry.id)
      toast.success('Sesi dicabut.')
      await loadSessions()
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      revokingSession = null
    }
  }

  async function revokeUserSessions(entry: AdminSession): Promise<void> {
    const confirmed = await dialog.confirm({
      title: 'Cabut semua sesi?',
      message: `Semua sesi milik ${entry.username} akan diakhiri.`,
      confirmLabel: 'Cabut semua',
      tone: 'danger',
    })
    if (!confirmed) return
    try {
      const result = await usersApi.revokeUserSessions(entry.user_id)
      toast.success(`${result.revoked} sesi dicabut.`)
      await loadSessions()
    } catch (caught) {
      toast.error(errorMessage(caught))
    }
  }

  async function create(event: SubmitEvent) {
    event.preventDefault()
    formError = ''
    if (form.username.trim().length < 3) {
      formError = 'Nama pengguna minimal 3 karakter.'
      return
    }
    if (form.password.length < 12) {
      formError = 'Kata sandi minimal 12 karakter.'
      return
    }
    if (form.role === 'polres' && !form.polres.trim()) {
      formError = 'Peran polres wajib memiliki nama polres.'
      return
    }
    formLoading = true
    try {
      await usersApi.create({ ...form, username: form.username.trim() })
      toast.success('Akun baru berhasil ditambahkan.')
      form = { username: '', password: '', role: 'operator', polres: '', satker: '' }
      showForm = false
      await load()
    } catch (caught) {
      formError = errorMessage(caught)
    } finally {
      formLoading = false
    }
  }

  async function update(
    account: AdminUser,
    changes: {
      role?: string
      active?: boolean
      polres?: string
      satker?: string
      password?: string
    },
  ): Promise<boolean> {
    busyId = account.id
    error = ''
    try {
      const updated = await usersApi.update(account.id, changes)
      users = users.map((item) => (item.id === updated.id ? updated : item))
      if (updated.id === user.id) {
        auth.user = {
          ...user,
          role: updated.role as User['role'],
          polres: updated.polres,
          satker: updated.satker,
        }
      }
      toast.success(
        changes.password
          ? `Kata sandi ${account.username} diperbarui.`
          : `Akses ${account.username} diperbarui.`,
      )
      return true
    } catch (caught) {
      error = errorMessage(caught)
      return false
    } finally {
      busyId = ''
    }
  }

  async function saveEdit(event: SubmitEvent) {
    event.preventDefault()
    if (!editing) return
    editError = ''
    if (editForm.role === 'polres' && !editForm.polres.trim()) {
      editError = 'Peran polres wajib memiliki nama polres.'
      return
    }
    const canResetPassword = editing.role !== 'admin' && editForm.role !== 'admin'
    if (canResetPassword && editForm.password && editForm.password.length < 12) {
      editError = 'Kata sandi baru minimal 12 karakter.'
      return
    }
    editLoading = true
    try {
      const ok = await update(editing, {
        role: editForm.role,
        polres: editForm.polres,
        satker: editForm.satker,
        ...(canResetPassword && editForm.password ? { password: editForm.password } : {}),
      })
      if (ok) editing = null
    } finally {
      editLoading = false
    }
  }
</script>

<svelte:head><title>Pengguna & akses · Laporan Mingguan PBM</title></svelte:head>

<PageHeader
  eyebrow="ADMINISTRASI"
  title="Pengguna & akses"
  description="Kelola akun lokal, peran, kata sandi, dan status aktif ruang kerja laporan."
>
  {#snippet actions()}
    <Button onclick={() => (showForm = true)}><UserPlus size={16} /> Tambah pengguna</Button>
  {/snippet}
</PageHeader>

{#if error}
  <div class="mb-5 flex items-center gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
    <AlertCircle size={17} /> {error}
  </div>
{/if}

<section class="card overflow-hidden">
  <div class="panel-heading">
    <div><span class="eyebrow">AKUN TERDAFTAR</span><h2 class="panel-title mt-1">Pengguna ruang kerja</h2></div>
    <span class="badge badge-neutral">{users.length} akun</span>
  </div>

  {#if loading}
    <div class="space-y-3 p-5">{#each Array(4) as _}<div class="skeleton h-12 w-full"></div>{/each}</div>
  {:else if users.length}
    <div class="overflow-x-auto">
      <table class="table">
        <thead><tr><th>Pengguna</th><th>Peran & unit</th><th>Status</th><th>Terdaftar</th><th class="text-right">Akses</th></tr></thead>
        <tbody>
          {#each users as account}
            <tr>
              <td>
                <div class="flex items-center gap-3">
                  <span class="flex h-9 w-9 items-center justify-center rounded-full bg-forest-50 text-xs font-bold text-forest-700">
                    {account.username.slice(0, 1).toUpperCase()}
                  </span>
                  <div>
                    <strong class="block text-xs font-semibold text-ink">{account.username}</strong>
                    {#if account.id === user.id}<small class="text-[10px] text-ink-muted">Akun Anda</small>{/if}
                  </div>
                </div>
              </td>
              <td>
                <div class="space-y-1">
                  <Badge tone={account.role === 'admin' ? 'success' : 'neutral'}>{roleLabel(account.role)}</Badge>
                  {#if account.role === 'polres'}
                    <small class="block text-[10px] text-ink-muted">{account.polres}</small>
                  {:else if account.role === 'polda'}
                    <small class="block text-[10px] text-ink-muted">{account.satker || 'Semua satker Polda'}</small>
                  {/if}
                </div>
              </td>
              <td><Badge tone={account.active ? 'success' : 'danger'}>{account.active ? 'Aktif' : 'Nonaktif'}</Badge></td>
              <td class="text-xs whitespace-nowrap text-ink-muted">{formatDateTime(account.created_at)}</td>
              <td class="text-right">
                <div class="flex justify-end gap-1">
                  <button
                    class="btn btn-ghost btn-sm"
                    disabled={busyId === account.id}
                    onclick={() => openEdit(account)}
                  >
                    Edit
                  </button>
                  <button
                    class="btn btn-ghost btn-sm"
                    disabled={account.id === user.id || busyId === account.id}
                    onclick={() => update(account, { active: !account.active })}
                  >
                    {#if account.active}<X size={13} /> Nonaktifkan{:else}<Check size={13} /> Aktifkan{/if}
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <EmptyState title="Belum ada akun" description="Tambahkan akun operator atau administrator untuk mulai menggunakan aplikasi.">
      <Button onclick={() => (showForm = true)}><Plus size={15} /> Tambah pengguna</Button>
    </EmptyState>
  {/if}

  <div class="flex items-start gap-2 border-t border-line bg-stone-50/60 px-5 py-3 text-[11px] text-ink-muted">
    <ShieldCheck size={14} class="mt-0.5 shrink-0 text-forest-600" />
    Kata sandi di-hash sebelum disimpan. Administrator dapat mereset sandi akun operator, polres, dan polda; sandi administrator hanya diubah pemiliknya. Minimal satu administrator aktif harus selalu tersedia.
  </div>
</section>

<section class="card mt-5 overflow-hidden">
  <div class="panel-heading">
    <div>
      <span class="eyebrow">SESI AKTIF</span>
      <h2 class="panel-title mt-1">Perangkat yang masuk</h2>
    </div>
    <Button variant="secondary" size="sm" loading={sessionsLoading} onclick={loadSessions}>Muat ulang</Button>
  </div>
  {#if sessions.length}
    <div class="overflow-x-auto">
      <table class="table">
        <thead>
          <tr>
            <th>Pengguna</th>
            <th>Peran</th>
            <th>Alamat</th>
            <th>Perangkat</th>
            <th>Terakhir aktif</th>
            <th class="text-right">Aksi</th>
          </tr>
        </thead>
        <tbody>
          {#each sessions as entry (entry.id)}
            <tr>
              <td><strong class="text-xs font-semibold text-ink">{entry.username}</strong></td>
              <td class="text-xs text-ink-muted">{roleLabel(entry.role)}</td>
              <td class="text-xs text-ink-muted">{entry.ip || '—'}</td>
              <td class="max-w-[220px] truncate text-[10px] text-ink-muted" title={entry.user_agent}>
                {entry.user_agent || '—'}
              </td>
              <td class="text-xs whitespace-nowrap">
                {#if sessionOnline(entry.last_seen_at)}
                  <span class="badge badge-success">Aktif</span>
                {:else}
                  <span class="text-ink-muted">
                    {entry.last_seen_at ? formatDateTime(entry.last_seen_at) : '—'}
                  </span>
                {/if}
              </td>
              <td class="text-right">
                <div class="flex items-center justify-end gap-1">
                  <button
                    class="btn btn-ghost btn-sm text-red-700"
                    disabled={revokingSession === entry.id}
                    onclick={() => revokeSession(entry)}
                  >
                    {revokingSession === entry.id ? 'Mencabut…' : 'Cabut'}
                  </button>
                  <button class="btn btn-ghost btn-sm" onclick={() => revokeUserSessions(entry)}>
                    Cabut semua
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="px-5 py-6 text-center text-[11px] text-ink-muted">Tidak ada sesi aktif.</p>
  {/if}
</section>

<datalist id="polres-options">
  {#each options.polres as option (option)}<option value={option}></option>{/each}
</datalist>
<datalist id="satker-options">
  {#each options.satker as option (option)}<option value={option}></option>{/each}
</datalist>

<Modal
  open={showForm}
  onclose={() => (showForm = false)}
  title="Tambah pengguna"
  description="Buat akun lokal baru dan tentukan perannya."
  width="max-w-lg"
>
  <form class="space-y-4" onsubmit={create}>
    <div>
      <label class="label" for="new-username">Nama pengguna</label>
      <input id="new-username" class="input" bind:value={form.username} minlength="3" autocomplete="off" required />
    </div>
    <div>
      <label class="label" for="new-password">Kata sandi sementara</label>
      <PasswordInput
        id="new-password"
        bind:value={form.password}
        autocomplete="new-password"
        placeholder="Minimal 12 karakter"
        maxlength={120}
        required
      />
      <p class="hint mt-1.5">Minimal 12 karakter. Minta pengguna menggantinya setelah masuk.</p>
    </div>
    <div>
      <label class="label" for="new-role">Peran</label>
      <select id="new-role" class="input" bind:value={form.role}>
        <option value="operator">Operator</option>
        <option value="admin">Administrator</option>
        <option value="polres">Polres (baca-saja)</option>
        <option value="polda">Polda (baca-saja)</option>
      </select>
    </div>
    {#if form.role === 'polres'}
      <div>
        <label class="label" for="new-polres">Polres</label>
        <input id="new-polres" class="input" bind:value={form.polres} list="polres-options" placeholder="Pilih atau ketik nama polres" required />
        <p class="hint mt-1.5">Saran diambil dari data import; Anda tetap bisa mengetik nama lain.</p>
      </div>
    {:else if form.role === 'polda'}
      <div>
        <label class="label" for="new-satker">Satker Polda</label>
        <input id="new-satker" class="input" bind:value={form.satker} list="satker-options" placeholder="Pilih atau ketik satker Polda" />
        <p class="hint mt-1.5">Kosongkan untuk melihat seluruh satker Polda.</p>
      </div>
    {/if}

    {#if formError}
      <div class="flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
        <AlertCircle size={17} class="mt-0.5 shrink-0" /> {formError}
      </div>
    {/if}

    <div class="flex justify-end gap-2 border-t border-line pt-4">
      <Button variant="ghost" onclick={() => (showForm = false)}>Batal</Button>
      <Button type="submit" loading={formLoading}><UserPlus size={15} /> Buat akun</Button>
    </div>
  </form>
</Modal>

<Modal
  open={editing !== null}
  onclose={() => (editing = null)}
  title="Ubah akses"
  description="Perbarui peran, unit, dan kata sandi pengguna."
  width="max-w-lg"
>
  <form class="space-y-4" onsubmit={saveEdit}>
    <div>
      <label class="label" for="edit-role">Peran</label>
      <select id="edit-role" class="input" bind:value={editForm.role}>
        <option value="operator">Operator</option>
        <option value="admin">Administrator</option>
        <option value="polres">Polres (baca-saja)</option>
        <option value="polda">Polda (baca-saja)</option>
      </select>
    </div>
    {#if editForm.role === 'polres'}
      <div>
        <label class="label" for="edit-polres">Polres</label>
        <input id="edit-polres" class="input" bind:value={editForm.polres} list="polres-options" placeholder="Pilih atau ketik nama polres" required />
        <p class="hint mt-1.5">Saran diambil dari data import; Anda tetap bisa mengetik nama lain.</p>
      </div>
    {:else if editForm.role === 'polda'}
      <div>
        <label class="label" for="edit-satker">Satker Polda</label>
        <input id="edit-satker" class="input" bind:value={editForm.satker} list="satker-options" placeholder="Pilih atau ketik satker Polda" />
        <p class="hint mt-1.5">Kosongkan untuk melihat seluruh satker Polda.</p>
      </div>
    {/if}

    {#if editing && editing.role !== 'admin' && editForm.role !== 'admin'}
      <div>
        <label class="label" for="edit-password">Kata sandi baru (opsional)</label>
        <PasswordInput
          id="edit-password"
          bind:value={editForm.password}
          autocomplete="new-password"
          placeholder="Kosongkan jika tidak diganti"
          maxlength={120}
        />
        <p class="hint mt-1.5">Minimal 12 karakter. Sesi pengguna ini akan dikeluarkan setelah kata sandi diganti.</p>
      </div>
    {:else if editing && (editing.role === 'admin' || editForm.role === 'admin')}
      <p class="rounded-xl border border-line bg-stone-50 px-4 py-3 text-[11px] text-ink-muted">
        Kata sandi akun administrator hanya dapat diubah pemiliknya melalui menu Ganti kata sandi.
      </p>
    {/if}

    {#if editError}
      <div class="flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
        <AlertCircle size={17} class="mt-0.5 shrink-0" /> {editError}
      </div>
    {/if}

    <div class="flex justify-end gap-2 border-t border-line pt-4">
      <Button variant="ghost" onclick={() => (editing = null)}>Batal</Button>
      <Button type="submit" loading={editLoading}>Simpan perubahan</Button>
    </div>
  </form>
</Modal>
