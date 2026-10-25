<script lang="ts">
  import { onMount } from 'svelte'
  import {
    AlertCircle,
    ChevronLeft,
    ChevronRight,
    Copy,
    Expand,
    FileText,
    LoaderCircle,
    Minimize2,
  } from 'lucide-svelte'
  import { publicShareApi } from '../../lib/api'
  import { errorMessage } from '../../lib/api/client'
  import { toast } from '../../lib/stores/toast.svelte'
  import type { PublicShareView } from '../../lib/types'
  import { copyText } from '../../lib/utils/share'

  type Props = { token: string }
  type TransitionId = 'book' | 'slide' | 'fade' | 'zoom' | 'none'
  type Direction = 'next' | 'prev'

  const TRANSITION_KEY = 'laporan-presentation-transition'
  const STYLE_ID = 'laporan-presentation-mode'
  const transitions: { id: TransitionId; label: string }[] = [
    { id: 'book', label: 'Buka buku' },
    { id: 'slide', label: 'Geser' },
    { id: 'fade', label: 'Pudar' },
    { id: 'zoom', label: 'Zoom' },
    { id: 'none', label: 'Tanpa efek' },
  ]

  const PRESENTATION_CSS = `
    html, body {
      width: 100% !important;
      height: 100% !important;
      margin: 0 !important;
      padding: 0 !important;
      background: transparent !important;
      overflow: hidden !important;
    }
    body {
      display: flex !important;
      align-items: center !important;
      justify-content: center !important;
      perspective: 2600px;
      perspective-origin: 50% 50%;
    }
    .page {
      display: none !important;
      flex: 0 0 auto !important;
      margin: 0 !important;
      box-shadow: none !important;
      zoom: var(--present-zoom, 1);
      backface-visibility: hidden;
    }
    .page.present-current {
      display: flex !important;
      box-shadow: 0 45px 110px -35px rgba(0, 0, 0, .9) !important;
    }
    .page.present-current[data-present-transition="book"][data-present-direction="next"] {
      animation: present-book-next .78s cubic-bezier(.2,.72,.22,1) both;
      transform-origin: left center;
    }
    .page.present-current[data-present-transition="book"][data-present-direction="prev"] {
      animation: present-book-prev .78s cubic-bezier(.2,.72,.22,1) both;
      transform-origin: right center;
    }
    .page.present-current[data-present-transition="slide"][data-present-direction="next"] {
      animation: present-slide-next .5s cubic-bezier(.2,.72,.22,1) both;
    }
    .page.present-current[data-present-transition="slide"][data-present-direction="prev"] {
      animation: present-slide-prev .5s cubic-bezier(.2,.72,.22,1) both;
    }
    .page.present-current[data-present-transition="fade"] {
      animation: present-fade .46s ease-out both;
    }
    .page.present-current[data-present-transition="zoom"] {
      animation: present-zoom .52s cubic-bezier(.2,.72,.22,1) both;
    }
    @keyframes present-book-next {
      0% { opacity: 0; transform: rotateY(-84deg) scale(.98); filter: brightness(.55); }
      55% { opacity: 1; }
      100% { opacity: 1; transform: rotateY(0deg) scale(1); filter: brightness(1); }
    }
    @keyframes present-book-prev {
      0% { opacity: 0; transform: rotateY(84deg) scale(.98); filter: brightness(.55); }
      55% { opacity: 1; }
      100% { opacity: 1; transform: rotateY(0deg) scale(1); filter: brightness(1); }
    }
    @keyframes present-slide-next {
      from { opacity: 0; transform: translateX(9vw) scale(.985); }
      to { opacity: 1; transform: translateX(0) scale(1); }
    }
    @keyframes present-slide-prev {
      from { opacity: 0; transform: translateX(-9vw) scale(.985); }
      to { opacity: 1; transform: translateX(0) scale(1); }
    }
    @keyframes present-fade {
      from { opacity: 0; }
      to { opacity: 1; }
    }
    @keyframes present-zoom {
      from { opacity: 0; transform: scale(.92); }
      to { opacity: 1; transform: scale(1); }
    }
    @media (prefers-reduced-motion: reduce) {
      .page.present-current { animation: none !important; }
    }
  `

  let { token }: Props = $props()

  let share = $state<PublicShareView | null>(null)
  let loading = $state(true)
  let error = $state('')
  let iframeEl = $state<HTMLIFrameElement | null>(null)
  let total = $state(0)
  let currentPage = $state(1)
  let transition = $state<TransitionId>('book')
  let isFullscreen = $state(false)

  let pages: HTMLElement[] = []
  let boundDoc: Document | null = null
  let touchStartX = 0
  let touchStartY = 0
  let suppressClickUntil = 0

  const progress = $derived(total ? (currentPage / total) * 100 : 0)
  const title = $derived(
    share ? `${share.period_label} · Presentasi` : 'Presentasi laporan',
  )

  onMount(() => {
    const saved = window.localStorage.getItem(TRANSITION_KEY)
    if (saved && transitions.some((item) => item.id === saved)) {
      transition = saved as TransitionId
    }
    void load()
    window.addEventListener('resize', applyScale)
    document.addEventListener('fullscreenchange', syncFullscreen)
    return () => {
      window.removeEventListener('resize', applyScale)
      document.removeEventListener('fullscreenchange', syncFullscreen)
    }
  })

  async function load(): Promise<void> {
    loading = true
    error = ''
    try {
      share = await publicShareApi.info(token)
    } catch (caught) {
      error = errorMessage(caught)
      loading = false
    }
  }

  function handleFrameLoad(): void {
    const doc = iframeEl?.contentDocument
    if (!doc) {
      error = 'Presentasi tidak dapat dimuat.'
      loading = false
      return
    }

    installStyles(doc)
    pages = Array.from(doc.querySelectorAll<HTMLElement>('.page'))
    total = pages.length
    if (!total) {
      error = 'Halaman presentasi tidak ditemukan. Tautan mungkin sudah tidak aktif.'
      loading = false
      return
    }

    if (boundDoc !== doc) {
      boundDoc = doc
      doc.addEventListener('keydown', handleKeydown)
      doc.addEventListener('click', onPageClick)
      doc.addEventListener('touchstart', onTouchStart, { passive: true })
      doc.addEventListener('touchend', onTouchEnd, { passive: true })
    }

    currentPage = 1
    applyCurrent('next', false)
    loading = false
    requestAnimationFrame(applyScale)
  }

  function installStyles(doc: Document): void {
    doc.getElementById(STYLE_ID)?.remove()
    const style = doc.createElement('style')
    style.id = STYLE_ID
    style.textContent = PRESENTATION_CSS
    doc.head.appendChild(style)
  }

  function applyCurrent(direction: Direction, animate = true): void {
    const target = pages[currentPage - 1]
    for (const page of pages) {
      if (page === target) continue
      page.classList.remove('present-current')
      delete page.dataset.presentTransition
      delete page.dataset.presentDirection
    }
    if (!target) return
    target.classList.add('present-current')
    if (animate && transition !== 'none') {
      target.dataset.presentTransition = transition
      target.dataset.presentDirection = direction
    } else {
      delete target.dataset.presentTransition
      delete target.dataset.presentDirection
    }
    applyScale()
  }

  function applyScale(): void {
    const doc = iframeEl?.contentDocument
    const page = pages[currentPage - 1]
    if (!doc || !page) return
    doc.documentElement.style.setProperty('--present-zoom', '1')
    void page.offsetWidth
    const width = page.offsetWidth
    const height = page.offsetHeight
    if (!width || !height) return
    const compact = window.innerWidth < 700
    const availableWidth = Math.max(160, window.innerWidth - (compact ? 24 : 64))
    const availableHeight = Math.max(160, window.innerHeight - (compact ? 230 : 190))
    const scale = Math.min(availableWidth / width, availableHeight / height, 1.75)
    doc.documentElement.style.setProperty(
      '--present-zoom',
      String(Math.max(0.1, Math.round(scale * 1000) / 1000)),
    )
  }

  function go(delta: number): void {
    if (!total) return
    const next = Math.min(total, Math.max(1, currentPage + delta))
    if (next === currentPage) return
    currentPage = next
    applyCurrent(delta > 0 ? 'next' : 'prev')
  }

  function replayTransition(): void {
    const page = pages[currentPage - 1]
    if (!page || transition === 'none') {
      if (page) {
        delete page.dataset.presentTransition
        delete page.dataset.presentDirection
      }
      return
    }
    page.classList.remove('present-current')
    void page.offsetWidth
    applyCurrent('next', true)
  }

  function changeTransition(value: string): void {
    transition = value as TransitionId
    window.localStorage.setItem(TRANSITION_KEY, transition)
    replayTransition()
  }

  function handleKeydown(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null
    const tag = target?.tagName?.toLowerCase() ?? ''
    if (
      tag === 'input' ||
      tag === 'textarea' ||
      tag === 'select' ||
      target?.isContentEditable
    ) {
      return
    }
    if ((tag === 'button' || tag === 'a') && (event.key === ' ' || event.key === 'Enter')) {
      return
    }

    if (event.key === 'ArrowRight' || event.key === 'ArrowDown' || event.key === 'PageDown' || event.key === ' ') {
      event.preventDefault()
      go(1)
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp' || event.key === 'PageUp') {
      event.preventDefault()
      go(-1)
    } else if (event.key === 'Home') {
      event.preventDefault()
      if (currentPage !== 1) {
        currentPage = 1
        applyCurrent('prev')
      }
    } else if (event.key === 'End') {
      event.preventDefault()
      if (currentPage !== total) {
        currentPage = total
        applyCurrent('next')
      }
    } else if (event.key.toLowerCase() === 'f') {
      event.preventDefault()
      void toggleFullscreen()
    }
  }

  function onPageClick(event: MouseEvent): void {
    if (!total || event.defaultPrevented || event.button !== 0) return
    if (Date.now() < suppressClickUntil) return
    const target = event.target as HTMLElement | null
    if (
      target?.closest(
        'a, button, input, select, textarea, label, summary, video, audio, [contenteditable="true"], [data-present-no-advance]',
      )
    ) {
      return
    }
    go(1)
  }

  function onTouchStart(event: TouchEvent): void {
    const touch = event.touches[0]
    if (!touch) return
    touchStartX = touch.clientX
    touchStartY = touch.clientY
  }

  function onTouchEnd(event: TouchEvent): void {
    const touch = event.changedTouches[0]
    if (!touch) return
    const dx = touch.clientX - touchStartX
    const dy = touch.clientY - touchStartY
    if (Math.abs(dx) > 55 && Math.abs(dx) > Math.abs(dy) * 1.3) {
      suppressClickUntil = Date.now() + 500
      go(dx < 0 ? 1 : -1)
    }
  }

  async function copyPresentationLink(): Promise<void> {
    if (await copyText(window.location.href)) {
      toast.success('Tautan presentasi disalin.')
    } else {
      toast.error('Tautan tidak dapat disalin.')
    }
  }

  async function toggleFullscreen(): Promise<void> {
    try {
      if (document.fullscreenElement) await document.exitFullscreen()
      else await document.documentElement.requestFullscreen()
    } catch {
      toast.error('Mode layar penuh tidak tersedia pada browser ini.')
    }
  }

  function syncFullscreen(): void {
    isFullscreen = Boolean(document.fullscreenElement)
    applyScale()
  }
