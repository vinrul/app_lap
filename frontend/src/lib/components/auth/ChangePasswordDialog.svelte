<script lang="ts">
  import { KeyRound, ShieldCheck } from 'lucide-svelte'
  import { authApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import { toast } from '../../stores/toast.svelte'
  import Button from '../ui/Button.svelte'
  import Modal from '../ui/Modal.svelte'
  import PasswordInput from '../ui/PasswordInput.svelte'

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  let currentPassword = $state('')
  let newPassword = $state('')
  let confirmPassword = $state('')
  let loading = $state(false)
  let error = $state('')

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault()
    error = ''

    if (!currentPassword || !newPassword || !confirmPassword) {
      error = 'Semua kolom kata sandi wajib diisi.'
      return
    }
    if (newPassword.length < 12) {
      error = 'Kata sandi baru minimal 12 karakter.'
      return
    }
    if (newPassword === currentPassword) {
      error = 'Kata sandi baru harus berbeda dari kata sandi saat ini.'
      return
    }
    if (newPassword !== confirmPassword) {
      error = 'Konfirmasi kata sandi baru tidak cocok.'
      return
    }

    loading = true
    try {
      await authApi.changePassword(currentPassword, newPassword)
      toast.success('Kata sandi berhasil diperbarui. Sesi di perangkat lain telah dikeluarkan.')
      onclose()
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }
</script>

{#snippet actions()}
  <Button variant="ghost" onclick={onclose}>Batal</Button>
  <Button type="submit" form="change-password-form" loading={loading}>
    <ShieldCheck size={15} /> Simpan kata sandi
  </Button>
{/snippet}

<Modal
  open={true}
  title="Ganti kata sandi"
  description="Masukkan kata sandi saat ini, lalu tentukan kata sandi baru minimal 12 karakter."
  onclose={onclose}
  width="max-w-md"
  footer={actions}
>
  <form id="change-password-form" class="space-y-4" onsubmit={submit}>
    <div class="flex items-center gap-3 rounded-xl border border-forest-200 bg-forest-50 px-4 py-3">
      <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-white text-forest-700">
        <KeyRound size={17} />
      </span>
      <p class="text-[11px] leading-relaxed text-forest-800">
        Setelah kata sandi berubah, sesi di perangkat lain otomatis dikeluarkan. Sesi di perangkat ini tetap aktif.
      </p>
    </div>

    <div>
      <label class="label" for="current-password">Kata sandi saat ini</label>
      <PasswordInput
        id="current-password"
        bind:value={currentPassword}
        autocomplete="current-password"
        required
      />
    </div>

    <div>
      <label class="label" for="new-password">Kata sandi baru</label>
      <PasswordInput
        id="new-password"
        bind:value={newPassword}
        autocomplete="new-password"
        placeholder="Minimal 12 karakter"
        maxlength={120}
        required
      />
    </div>

    <div>
      <label class="label" for="confirm-password">Ulangi kata sandi baru</label>
      <PasswordInput
        id="confirm-password"
        bind:value={confirmPassword}
        autocomplete="new-password"
        placeholder="Ulangi kata sandi baru"
        maxlength={120}
        required
      />
    </div>

    {#if newPassword && newPassword.length < 12}
      <p class="text-[10px] text-gold-700">Kata sandi baru masih kurang {12 - newPassword.length} karakter.</p>
    {/if}

    {#if error}
      <div class="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800" role="alert">
        {error}
      </div>
    {/if}
  </form>
</Modal>
