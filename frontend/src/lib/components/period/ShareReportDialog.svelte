<script lang="ts">
  import {
    AlertCircle,
    Copy,
    ExternalLink,
    FileText,
    LoaderCircle,
    Mail,
    MessageCircle,
    Play,
  } from 'lucide-svelte'
  import { reportsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import { toast } from '../../stores/toast.svelte'
  import type { ReportShareView, ReportView } from '../../types'
  import { copyText, presentationLink, publicPdfLink } from '../../utils/share'
  import Button from '../ui/Button.svelte'
  import Modal from '../ui/Modal.svelte'

  type Props = {
    open: boolean
    report: ReportView | null
    onclose: () => void
  }

  let { open, report, onclose }: Props = $props()

  let busy = $state(false)
  let error = $state('')
  let share = $state<ReportShareView | null>(null)
  let loadedFor = ''
  let requestSeq = 0

  const presentUrl = $derived(share ? presentationLink(share.token) : '')
  const pdfUrl = $derived(share ? publicPdfLink(share.token) : '')
  const message = $derived(
    report && share
      ? `Laporan ${report.period_label}\nPDF: ${pdfUrl}\nPresentasi: ${presentUrl}`
      : '',
  )
  const whatsappUrl = $derived(
    message ? `https://wa.me/?text=${encodeURIComponent(message)}` : '',
  )
  const emailUrl = $derived(
    report && message
      ? `mailto:?subject=${encodeURIComponent(`Laporan ${report.period_label}`)}&body=${encodeURIComponent(message)}`
      : '',
  )

  $effect(() => {
    const current = report
    if (!open || !current) return
    if (loadedFor === current.id) return
    void loadShare(current.id)
  })

  async function loadShare(reportId: string): Promise<void> {
    const seq = ++requestSeq
    busy = true
    error = ''
    share = null
    loadedFor = ''
    try {
      const created = await reportsApi.share(reportId)
      if (seq !== requestSeq) return
      share = created
      loadedFor = reportId
    } catch (caught) {
      if (seq === requestSeq) error = errorMessage(caught)
    } finally {
      if (seq === requestSeq) busy = false
    }
  }

  function retry(): void {
    const current = report
    if (!current) return
    void loadShare(current.id)
  }

  async function copyLink(value: string, label: string): Promise<void> {
    if (!value) return
    if (await copyText(value)) toast.success(`${label} disalin.`)
    else toast.error('Tautan tidak dapat disalin. Salin manual dari kolom tautan.')
  }

  function portal(node: HTMLElement): { destroy: () => void } {
    document.body.appendChild(node)
    return { destroy: () => node.remove() }
  }
</script>

{#if open}
  <div use:portal>
    <Modal
      open={true}
      title="Bagikan & presentasi"
      description="Tautan publik dapat dibuka tanpa login. Siapa pun yang memiliki tautan dapat melihat laporan ini."
      onclose={onclose}
      width="max-w-2xl"
    >
      {#if busy}
        <div class="flex items-center gap-3 rounded-xl border border-line bg-stone-50 px-4 py-5 text-sm text-ink-muted">
          <LoaderCircle size={18} class="animate-spin text-forest-700" />
          Menyiapkan tautan publik…
        </div>
      {:else if error}
        <div class="rounded-xl border border-red-200 bg-red-50 px-4 py-4 text-sm text-red-800">
          <div class="flex items-start gap-2">
            <AlertCircle size={17} class="mt-0.5 shrink-0" />
            <p class="leading-relaxed">{error}</p>
          </div>
          <Button variant="secondary" size="sm" class="mt-3" onclick={retry}>Coba lagi</Button>
        </div>
      {:else if share}
        <div class="space-y-4">
          <section class="rounded-2xl border border-line bg-white p-4">
            <div class="flex items-start gap-3">
              <span class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-forest-50 text-forest-700">
                <Play size={18} />
              </span>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-semibold text-ink">Presentasi seperti PowerPoint</p>
                <p class="mt-0.5 text-[11px] leading-relaxed text-ink-muted">
                  Satu halaman ditampilkan penuh dengan navigasi, mode layar penuh, dan pilihan transisi.
                </p>
              </div>
            </div>
            <input class="input mt-3 w-full py-2 text-xs" readonly value={presentUrl} data-share-presentation-url />
            <div class="mt-2 flex flex-wrap gap-2">
              <Button size="sm" onclick={() => copyLink(presentUrl, 'Tautan presentasi')}>
                <Copy size={14} /> Salin tautan
              </Button>
              <a class="btn btn-secondary btn-sm" href={presentUrl} target="_blank" rel="noopener">
                <ExternalLink size={14} /> Buka presentasi
              </a>
            </div>
          </section>

          <section class="rounded-2xl border border-line bg-white p-4">
            <div class="flex items-start gap-3">
              <span class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gold-50 text-gold-700">
                <FileText size={18} />
              </span>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-semibold text-ink">Bagikan PDF</p>
                <p class="mt-0.5 text-[11px] leading-relaxed text-ink-muted">
                  Kirim tautan PDF melalui WhatsApp atau email, atau salin untuk dibagikan manual.
                </p>
              </div>
            </div>
            <input class="input mt-3 w-full py-2 text-xs" readonly value={pdfUrl} data-share-pdf-url />
            <div class="mt-2 flex flex-wrap gap-2">
              <Button variant="secondary" size="sm" onclick={() => copyLink(pdfUrl, 'Tautan PDF')}>
                <Copy size={14} /> Salin tautan PDF
              </Button>
              <a class="btn btn-secondary btn-sm" href={whatsappUrl} target="_blank" rel="noopener">
                <MessageCircle size={14} /> WhatsApp
              </a>
              <a class="btn btn-secondary btn-sm" href={emailUrl}>
                <Mail size={14} /> Email
              </a>
              <a class="btn btn-ghost btn-sm" href={pdfUrl} target="_blank" rel="noopener">
                <ExternalLink size={14} /> Buka PDF
              </a>
            </div>
          </section>

          <p class="text-[10px] leading-relaxed text-ink-muted">
            Tautan ini tetap sama setiap kali dialog dibuka. Simpan tautan hanya untuk penerima yang berwenang.
          </p>
        </div>
      {/if}
    </Modal>
  </div>
{/if}
