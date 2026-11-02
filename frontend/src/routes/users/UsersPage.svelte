<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertCircle, Check, Plus, ShieldCheck, UserPlus, X } from 'lucide-svelte'
  import { usersApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import type { AdminUser, User, UserOptions } from '../../lib/types'
  import { formatDateTime } from '../../lib/utils/format'
  import { auth } from '../../lib/stores/auth.svelte'
  import { toast } from '../../lib/stores/toast.svelte'
  import PageHeader from '../../lib/components/ui/PageHeader.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import Modal from '../../lib/components/ui/Modal.svelte'
  import EmptyState from '../../lib/components/ui/EmptyState.svelte'

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
  let editForm = $state({ role: 'operator', polres: '', satker: '' })
  let editLoading = $state(false)
  let editError = $state('')

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
    editForm = { role: account.role, polres: account.polres, satker: account.satker }
  }

  onMount(load)

  async function load() {
    loading = true
    error = ''
    try {
      const [accounts, unitOptions] = await Promise.all([usersApi.list(), usersApi.options()])
      users = accounts
      options = unitOptions
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
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
    changes: { role?: string; active?: boolean; polres?: string; satker?: string },
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
      toast.success(`Akses ${account.username} diperbarui.`)
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
    editLoading = true
    try {
      const ok = await update(editing, {
        role: editForm.role,
        polres: editForm.polres,
        satker: editForm.satker,
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
  description="Kelola akun lokal, peran, dan status aktif ruang kerja laporan."
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
    Kata sandi di-hash sebelum disimpan. Minimal satu administrator aktif harus selalu tersedia.
  </div>
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
      <input id="new-password" class="input" type="password" bind:value={form.password} minlength="12" autocomplete="new-password" required />
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
  description="Perbarui peran dan unit pengguna."
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
