<script lang="ts">
  import { Download, Eye, FileText, RefreshCw, Share2, Trash2 } from 'lucide-svelte'
  import { downloadReport, openReportPreview, reportsApi } from '../../api'
  import type { ReportView } from '../../types'
  import { formatDateTime } from '../../utils/format'
  import { toast } from '../../stores/toast.svelte'
  import { dialog } from '../../stores/dialog.svelte'
  import Button from '../ui/Button.svelte'
  import EmptyState from '../ui/EmptyState.svelte'
  import ShareReportDialog from './ShareReportDialog.svelte'

  type Props = {
    reports: ReportView[]
    onrefresh?: () => void | Promise<void>
  }

  let { reports, onrefresh }: Props = $props()

  let busyId = $state('')
  let deletingId = $state('')
  let shareTarget = $state<ReportView | null>(null)

  async function download(report: ReportView) {
    busyId = report.id
    try {
      await downloadReport(report)
      toast.success('PDF berhasil diunduh.')
    } catch (caught) {
      toast.error(caught instanceof Error ? caught.message : 'PDF tidak dapat diunduh.')
    } finally {
      busyId = ''
    }
  }

  async function remove(report: ReportView) {
    const confirmed = await dialog.confirm({
      title: 'Hapus dokumen PDF?',
      message: `Dokumen "${report.period_label || 'Laporan mingguan'}" yang dibuat ${formatDateTime(report.created_at)} akan dihapus permanen. File PDF, HTML, dan tautan presentasinya ikut dihapus. Tindakan ini tidak dapat dibatalkan.`,
      confirmLabel: 'Hapus dokumen',
      cancelLabel: 'Batal',
      tone: 'danger',
    })
    if (!confirmed) return

    deletingId = report.id
    try {
      await reportsApi.remove(report.id)
      toast.success('Dokumen dan tautan publiknya berhasil dihapus.')
      await onrefresh?.()
    } catch (caught) {
      toast.error(caught instanceof Error ? caught.message : 'Dokumen tidak dapat dihapus.')
    } finally {
      deletingId = ''
    }
  }
</script>

<section class="card overflow-hidden">
  <div class="panel-heading">
    <div>
      <span class="eyebrow">ARSIP DOKUMEN</span>
      <h2 class="panel-title mt-1">PDF yang dihasilkan</h2>
      <p class="mt-1 text-xs text-ink-muted">Setiap dokumen menyimpan snapshot metrik dan desain saat diterbitkan.</p>
    </div>
    <div class="flex items-center gap-2">
      <span class="badge badge-neutral">{reports.length} dokumen</span>
      {#if onrefresh}<Button variant="ghost" size="sm" onclick={onrefresh}><RefreshCw size={14} /> Muat ulang</Button>{/if}
    </div>
  </div>

  {#if reports.length}
    <div class="divide-y divide-stone-100">
      {#each reports as report}
        <div class="flex flex-wrap items-center gap-3 px-5 py-4">
          <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><FileText size={18} /></span>
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-semibold text-ink">{report.period_label || 'Laporan mingguan'}</p>
            <p class="mt-0.5 text-[11px] text-ink-muted">{formatDateTime(report.created_at)} · Sumber {report.source_filename}</p>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <Button variant="secondary" size="sm" disabled={deletingId !== ''} onclick={() => openReportPreview(report)}>
              <Eye size={14} /> Pratinjau
            </Button>
            <Button variant="secondary" size="sm" disabled={deletingId !== ''} onclick={() => (shareTarget = report)}>
              <Share2 size={14} /> Bagikan & presentasi
            </Button>
            <Button size="sm" loading={busyId === report.id} disabled={deletingId !== ''} onclick={() => download(report)}>
              <Download size={14} /> Unduh
            </Button>
            <Button variant="danger" size="sm" loading={deletingId === report.id}
              disabled={deletingId !== '' && deletingId !== report.id || busyId === report.id}
              onclick={() => remove(report)}>
              <Trash2 size={14} /> Hapus
            </Button>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <EmptyState
      title="Belum ada PDF"
      description="Susun desain lalu terbitkan PDF. Dokumen yang dihasilkan akan muncul di sini beserta snapshot datanya."
    />
  {/if}
</section>

<ShareReportDialog
  open={shareTarget !== null}
  report={shareTarget}
  onclose={() => (shareTarget = null)}
/>
