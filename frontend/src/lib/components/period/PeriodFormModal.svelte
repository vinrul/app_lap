<script lang="ts">
  import { AlertCircle, CalendarDays, Sparkles } from 'lucide-svelte'
  import { periodsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type { Period } from '../../types'
  import { addDays, formatRange, startOfWeek, toIsoDate } from '../../utils/format'
  import { navigate } from '../../stores/router.svelte'
  import { toast } from '../../stores/toast.svelte'
  import Modal from '../ui/Modal.svelte'
  import Button from '../ui/Button.svelte'

  type Props = { open?: boolean; onclose: () => void }

  let {
    open = false,
    onclose,
  }: Props = $props()

  let startDate = $state('')
  let endDate = $state('')
  let label = $state('')
  let loading = $state(false)
  let error = $state('')
  let initialized = $state(false)

  $effect(() => {
    if (open && !initialized) {
      applyPreset('this')
      initialized = true
    }
    if (!open) initialized = false
  })

  const preview = $derived(
    startDate && endDate ? formatRange(startDate, endDate) : 'Pilih rentang tanggal laporan',
  )

  function applyPreset(kind: 'this' | 'last' | 'weekend') {
    const today = new Date()
    let start = startOfWeek(today)
    if (kind === 'last') start = addDays(start, -7)
    if (kind === 'weekend') start = addDays(start, 4)
    const end = kind === 'weekend' ? addDays(start, 2) : addDays(start, 6)
    startDate = toIsoDate(start)
    endDate = toIsoDate(end)
    label = ''
    error = ''
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault()
    error = ''
    if (!startDate || !endDate) {
      error = 'Tanggal awal dan akhir wajib diisi.'
      return
    }
    if (startDate > endDate) {
      error = 'Tanggal awal harus sebelum atau sama dengan tanggal akhir.'
      return
    }
    loading = true
    try {
      const created: Period = await periodsApi.create({
        start_date: startDate,
        end_date: endDate,
        label: label.trim() || undefined,
      })
      toast.success('Periode laporan berhasil dibuat.')
      onclose()
      navigate(`/periods/${created.id}/imports`)
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }
</script>

<Modal
  {open}
  onclose={onclose}
  title="Buat periode laporan"
  description="Tentukan rentang tanggal yang jelas. Semua baris di luar rentang akan ditandai."
  width="max-w-lg"
>
  <form class="space-y-5" onsubmit={submit}>
    <div class="flex flex-wrap gap-2">
      <button type="button" class="btn btn-secondary btn-sm" onclick={() => applyPreset('this')}>Minggu ini</button>
      <button type="button" class="btn btn-secondary btn-sm" onclick={() => applyPreset('last')}>Minggu lalu</button>
      <button type="button" class="btn btn-secondary btn-sm" onclick={() => applyPreset('weekend')}>Jumat–Minggu</button>
    </div>

    <div class="grid gap-4 sm:grid-cols-2">
      <div>
        <label class="label" for="period-start">Tanggal awal</label>
        <input id="period-start" type="date" class="input" bind:value={startDate} required />
      </div>
      <div>
        <label class="label" for="period-end">Tanggal akhir</label>
        <input id="period-end" type="date" class="input" bind:value={endDate} required />
      </div>
    </div>

    <div>
      <label class="label" for="period-label">Nama periode <span class="font-normal text-ink-muted normal-case">(opsional)</span></label>
      <input id="period-label" class="input" bind:value={label} maxlength="120" placeholder={preview} />
      <p class="hint mt-1.5">Jika dikosongkan, nama dibuat otomatis dari rentang tanggal.</p>
    </div>

    <div class="flex items-center gap-2.5 rounded-xl border border-forest-200 bg-forest-50 px-4 py-3">
      <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-white text-forest-700"><CalendarDays size={16} /></span>
      <div>
        <p class="text-[10px] font-bold tracking-[0.14em] text-forest-600 uppercase">Pratinjau periode</p>
        <p class="text-sm font-semibold text-forest-900">{preview}</p>
      </div>
    </div>

    {#if error}
      <div class="flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800" role="alert">
        <AlertCircle size={17} class="mt-0.5 shrink-0" /> {error}
      </div>
    {/if}

    <div class="flex flex-wrap justify-end gap-2 border-t border-line pt-4">
      <Button variant="ghost" onclick={onclose}>Batal</Button>
      <Button type="submit" loading={loading}><Sparkles size={15} /> Simpan periode</Button>
    </div>
  </form>
</Modal>