</script>

<svelte:head>
  <title>{title} · Laporan Mingguan PBM</title>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 overflow-hidden bg-forest-950 text-white"
  style="background: radial-gradient(circle at 50% 0%, rgba(184,137,75,.16), transparent 55%), linear-gradient(160deg, #0c1a14, #07110d 65%, #0b1712);"
>
  {#if share && !error}
    <iframe
      bind:this={iframeEl}
      title={`Presentasi ${share.period_label}`}
      src={publicShareApi.previewUrl(token)}
      class="absolute inset-0 h-full w-full border-0 bg-transparent transition-opacity duration-300 {loading ? 'opacity-0' : 'opacity-100'}"
      onload={handleFrameLoad}
    ></iframe>
  {/if}

  {#if loading}
    <div class="absolute inset-0 z-10 flex flex-col items-center justify-center gap-4 bg-forest-950/70 backdrop-blur-sm">
      <span class="flex h-16 w-16 items-center justify-center rounded-2xl border border-gold-400/30 bg-white/5 p-2">
        <img src="/logo_polda_bali.png" alt="Logo Polda Bali" class="h-full w-full object-contain" />
      </span>
      <LoaderCircle size={22} class="animate-spin text-gold-300" />
      <p class="text-sm text-forest-100/70">Menyiapkan presentasi…</p>
    </div>
  {:else if error}
    <div class="absolute inset-0 z-20 flex items-center justify-center p-6">
      <div class="max-w-md rounded-3xl border border-white/10 bg-white/5 p-7 text-center shadow-2xl backdrop-blur">
        <span class="mx-auto flex h-14 w-14 items-center justify-center rounded-2xl bg-red-500/15 text-red-300">
          <AlertCircle size={26} />
        </span>
        <h1 class="mt-4 font-display text-xl font-semibold text-white">Presentasi tidak tersedia</h1>
        <p class="mt-2 text-sm leading-relaxed text-forest-100/65">{error}</p>
      </div>
    </div>
  {/if}

  {#if share && !error}
    <header class="absolute inset-x-0 top-0 z-30 bg-gradient-to-b from-black/75 via-black/35 to-transparent px-4 pb-12 pt-4 sm:px-6">
      <div class="mx-auto flex max-w-[1500px] flex-wrap items-center justify-between gap-3">
        <div class="flex min-w-0 items-center gap-3">
          <span class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/10 p-1.5 backdrop-blur">
            <img src="/logo_polda_bali.png" alt="Logo Polda Bali" class="h-full w-full object-contain" />
          </span>
          <div class="min-w-0">
            <p class="truncate text-sm font-semibold text-white">{share.period_label}</p>
            <p class="text-[10px] font-bold tracking-[0.16em] text-gold-200/75 uppercase">Presentasi laporan · Tautan publik</p>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="inline-flex items-center gap-2 rounded-full border border-white/15 bg-white/10 px-3 py-2 text-xs font-semibold text-white backdrop-blur transition hover:bg-white/20"
            onclick={copyPresentationLink}
            title="Salin tautan presentasi"
          >
            <Copy size={14} />
            <span class="hidden sm:inline">Salin tautan</span>
          </button>
          <a
            class="inline-flex items-center gap-2 rounded-full border border-white/15 bg-white/10 px-3 py-2 text-xs font-semibold text-white backdrop-blur transition hover:bg-white/20"
            href={publicShareApi.pdfUrl(token)}
            target="_blank"
            rel="noopener"
            title="Buka PDF"
          >
            <FileText size={14} />
            <span class="hidden sm:inline">PDF</span>
          </a>
          <button
            type="button"
            class="inline-flex items-center gap-2 rounded-full border border-white/15 bg-white/10 px-3 py-2 text-xs font-semibold text-white backdrop-blur transition hover:bg-white/20"
            onclick={() => void toggleFullscreen()}
            title={isFullscreen ? 'Keluar layar penuh' : 'Layar penuh'}
          >
            {#if isFullscreen}<Minimize2 size={14} />{:else}<Expand size={14} />{/if}
            <span class="hidden sm:inline">{isFullscreen ? 'Keluar' : 'Layar penuh'}</span>
          </button>
        </div>
      </div>
    </header>

    {#if total > 1}
      <button
        type="button"
        class="absolute top-1/2 left-2 z-30 hidden h-12 w-12 -translate-y-1/2 items-center justify-center rounded-full border border-white/10 bg-black/30 text-white backdrop-blur transition hover:bg-black/55 disabled:cursor-default disabled:opacity-25 sm:flex"
        aria-label="Halaman sebelumnya"
        disabled={currentPage <= 1}
        onclick={() => go(-1)}
      >
        <ChevronLeft size={26} />
      </button>
      <button
        type="button"
        class="absolute top-1/2 right-2 z-30 hidden h-12 w-12 -translate-y-1/2 items-center justify-center rounded-full border border-white/10 bg-black/30 text-white backdrop-blur transition hover:bg-black/55 disabled:cursor-default disabled:opacity-25 sm:flex"
        aria-label="Halaman berikutnya"
        disabled={currentPage >= total}
        onclick={() => go(1)}
      >
        <ChevronRight size={26} />
      </button>
    {/if}

    <footer class="absolute inset-x-0 bottom-0 z-30 bg-gradient-to-t from-black/80 via-black/35 to-transparent px-4 pb-4 pt-12 sm:px-6">
      <div class="mx-auto max-w-[1500px]">
        <div class="mb-2 h-0.5 overflow-hidden rounded-full bg-white/10">
          <div class="h-full rounded-full bg-gold-300 transition-all duration-300" style={`width:${progress}%`}></div>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="flex h-9 w-9 items-center justify-center rounded-full border border-white/15 bg-white/10 text-white transition hover:bg-white/20 disabled:cursor-default disabled:opacity-25"
              aria-label="Halaman sebelumnya"
              disabled={currentPage <= 1}
              onclick={() => go(-1)}
            >
              <ChevronLeft size={18} />
            </button>
            <span class="min-w-[110px] text-center text-[11px] font-semibold tracking-wide text-forest-100/80">
              Halaman {currentPage} / {total || '—'}
            </span>
            <button
              type="button"
              class="flex h-9 w-9 items-center justify-center rounded-full border border-white/15 bg-white/10 text-white transition hover:bg-white/20 disabled:cursor-default disabled:opacity-25"
              aria-label="Halaman berikutnya"
              disabled={currentPage >= total}
              onclick={() => go(1)}
            >
              <ChevronRight size={18} />
            </button>
          </div>

          <div class="flex items-center gap-2">
            <span class="hidden text-[10px] font-bold tracking-[0.14em] text-forest-100/45 uppercase sm:inline">Transisi</span>
            <select
              class="rounded-full border border-white/15 bg-white/10 px-3 py-1.5 text-[11px] font-semibold text-white outline-none transition hover:bg-white/20"
              value={transition}
              aria-label="Efek transisi halaman"
              onchange={(event) => changeTransition((event.currentTarget as HTMLSelectElement).value)}
            >
              {#each transitions as item}
                <option value={item.id} class="bg-forest-950 text-white">{item.label}</option>
              {/each}
            </select>
          </div>
        </div>
        <p class="mt-2 text-center text-[9px] tracking-[0.12em] text-forest-100/35 uppercase">
          Klik/ketuk halaman, panah / spasi, atau geser untuk berpindah · F untuk layar penuh
        </p>
      </div>
    </footer>
  {/if}
</div>
