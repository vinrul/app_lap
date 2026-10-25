<script lang="ts">
  import { untrack } from 'svelte'
  import {
    AlertCircle,
    Check,
    CloudUpload,
    Eye,
    FileDown,
    FileText,
    LockKeyhole,
    PenLine,
    Save,
    Sparkles,
  } from 'lucide-svelte'
  import { reportsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type { DraftView, PeriodDetail, ReportContent, ReportMetrics, ReportView } from '../../types'
  import { emptyContent, reportSections } from '../../utils/constants'
  import { formatDateTime, formatNumber } from '../../utils/format'
  import { toast } from '../../stores/toast.svelte'
  import Button from '../ui/Button.svelte'
  import EmptyState from '../ui/EmptyState.svelte'

  type Props = {
    periodId: string
    detail: PeriodDetail
    metrics: ReportMetrics | null
    ongenerated: () => Promise<void>
  }

  let {
    periodId,
    detail,
    metrics,
    ongenerated,
  }: Props = $props()

  let content = $state<ReportContent>({ ...emptyContent })
  let savedJson = $state('')
  let updatedAt = $state<string | null>(null)
  let updatedBy = $state<string | null>(null)
  let loading = $state(true)
  let saving = $state(false)
  let previewing = $state(false)
  let generating = $state(false)
  let error = $state('')
  let lastKey = ''

  const dirty = $derived(JSON.stringify(content) !== savedJson)
  const hasImport = $derived(detail.imports.length > 0)

  $effect(() => {
    const key = periodId
    if (key === lastKey) return
    lastKey = key
    untrack(() => void loadDraft())
  })

  async function loadDraft() {
    loading = true
    error = ''
    if (!hasImport) {
      content = { ...emptyContent }
      savedJson = ''
      updatedAt = null
      updatedBy = null
      loading = false
      return
    }
    try {
      const draft: DraftView = await reportsApi.getDraft(periodId)
      content = { ...emptyContent, ...draft.content }
      savedJson = JSON.stringify(content)
      updatedAt = draft.updated_at
      updatedBy = draft.updated_by
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  async function save(silent = false): Promise<boolean> {
    if (!hasImport) return false
    saving = true
    error = ''
    try {
      const draft = await reportsApi.saveDraft(periodId, content)
      savedJson = JSON.stringify(content)
      updatedAt = draft.updated_at
      updatedBy = draft.updated_by
      if (!silent) toast.success('Perubahan editorial tersimpan.')
      return true
    } catch (caught) {
      error = errorMessage(caught)
      return false
    } finally {
      saving = false
    }
  }

  async function preview() {
    if (!hasImport) return
    const target = window.open('', '_blank')
    if (!target) {
      toast.error('Izinkan popup untuk membuka pratinjau laporan.')
      return
    }
    previewing = true
    error = ''
    try {
      const result = await reportsApi.preview(periodId, content)
      const url = URL.createObjectURL(new Blob([result.html], { type: 'text/html' }))
      target.location.href = url
      window.setTimeout(() => URL.revokeObjectURL(url), 60_000)
    } catch (caught) {
      target.close()
      error = errorMessage(caught)
    } finally {
      previewing = false
    }
  }

  async function generate() {
    if (!hasImport) return
    if (dirty) {
      const ok = await save(true)
      if (!ok) return
    }
    generating = true
    error = ''
    try {
      const report: ReportView = await reportsApi.generate(periodId, content)
      toast.success('PDF laporan berhasil dibuat.')
      await ongenerated()
      window.open(reportsApi.previewUrl(report.id), '_blank', 'noopener')
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      generating = false
    }
  }
</script>

{#if !hasImport}
  <section class="card">
    <EmptyState
      title="Belum ada versi data"
      description="Impor workbook terlebih dahulu sebelum menyusun narasi dan menerbitkan PDF."
    >
      <Button onclick={() => (window.location.hash = '/imports')}><CloudUpload size={15} /> Import workbook</Button>
    </EmptyState>
  </section>
{:else}
  <section class="card mb-4 flex flex-wrap items-center justify-between gap-3 px-5 py-4">
    <div>
      <div class="flex flex-wrap items-center gap-2">
        <span class="eyebrow">EDITORIAL LAPORAN</span>
        {#if dirty}<span class="badge badge-progress">Belum disimpan</span>{:else}<span class="badge badge-success"><Check size={11} /> Tersimpan</span>{/if}
      </div>
      <p class="mt-1 text-xs text-ink-muted">
        {#if updatedAt}Terakhir disimpan {formatDateTime(updatedAt)}{#if updatedBy} oleh {updatedBy}{/if}{:else}Belum pernah disimpan{/if}
      </p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <Button variant="secondary" size="sm" loading={previewing} onclick={preview}><Eye size={14} /> Pratinjau</Button>
      <Button variant="secondary" size="sm" loading={saving} onclick={() => save()}><Save size={14} /> Simpan</Button>
      <Button size="sm" loading={generating} onclick={generate}><FileDown size={14} /> Buat PDF</Button>
    </div>
  </section>

  {#if error}
    <div class="mb-4 flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertCircle size={17} class="mt-0.5 shrink-0" /> <span>{error}</span>
    </div>
  {/if}

  {#if loading}
    <div class="grid gap-4 lg:grid-cols-2">{#each Array(4) as _}<div class="skeleton h-40"></div>{/each}</div>
  {:else}
    <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
      <div class="space-y-4">
        <section class="card overflow-hidden">
          <div class="panel-heading">
            <div><span class="eyebrow">NARASI</span><h2 class="panel-title mt-1">Isi bagian laporan</h2><p class="mt-1 text-xs text-ink-muted">Narasi ini disimpan sebagai snapshot saat PDF dibuat.</p></div>
            <PenLine size={18} class="text-ink-muted" />
          </div>
          <div class="grid gap-0 lg:grid-cols-2">
            {#each reportSections as section, index}
              <div class="border-b border-line p-5 {index % 2 === 0 ? 'lg:border-r' : ''}">
                <div class="mb-2 flex items-start gap-2.5">
                  <span class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-forest-50 text-[10px] font-bold text-forest-700">{section.number}</span>
                  <div>
                    <label class="block text-sm font-semibold text-ink" for={`editorial-${section.key}`}>{section.label}</label>
                    <p class="mt-0.5 text-[11px] leading-relaxed text-ink-muted">{section.hint}</p>
                  </div>
                </div>
                <textarea
                  id={`editorial-${section.key}`}
                  class="textarea mt-2 text-sm"
                  rows={section.rows}
                  bind:value={content[section.key]}
                  placeholder={section.placeholder}
                ></textarea>
              </div>
            {/each}
          </div>
        </section>

        <section class="card overflow-hidden">
          <div class="panel-heading">
            <div><span class="eyebrow">PENANDATANGAN</span><h2 class="panel-title mt-1">Blok tanda tangan laporan</h2></div>
          </div>
          <div class="grid gap-4 p-5 sm:grid-cols-2">
            <div>
              <label class="label" for="report-city">Kota</label>
              <input id="report-city" class="input" bind:value={content.report_city} placeholder="Denpasar" />
            </div>
            <div>
              <label class="label" for="report-date">Tanggal laporan</label>
              <input id="report-date" class="input" type="date" bind:value={content.report_date} />
            </div>
            <div>
              <label class="label" for="signatory-rank">Pangkat / jabatan</label>
              <input id="signatory-rank" class="input" bind:value={content.signatory_rank} placeholder="KAPOLDA BALI" />
            </div>
            <div>
              <label class="label" for="signatory-name">Nama pejabat</label>
              <input id="signatory-name" class="input" bind:value={content.signatory_name} placeholder="Nama lengkap" />
            </div>
            <div class="sm:col-span-2">
              <label class="label" for="signatory-nip">NIP <span class="font-normal text-ink-muted normal-case">(opsional)</span></label>
              <input id="signatory-nip" class="input" bind:value={content.signatory_nip} placeholder="NIP pejabat" />
            </div>
          </div>
        </section>
      </div>

      <aside class="space-y-4 self-start xl:sticky xl:top-24">
        <section class="card overflow-hidden">
          <div class="panel-heading">
            <div><span class="eyebrow">RINGKASAN</span><h2 class="panel-title mt-1">Metrik laporan</h2></div>
            <Sparkles size={17} class="text-gold-600" />
          </div>
          {#if metrics}
            <div class="divide-y divide-stone-100">
              {#each metrics.cards as card}
                <div class="flex items-center justify-between gap-3 px-5 py-3">
                  <div class="min-w-0">
                    <p class="truncate text-[11px] font-semibold text-ink">{card.label}</p>
                    <p class="truncate text-[10px] text-ink-muted">{card.unit}</p>
                  </div>
                  <strong class="font-display text-lg text-forest-900">{formatNumber(card.value)}</strong>
                </div>
              {/each}
            </div>
            <div class="border-t border-line bg-stone-50/60 px-5 py-3 text-[10px] text-ink-muted">
              {formatNumber(metrics.total_rows)} baris · {metrics.outside_period} di luar rentang
            </div>
          {:else}
            <p class="px-5 py-4 text-xs text-ink-muted">Metrik belum tersedia.</p>
          {/if}
        </section>

        <section class="card p-5">
          <div class="flex items-center gap-2"><LockKeyhole size={15} class="text-gold-600" /><span class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Sebelum menerbitkan</span></div>
          <ul class="mt-3 space-y-2 text-[11px] leading-relaxed text-ink-muted">
            <li class="flex gap-2"><Check size={13} class="mt-0.5 shrink-0 text-forest-600" /> Periksa angka metrik dan sebaran wilayah.</li>
            <li class="flex gap-2"><Check size={13} class="mt-0.5 shrink-0 text-forest-600" /> Lengkapi narasi aspirasi dan tindak lanjut.</li>
            <li class="flex gap-2"><Check size={13} class="mt-0.5 shrink-0 text-forest-600" /> Cocokkan penandatangan dan tanggal laporan.</li>
            <li class="flex gap-2"><Check size={13} class="mt-0.5 shrink-0 text-forest-600" /> Gunakan pratinjau sebelum membuat PDF.</li>
          </ul>
        </section>

        <section class="card p-5">
          <div class="flex items-center gap-2"><FileText size={15} class="text-forest-600" /><span class="text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Dokumen tersimpan</span></div>
          <p class="mt-2 font-display text-2xl text-forest-900">{detail.reports.length}</p>
          <p class="text-[11px] text-ink-muted">PDF pada periode ini</p>
        </section>
      </aside>
    </div>
  {/if}
{/if}
