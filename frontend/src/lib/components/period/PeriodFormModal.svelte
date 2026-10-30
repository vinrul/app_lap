<script lang="ts">
  import { untrack } from 'svelte'
  import { AlertCircle, CalendarDays, Sparkles } from 'lucide-svelte'
  import { periodsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type { Period } from '../../types'
  import { addDays, formatDate, formatRange, startOfWeek, toIsoDate } from '../../utils/format'
  import { navigate } from '../../stores/router.svelte'
  import { toast } from '../../stores/toast.svelte'
  import Modal from '../ui/Modal.svelte'
  import Button from '../ui/Button.svelte'

  type Props = {
    open?: boolean
    onclose: () => void
    period?: Period | null
    onsaved?: (period: Period) => void
  }
  type PeriodType = 'daily' | 'range' | 'monthly'

  let {
    open = false,
    onclose,
    period = null,
    onsaved,
  }: Props = $props()

  const initialPeriod = untrack(() => period)
  const editing = initialPeriod !== null
  const today = new Date()
  let periodType = $state<PeriodType>(initialPeriod?.period_type ?? 'range')
  let dailyDate = $state(initialPeriod?.start_date ?? toIsoDate(today))
  let startDate = $state(initialPeriod?.start_date ?? toIsoDate(startOfWeek(today)))
  let endDate = $state(initialPeriod?.end_date ?? toIsoDate(addDays(startOfWeek(today), 6)))
  let monthValue = $state(
    initialPeriod
      ? initialPeriod.start_date.slice(0, 7)
      : `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}`,
  )
  let label = $state(initialPeriod?.label ?? '')
  let loading = $state(false)
  let error = $state('')

  const computed = $derived.by(() => {
    if (periodType === 'daily') {
      return { start: dailyDate, end: dailyDate }
    }
    if (periodType === 'monthly') {
      const [year, month] = monthValue.split('-').map(Number)
      if (!year || !month) return { start: '', end: '' }
      const first = new Date(year, month - 1, 1)
      const last = new Date(year, month, 0)
      return { start: toIsoDate(first), end: toIsoDate(last) }
    }
    return { start: startDate, end: endDate }
  })

  const preview = $derived.by(() => {
    if (!computed.start || !computed.end) return 'Lengkapi tanggal laporan'
    if (periodType === 'daily') return `Laporan Harian · ${formatDate(computed.start)}`
    if (periodType === 'monthly') {
      const [year, month] = monthValue.split('-').map(Number)
      const date = new Date(year, month - 1, 1)
      return `Laporan Bulanan · ${new Intl.DateTimeFormat('id-ID', { month: 'long', year: 'numeric' }).format(date)}`
    }
    return `Rentang · ${formatRange(computed.start, computed.end)}`
  })

  function applyRangePreset(kind: 'this' | 'last' | 'weekend') {
    const base = new Date()
    let start = startOfWeek(base)
    if (kind === 'last') start = addDays(start, -7)
    if (kind === 'weekend') start = addDays(start, 4)
    const end = kind === 'weekend' ? addDays(start, 2) : addDays(start, 6)
    startDate = toIsoDate(start)
    endDate = toIsoDate(end)
    if (!editing) label = ''
    error = ''
  }

  function setType(next: PeriodType) {
    periodType = next
    if (!editing) label = ''
    error = ''
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault()
    error = ''
    if (!computed.start || !computed.end) {
      error = 'Tanggal laporan wajib diisi.'
      return
    }
    if (computed.start > computed.end) {
      error = 'Tanggal awal harus sebelum atau sama dengan tanggal akhir.'
      return
    }
    loading = true
    try {
      if (period) {
        const updated: Period = await periodsApi.update(period.id, {
          start_date: computed.start,
          end_date: computed.end,
          label: label.trim(),
          period_type: periodType,
        })
        toast.success('Periode laporan berhasil diperbarui.')
        onsaved?.(updated)
        onclose()
        return
      }
      const created: Period = await periodsApi.create({
        start_date: computed.start,
        end_date: computed.end,
        label: label.trim() || undefined,
        period_type: periodType,
      })
      toast.success('Periode laporan berhasil dibuat.')
      onclose()
      navigate(`/periods/${created.id}/overview`)
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
  title={editing ? 'Edit periode laporan' : 'Buat periode laporan'}
  description={editing
    ? 'Perbarui nama, jenis, atau rentang tanggal periode. Metrik dihitung ulang dari import yang mencakup rentang baru; dokumen PDF yang sudah dibuat tetap memakai data saat dibuat.'
    : 'Pilih jenis laporan. Data diambil otomatis dari semua import yang mencakup rentang tanggal periode.'}
  width="max-w-xl"
>
  <form class="space-y-5" onsubmit={submit}>
    <div class="grid grid-cols-3 gap-2">
      {#each [
        { id: 'daily', label: 'Harian' },
        { id: 'range', label: 'Mingguan / Rentang' },
        { id: 'monthly', label: 'Bulanan' },
      ] as option}
        <button type="button" class="rounded-xl border px-2 py-2.5 text-xs font-semibold transition-colors {periodType === option.id
          ? 'border-forest-700 bg-forest-50 text-forest-800'
          : 'border-line text-ink-muted hover:border-gold-300 hover:bg-gold-50'}"
          data-period-type={option.id}
          onclick={() => setType(option.id as PeriodType)}>
          {option.label}
        </button>
      {/each}
    </div>

    {#if periodType === 'daily'}
      <div>
        <label class="label" for="period-daily">Tanggal laporan</label>
        <input id="period-daily" type="date" class="input" bind:value={dailyDate} required />
      </div>
    {:else if periodType === 'monthly'}
      <div>
        <label class="label" for="period-month">Bulan dan tahun</label>
        <input id="period-month" type="month" class="input" bind:value={monthValue} required />
      </div>
    {:else}
      <div class="flex flex-wrap gap-2">
        <button type="button" class="btn btn-secondary btn-sm" onclick={() => applyRangePreset('this')}>Minggu ini</button>
        <button type="button" class="btn btn-secondary btn-sm" onclick={() => applyRangePreset('last')}>Minggu lalu</button>
        <button type="button" class="btn btn-secondary btn-sm" onclick={() => applyRangePreset('weekend')}>Jumat–Minggu</button>
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
    {/if}

    <div>
      <label class="label" for="period-label">Nama periode <span class="font-normal text-ink-muted normal-case">(opsional)</span></label>
      <input id="period-label" class="input" bind:value={label} maxlength="120" placeholder={preview} />
      <p class="hint mt-1.5">Jika dikosongkan, nama dibuat otomatis sesuai jenis periode.</p>
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
      <Button type="submit" loading={loading}><Sparkles size={15} /> {editing ? 'Simpan perubahan' : 'Simpan periode'}</Button>
    </div>
  </form>
</Modal>
