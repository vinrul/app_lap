<script lang="ts">
  import { tick, untrack } from 'svelte'
  import {
    AlertCircle,
    Check,
    Copy,
    Eye,
    FileDown,
    ImagePlus,
    Link2,
    Redo2,
    Save,
    Square,
    Trash2,
    Type,
    Undo2,
  } from 'lucide-svelte'
  import { designApi, reportsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type {
    LayoutBlockOffset,
    LayoutElement,
    LayoutPage,
    PeriodDetail,
    ReportContent,
    ReportLayout,
  } from '../../types'
  import { emptyContent } from '../../utils/constants'
  import { formatDateTime } from '../../utils/format'
  import { toast } from '../../stores/toast.svelte'
  import Button from '../ui/Button.svelte'
  import EmptyState from '../ui/EmptyState.svelte'

  type Props = {
    periodId: string
    detail: PeriodDetail
    selectedImportId: string
    onchangeimport: (event: Event) => void
    ongenerated: () => Promise<void>
  }

  type Selection =
    | { kind: 'element'; page: number; id: string }
    | { kind: 'block'; page: number; id: string }
    | null

  type DragState = {
    type: 'block' | 'element' | 'resize'
    page: number
    id: string
    startX: number
    startY: number
    startDx: number
    startDy: number
    startElX: number
    startElY: number
    startElW: number
    startElH: number
    handle?: string
    moved: boolean
    historyPushed: boolean
    before: ReportLayout
  }

  const PAGE_WIDTH = 210
  const PAGE_HEIGHT = 297
  const MAX_HISTORY = 40

  let {
    periodId,
    detail,
    selectedImportId,
    onchangeimport,
    ongenerated,
  }: Props = $props()

  let iframeEl = $state<HTMLIFrameElement | null>(null)
  let iframeHtml = $state('')
  let fileInput = $state<HTMLInputElement | null>(null)
  let textAreaEl = $state<HTMLTextAreaElement | null>(null)

  let content = $state<ReportContent>({ ...emptyContent })
  let layout = $state<ReportLayout>(emptyLayout())
  let savedJson = $state('')
  let updatedAt = $state<string | null>(null)
  let updatedBy = $state<string | null>(null)
  let loading = $state(true)
  let saving = $state(false)
  let generating = $state(false)
  let previewing = $state(false)
  let uploading = $state(false)
  let error = $state('')
  let currentPage = $state(1)
  let selected = $state<Selection>(null)
  let history = $state<ReportLayout[]>([])
  let redoStack = $state<ReportLayout[]>([])

  let loadedKey = ''
  let boundDoc: Document | null = null
  let drag: DragState | null = null

  const hasImport = $derived(Boolean(selectedImportId))
  const dirty = $derived(JSON.stringify(layout) !== savedJson)
  const pageNumbers = Array.from({ length: 9 }, (_, index) => index + 1)
  const activeElement = $derived.by(() => {
    if (!selected || selected.kind !== 'element') return null
    return getElement(selected.page, selected.id)
  })
  const activeBlock = $derived.by(() => {
    if (!selected || selected.kind !== 'block') return null
    return getPage(selected.page)?.blocks[selected.id] ?? { dx: 0, dy: 0, hidden: false }
  })

  $effect(() => {
    const key = `${periodId}:${selectedImportId}`
    if (key === loadedKey) return
    loadedKey = key
    untrack(() => void load())
  })

  function emptyLayout(): ReportLayout {
    return {
      version: 1,
      pages: Array.from({ length: 9 }, (_, index) => ({
        page: index + 1,
        blocks: {},
        elements: [],
      })),
    }
  }

  function normalizeLayout(input: ReportLayout | null | undefined): ReportLayout {
    return {
      version: 1,
      pages: Array.from({ length: 9 }, (_, index) => {
        const found = input?.pages?.find((page) => page.page === index + 1)
        return {
          page: index + 1,
          blocks: { ...(found?.blocks ?? {}) },
          elements: (found?.elements ?? []).map((element) => ({ ...element })),
        }
      }),
    }
  }

  function cloneLayout(value: ReportLayout): ReportLayout {
    return JSON.parse(JSON.stringify(value)) as ReportLayout
  }

  function getPage(page: number): LayoutPage | undefined {
    return layout.pages.find((item) => item.page === page)
  }

  function getElement(page: number, id: string): LayoutElement | undefined {
    return getPage(page)?.elements.find((element) => element.id === id)
  }

  function newId(): string {
    if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
      return crypto.randomUUID()
    }
    return `el-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, Number.isFinite(value) ? value : min))
  }

  function clampElement(element: LayoutElement): LayoutElement {
    element.width = clamp(Number(element.width), 5, PAGE_WIDTH)
    element.height = clamp(Number(element.height), 3, PAGE_HEIGHT)
    element.x = clamp(Number(element.x), 0, PAGE_WIDTH - element.width)
    element.y = clamp(Number(element.y), 0, PAGE_HEIGHT - element.height)
    element.rotation = clamp(Number(element.rotation), -360, 360)
    element.opacity = clamp(Number(element.opacity), 0, 1)
    element.z = Math.round(Number(element.z) || 0)
    return element
  }

  function baseElement(type: LayoutElement['type']): LayoutElement {
    return {
      id: newId(),
      type,
      x: 0,
      y: 0,
      width: type === 'rect' ? 40 : 60,
      height: type === 'rect' ? 20 : type === 'image' ? 45 : 15,
      rotation: 0,
      opacity: 1,
      z: 10,
      hidden: false,
      src: '',
      alt: '',
      object_fit: 'contain',
      fill: '#ffffff',
      border_color: '#143d30',
      border_width: 0.5,
      radius: 1,
      text: '',
      font_size: 12,
      font_family: 'sans',
      color: '#143d30',
      bold: false,
      italic: false,
      align: 'left',
      background: 'transparent',
    }
  }

  async function load() {
    loading = true
    error = ''
    selected = null
    drag = null
    currentPage = 1
    history = []
    redoStack = []
    if (!selectedImportId) {
      content = { ...emptyContent }
      layout = emptyLayout()
      savedJson = ''
      updatedAt = null
      updatedBy = null
      iframeHtml = ''
      loading = false
      return
    }
    try {
      const [draft, layoutView] = await Promise.all([
        reportsApi.getDraft(periodId, selectedImportId),
        designApi.getLayout(periodId, selectedImportId),
      ])
      content = { ...emptyContent, ...draft.content }
      layout = normalizeLayout(layoutView.layout)
      savedJson = JSON.stringify(layout)
      updatedAt = layoutView.updated_at
      updatedBy = layoutView.updated_by
      const preview = await reportsApi.preview(periodId, selectedImportId, content)
      iframeHtml = preview.html
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  function resolveImageSrc(src: string | undefined): string {
    if (!src) return ''
    if (src.startsWith('upload:')) {
      return designApi.imageUrl(periodId, src.slice(7))
    }
    return src
  }

  function installEditorStyles(doc: Document) {
    if (doc.getElementById('laporan-editor-style')) return
    const style = doc.createElement('style')
    style.id = 'laporan-editor-style'
    style.textContent = `
      body { user-select: none; }
      [data-block] { cursor: move; }
      [data-block]:hover { outline: 1px dashed rgba(184,137,75,.55); outline-offset: 2px; }
      .editor-block-selected { outline: 2px dashed #b8894b !important; outline-offset: 2px; }
      .layout-element { cursor: move; }
      .layout-element:hover { outline: 1px dashed rgba(184,137,75,.7); }
      .editor-selection { position: absolute; border: 1.5px solid #b8894b; z-index: 9999; pointer-events: none; box-sizing: border-box; }
      .editor-handle { position: absolute; width: 10px; height: 10px; background: #fff; border: 1.5px solid #b8894b; border-radius: 2px; pointer-events: auto; box-sizing: border-box; }
      .editor-dragging * { cursor: grabbing !important; }
    `
    doc.head.appendChild(style)
  }

  function handleIframeLoad() {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    installEditorStyles(doc)
    if (boundDoc !== doc) {
      boundDoc = doc
      doc.addEventListener('pointerdown', onPointerDown)
      doc.addEventListener('keydown', handleKeydown)
    }
    applyLayoutToDom()
  }

  function createElementNode(doc: Document, element: LayoutElement): HTMLElement {
    let node: HTMLElement
    if (element.type === 'image') {
      const image = doc.createElement('img')
      image.src = resolveImageSrc(element.src)
      image.alt = element.alt ?? ''
      image.draggable = false
      image.style.objectFit = element.object_fit ?? 'contain'
      node = image
    } else if (element.type === 'rect') {
      const rect = doc.createElement('div')
      rect.style.background = element.fill ?? '#ffffff'
      rect.style.border = `${element.border_width ?? 0.5}pt solid ${element.border_color ?? '#143d30'}`
      rect.style.borderRadius = `${element.radius ?? 1}mm`
      node = rect
    } else {
      const text = doc.createElement('div')
      text.textContent = element.text ?? ''
      text.style.fontFamily =
        element.font_family === 'serif'
          ? "Georgia, 'Times New Roman', serif"
          : "'Segoe UI', Arial, sans-serif"
      text.style.fontSize = `${element.font_size ?? 12}pt`
      text.style.color = element.color ?? '#143d30'
      text.style.fontWeight = element.bold ? '700' : '400'
      text.style.fontStyle = element.italic ? 'italic' : 'normal'
      text.style.textAlign = element.align ?? 'left'
      text.style.background = element.background ?? 'transparent'
      text.style.whiteSpace = 'pre-wrap'
      text.style.padding = '.8mm'
      text.style.overflow = 'hidden'
      text.style.boxSizing = 'border-box'
      node = text
    }

    node.classList.add('layout-element', `layout-${element.type}`)
    node.dataset.elementId = element.id
    node.style.position = 'absolute'
    node.style.left = `${element.x}mm`
    node.style.top = `${element.y}mm`
    node.style.width = `${element.width}mm`
    node.style.height = `${element.height}mm`
    node.style.opacity = String(element.opacity ?? 1)
    node.style.zIndex = String(element.z ?? 0)
    if (element.rotation) node.style.transform = `rotate(${element.rotation}deg)`
    return node
  }

  function applyLayoutToDom() {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    const pages = Array.from(doc.querySelectorAll<HTMLElement>('.page'))
    pages.forEach((pageEl, index) => {
      const pageNo = index + 1
      const page = getPage(pageNo)
      if (!page) return

      pageEl.querySelectorAll<HTMLElement>('[data-block]').forEach((blockEl) => {
        const id = blockEl.dataset.block
        if (!id) return
        const offset = page.blocks[id]
        blockEl.classList.remove('editor-block-selected')
        if (offset?.hidden) {
          blockEl.style.display = 'none'
        } else {
          blockEl.style.removeProperty('display')
        }
        // Selalu set inline agar rule transform dari layout tersimpan tidak
        // menempel saat blok di-reset ke posisi awal.
        blockEl.style.transform = `translate(${offset?.dx ?? 0}mm, ${offset?.dy ?? 0}mm)`
      })

      pageEl.querySelectorAll('.editor-selection').forEach((node) => node.remove())
      let layer = pageEl.querySelector<HTMLElement>('.layout-layer')
      if (!layer) {
        layer = doc.createElement('div')
        layer.className = 'layout-layer'
        layer.dataset.layoutPage = String(pageNo)
        pageEl.appendChild(layer)
      }
      layer.innerHTML = ''
      for (const element of page.elements) {
        if (!element.hidden) layer.appendChild(createElementNode(doc, element))
      }
    })
    renderSelection()
  }

  function renderSelection() {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    doc.querySelectorAll('.editor-selection').forEach((node) => node.remove())
    doc.querySelectorAll('.editor-block-selected').forEach((node) => {
      node.classList.remove('editor-block-selected')
    })
    if (!selected) return
    const pageEl = doc.querySelectorAll<HTMLElement>('.page')[selected.page - 1]
    if (!pageEl) return

    if (selected.kind === 'block') {
      const blockEl = pageEl.querySelector<HTMLElement>(`[data-block="${selected.id}"]`)
      blockEl?.classList.add('editor-block-selected')
      return
    }

    const element = getElement(selected.page, selected.id)
    if (!element) {
      selected = null
      return
    }
    const frame = doc.createElement('div')
    frame.className = 'editor-selection'
    frame.style.left = `${element.x}mm`
    frame.style.top = `${element.y}mm`
    frame.style.width = `${element.width}mm`
    frame.style.height = `${element.height}mm`
    if (element.rotation) frame.style.transform = `rotate(${element.rotation}deg)`

    const handles: Record<string, [string, string]> = {
      nw: ['-5px', '-5px'],
      ne: ['calc(100% - 5px)', '-5px'],
      sw: ['-5px', 'calc(100% - 5px)'],
      se: ['calc(100% - 5px)', 'calc(100% - 5px)'],
    }
    for (const [handle, [left, top]] of Object.entries(handles)) {
      const node = doc.createElement('div')
      node.className = 'editor-handle'
      node.dataset.handle = handle
      node.style.left = left
      node.style.top = top
      node.style.cursor = handle === 'nw' || handle === 'se' ? 'nwse-resize' : 'nesw-resize'
      frame.appendChild(node)
    }
    pageEl.appendChild(frame)
  }

  function pageIndexOf(pageEl: HTMLElement): number {
    const doc = iframeEl?.contentDocument
    if (!doc) return 1
    return Array.from(doc.querySelectorAll<HTMLElement>('.page')).indexOf(pageEl) + 1
  }

  function beginDrag(state: DragState) {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    drag = state
    doc.body.classList.add('editor-dragging')
    doc.addEventListener('pointermove', onPointerMove)
    doc.addEventListener('pointerup', onPointerUp)
    doc.addEventListener('pointercancel', onPointerUp)
  }

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0) return
    const doc = iframeEl?.contentDocument
    if (!doc) return
    const target = event.target as HTMLElement | null
    if (!target) return
    const pageEl = target.closest<HTMLElement>('.page')
    if (!pageEl) return
    const page = pageIndexOf(pageEl)
    currentPage = page

    const handleEl = target.closest<HTMLElement>('[data-handle]')
    if (handleEl && selected?.kind === 'element') {
      const element = getElement(selected.page, selected.id)
      if (!element) return
      event.preventDefault()
      beginDrag({
        type: 'resize',
        page: selected.page,
        id: selected.id,
        startX: event.clientX,
        startY: event.clientY,
        startDx: 0,
        startDy: 0,
        startElX: element.x,
        startElY: element.y,
        startElW: element.width,
        startElH: element.height,
        handle: handleEl.dataset.handle,
        moved: false,
        historyPushed: false,
        before: cloneLayout(layout),
      })
      return
    }

    const elementEl = target.closest<HTMLElement>('[data-element-id]')
    if (elementEl) {
      const id = elementEl.dataset.elementId
      if (!id) return
      const element = getElement(page, id)
      if (!element) return
      selected = { kind: 'element', page, id }
      renderSelection()
      event.preventDefault()
      beginDrag({
        type: 'element',
        page,
        id,
        startX: event.clientX,
        startY: event.clientY,
        startDx: 0,
        startDy: 0,
        startElX: element.x,
        startElY: element.y,
        startElW: element.width,
        startElH: element.height,
        moved: false,
        historyPushed: false,
        before: cloneLayout(layout),
      })
      return
    }

    const blockEl = target.closest<HTMLElement>('[data-block]')
    if (blockEl) {
      const id = blockEl.dataset.block
      if (!id) return
      const offset = getPage(page)?.blocks[id] ?? { dx: 0, dy: 0, hidden: false }
      selected = { kind: 'block', page, id }
      renderSelection()
      event.preventDefault()
      beginDrag({
        type: 'block',
        page,
        id,
        startX: event.clientX,
        startY: event.clientY,
        startDx: offset.dx,
        startDy: offset.dy,
        startElX: 0,
        startElY: 0,
        startElW: 0,
        startElH: 0,
        moved: false,
        historyPushed: false,
        before: cloneLayout(layout),
      })
      return
    }

    selected = null
    renderSelection()
  }

  function onPointerMove(event: PointerEvent) {
    if (!drag) return
    const doc = iframeEl?.contentDocument
    if (!doc) return
    const pageEl = doc.querySelectorAll<HTMLElement>('.page')[drag.page - 1]
    if (!pageEl) return
    const mmPerPixel = PAGE_WIDTH / Math.max(pageEl.getBoundingClientRect().width, 1)
    const dx = (event.clientX - drag.startX) * mmPerPixel
    const dy = (event.clientY - drag.startY) * mmPerPixel
    if (!drag.moved && (Math.abs(dx) > 0.2 || Math.abs(dy) > 0.2)) {
      drag.moved = true
      if (!drag.historyPushed) {
        pushSnapshot(drag.before)
        drag.historyPushed = true
      }
    }
    if (!drag.moved) return
    event.preventDefault()

    if (drag.type === 'block') {
      const page = getPage(drag.page)
      const blockEl = pageEl.querySelector<HTMLElement>(`[data-block="${drag.id}"]`)
      if (!page || !blockEl) return
      const next: LayoutBlockOffset = {
        dx: Math.round((drag.startDx + dx) * 1000) / 1000,
        dy: Math.round((drag.startDy + dy) * 1000) / 1000,
      }
      page.blocks[drag.id] = next
      blockEl.style.transform = `translate(${next.dx}mm, ${next.dy}mm)`
      return
    }

    const element = getElement(drag.page, drag.id)
    if (!element) return
    const node = pageEl.querySelector<HTMLElement>(`[data-element-id="${drag.id}"]`)
    if (!node) return

    if (drag.type === 'element') {
      element.x = Math.round(clamp(drag.startElX + dx, 0, PAGE_WIDTH - element.width) * 1000) / 1000
      element.y = Math.round(clamp(drag.startElY + dy, 0, PAGE_HEIGHT - element.height) * 1000) / 1000
    } else {
      const handle = drag.handle ?? 'se'
      let x = drag.startElX
      let y = drag.startElY
      let width = drag.startElW
      let height = drag.startElH
      if (handle.includes('e')) width = drag.startElW + dx
      if (handle.includes('s')) height = drag.startElH + dy
      if (handle.includes('w')) {
        x = drag.startElX + dx
        width = drag.startElW - dx
      }
      if (handle.includes('n')) {
        y = drag.startElY + dy
        height = drag.startElH - dy
      }
      width = clamp(width, 5, PAGE_WIDTH)
      height = clamp(height, 3, PAGE_HEIGHT)
      x = clamp(x, 0, PAGE_WIDTH - width)
      y = clamp(y, 0, PAGE_HEIGHT - height)
      element.x = Math.round(x * 1000) / 1000
      element.y = Math.round(y * 1000) / 1000
      element.width = Math.round(width * 1000) / 1000
      element.height = Math.round(height * 1000) / 1000
    }

    node.style.left = `${element.x}mm`
    node.style.top = `${element.y}mm`
    node.style.width = `${element.width}mm`
    node.style.height = `${element.height}mm`
    const frame = pageEl.querySelector<HTMLElement>('.editor-selection')
    if (frame) {
      frame.style.left = `${element.x}mm`
      frame.style.top = `${element.y}mm`
      frame.style.width = `${element.width}mm`
      frame.style.height = `${element.height}mm`
    }
  }

  function onPointerUp() {
    const doc = iframeEl?.contentDocument
    if (doc) {
      doc.body.classList.remove('editor-dragging')
      doc.removeEventListener('pointermove', onPointerMove)
      doc.removeEventListener('pointerup', onPointerUp)
      doc.removeEventListener('pointercancel', onPointerUp)
    }
    drag = null
    applyLayoutToDom()
  }

  function pushSnapshot(snapshot: ReportLayout) {
    history.push(snapshot)
    if (history.length > MAX_HISTORY) history.shift()
    redoStack = []
  }

  function pushHistory() {
    pushSnapshot(cloneLayout(layout))
  }

  function undo() {
    if (!history.length) return
    redoStack.push(cloneLayout(layout))
    layout = history.pop()!
    selected = null
    applyLayoutToDom()
  }

  function redo() {
    if (!redoStack.length) return
    history.push(cloneLayout(layout))
    layout = redoStack.pop()!
    selected = null
    applyLayoutToDom()
  }

  function goToPage(page: number, scroll = true) {
    currentPage = page
    if (!scroll) return
    const doc = iframeEl?.contentDocument
    const pageEl = doc?.querySelectorAll<HTMLElement>('.page')[page - 1]
    pageEl?.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }

  function addElement(element: LayoutElement) {
    pushHistory()
    const page = getPage(currentPage)
    if (!page) return
    element.x = Math.round((PAGE_WIDTH - element.width) / 2 * 1000) / 1000
    element.y = Math.round((PAGE_HEIGHT - element.height) / 2 * 1000) / 1000
    clampElement(element)
    page.elements.push(element)
    selected = { kind: 'element', page: currentPage, id: element.id }
    applyLayoutToDom()
  }

  function addText() {
    const element = baseElement('text')
    element.text = 'Teks baru'
    element.width = 80
    element.height = 15
    addElement(element)
    void tick().then(() => textAreaEl?.focus())
  }

  function addRect() {
    const element = baseElement('rect')
    addElement(element)
  }

  function pickImage() {
    fileInput?.click()
  }

  async function handleFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    if (!['image/png', 'image/jpeg'].includes(file.type)) {
      toast.error('Gambar harus PNG atau JPEG.')
      return
    }
    if (file.size > 8 * 1024 * 1024) {
      toast.error('Ukuran gambar maksimal 8 MB.')
      return
    }
    uploading = true
    try {
      const uploaded = await designApi.uploadImage(periodId, file)
      const element = baseElement('image')
      element.src = `upload:${uploaded.id}`
      element.alt = file.name
      addElement(element)
      toast.success('Gambar ditambahkan ke halaman.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      uploading = false
    }
  }

  function addImageUrl() {
    const value = window.prompt('URL gambar (http atau https):')
    if (!value) return
    try {
      const url = new URL(value)
      if (url.protocol !== 'http:' && url.protocol !== 'https:') {
        toast.error('URL harus memakai http atau https.')
        return
      }
    } catch {
      toast.error('URL gambar tidak valid.')
      return
    }
    const element = baseElement('image')
    element.src = value
    element.alt = 'Gambar dari URL'
    addElement(element)
  }

  function deleteSelected() {
    if (!selected) return
    if (selected.kind === 'element') {
      const page = getPage(selected.page)
      if (!page) return
      pushHistory()
      page.elements = page.elements.filter((element) => element.id !== selected?.id)
      selected = null
      applyLayoutToDom()
      return
    }
    const page = getPage(selected.page)
    if (!page) return
    pushHistory()
    delete page.blocks[selected.id]
    selected = null
    applyLayoutToDom()
  }

  function duplicateSelected() {
    if (!selected || selected.kind !== 'element') return
    const page = getPage(selected.page)
    const element = getElement(selected.page, selected.id)
    if (!page || !element) return
    pushHistory()
    const copy: LayoutElement = {
      ...element,
      id: newId(),
      x: element.x + 8,
      y: element.y + 8,
      z: element.z + 1,
    }
    clampElement(copy)
    page.elements.push(copy)
    selected = { kind: 'element', page: selected.page, id: copy.id }
    applyLayoutToDom()
  }

  function moveLayer(delta: number) {
    if (!selected || selected.kind !== 'element') return
    const element = getElement(selected.page, selected.id)
    if (!element) return
    pushHistory()
    element.z = clamp(element.z + delta, -100, 1000)
    applyLayoutToDom()
  }

  function updateElement(patch: Partial<LayoutElement>, withHistory = false) {
    if (!selected || selected.kind !== 'element') return
    const element = getElement(selected.page, selected.id)
    if (!element) return
    if (withHistory) pushHistory()
    Object.assign(element, patch)
    clampElement(element)
    applyLayoutToDom()
  }

  function updateBlock(patch: Partial<LayoutBlockOffset>) {
    if (!selected || selected.kind !== 'block') return
    const page = getPage(selected.page)
    if (!page) return
    const current = page.blocks[selected.id] ?? { dx: 0, dy: 0 }
    page.blocks[selected.id] = { ...current, ...patch }
    applyLayoutToDom()
  }

  function resetBlock() {
    if (!selected || selected.kind !== 'block') return
    const page = getPage(selected.page)
    if (!page) return
    pushHistory()
    delete page.blocks[selected.id]
    applyLayoutToDom()
  }

  function pickerColor(value: string | undefined): string {
    return value && /^#[0-9a-fA-F]{6}$/.test(value) ? value : '#143d30'
  }

  function numberValue(value: string): number {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : 0
  }

  async function save(silent = false): Promise<boolean> {
    if (!selectedImportId) return false
    saving = true
    error = ''
    try {
      const view = await designApi.saveLayout(periodId, selectedImportId, layout)
      layout = normalizeLayout(view.layout)
      savedJson = JSON.stringify(layout)
      updatedAt = view.updated_at
      updatedBy = view.updated_by
      selected = null
      applyLayoutToDom()
      if (!silent) toast.success('Layout desain tersimpan.')
      return true
    } catch (caught) {
      error = errorMessage(caught)
      return false
    } finally {
      saving = false
    }
  }

  async function preview() {
    if (!selectedImportId) return
    if (dirty && !(await save(true))) return
    const target = window.open('', '_blank')
    if (!target) {
      toast.error('Izinkan popup untuk membuka pratinjau.')
      return
    }
    previewing = true
    try {
      const result = await reportsApi.preview(periodId, selectedImportId, content)
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
    if (!selectedImportId) return
    if (dirty && !(await save(true))) return
    generating = true
    error = ''
    try {
      const report = await reportsApi.generate(periodId, selectedImportId, content)
      toast.success('PDF laporan berhasil dibuat dengan layout desain.')
      await ongenerated()
      window.open(reportsApi.previewUrl(report.id), '_blank', 'noopener')
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      generating = false
    }
  }

  function nudge(dx: number, dy: number) {
    if (!selected) return
    pushHistory()
    if (selected.kind === 'element') {
      const element = getElement(selected.page, selected.id)
      if (!element) return
      element.x = clamp(element.x + dx, 0, PAGE_WIDTH - element.width)
      element.y = clamp(element.y + dy, 0, PAGE_HEIGHT - element.height)
    } else {
      const page = getPage(selected.page)
      if (!page) return
      const current = page.blocks[selected.id] ?? { dx: 0, dy: 0 }
      page.blocks[selected.id] = { ...current, dx: current.dx + dx, dy: current.dy + dy }
    }
    applyLayoutToDom()
  }

  function handleKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null
    const tag = target?.tagName?.toLowerCase()
    if (tag === 'input' || tag === 'textarea' || tag === 'select') return
    const mod = event.ctrlKey || event.metaKey
    if (mod && event.key.toLowerCase() === 's') {
      event.preventDefault()
      void save()
      return
    }
    if (mod && event.key.toLowerCase() === 'z') {
      event.preventDefault()
      if (event.shiftKey) redo()
      else undo()
      return
    }
    if (event.key === 'Delete' || event.key === 'Backspace') {
      if (selected) {
        event.preventDefault()
        deleteSelected()
      }
      return
    }
    if (event.key === 'Escape') {
      selected = null
      renderSelection()
      return
    }
    const step = event.shiftKey ? 5 : 1
    if (event.key === 'ArrowLeft') {
      event.preventDefault()
      nudge(-step, 0)
    } else if (event.key === 'ArrowRight') {
      event.preventDefault()
      nudge(step, 0)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      nudge(0, -step)
    } else if (event.key === 'ArrowDown') {
      event.preventDefault()
      nudge(0, step)
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if !hasImport}
  <section class="card">
    <EmptyState
      title="Belum ada versi data"
      description="Impor workbook terlebih dahulu sebelum menata desain laporan."
    >
      <Button onclick={() => (window.location.hash = `/periods/${periodId}/imports`)}>Impor workbook</Button>
    </EmptyState>
  </section>
{:else}
  <section class="card mb-4 flex flex-wrap items-center justify-between gap-3 px-5 py-4">
    <div>
      <div class="flex flex-wrap items-center gap-2">
        <span class="eyebrow">EDITOR DESAIN</span>
        {#if dirty}
          <span class="badge badge-progress">Belum disimpan</span>
        {:else}
          <span class="badge badge-success"><Check size={11} /> Tersimpan</span>
        {/if}
      </div>
      <p class="mt-1 text-xs text-ink-muted">
        {#if updatedAt}
          Terakhir disimpan {formatDateTime(updatedAt)}{#if updatedBy} oleh {updatedBy}{/if}
        {:else}
          Belum pernah disimpan
        {/if}
      </p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      {#if detail.imports.length > 1}
        <select class="input max-w-xs py-2 text-xs" value={selectedImportId} onchange={onchangeimport}>
          {#each detail.imports as item}<option value={item.id}>{item.filename}</option>{/each}
        </select>
      {/if}
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

  <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
    <section class="card overflow-hidden">
      <div class="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2.5">
        <Button variant="secondary" size="sm" onclick={addText}><Type size={14} /> Teks</Button>
        <Button variant="secondary" size="sm" onclick={addRect}><Square size={14} /> Kotak</Button>
        <Button variant="secondary" size="sm" loading={uploading} onclick={pickImage}><ImagePlus size={14} /> Gambar</Button>
        <Button variant="secondary" size="sm" onclick={addImageUrl}><Link2 size={14} /> URL</Button>
        <span class="mx-1 h-6 w-px bg-line"></span>
        <Button variant="ghost" size="sm" disabled={!history.length} onclick={undo}><Undo2 size={14} /> Undo</Button>
        <Button variant="ghost" size="sm" disabled={!redoStack.length} onclick={redo}><Redo2 size={14} /> Redo</Button>
        <span class="ml-auto text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Halaman</span>
        <select class="input max-w-[130px] py-1.5 text-xs" value={currentPage} onchange={(event) => goToPage(Number((event.currentTarget as HTMLSelectElement).value))}>
          {#each pageNumbers as page}
            <option value={page}>Halaman {page}</option>
          {/each}
        </select>
        <input bind:this={fileInput} class="hidden" type="file" accept="image/png,image/jpeg" onchange={handleFile} />
      </div>

      <div class="max-h-[76vh] overflow-auto bg-stone-100 p-3">
        {#if loading}
          <div class="flex h-[72vh] items-center justify-center text-sm text-ink-muted">Menyiapkan kanvas laporan…</div>
        {:else}
          <iframe
            bind:this={iframeEl}
            title="Editor tata letak laporan"
            class="mx-auto block h-[72vh] w-[900px] rounded-lg border border-line bg-white"
            srcdoc={iframeHtml}
            onload={handleIframeLoad}
          ></iframe>
        {/if}
      </div>

      <div class="border-t border-line bg-stone-50/60 px-4 py-2.5 text-[10px] leading-relaxed text-ink-muted">
        Klik halaman untuk memilih tujuan elemen baru. Tarik blok laporan atau elemen untuk menggeser.
        Gunakan sudut seleksi untuk mengubah ukuran. Perubahan baru masuk ke PDF setelah disimpan.
      </div>
    </section>

    <aside class="space-y-4 self-start xl:sticky xl:top-24">
      {#if activeElement}
        <section class="card overflow-hidden">
          <div class="panel-heading">
            <div>
              <span class="eyebrow">ELEMEN TERPILIH</span>
              <h2 class="panel-title mt-1">
                {activeElement.type === 'text' ? 'Teks' : activeElement.type === 'rect' ? 'Kotak' : 'Gambar'}
              </h2>
            </div>
            <span class="badge badge-neutral">Halaman {selected?.page}</span>
          </div>

          <div class="space-y-4 p-5">
            <div class="grid grid-cols-2 gap-3">
              <label class="block">
                <span class="label">X (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.x}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ x: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Y (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.y}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ y: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Lebar (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.width}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ width: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Tinggi (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.height}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ height: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Rotasi (°)</span>
                <input class="input py-1.5 text-xs" type="number" step="1" value={activeElement.rotation}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ rotation: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Opacity</span>
                <input class="input py-1.5 text-xs" type="number" min="0" max="1" step="0.05" value={activeElement.opacity}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ opacity: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
            </div>

            {#if activeElement.type === 'text'}
              <label class="block">
                <span class="label">Teks</span>
                <textarea bind:this={textAreaEl} class="textarea text-xs" rows="4" value={activeElement.text ?? ''}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ text: (event.currentTarget as HTMLTextAreaElement).value })}
                ></textarea>
              </label>
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Ukuran (pt)</span>
                  <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.font_size ?? 12}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ font_size: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
                <label class="block">
                  <span class="label">Font</span>
                  <select class="input py-1.5 text-xs" value={activeElement.font_family ?? 'sans'}
                    onchange={(event) => updateElement({ font_family: (event.currentTarget as HTMLSelectElement).value as 'sans' | 'serif' }, true)}>
                    <option value="sans">Sans</option>
                    <option value="serif">Serif</option>
                  </select>
                </label>
                <label class="block">
                  <span class="label">Warna</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.color)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Perataan</span>
                  <select class="input py-1.5 text-xs" value={activeElement.align ?? 'left'}
                    onchange={(event) => updateElement({ align: (event.currentTarget as HTMLSelectElement).value as 'left' | 'center' | 'right' }, true)}>
                    <option value="left">Kiri</option>
                    <option value="center">Tengah</option>
                    <option value="right">Kanan</option>
                  </select>
                </label>
              </div>
              <div class="flex flex-wrap items-center gap-4 text-xs">
                <label class="flex items-center gap-2"><input type="checkbox" checked={activeElement.bold}
                  onchange={(event) => updateElement({ bold: (event.currentTarget as HTMLInputElement).checked }, true)} /> Tebal</label>
                <label class="flex items-center gap-2"><input type="checkbox" checked={activeElement.italic}
                  onchange={(event) => updateElement({ italic: (event.currentTarget as HTMLInputElement).checked }, true)} /> Miring</label>
              </div>
              <label class="block">
                <span class="label">Latar</span>
                <input class="input py-1.5 text-xs" value={activeElement.background ?? 'transparent'} placeholder="transparent atau #ffffff"
                  onfocus={pushHistory}
                  onchange={(event) => updateElement({ background: (event.currentTarget as HTMLInputElement).value }, true)} />
              </label>
            {:else if activeElement.type === 'rect'}
              <label class="block">
                <span class="label">Isi</span>
                <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.fill)}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ fill: (event.currentTarget as HTMLInputElement).value })} />
              </label>
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Garis</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.border_color)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ border_color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Tebal garis (pt)</span>
                  <input class="input py-1.5 text-xs" type="number" min="0" step="0.25" value={activeElement.border_width ?? 0.5}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ border_width: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
                <label class="block">
                  <span class="label">Sudut (mm)</span>
                  <input class="input py-1.5 text-xs" type="number" min="0" step="0.5" value={activeElement.radius ?? 1}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ radius: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
              </div>
            {:else}
              <label class="block">
                <span class="label">Sumber gambar</span>
                <input class="input py-1.5 text-xs" value={activeElement.src ?? ''}
                  onfocus={pushHistory}
                  onchange={(event) => updateElement({ src: (event.currentTarget as HTMLInputElement).value }, true)} />
              </label>
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Object fit</span>
                  <select class="input py-1.5 text-xs" value={activeElement.object_fit ?? 'contain'}
                    onchange={(event) => updateElement({ object_fit: (event.currentTarget as HTMLSelectElement).value as 'contain' | 'cover' | 'fill' }, true)}>
                    <option value="contain">Contain</option>
                    <option value="cover">Cover</option>
                    <option value="fill">Fill</option>
                  </select>
                </label>
                <label class="block">
                  <span class="label">Alt</span>
                  <input class="input py-1.5 text-xs" value={activeElement.alt ?? ''}
                    onfocus={pushHistory}
                    onchange={(event) => updateElement({ alt: (event.currentTarget as HTMLInputElement).value }, true)} />
                </label>
              </div>
            {/if}

            <div class="flex flex-wrap gap-2 border-t border-line pt-4">
              <Button variant="secondary" size="sm" onclick={duplicateSelected}><Copy size={14} /> Duplikat</Button>
              <Button variant="secondary" size="sm" onclick={() => moveLayer(1)}>Naik</Button>
              <Button variant="secondary" size="sm" onclick={() => moveLayer(-1)}>Turun</Button>
              <Button variant="danger" size="sm" onclick={deleteSelected}><Trash2 size={14} /> Hapus</Button>
            </div>
          </div>
        </section>
      {:else if activeBlock}
        <section class="card overflow-hidden">
          <div class="panel-heading">
            <div>
              <span class="eyebrow">BLOK LAPORAN</span>
              <h2 class="panel-title mt-1">Geser blok</h2>
            </div>
            <span class="badge badge-neutral">Halaman {selected?.page}</span>
          </div>
          <div class="space-y-4 p-5">
            <p class="text-xs leading-relaxed text-ink-muted">
              Blok ini bagian dari laporan otomatis. Geser dengan mouse atau atur offset berikut.
            </p>
            <div class="grid grid-cols-2 gap-3">
              <label class="block">
                <span class="label">Geser X (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeBlock.dx}
                  onfocus={pushHistory}
                  oninput={(event) => updateBlock({ dx: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Geser Y (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeBlock.dy}
                  onfocus={pushHistory}
                  oninput={(event) => updateBlock({ dy: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
            </div>
            <p class="break-all text-[10px] text-ink-muted">ID: {selected?.id}</p>
            <Button variant="secondary" size="sm" onclick={resetBlock}>Reset posisi</Button>
          </div>
        </section>
      {:else}
        <section class="card p-5">
          <span class="eyebrow">CARA PAKAI</span>
          <ul class="mt-3 space-y-2 text-[11px] leading-relaxed text-ink-muted">
            <li>1. Klik halaman yang ingin ditata.</li>
            <li>2. Tambah teks, kotak, gambar, atau URL.</li>
            <li>3. Tarik elemen untuk menggeser; tarik sudut untuk mengubah ukuran.</li>
            <li>4. Klik blok laporan yang ada untuk menggeser posisinya.</li>
            <li>5. Simpan, lalu Buat PDF.</li>
          </ul>
        </section>
      {/if}
    </aside>
  </div>
{/if}
