<script lang="ts">
  import { tick, untrack } from 'svelte'
  import {
    AlertCircle,
    AlignLeft,
    ArrowDown,
    ArrowUp,
    BarChart3,
    Check,
    ChevronLeft,
    ChevronRight,
    Copy,
    CreditCard,
    Eye,
    EyeOff,
    FileDown,
    FilePlus2,
    ImagePlus,
    Layers,
    Link2,
    Maximize2,
    Minus,
    Plus,
    Redo2,
    Save,
    Shapes,
    Star,
    Table2,
    Trash2,
    Type,
    Undo2,
  } from 'lucide-svelte'
  import { designApi, reportsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type {
    DesignSource,
    DesignTemplateView,
    LayoutBlockOffset,
    LayoutElement,
    LayoutPage,
    PeriodDetail,
    ReportContent,
    ReportLayout,
  } from '../../types'
  import { iconByName, iconCatalog, type IconDefinition } from '../../icons/catalog'
  import { fontOptions, fontStack } from '../../fonts/catalog'
  import { shapeCatalog, shapeDefinition, shapeLabel, type ShapeDefinition } from '../../shapes/catalog'
  import { emptyContent } from '../../utils/constants'
  import { formatDateTime } from '../../utils/format'
  import { toast } from '../../stores/toast.svelte'
  import { dialog } from '../../stores/dialog.svelte'
  import Button from '../ui/Button.svelte'
  import EmptyState from '../ui/EmptyState.svelte'
  import Modal from '../ui/Modal.svelte'

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
    type: 'block' | 'block-resize' | 'element' | 'resize'
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
    groupStarts?: { id: string; x: number; y: number }[]
  }

  const PAGE_PORTRAIT_WIDTH_PX = 793.7
  const PAGE_PORTRAIT_HEIGHT_PX = 1122.5
  const MIN_ZOOM = 0.25
  const MAX_ZOOM = 2
  const MAX_HISTORY = 40
  const MAX_BLANK_PAGES = 30

  let {
    periodId,
    detail,
    selectedImportId,
    onchangeimport,
    ongenerated,
  }: Props = $props()

  let iframeEl = $state<HTMLIFrameElement | null>(null)
  let canvasAreaEl = $state<HTMLDivElement | null>(null)
  let iframeHtml = $state('')
  let canvasWidth = $state(PAGE_PORTRAIT_WIDTH_PX)
  let canvasHeight = $state(PAGE_PORTRAIT_HEIGHT_PX)
  let canvasZoom = $state(1)
  let fitZoom = $state(1)
  let zoomOverride = $state<number | null>(null)
  let fileInput = $state<HTMLInputElement | null>(null)
  let textAreaEl = $state<HTMLTextAreaElement | null>(null)

  let content = $state<ReportContent>({ ...emptyContent })
  let layout = $state<ReportLayout>(emptyLayout())
  const isBlank = $derived(layout.mode === 'blank')
  const PAGE_WIDTH = $derived(layout.orientation === 'landscape' ? 297 : 210)
  const PAGE_HEIGHT = $derived(layout.orientation === 'landscape' ? 210 : 297)
  const PAGE_WIDTH_PX = $derived((PAGE_WIDTH / 25.4) * 96)
  const PAGE_HEIGHT_PX = $derived((PAGE_HEIGHT / 25.4) * 96)
  let blankOrientation = $state<'portrait' | 'landscape'>('portrait')
  let blankPageCount = $state(1)
  let sources = $state<DesignSource[]>([])
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
  let checkedIds = $state<string[]>([])
  let history = $state<ReportLayout[]>([])
  let redoStack = $state<ReportLayout[]>([])
  let templates = $state<DesignTemplateView[]>([])
  let templateName = $state('')
  let selectedTemplateId = $state('')
  let templateBusy = $state(false)
  let iconPickerOpen = $state(false)
  let shapePickerOpen = $state(false)
  let iconPickerTarget = $state<'new' | 'replace'>('new')
  let iconSearch = $state('')
  let iconCategory = $state('Semua')

  let loadedKey = ''
  let canvasRequest = 0
  let boundDoc: Document | null = null
  let drag: DragState | null = null
  const defaultTexts = new Map<string, string>()

  const hasImport = $derived(Boolean(selectedImportId))
  const dirty = $derived(JSON.stringify(layout) !== savedJson)
  const pageNumbers = $derived(Array.from({ length: layout.pages.length }, (_, index) => index + 1))
  const zoomPercent = $derived(Math.round((zoomOverride ?? fitZoom) * 100))
  const atFitZoom = $derived(zoomOverride === null)
  const canZoomOut = $derived((zoomOverride ?? fitZoom) > MIN_ZOOM + 0.001)
  const canZoomIn = $derived((zoomOverride ?? fitZoom) < MAX_ZOOM - 0.001)
  const iconCategories = ['Semua', ...new Set(iconCatalog.map((icon) => icon.category))]
  const systemFontOptions = fontOptions.filter((font) => font.category === 'Sistem')
  const sansFontOptions = fontOptions.filter((font) => font.category === 'Sans')
  const serifFontOptions = fontOptions.filter((font) => font.category === 'Serif')
  const iconPickerTitle = $derived(iconPickerTarget === 'replace' ? 'Ganti ikon' : 'Tambah ikon')
  const filteredIcons = $derived.by(() => {
    const query = iconSearch.trim().toLowerCase()
    return iconCatalog.filter((icon) => {
      if (iconCategory !== 'Semua' && icon.category !== iconCategory) return false
      if (!query) return true
      return icon.label.toLowerCase().includes(query) || icon.name.includes(query)
    })
  })
  const activeElement = $derived.by(() => {
    if (!selected || selected.kind !== 'element') return null
    return getElement(selected.page, selected.id)
  })
  const activeBlock = $derived.by(() => {
    if (!selected || selected.kind !== 'block') return null
    return getPage(selected.page)?.blocks[selected.id] ?? { dx: 0, dy: 0, hidden: false }
  })
  const rectNoFill = $derived(
    activeElement?.type === 'rect' && isTransparentColor(activeElement.fill),
  )
  const rectNoStroke = $derived(
    activeElement?.type === 'rect' &&
      ((activeElement.border_width ?? 0) <= 0 || isTransparentColor(activeElement.border_color)),
  )
  const hiddenBlocks = $derived.by(() => {
    const page = getPage(currentPage)
    if (!page) return []
    return Object.entries(page.blocks)
      .filter(([, block]) => block.hidden && !block.preserve_space)
      .map(([id]) => id)
  })
  const activeBlockDetachKind = $derived.by(() => {
    if (!selected || selected.kind !== 'block') return ''
    const doc = iframeEl?.contentDocument
    const pageEl = doc?.querySelectorAll<HTMLElement>('.page')[selected.page - 1]
    const blockEl = pageEl?.querySelector<HTMLElement>(`[data-block="${selected.id}"]`)
    if (!blockEl) return ''
    if (blockEl.querySelector('.kpi')) return 'cards'
    if (blockEl.querySelector('table')) return 'table'
    if (blockEl.querySelector('.bar-row')) return 'chart'
    return ''
  })
  const tableSourceGroups = $derived(groupSources(sources))
  const chartSourceGroups = $derived(groupSources(sources.filter((source) => source.can_chart)))
  const pageElements = $derived.by(() => {
    const page = getPage(currentPage)
    if (!page) return []
    return [...page.elements].sort((left, right) => right.z - left.z)
  })
  const checkedHasGroup = $derived.by(() =>
    pageElements.some((element) => checkedIds.includes(element.id) && Boolean(element.group)),
  )

  $effect(() => {
    const key = `${periodId}:${selectedImportId}`
    if (key === loadedKey) return
    loadedKey = key
    untrack(() => void load())
  })

  $effect(() => {
    const node = canvasAreaEl
    if (!node) return
    const observer = new ResizeObserver(() => updateCanvasSize())
    observer.observe(node)
    updateCanvasSize()
    return () => observer.disconnect()
  })

  $effect(() => {
    void PAGE_WIDTH_PX
    void PAGE_HEIGHT_PX
    untrack(() => updateCanvasSize())
  })

  function emptyLayout(): ReportLayout {
    return {
      version: 1,
      mode: 'report',
      orientation: 'portrait',
      pages: Array.from({ length: 9 }, (_, index) => ({
        page: index + 1,
        blocks: {},
        elements: [],
      })),
    }
  }

  function normalizeLayout(input: ReportLayout | null | undefined): ReportLayout {
    const mode = input?.mode === 'blank' ? 'blank' : 'report'
    const orientation =
      mode === 'blank' && input?.orientation === 'landscape' ? 'landscape' : 'portrait'
    if (mode === 'blank') {
      const source = [...(input?.pages ?? [])]
        .sort((a, b) => a.page - b.page)
        .slice(0, MAX_BLANK_PAGES)
      const pages = source.map((page, index) => ({
        page: index + 1,
        blocks: {},
        elements: (page.elements ?? []).map((element) => ({ ...element })),
      }))
      return {
        version: 1,
        mode,
        orientation,
        pages: pages.length ? pages : [{ page: 1, blocks: {}, elements: [] }],
      }
    }
    return {
      version: 1,
      mode: 'report',
      orientation: 'portrait',
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

  function sourceFor(key: string | undefined | null): DesignSource | undefined {
    if (!key) return undefined
    return sources.find((source) => source.key === key)
  }

  function groupSources(list: DesignSource[]): { category: string; items: DesignSource[] }[] {
    const groups: { category: string; items: DesignSource[] }[] = []
    for (const source of list) {
      let group = groups.find((item) => item.category === source.category)
      if (!group) {
        group = { category: source.category, items: [] }
        groups.push(group)
      }
      group.items.push(source)
    }
    return groups
  }

  function applySourceSnapshot(element: LayoutElement, source: DesignSource): void {
    if (element.type === 'table') {
      element.table_rows = source.table_rows.map((row) => [...row])
      return
    }
    if (element.type === 'chart') {
      element.chart_labels = [...source.chart_labels]
      element.chart_values = [...source.chart_values]
      const colors = element.chart_colors ?? []
      element.chart_colors = source.chart_labels.map(
        (_, index) => colors[index] ?? CHART_PALETTE[index % CHART_PALETTE.length],
      )
      if (!element.chart_title?.trim()) element.chart_title = source.chart_title
    }
  }

  function bindSource(key: string): void {
    if (!activeElement) return
    const current = activeElement.binding ?? ''
    if (current === key) return
    const source = key ? sourceFor(key) : undefined
    if (key && !source) return
    pushHistory()
    if (!source) {
      const previous = sourceFor(current)
      if (previous) applySourceSnapshot(activeElement, previous)
      activeElement.binding = ''
      applyLayoutToDom()
      return
    }
    activeElement.binding = source.key
    applySourceSnapshot(activeElement, source)
    applyLayoutToDom()
  }

  async function refreshSources(): Promise<void> {
    if (!selectedImportId) return
    try {
      sources = await designApi.sources(periodId, selectedImportId)
      applyLayoutToDom()
      toast.success('Sumber data dimuat ulang.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    }
  }

  function elementLabel(element: LayoutElement): string {
    const type =
      element.type === 'text'
        ? 'Teks'
        : element.type === 'rect'
          ? shapeLabel(element.shape)
          : element.type === 'table'
            ? 'Tabel'
            : element.type === 'chart'
              ? 'Grafik'
              : element.type === 'card'
                ? 'Kartu'
                : element.type === 'icon'
                  ? 'Ikon'
                  : 'Gambar'
  const detail =
    element.type === 'text'
      ? (element.text ?? '').replace(/\s+/g, ' ').trim()
      : element.type === 'table'
        ? (element.table_rows?.[0] ?? []).join(' · ')
        : element.type === 'chart'
          ? (element.chart_title ?? '').trim()
          : element.type === 'card'
            ? (element.card_label ?? '').trim()
            : element.type === 'icon'
              ? (iconByName.get(element.icon_name ?? '')?.label ?? '')
              : (element.alt ?? '').trim()
    const summary = detail.length > 32 ? `${detail.slice(0, 31)}…` : detail
    return summary ? `${type} — ${summary}` : type
  }

  function toggleChecked(id: string): void {
    checkedIds = checkedIds.includes(id)
      ? checkedIds.filter((item) => item !== id)
      : [...checkedIds, id]
  }

  function selectLayer(id: string): void {
    const element = getElement(currentPage, id)
    if (!element) return
    selected = { kind: 'element', page: currentPage, id }
    renderSelection()
  }

  function toggleElementHidden(id: string): void {
    const element = getElement(currentPage, id)
    if (!element) return
    pushHistory()
    element.hidden = !element.hidden
    applyLayoutToDom()
  }

  function toggleSelectedHidden(): void {
    if (!selected || selected.kind !== 'element') return
    toggleElementHidden(selected.id)
  }

  function moveLayerOf(id: string, delta: number): void {
    const element = getElement(currentPage, id)
    if (!element) return
    pushHistory()
    element.z = clamp(element.z + delta, -100, 1000)
    applyLayoutToDom()
  }

  function groupChecked(): void {
    const page = getPage(currentPage)
    if (!page || checkedIds.length < 2) return
    const group = newId()
    pushHistory()
    for (const element of page.elements) {
      if (checkedIds.includes(element.id)) element.group = group
    }
    toast.success(`${checkedIds.length} objek digabungkan.`)
    checkedIds = []
    applyLayoutToDom()
  }

  function ungroupChecked(): void {
    const page = getPage(currentPage)
    if (!page) return
    const groups = new Set(
      page.elements
        .filter((element) => checkedIds.includes(element.id) && element.group)
        .map((element) => element.group as string),
    )
    if (!groups.size) return
    pushHistory()
    for (const element of page.elements) {
      if (element.group && groups.has(element.group)) element.group = ''
    }
    checkedIds = []
    applyLayoutToDom()
  }

  function groupMembers(page: LayoutPage, element: LayoutElement): LayoutElement[] {
    if (!element.group) return [element]
    const members = page.elements.filter((item) => item.group === element.group)
    return members.length > 1 ? members : [element]
  }

  async function saveAsTemplate(): Promise<void> {
    const name = templateName.trim()
    if (!name) return
    templateBusy = true
    try {
      const saved = await designApi.saveTemplate(name, layout)
      templates = [saved, ...templates]
      selectedTemplateId = saved.id
      templateName = ''
      toast.success('Template desain tersimpan.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      templateBusy = false
    }
  }

  async function applySelectedTemplate(): Promise<void> {
    if (!selectedImportId || !selectedTemplateId) return
    const template = templates.find((item) => item.id === selectedTemplateId)
    const label = template?.name ?? 'ini'
    const confirmed = await dialog.confirm({
      title: 'Terapkan template?',
      message: `Desain saat ini akan diganti dengan template "${label}". Perubahan masih bisa di-undo.`,
      confirmLabel: 'Terapkan',
    })
    if (!confirmed) return
    templateBusy = true
    const before = cloneLayout(layout)
    try {
      const view = await designApi.applyTemplate(periodId, selectedImportId, selectedTemplateId)
      pushSnapshot(before)
      layout = normalizeLayout(view.layout)
      savedJson = JSON.stringify(layout)
      updatedAt = view.updated_at
      updatedBy = view.updated_by
      selected = null
      checkedIds = []
      applyLayoutToDom()
      void refreshCanvasStructure()
      toast.success('Template diterapkan ke versi impor ini.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      templateBusy = false
    }
  }

  async function deleteSelectedTemplate(): Promise<void> {
    if (!selectedTemplateId) return
    const template = templates.find((item) => item.id === selectedTemplateId)
    const confirmed = await dialog.confirm({
      title: 'Hapus template?',
      message: `Template "${template?.name ?? 'ini'}" akan dihapus secara permanen.`,
      confirmLabel: 'Hapus',
      tone: 'danger',
    })
    if (!confirmed) return
    templateBusy = true
    try {
      await designApi.deleteTemplate(selectedTemplateId)
      templates = templates.filter((item) => item.id !== selectedTemplateId)
      selectedTemplateId = ''
      toast.success('Template dihapus.')
    } catch (caught) {
      toast.error(errorMessage(caught))
    } finally {
      templateBusy = false
    }
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
      shape: 'rect',
      fill: type === 'card' ? '#f6f8f5' : '#ffffff',
      fill_mode: 'solid',
      gradient_angle: 90,
      gradient_colors: [],
      border_color: '#143d30',
      border_width: type === 'card' ? 2 : 0.5,
      radius: type === 'card' ? 1.5 : 1,
      text: '',
      font_size: type === 'card' ? 9 : 12,
      font_family: 'sans',
      color: '#143d30',
      bold: false,
      italic: false,
      align: 'left',
      valign: 'top',
      background: 'transparent',
      source_block: '',
      binding: '',
      group: '',
      table_rows: [
        ['Kolom 1', 'Kolom 2', 'Kolom 3'],
        ['', '', ''],
        ['', '', ''],
      ],
      table_header: true,
      table_header_bg: '#143d30',
      table_header_color: '#ffffff',
      table_stripe: '#f6f8f5',
      chart_type: 'bar',
      chart_title: '',
      chart_labels: ['A', 'B', 'C', 'D'],
      chart_values: [10, 20, 15, 25],
      chart_colors: ['#143d30', '#b8894b', '#3f765d', '#5b8fa8'],
      chart_show_values: true,
      chart_show_legend: true,
      chart_axis_color: '#6d7872',
      card_label: 'Label kartu',
      card_value: '0',
      card_unit: 'unit',
      card_detail: '',
      card_accent: '#b8894b',
      icon_name: '',
      icon_color: '#143d30',
      icon_stroke: 2,
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
    sources = []
    checkedIds = []
    selectedTemplateId = ''
    zoomOverride = null
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
      const [draft, layoutView, sourceList, templateList] = await Promise.all([
        reportsApi.getDraft(periodId, selectedImportId),
        designApi.getLayout(periodId, selectedImportId),
        designApi.sources(periodId, selectedImportId),
        designApi.templates(),
      ])
      content = { ...emptyContent, ...draft.content }
      layout = normalizeLayout(layoutView.layout)
      sources = sourceList
      templates = templateList
      savedJson = JSON.stringify(layout)
      updatedAt = layoutView.updated_at
      updatedBy = layoutView.updated_by
      const preview = await reportsApi.preview(periodId, selectedImportId, content, layout)
      iframeHtml = preview.html
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      loading = false
    }
  }

  async function refreshCanvasStructure(): Promise<void> {
    if (!selectedImportId) return
    const request = ++canvasRequest
    try {
      const preview = await reportsApi.preview(periodId, selectedImportId, content, layout)
      if (request !== canvasRequest) return
      iframeHtml = preview.html
    } catch (caught) {
      if (request === canvasRequest) toast.error(errorMessage(caught))
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
      html, body { margin: 0; padding: 0; overflow: hidden; background: #fff; }
      .page { display: none !important; margin: 0 auto !important; box-shadow: none !important; }
      .page.editor-current-page { display: flex !important; zoom: var(--editor-page-zoom, 1); }
      body { user-select: none; }
      [data-block] { cursor: move; }
      [data-block]:hover { outline: 1px dashed rgba(184,137,75,.55); outline-offset: 2px; }
      .editor-block-selected { outline: 2px dashed #b8894b !important; outline-offset: 2px; }
      .layout-element { cursor: move; }
      .layout-element:hover { outline: 1px dashed rgba(184,137,75,.7); }
      .layout-shape-svg { display: block; width: 100%; height: 100%; overflow: visible; }
      .editor-selection { position: absolute; border: 1.5px solid #b8894b; z-index: 9999; pointer-events: none; box-sizing: border-box; }
      .editor-handle { position: absolute; width: 10px; height: 10px; background: #fff; border: 1.5px solid #b8894b; border-radius: 2px; pointer-events: auto; box-sizing: border-box; }
      .editor-dragging * { cursor: grabbing !important; }
      .layout-table table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      .layout-table th, .layout-table td { border: var(--table-border, .5pt solid #d7ddd6); padding: 1.2mm 1.5mm; vertical-align: top; word-break: break-word; }
      .layout-table th { font-weight: 700; }
      .layout-source-empty { display: flex; align-items: center; justify-content: center; width: 100%; height: 100%; box-sizing: border-box; padding: 2mm; text-align: center; font-size: 8pt; color: #6d7872; background: #f6f8f5; border: .5pt dashed #d7ddd6; }
    `
    doc.head.appendChild(style)
  }

  function handleIframeLoad() {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    installEditorStyles(doc)
    if (boundDoc !== doc) {
      boundDoc = doc
      defaultTexts.clear()
      doc.addEventListener('pointerdown', onPointerDown)
      doc.addEventListener('dblclick', onDoubleClick)
      doc.addEventListener('keydown', handleKeydown)
    }
    applyLayoutToDom()
    updateCanvasSize()
    applyPageVisibility()
  }

  function applyPageVisibility(): void {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    doc.documentElement.style.setProperty('--editor-page-zoom', String(canvasZoom))
    const pages = doc.querySelectorAll<HTMLElement>('.page')
    pages.forEach((pageEl, index) => {
      pageEl.classList.toggle('editor-current-page', index + 1 === currentPage)
    })
  }

  function updateCanvasSize(): void {
    const area = canvasAreaEl
    if (!area) return
    const availableWidth = area.clientWidth - 28
    const availableHeight = area.clientHeight - 28
    if (availableWidth <= 0 || availableHeight <= 0) return
    const scale = Math.min(
      availableWidth / PAGE_WIDTH_PX,
      availableHeight / PAGE_HEIGHT_PX,
      1,
    ) * 0.998
    fitZoom = Math.max(0.05, scale)
    applyZoom()
  }

  function applyZoom(): void {
    canvasZoom = zoomOverride === null ? fitZoom : clamp(zoomOverride, MIN_ZOOM, MAX_ZOOM)
    canvasWidth = Math.ceil(PAGE_WIDTH_PX * canvasZoom) + 3
    canvasHeight = Math.ceil(PAGE_HEIGHT_PX * canvasZoom) + 3
    applyPageVisibility()
  }

  function setZoom(value: number): void {
    const next = Math.round(clamp(value, MIN_ZOOM, MAX_ZOOM) * 100) / 100
    zoomOverride = Math.abs(next - fitZoom) < 0.005 ? null : next
    applyZoom()
    centerCanvas()
  }

  function zoomBy(delta: number): void {
    setZoom((zoomOverride ?? fitZoom) + delta)
  }

  function resetZoom(): void {
    zoomOverride = null
    applyZoom()
    centerCanvas()
  }

  function centerCanvas(): void {
    const area = canvasAreaEl
    if (!area) return
    requestAnimationFrame(() => {
      area.scrollLeft = Math.max(0, (area.scrollWidth - area.clientWidth) / 2)
      area.scrollTop = Math.max(0, (area.scrollHeight - area.clientHeight) / 2)
    })
  }

  function sourceMessage(doc: Document, message: string): HTMLElement {
    const node = doc.createElement('div')
    node.className = 'layout-source-empty'
    node.textContent = message
    return node
  }

  function linearGradientCoords(
    angle: number,
    width: number,
    height: number,
  ): [number, number, number, number] {
    const radians = (angle * Math.PI) / 180
    let dx = Math.sin(radians) / Math.max(width, 0.001)
    let dy = -Math.cos(radians) / Math.max(height, 0.001)
    const length = Math.hypot(dx, dy)
    if (length > 0) {
      dx /= length
      dy /= length
    }
    const span = (Math.abs(dx) + Math.abs(dy)) * 0.5
    return [0.5 - dx * span, 0.5 - dy * span, 0.5 + dx * span, 0.5 + dy * span]
  }

  function createShapeSvg(doc: Document, element: LayoutElement): SVGElement {
    const shape = shapeDefinition(element.shape)
    const svg = svgNode(doc, 'svg', {
      viewBox: '0 0 100 100',
      preserveAspectRatio: 'none',
      width: '100%',
      height: '100%',
      'aria-hidden': 'true',
    })
    const rawColors = element.gradient_colors ?? []
    const mode = rectFillMode(element)
    let fill = isTransparentColor(element.fill) ? 'none' : (element.fill ?? '#ffffff')
    if (fill !== 'none' && mode !== 'solid' && rawColors.length >= 2) {
      const gradientId = `grad-${element.id}`
      const defs = svgNode(doc, 'defs')
      let gradient: SVGElement
      if (mode === 'radial') {
        gradient = svgNode(doc, 'radialGradient', { id: gradientId, cx: '50%', cy: '50%', r: '50%' })
      } else {
        const [x1, y1, x2, y2] = linearGradientCoords(
          element.gradient_angle ?? 90,
          element.width,
          element.height,
        )
        gradient = svgNode(doc, 'linearGradient', { id: gradientId, x1, y1, x2, y2 })
      }
      const last = rawColors.length - 1
      rawColors.forEach((color, index) => {
        gradient.appendChild(
          svgNode(doc, 'stop', {
            offset: `${((index / last) * 100).toFixed(1)}%`,
            'stop-color': color,
          }),
        )
      })
      defs.appendChild(gradient)
      svg.appendChild(defs)
      fill = `url(#${gradientId})`
    }
    const attributes: Record<string, string | number> = { fill }
    const borderWidth = element.border_width ?? 0.5
    const borderColor = element.border_color ?? '#143d30'
    if (borderWidth > 0 && !isTransparentColor(borderColor)) {
      attributes.stroke = borderColor
      attributes['stroke-width'] = `${borderWidth}pt`
      attributes['vector-effect'] = 'non-scaling-stroke'
      attributes['stroke-linejoin'] = 'round'
    }
    if (shape.ellipse) {
      svg.appendChild(svgNode(doc, 'ellipse', { cx: 50, cy: 50, rx: 50, ry: 50, ...attributes }))
    } else if (shape.path) {
      svg.appendChild(svgNode(doc, 'path', { d: shape.path, ...attributes }))
    } else {
      svg.appendChild(svgNode(doc, 'polygon', { points: shape.points ?? '', ...attributes }))
    }
    return svg
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
      const shape = shapeDefinition(element.shape)
      if (shape.kind !== 'rect') {
        const wrapper = doc.createElement('div')
        wrapper.dataset.shape = shape.kind
        wrapper.appendChild(createShapeSvg(doc, element))
        node = wrapper
      } else {
        const rect = doc.createElement('div')
        const fill = element.fill ?? '#ffffff'
        const borderWidth = element.border_width ?? 0.5
        const borderColor = element.border_color ?? '#143d30'
        rect.style.background = isTransparentColor(fill) ? 'transparent' : gradientCss(element)
        rect.style.border =
          borderWidth > 0 && !isTransparentColor(borderColor) ? `${borderWidth}pt solid ${borderColor}` : 'none'
        rect.style.borderRadius = `${element.radius ?? 1}mm`
        node = rect
      }
    } else if (element.type === 'table') {
      const source = sourceFor(element.binding)
      const rows = element.binding ? (source?.table_rows ?? []) : (element.table_rows ?? [])
      if (element.binding && !source) {
        node = sourceMessage(doc, 'Sumber data tidak tersedia.')
      } else if (!rows.length) {
        node = sourceMessage(doc, source ? 'Sumber data kosong.' : 'Belum ada baris tabel.')
      } else {
        const wrapper = doc.createElement('div')
        wrapper.style.background = element.fill ?? '#ffffff'
        const table = doc.createElement('table')
        table.style.fontSize = `${element.font_size ?? 9}pt`
        table.style.color = element.color ?? '#143d30'
        table.style.setProperty(
          '--table-border',
          `${element.border_width ?? 0.5}pt solid ${element.border_color ?? '#d7ddd6'}`,
        )
        const hasHeader = element.table_header !== false && rows.length > 1
        const headerRow = hasHeader ? rows[0] : null
        const bodyRows = hasHeader ? rows.slice(1) : rows
        if (headerRow) {
          const thead = doc.createElement('thead')
          const tr = doc.createElement('tr')
          for (const cell of headerRow) {
            const th = doc.createElement('th')
            th.textContent = cell
            th.style.background = element.table_header_bg ?? '#143d30'
            th.style.color = element.table_header_color ?? '#ffffff'
            th.style.textAlign = isNumericCell(cell) ? 'right' : 'left'
            tr.appendChild(th)
          }
          thead.appendChild(tr)
          table.appendChild(thead)
        }
        const tbody = doc.createElement('tbody')
        bodyRows.forEach((row, rowIndex) => {
          const tr = doc.createElement('tr')
          for (const cell of row) {
            const td = doc.createElement('td')
            td.textContent = cell
            if (
              rowIndex % 2 === 1 &&
              element.table_stripe &&
              element.table_stripe !== 'transparent'
            ) {
              td.style.background = element.table_stripe
            }
            td.style.textAlign = isNumericCell(cell) ? 'right' : 'left'
            tr.appendChild(td)
          }
          tbody.appendChild(tr)
        })
        table.appendChild(tbody)
        wrapper.appendChild(table)
        node = wrapper
      }
    } else if (element.type === 'chart') {
      const source = sourceFor(element.binding)
      const wrapper = doc.createElement('div')
      if (element.binding && (!source || !source.can_chart || !source.chart_labels.length)) {
        wrapper.appendChild(
          sourceMessage(
            doc,
            source ? 'Sumber data tidak memiliki seri grafik.' : 'Sumber data tidak tersedia.',
          ),
        )
      } else {
        const chartElement = source
          ? {
              ...element,
              chart_labels: source.chart_labels,
              chart_values: source.chart_values,
              chart_title: element.chart_title?.trim() ? element.chart_title : source.chart_title,
            }
          : element
        wrapper.appendChild(createChartSvg(doc, chartElement))
      }
      node = wrapper
    } else if (element.type === 'card') {
      const card = doc.createElement('div')
      card.style.display = 'flex'
      card.style.flexDirection = 'column'
      card.style.padding = '3mm 3.5mm'
      card.style.background = element.fill ?? '#f6f8f5'
      card.style.borderLeft = `${element.border_width ?? 2}pt solid ${element.card_accent ?? '#b8894b'}`
      card.style.borderRadius = `${element.radius ?? 1.5}mm`
      card.style.overflow = 'hidden'
      const label = doc.createElement('span')
      label.className = 'layout-card-label'
      label.textContent = element.card_label ?? ''
      label.style.fontSize = `${element.font_size ?? 9}pt`
      const value = doc.createElement('strong')
      value.className = 'layout-card-value'
      value.textContent = element.card_value ?? ''
      value.style.fontSize = `${Math.min(40, Math.max(14, (element.font_size ?? 9) * 2.35))}pt`
      value.style.color = element.color ?? '#143d30'
      const unit = doc.createElement('span')
      unit.className = 'layout-card-unit'
      unit.textContent = element.card_unit ?? ''
      unit.style.fontSize = `${(element.font_size ?? 9) * 0.82}pt`
      card.append(label, value, unit)
      if (element.card_detail?.trim()) {
        const detail = doc.createElement('span')
        detail.className = 'layout-card-detail'
        detail.textContent = element.card_detail
        detail.style.fontSize = `${(element.font_size ?? 9) * 0.78}pt`
        card.appendChild(detail)
      }
      node = card
    } else if (element.type === 'icon') {
      const wrapper = doc.createElement('div')
      wrapper.style.color = element.icon_color ?? '#143d30'
      wrapper.style.display = 'block'
      const icon = iconByName.get(element.icon_name ?? '')
      if (!icon) {
        wrapper.appendChild(sourceMessage(doc, 'Ikon tidak ditemukan.'))
      } else {
        const svg = doc.createElementNS(SVG_NS, 'svg')
        svg.setAttribute('viewBox', '0 0 24 24')
        svg.setAttribute('fill', 'none')
        svg.setAttribute('stroke', 'currentColor')
        svg.setAttribute('stroke-width', String(element.icon_stroke ?? 2))
        svg.setAttribute('stroke-linecap', 'round')
        svg.setAttribute('stroke-linejoin', 'round')
        svg.setAttribute('preserveAspectRatio', 'xMidYMid meet')
        svg.setAttribute('width', '100%')
        svg.setAttribute('height', '100%')
        svg.innerHTML = icon.svg
        wrapper.appendChild(svg)
      }
      node = wrapper
    } else {
      const text = doc.createElement('div')
      const inner = doc.createElement('span')
      inner.className = 'layout-text-inner'
      inner.textContent = element.text ?? ''
      inner.style.display = 'block'
      inner.style.width = '100%'
      inner.style.whiteSpace = 'pre-wrap'
      text.appendChild(inner)
      text.style.display = 'flex'
      text.style.flexDirection = 'column'
      text.style.justifyContent =
        element.valign === 'middle' ? 'center' : element.valign === 'bottom' ? 'flex-end' : 'flex-start'
      text.style.fontFamily = fontStack(element.font_family)
      text.style.fontSize = `${element.font_size ?? 12}pt`
      text.style.color = element.color ?? '#143d30'
      text.style.fontWeight = element.bold ? '700' : '400'
      text.style.fontStyle = element.italic ? 'italic' : 'normal'
      text.style.textAlign = element.align ?? 'left'
      text.style.background = element.background ?? 'transparent'
      text.style.padding = '.8mm'
      text.style.overflow = 'hidden'
      text.style.boxSizing = 'border-box'
      node = text
    }

    node.classList.add('layout-element', `layout-${element.type}`)
    if (element.type === 'rect' && shapeDefinition(element.shape).kind !== 'rect') {
      node.classList.add('layout-shape')
    }
    node.dataset.elementId = element.id
    node.style.position = 'absolute'
    node.style.left = `${element.x}mm`
    node.style.top = `${element.y}mm`
    node.style.width = `${element.width}mm`
    node.style.height = `${element.height}mm`
    node.style.opacity = String(element.opacity ?? 1)
    node.style.zIndex = String(element.z ?? 0)
    if (element.type === 'table' || element.type === 'chart') {
      node.style.overflow = 'hidden'
    }
    if (element.rotation) node.style.transform = `rotate(${element.rotation}deg)`
    return node
  }

  const SVG_NS = 'http://www.w3.org/2000/svg'
  const CHART_PALETTE = ['#143d30', '#b8894b', '#3f765d', '#5b8fa8', '#a4743d', '#d9b87e', '#7a857f', '#2f5d7c']

  function chartColor(element: LayoutElement, index: number): string {
    return element.chart_colors?.[index] ?? CHART_PALETTE[index % CHART_PALETTE.length]
  }

  function chartValueText(value: number): string {
    return Number.isInteger(value)
      ? new Intl.NumberFormat('id-ID').format(value)
      : value.toFixed(1)
  }

  function isNumericCell(value: string): boolean {
    const normalized = value.trim().replace(/\./g, '').replace(',', '.')
    return normalized !== '' && Number.isFinite(Number(normalized))
  }

  function svgNode(
    doc: Document,
    name: string,
    attributes: Record<string, string | number> = {},
  ): SVGElement {
    const node = doc.createElementNS(SVG_NS, name) as SVGElement
    for (const [key, value] of Object.entries(attributes)) {
      node.setAttribute(key, String(value))
    }
    return node
  }

  function svgText(
    doc: Document,
    svg: SVGElement,
    x: number,
    y: number,
    text: string,
    options: { fill?: string; size?: number; anchor?: string; weight?: number } = {},
  ): void {
    const node = svgNode(doc, 'text', {
      x,
      y,
      fill: options.fill ?? '#143d30',
      'font-size': options.size ?? 14,
      'text-anchor': options.anchor ?? 'start',
      'font-weight': options.weight ?? 400,
    })
    node.textContent = text
    svg.appendChild(node)
  }

  function createChartSvg(doc: Document, element: LayoutElement): SVGElement {
    const svg = svgNode(doc, 'svg', {
      viewBox: '0 0 600 360',
      preserveAspectRatio: 'xMidYMid meet',
      width: '100%',
      height: '100%',
      'font-family': 'Segoe UI, Arial, sans-serif',
    })
    const fill = element.fill ?? '#ffffff'
    svg.appendChild(svgNode(doc, 'rect', { x: 0, y: 0, width: 600, height: 360, fill }))
    const base = Math.min(22, Math.max(10, (element.font_size ?? 9) * 1.6))
    const isHbar = element.chart_type === 'hbar'
    const top = isHbar ? (element.chart_title?.trim() ? 58 : 20) : element.chart_title?.trim() ? 50 : 18
    if (element.chart_title?.trim() && !isHbar) {
      svgText(doc, svg, 300, 30, element.chart_title, {
        fill: element.color ?? '#143d30',
        size: base + 4,
        anchor: 'middle',
        weight: 700,
      })
    }
    if (element.chart_type === 'line') {
      appendLineChart(doc, svg, element, top, base)
    } else if (element.chart_type === 'hbar') {
      appendHorizontalBarChart(doc, svg, element, top, base)
    } else if (element.chart_type === 'pie' || element.chart_type === 'donut') {
      appendPieChart(doc, svg, element, top, base, element.chart_type === 'donut')
    } else {
      appendBarChart(doc, svg, element, top, base)
    }
    return svg
  }

  function appendBarChart(
    doc: Document,
    svg: SVGElement,
    element: LayoutElement,
    top: number,
    base: number,
  ): void {
    const left = 55
    const right = 575
    const bottom = 316
    const plotHeight = Math.max(40, bottom - top)
    const values = element.chart_values ?? []
    const count = Math.max(1, values.length)
    const step = (right - left) / count
    const barWidth = Math.max(3, step * 0.64)
    const max = Math.max(1, ...values)
    svg.appendChild(svgNode(doc, 'line', { x1: left, y1: bottom, x2: right, y2: bottom, stroke: element.chart_axis_color ?? '#6d7872', 'stroke-width': 1.2 }))
    svgText(doc, svg, left - 8, top + base, chartValueText(max), { fill: element.chart_axis_color ?? '#6d7872', size: base - 2, anchor: 'end' })
    values.forEach((value, index) => {
      const height = (value / max) * plotHeight
      const x = left + step * index + (step - barWidth) / 2
      const y = bottom - height
      const color = chartColor(element, index)
      svg.appendChild(svgNode(doc, 'rect', { x, y, width: barWidth, height, rx: 2, fill: color }))
      if (element.chart_show_values !== false) {
        svgText(doc, svg, x + barWidth / 2, Math.max(top + base, y - 4), chartValueText(value), { fill: element.color ?? '#143d30', size: base - 2, anchor: 'middle' })
      }
      const label = element.chart_labels?.[index]
      if (label) {
        svgText(doc, svg, x + barWidth / 2, bottom + base + 8, truncateText(label, 14), { fill: element.chart_axis_color ?? '#6d7872', size: base - 2, anchor: 'middle' })
      }
    })
  }

  function appendHorizontalBarChart(
    doc: Document,
    svg: SVGElement,
    element: LayoutElement,
    top: number,
    base: number,
  ): void {
    const left = 24
    const right = 576
    const bottom = 336
    const gap = 12
    const valueWidth = element.chart_show_values === false ? 0 : 46
    const labelWidth = (right - left) * 0.38
    const trackLeft = left + labelWidth + gap
    const trackWidth = Math.max(40, right - valueWidth - gap - trackLeft)
    const values = element.chart_values ?? []
    const labels = element.chart_labels ?? []
    const count = Math.max(1, values.length)
    const rowHeight = Math.min(44, Math.max(12, (bottom - top) / count))
    const barHeight = Math.min(16, Math.max(6, rowHeight * 0.36))
    const barRadius = barHeight / 2
    const labelSize = count > 12 ? Math.max(7, base - 4) : Math.max(8, base - 3)
    const valueSize = Math.max(8, base - 2)
    const max = Math.max(1, ...values)
    const barColor = chartColor(element, 0)

    svg.appendChild(
      svgNode(doc, 'rect', {
        x: 1,
        y: 1,
        width: 598,
        height: 358,
        rx: 5,
        fill: 'none',
        stroke: '#d7ddd6',
        'stroke-width': 1.4,
      }),
    )
    if (element.chart_title?.trim()) {
      svgText(doc, svg, left, 32, element.chart_title, {
        fill: element.color ?? '#143d30',
        size: Math.max(13, base + 4),
        weight: 700,
      })
    }
    values.forEach((value, index) => {
      const center = top + rowHeight * (index + 0.5)
      const baseline = center + labelSize * 0.34
      const label = labels[index]
      if (label) {
        svgText(doc, svg, left, baseline, truncateText(label, 26), {
          fill: '#46534b',
          size: labelSize,
        })
      }
      const y = center - barHeight / 2
      svg.appendChild(
        svgNode(doc, 'rect', {
          x: trackLeft,
          y,
          width: trackWidth,
          height: barHeight,
          rx: barRadius,
          fill: '#eef1ec',
        }),
      )
      const width = trackWidth * Math.min(1, Math.max(0, value / max))
      if (width > 0.5) {
        svg.appendChild(
          svgNode(doc, 'rect', {
            x: trackLeft,
            y,
            width: Math.max(width, barHeight),
            height: barHeight,
            rx: barRadius,
            fill: barColor,
          }),
        )
      }
      if (element.chart_show_values !== false) {
        svgText(doc, svg, right, baseline, chartValueText(value), {
          fill: element.color ?? '#143d30',
          size: valueSize,
          anchor: 'end',
          weight: 700,
        })
      }
    })
  }

  function appendLineChart(
    doc: Document,
    svg: SVGElement,
    element: LayoutElement,
    top: number,
    base: number,
  ): void {
    const left = 55
    const right = 575
    const bottom = 316
    const plotHeight = Math.max(40, bottom - top)
    const values = element.chart_values ?? []
    const count = Math.max(1, values.length)
    const step = count > 1 ? (right - left) / (count - 1) : 0
    const max = Math.max(1, ...values)
    const color = chartColor(element, 0)
    svg.appendChild(svgNode(doc, 'line', { x1: left, y1: bottom, x2: right, y2: bottom, stroke: element.chart_axis_color ?? '#6d7872', 'stroke-width': 1.2 }))
    const coordinates = values.map((value, index) => {
      const x = count > 1 ? left + step * index : (left + right) / 2
      const y = bottom - (value / max) * plotHeight
      return { x, y, value }
    })
    const polyline = svgNode(doc, 'polyline', {
      points: coordinates.map((point) => `${point.x},${point.y}`).join(' '),
      fill: 'none',
      stroke: color,
      'stroke-width': 3,
      'stroke-linejoin': 'round',
      'stroke-linecap': 'round',
    })
    svg.appendChild(polyline)
    coordinates.forEach((point, index) => {
      svg.appendChild(svgNode(doc, 'circle', { cx: point.x, cy: point.y, r: 4.5, fill: color }))
      if (element.chart_show_values !== false) {
        svgText(doc, svg, point.x, Math.max(top + base, point.y - 8), chartValueText(point.value), { fill: element.color ?? '#143d30', size: base - 2, anchor: 'middle' })
      }
      const label = element.chart_labels?.[index]
      if (label) {
        svgText(doc, svg, point.x, bottom + base + 8, truncateText(label, 14), { fill: element.chart_axis_color ?? '#6d7872', size: base - 2, anchor: 'middle' })
      }
    })
  }

  function appendPieChart(
    doc: Document,
    svg: SVGElement,
    element: LayoutElement,
    top: number,
    base: number,
    donut: boolean,
  ): void {
    const values = element.chart_values ?? []
    const total = values.reduce((sum, value) => sum + value, 0)
    if (total <= 0) {
      svgText(doc, svg, 300, 180, 'Belum ada data', { fill: element.chart_axis_color ?? '#6d7872', size: base + 2, anchor: 'middle' })
      return
    }
    const showLegend = element.chart_show_legend !== false
    const cx = showLegend ? 205 : 300
    const cy = top + (316 - top) / 2
    const outer = Math.min(125, Math.max(40, (316 - top) / 2 - 12))
    const inner = donut ? outer * 0.55 : 0
    const positives = values.filter((value) => value > 0).length
    let angle = -Math.PI / 2
    values.forEach((value, index) => {
      if (value <= 0) return
      const sweep = (value / total) * Math.PI * 2
      const end = angle + sweep
      const color = chartColor(element, index)
      const x1 = cx + outer * Math.cos(angle)
      const y1 = cy + outer * Math.sin(angle)
      const x2 = cx + outer * Math.cos(end)
      const y2 = cy + outer * Math.sin(end)
      const large = sweep > Math.PI ? 1 : 0
      if (positives === 1) {
        svg.appendChild(svgNode(doc, 'circle', { cx, cy, r: outer, fill: color }))
      } else if (donut) {
        const ix1 = cx + inner * Math.cos(angle)
        const iy1 = cy + inner * Math.sin(angle)
        const ix2 = cx + inner * Math.cos(end)
        const iy2 = cy + inner * Math.sin(end)
        svg.appendChild(svgNode(doc, 'path', { d: `M ${x1} ${y1} A ${outer} ${outer} 0 ${large} 1 ${x2} ${y2} L ${ix2} ${iy2} A ${inner} ${inner} 0 ${large} 0 ${ix1} ${iy1} Z`, fill: color }))
      } else {
        svg.appendChild(svgNode(doc, 'path', { d: `M ${cx} ${cy} L ${x1} ${y1} A ${outer} ${outer} 0 ${large} 1 ${x2} ${y2} Z`, fill: color }))
      }
      if (element.chart_show_values !== false && sweep > 0.35) {
        const mid = angle + sweep / 2
        const radius = donut ? (outer + inner) / 2 : outer * 0.62
        svgText(doc, svg, cx + radius * Math.cos(mid), cy + radius * Math.sin(mid) + base * 0.35, `${Math.round((sweep / (Math.PI * 2)) * 100)}%`, { fill: '#ffffff', size: base - 2, anchor: 'middle', weight: 700 })
      }
      angle = end
    })
    if (donut) {
      svg.appendChild(svgNode(doc, 'circle', { cx, cy, r: inner, fill: element.fill ?? '#ffffff' }))
    }
    if (showLegend) {
      let y = top + base
      element.chart_labels?.slice(0, 12).forEach((label, index) => {
        svg.appendChild(svgNode(doc, 'rect', { x: 355, y: y - 10, width: 12, height: 12, rx: 2, fill: chartColor(element, index) }))
        svgText(doc, svg, 374, y, truncateText(label, 20), { fill: element.color ?? '#143d30', size: base - 3 })
        y += base + 7
      })
    }
  }

  function truncateText(value: string, max: number): string {
    return value.length <= max ? value : `${value.slice(0, max - 1)}…`
  }

  function clearHiddenBlockRule(doc: Document, id: string): void {
    for (const sheet of Array.from(doc.styleSheets)) {
      const rules = (() => {
        try {
          return sheet.cssRules
        } catch {
          return null
        }
      })()
      if (!rules) continue
      for (let index = rules.length - 1; index >= 0; index -= 1) {
        const rule = rules[index]
        if (rule.type === 1 && (rule as CSSStyleRule).selectorText === `[data-block="${id}"]`) {
          sheet.deleteRule(index)
        }
      }
    }
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
        if (!offset?.hidden) clearHiddenBlockRule(doc, id)
        blockEl.classList.remove('editor-block-selected')
        if (offset?.hidden && !offset.preserve_space) {
          blockEl.style.display = 'none'
        } else {
          blockEl.style.removeProperty('display')
        }
        blockEl.style.visibility = offset?.hidden && offset.preserve_space ? 'hidden' : ''
        // Selalu set inline agar rule transform dari layout tersimpan tidak
        // menempel saat blok di-reset ke posisi awal.
        blockEl.style.transform = `translate(${offset?.dx ?? 0}mm, ${offset?.dy ?? 0}mm)`
        blockEl.style.width = offset?.width ? `${offset.width}mm` : ''
        blockEl.style.height = offset?.height ? `${offset.height}mm` : ''
        blockEl.style.overflow = offset?.width || offset?.height ? 'hidden' : ''
        blockEl.style.boxSizing = offset?.width || offset?.height ? 'border-box' : ''
        blockEl.style.background = offset?.background ?? ''
        blockEl.style.border =
          offset?.border_width && offset.border_width > 0
            ? `${offset.border_width}pt solid ${offset.border_color || '#143d30'}`
            : ''
        blockEl.style.borderRadius = offset?.radius ? `${offset.radius}mm` : ''

        blockEl.querySelectorAll<HTMLElement>('[data-text-id]').forEach((textEl) => {
          const textId = textEl.dataset.textId
          if (!textId) return
          const key = `${pageNo}:${id}:${textId}`
          if (!defaultTexts.has(key)) {
            defaultTexts.set(key, textEl.innerHTML)
          }
          const value = offset?.texts?.[textId]
          if (value !== undefined) {
            textEl.textContent = value
            textEl.style.whiteSpace = 'pre-line'
          } else {
            const fallback = defaultTexts.get(key)
            if (fallback !== undefined) textEl.innerHTML = fallback
            textEl.style.whiteSpace = ''
          }
        })
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
      if (!blockEl) {
        selected = null
        return
      }
      blockEl.classList.add('editor-block-selected')
      const pageRect = pageEl.getBoundingClientRect()
      const rect = blockEl.getBoundingClientRect()
      const frame = doc.createElement('div')
      frame.className = 'editor-selection'
      frame.dataset.kind = 'block'
      frame.style.left = `${((rect.left - pageRect.left) / pageRect.width) * PAGE_WIDTH}mm`
      frame.style.top = `${((rect.top - pageRect.top) / pageRect.height) * PAGE_HEIGHT}mm`
      frame.style.width = `${(rect.width / pageRect.width) * PAGE_WIDTH}mm`
      frame.style.height = `${(rect.height / pageRect.height) * PAGE_HEIGHT}mm`
      const handle = doc.createElement('div')
      handle.className = 'editor-handle'
      handle.dataset.handle = 'se'
      handle.style.left = 'calc(100% - 5px)'
      handle.style.top = 'calc(100% - 5px)'
      handle.style.cursor = 'nwse-resize'
      frame.appendChild(handle)
      pageEl.appendChild(frame)
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

  function positionBlockFrame() {
    if (!selected || selected.kind !== 'block') return
    const doc = iframeEl?.contentDocument
    const pageEl = doc?.querySelectorAll<HTMLElement>('.page')[selected.page - 1]
    const blockEl = pageEl?.querySelector<HTMLElement>(`[data-block="${selected.id}"]`)
    const frame = pageEl?.querySelector<HTMLElement>('.editor-selection')
    if (!pageEl || !blockEl || !frame) return
    const pageRect = pageEl.getBoundingClientRect()
    const rect = blockEl.getBoundingClientRect()
    frame.style.left = `${((rect.left - pageRect.left) / pageRect.width) * PAGE_WIDTH}mm`
    frame.style.top = `${((rect.top - pageRect.top) / pageRect.height) * PAGE_HEIGHT}mm`
    frame.style.width = `${(rect.width / pageRect.width) * PAGE_WIDTH}mm`
    frame.style.height = `${(rect.height / pageRect.height) * PAGE_HEIGHT}mm`
  }

  function onDoubleClick(event: MouseEvent) {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    const target = event.target as HTMLElement | null
    const textEl = target?.closest<HTMLElement>('[data-text-id]')
    const blockEl = textEl?.closest<HTMLElement>('[data-block]')
    const pageEl = blockEl?.closest<HTMLElement>('.page')
    if (!textEl || !blockEl || !pageEl) return
    const page = pageIndexOf(pageEl)
    const blockId = blockEl.dataset.block
    const textId = textEl.dataset.textId
    if (!blockId || !textId) return
    event.preventDefault()
    event.stopPropagation()
    pushHistory()
    selected = { kind: 'block', page, id: blockId }
    textEl.setAttribute('contenteditable', 'plaintext-only')
    textEl.style.whiteSpace = 'pre-line'
    textEl.focus()
    const selection = doc.defaultView?.getSelection()
    if (selection) {
      const range = doc.createRange()
      range.selectNodeContents(textEl)
      selection.removeAllRanges()
      selection.addRange(range)
    }
    const commit = (): string => {
      const value = textEl.innerText.replace(/\u00a0/g, ' ').replace(/\r\n/g, '\n')
      const pageData = getPage(page)
      if (pageData) {
        const block = pageData.blocks[blockId] ?? { dx: 0, dy: 0, hidden: false }
        block.texts = { ...(block.texts ?? {}), [textId]: value }
        pageData.blocks[blockId] = block
      }
      return value
    }
    const onInput = () => {
      commit()
    }
    const finish = () => {
      textEl.removeEventListener('blur', finish)
      textEl.removeEventListener('input', onInput)
      textEl.removeAttribute('contenteditable')
      const value = commit()
      textEl.textContent = value
      renderSelection()
    }
    textEl.addEventListener('input', onInput)
    textEl.addEventListener('blur', finish, { once: true })
    textEl.addEventListener(
      'keydown',
      (keyboardEvent) => {
        if (keyboardEvent.key === 'Escape') {
          keyboardEvent.preventDefault()
          textEl.blur()
        }
      },
      { once: true },
    )
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
    if (target.isContentEditable || target.closest('[contenteditable="true"]')) return
    const page = pageIndexOf(pageEl)
    if (page !== currentPage) checkedIds = []
    currentPage = page

    const handleEl = target.closest<HTMLElement>('[data-handle]')
    if (handleEl && selected?.kind === 'block') {
      const pageData = getPage(selected.page)
      const blockEl = pageEl.querySelector<HTMLElement>(`[data-block="${selected.id}"]`)
      if (!pageData || !blockEl) return
      const pageRect = pageEl.getBoundingClientRect()
      const rect = blockEl.getBoundingClientRect()
      const offset = pageData.blocks[selected.id] ?? { dx: 0, dy: 0, hidden: false }
      event.preventDefault()
      beginDrag({
        type: 'block-resize',
        page: selected.page,
        id: selected.id,
        startX: event.clientX,
        startY: event.clientY,
        startDx: 0,
        startDy: 0,
        startElX: 0,
        startElY: 0,
        startElW: offset.width || (rect.width / pageRect.width) * PAGE_WIDTH,
        startElH: offset.height || (rect.height / pageRect.height) * PAGE_HEIGHT,
        moved: false,
        historyPushed: false,
        before: cloneLayout(layout),
      })
      return
    }
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
      const pageData = getPage(page)
      const groupStarts =
        element.group && pageData
          ? pageData.elements
              .filter((item) => item.group === element.group)
              .map((item) => ({ id: item.id, x: item.x, y: item.y }))
          : []
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
        groupStarts,
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
      positionBlockFrame()
      return
    }

    if (drag.type === 'block-resize') {
      const page = getPage(drag.page)
      const blockEl = pageEl.querySelector<HTMLElement>(`[data-block="${drag.id}"]`)
      if (!page || !blockEl) return
      const block = page.blocks[drag.id] ?? { dx: 0, dy: 0, hidden: false }
      block.width = Math.round(clamp(drag.startElW + dx, 10, PAGE_WIDTH) * 1000) / 1000
      block.height = Math.round(clamp(drag.startElH + dy, 5, PAGE_HEIGHT) * 1000) / 1000
      page.blocks[drag.id] = block
      blockEl.style.width = `${block.width}mm`
      blockEl.style.height = `${block.height}mm`
      blockEl.style.overflow = 'hidden'
      blockEl.style.boxSizing = 'border-box'
      positionBlockFrame()
      return
    }

    const element = getElement(drag.page, drag.id)
    if (!element) return
    const node = pageEl.querySelector<HTMLElement>(`[data-element-id="${drag.id}"]`)
    const page = getPage(drag.page)

    if (drag.type === 'element') {
      const groupStarts = drag.groupStarts?.length ? drag.groupStarts : null
      if (groupStarts && page) {
        let deltaX = dx
        let deltaY = dy
        for (const start of groupStarts) {
          const member = page.elements.find((item) => item.id === start.id)
          if (!member) continue
          deltaX = clamp(deltaX, -start.x, PAGE_WIDTH - member.width - start.x)
          deltaY = clamp(deltaY, -start.y, PAGE_HEIGHT - member.height - start.y)
        }
        for (const start of groupStarts) {
          const member = page.elements.find((item) => item.id === start.id)
          if (!member) continue
          member.x = Math.round((start.x + deltaX) * 1000) / 1000
          member.y = Math.round((start.y + deltaY) * 1000) / 1000
          const memberNode = pageEl.querySelector<HTMLElement>(
            `[data-element-id="${member.id}"]`,
          )
          if (!memberNode) continue
          memberNode.style.left = `${member.x}mm`
          memberNode.style.top = `${member.y}mm`
        }
      } else {
        element.x =
          Math.round(clamp(drag.startElX + dx, 0, PAGE_WIDTH - element.width) * 1000) / 1000
        element.y =
          Math.round(clamp(drag.startElY + dy, 0, PAGE_HEIGHT - element.height) * 1000) / 1000
      }
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

    if (node) {
      node.style.left = `${element.x}mm`
      node.style.top = `${element.y}mm`
      node.style.width = `${element.width}mm`
      node.style.height = `${element.height}mm`
    }
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

  function layoutStructure(value: ReportLayout): string {
    return `${value.mode}:${value.orientation}:${value.pages.length}`
  }

  function undo() {
    if (!history.length) return
    redoStack.push(cloneLayout(layout))
    const next = history.pop()!
    const needsRefresh = layoutStructure(next) !== layoutStructure(layout)
    layout = next
    selected = null
    applyLayoutToDom()
    if (needsRefresh) void refreshCanvasStructure()
  }

  function redo() {
    if (!redoStack.length) return
    history.push(cloneLayout(layout))
    const next = redoStack.pop()!
    const needsRefresh = layoutStructure(next) !== layoutStructure(layout)
    layout = next
    selected = null
    applyLayoutToDom()
    if (needsRefresh) void refreshCanvasStructure()
  }

  async function startBlankDocument() {
    const pages = Math.round(clamp(blankPageCount, 1, MAX_BLANK_PAGES))
    const confirmed = await dialog.confirm({
      title: 'Mulai dokumen kosong?',
      message: `Dokumen ${blankOrientation === 'landscape' ? 'landscape' : 'portrait'} dengan ${pages} halaman akan menggantikan desain laporan saat ini. Perubahan bisa di-undo.`,
      confirmLabel: 'Mulai blank',
    })
    if (!confirmed) return
    pushHistory()
    layout = {
      version: 1,
      mode: 'blank',
      orientation: blankOrientation,
      pages: Array.from({ length: pages }, (_, index) => ({
        page: index + 1,
        blocks: {},
        elements: [],
      })),
    }
    currentPage = 1
    selected = null
    checkedIds = []
    void refreshCanvasStructure()
  }

  function setDocumentOrientation(orientation: 'portrait' | 'landscape') {
    if (!isBlank) {
      blankOrientation = orientation
      return
    }
    if (layout.orientation === orientation) return
    pushHistory()
    layout.orientation = orientation
    for (const page of layout.pages) {
      for (const element of page.elements) clampElement(element)
    }
    void refreshCanvasStructure()
  }

  function addBlankPage() {
    if (!isBlank || layout.pages.length >= MAX_BLANK_PAGES) return
    pushHistory()
    layout.pages = [
      ...layout.pages,
      { page: layout.pages.length + 1, blocks: {}, elements: [] },
    ]
    currentPage = layout.pages.length
    selected = null
    checkedIds = []
    void refreshCanvasStructure()
  }

  async function removeBlankPage() {
    if (!isBlank || layout.pages.length <= 1) return
    const page = layout.pages.find((item) => item.page === currentPage)
    if (page?.elements.length) {
      const confirmed = await dialog.confirm({
        title: 'Hapus halaman?',
        message: 'Halaman ini berisi objek. Semua objek pada halaman tersebut ikut dihapus.',
        confirmLabel: 'Hapus halaman',
        tone: 'danger',
      })
      if (!confirmed) return
    }
    pushHistory()
    layout.pages = layout.pages
      .filter((item) => item.page !== currentPage)
      .map((item, index) => ({ ...item, page: index + 1 }))
    if (currentPage > layout.pages.length) currentPage = layout.pages.length
    selected = null
    checkedIds = []
    void refreshCanvasStructure()
  }

  async function backToReport() {
    const confirmed = await dialog.confirm({
      title: 'Kembali ke laporan?',
      message: 'Halaman blank saat ini akan diganti dengan laporan 9 halaman. Perubahan bisa di-undo.',
      confirmLabel: 'Kembali ke laporan',
      tone: 'danger',
    })
    if (!confirmed) return
    pushHistory()
    layout = normalizeLayout({
      version: 1,
      mode: 'report',
      orientation: 'portrait',
      pages: layout.pages,
    })
    currentPage = 1
    selected = null
    checkedIds = []
    void refreshCanvasStructure()
  }

  function goToPage(page: number) {
    if (page < 1 || page > pageNumbers.length) return
    if (page !== currentPage) {
      checkedIds = []
      selected = null
    }
    currentPage = page
    applyPageVisibility()
    renderSelection()
    centerCanvas()
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

  function openShapePicker() {
    shapePickerOpen = true
  }

  function chooseShape(shape: ShapeDefinition) {
    const element = baseElement('rect')
    element.shape = shape.kind
    if (shape.kind !== 'rect') {
      element.width = 36
      element.height = 30
    }
    addElement(element)
    shapePickerOpen = false
  }

  function addTable() {
    const element = baseElement('table')
    element.width = 120
    element.height = 60
    element.font_size = 9
    addElement(element)
  }

  function addChart() {
    const element = baseElement('chart')
    element.width = 120
    element.height = 80
    element.font_size = 9
    element.chart_title = 'Grafik baru'
    element.chart_type = 'bar'
    element.chart_labels = ['A', 'B', 'C', 'D']
    element.chart_values = [10, 20, 15, 25]
    element.chart_colors = ['#143d30', '#b8894b', '#3f765d', '#5b8fa8']
    addElement(element)
  }

  function addHBarChart() {
    const element = baseElement('chart')
    element.width = 110
    element.height = 70
    element.font_size = 9
    element.chart_title = 'Jenis kegiatan'
    element.chart_type = 'hbar'
    element.chart_labels = ['QW2.1', 'QW2.3', 'QW2.4', 'QW2.2']
    element.chart_values = [17, 2, 2, 1]
    element.chart_colors = ['#3f765d', '#3f765d', '#3f765d', '#3f765d']
    addElement(element)
  }

  function addCard() {
    const element = baseElement('card')
    element.width = 48
    element.height = 33
    element.card_label = 'Label kartu'
    element.card_value = '0'
    element.card_unit = 'unit'
    element.card_detail = ''
    addElement(element)
  }

  function openIconPickerForNew() {
    iconPickerTarget = 'new'
    iconSearch = ''
    iconPickerOpen = true
  }

  function openIconPickerForReplace() {
    if (!activeElement || activeElement.type !== 'icon') return
    iconPickerTarget = 'replace'
    iconPickerOpen = true
  }

  function chooseIcon(icon: IconDefinition) {
    if (iconPickerTarget === 'replace' && activeElement?.type === 'icon') {
      updateElement({ icon_name: icon.name }, true)
    } else {
      const element = baseElement('icon')
      element.width = 14
      element.height = 14
      element.icon_name = icon.name
      element.icon_color = '#143d30'
      element.icon_stroke = 2
      addElement(element)
      toast.success(`Ikon ${icon.label} ditambahkan.`)
    }
    iconPickerOpen = false
  }

  function portal(node: HTMLElement): { destroy: () => void } {
    document.body.appendChild(node)
    return { destroy: () => node.remove() }
  }

  function updateTableCell(rowIndex: number, columnIndex: number, value: string) {
    if (!activeElement || activeElement.type !== 'table' || activeElement.binding) return
    const rows = activeElement.table_rows
    if (!rows?.[rowIndex]) return
    rows[rowIndex][columnIndex] = value
    activeElement.table_rows = rows
    applyLayoutToDom()
  }

  function addTableRow() {
    if (!activeElement || activeElement.type !== 'table' || activeElement.binding) return
    pushHistory()
    const rows = activeElement.table_rows ?? []
    const columns = Math.max(1, ...rows.map((row) => row.length))
    activeElement.table_rows = [...rows, Array.from({ length: columns }, () => '')]
    applyLayoutToDom()
  }

  function removeTableRow() {
    if (!activeElement || activeElement.type !== 'table' || activeElement.binding) return
    const rows = activeElement.table_rows ?? []
    if (rows.length <= 1) return
    pushHistory()
    activeElement.table_rows = rows.slice(0, -1)
    applyLayoutToDom()
  }

  function addTableColumn() {
    if (!activeElement || activeElement.type !== 'table' || activeElement.binding) return
    pushHistory()
    const rows = (activeElement.table_rows ?? []).map((row) => [...row, ''])
    activeElement.table_rows = rows
    applyLayoutToDom()
  }

  function removeTableColumn() {
    if (!activeElement || activeElement.type !== 'table' || activeElement.binding) return
    const rows = activeElement.table_rows ?? []
    const columns = Math.max(1, ...rows.map((row) => row.length))
    if (columns <= 1) return
    pushHistory()
    activeElement.table_rows = rows.map((row) => row.slice(0, -1))
    applyLayoutToDom()
  }

  function parseLabelList(value: string): string[] {
    return value
      .split(/[\n,]+/)
      .map((item) => item.trim())
      .filter(Boolean)
  }

  function parseNumberList(value: string): number[] {
    return value
      .split(/[\n,;]+/)
      .map((item) => item.trim())
      .filter(Boolean)
      .map((item) => {
        const normalized = item.replace(/\./g, '').replace(',', '.')
        const number = Number(normalized)
        return Number.isFinite(number) ? number : 0
      })
  }

  function normalizeChart(labels: string[], values: number[]) {
    if (!activeElement || activeElement.type !== 'chart' || activeElement.binding) return
    const nextLabels = labels.length ? labels : ['A', 'B', 'C']
    const nextValues = nextLabels.map((_, index) => values[index] ?? 0)
    const colors = activeElement.chart_colors ?? []
    activeElement.chart_labels = nextLabels
    activeElement.chart_values = nextValues
    activeElement.chart_colors = nextLabels.map(
      (_, index) => colors[index] ?? CHART_PALETTE[index % CHART_PALETTE.length],
    )
    applyLayoutToDom()
  }

  function updateChartLabels(value: string) {
    if (!activeElement || activeElement.type !== 'chart' || activeElement.binding) return
    normalizeChart(parseLabelList(value), activeElement.chart_values ?? [])
  }

  function updateChartValues(value: string) {
    if (!activeElement || activeElement.type !== 'chart' || activeElement.binding) return
    normalizeChart(activeElement.chart_labels ?? [], parseNumberList(value))
  }

  function updateChartColors(value: string) {
    if (!activeElement || activeElement.type !== 'chart') return
    activeElement.chart_colors = parseLabelList(value)
    applyLayoutToDom()
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

  async function addImageUrl() {
    const value = await dialog.prompt({
      title: 'Tambah gambar dari URL',
      message: 'Masukkan tautan langsung ke gambar yang dapat diakses publik.',
      inputLabel: 'URL gambar',
      inputPlaceholder: 'https://contoh.com/gambar.png',
      inputType: 'url',
      inputHint: 'Hanya URL http atau https yang didukung, sebaiknya berformat PNG atau JPEG.',
      confirmLabel: 'Tambahkan',
    })
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
      const element = getElement(selected.page, selected.id)
      const source = element?.source_block
      const group = element?.group
      const elementId = selected.id
      pushHistory()
      page.elements = page.elements.filter((item) => item.id !== elementId)
      if (source) delete page.blocks[source]
      if (group) {
        const remaining = page.elements.filter((item) => item.group === group)
        if (remaining.length <= 1) {
          for (const item of remaining) item.group = ''
        }
      }
      checkedIds = checkedIds.filter((id) => id !== elementId)
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
      source_block: '',
      group: '',
    }
    clampElement(copy)
    page.elements.push(copy)
    selected = { kind: 'element', page: selected.page, id: copy.id }
    applyLayoutToDom()
  }

  function moveLayer(delta: number) {
    if (!selected || selected.kind !== 'element') return
    moveLayerOf(selected.id, delta)
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
    const current = page.blocks[selected.id]
    if (current && current.texts && Object.keys(current.texts).length) {
      page.blocks[selected.id] = { dx: 0, dy: 0, hidden: false, texts: current.texts }
    } else {
      delete page.blocks[selected.id]
    }
    applyLayoutToDom()
  }

  function hideBlock() {
    if (!selected || selected.kind !== 'block') return
    const page = getPage(selected.page)
    if (!page) return
    pushHistory()
    const block = page.blocks[selected.id] ?? { dx: 0, dy: 0, hidden: false }
    block.hidden = true
    page.blocks[selected.id] = block
    selected = null
    applyLayoutToDom()
  }

  function blockRect(blockEl: HTMLElement, pageEl: HTMLElement) {
    const pageRect = pageEl.getBoundingClientRect()
    const rect = blockEl.getBoundingClientRect()
    const round = (value: number) => Math.round(value * 1000) / 1000
    return {
      x: round(((rect.left - pageRect.left) / pageRect.width) * PAGE_WIDTH),
      y: round(((rect.top - pageRect.top) / pageRect.height) * PAGE_HEIGHT),
      width: round((rect.width / pageRect.width) * PAGE_WIDTH),
      height: round((rect.height / pageRect.height) * PAGE_HEIGHT),
    }
  }

  function parseFormattedNumber(value: string): number {
    const normalized = value.trim().replace(/\./g, '').replace(',', '.')
    const number = Number(normalized)
    return Number.isFinite(number) ? number : 0
  }

  function detachBlock() {
    if (!selected || selected.kind !== 'block') return
    const doc = iframeEl?.contentDocument
    if (!doc) return
    const page = getPage(selected.page)
    const pageEl = doc.querySelectorAll<HTMLElement>('.page')[selected.page - 1]
    const blockEl = pageEl?.querySelector<HTMLElement>(`[data-block="${selected.id}"]`)
    if (!page || !pageEl || !blockEl) return
    const rect = blockRect(blockEl, pageEl)
    const tableEl = blockEl.querySelector<HTMLTableElement>('table')
    const barRows = Array.from(blockEl.querySelectorAll<HTMLElement>('.bar-row'))
    const cardEls = Array.from(blockEl.querySelectorAll<HTMLElement>('.kpi'))
    const elements: LayoutElement[] = []
    let message = ''

    if (tableEl) {
      const rows = Array.from(tableEl.querySelectorAll('tr'))
        .map((row) =>
          Array.from(row.querySelectorAll('th,td')).map((cell) => (cell.textContent ?? '').trim()),
        )
        .filter((row) => row.length > 0)
      if (rows.length) {
        const table = baseElement('table')
        table.table_rows = rows
        table.table_header = Boolean(tableEl.querySelector('thead')) || Boolean(tableEl.querySelector('tr th'))
        table.font_size = 8
        table.fill = '#ffffff'
        table.color = '#143d30'
        table.border_color = '#d7ddd6'
        table.border_width = 0.5
        table.table_header_bg = '#f6f8f5'
        table.table_header_color = '#5b6861'
        table.table_stripe = 'transparent'
        table.source_block = selected.id
        elements.push(table)
        message = 'Blok di-detach. Isi tabel sekarang bisa diedit.'
      }
    } else if (barRows.length) {
      const labels = barRows.map(
        (row) => row.querySelector('.bar-label')?.textContent?.trim() ?? '',
      )
      const values = barRows.map((row) =>
        parseFormattedNumber(row.querySelector('.bar-value')?.textContent ?? ''),
      )
      const chart = baseElement('chart')
      chart.chart_type = 'hbar'
      chart.chart_title = blockEl.querySelector('h3')?.textContent?.trim() ?? ''
      chart.chart_labels = labels
      chart.chart_values = values
      chart.chart_colors = labels.map(() => '#3f765d')
      chart.chart_show_values = true
      chart.chart_show_legend = false
      chart.chart_axis_color = '#6d7872'
      chart.fill = '#ffffff'
      chart.color = '#143d30'
      chart.font_size = 9
      chart.source_block = selected.id
      elements.push(chart)
      message = 'Blok di-detach. Grafik sekarang bisa diedit.'
    } else if (cardEls.length) {
      const group = newId()
      for (const cardEl of cardEls) {
        const cardRect = blockRect(cardEl, pageEl)
        const card = baseElement('card')
        card.card_label = cardEl.querySelector('.kpi-label')?.textContent?.trim() ?? ''
        card.card_value = cardEl.querySelector('strong')?.textContent?.trim() ?? ''
        card.card_unit = cardEl.querySelector('.kpi-unit')?.textContent?.trim() ?? ''
        card.x = cardRect.x
        card.y = cardRect.y
        card.width = cardRect.width
        card.height = cardRect.height
        card.z = 30
        card.source_block = selected.id
        card.group = group
        clampElement(card)
        elements.push(card)
      }
      message = `${elements.length} kartu di-detach dan digabung. Gunakan Pisahkan di panel Layer untuk memisahkannya.`
    }

    if (!elements.length) {
      toast.error('Blok ini tidak memiliki tabel, grafik, atau kartu yang bisa di-detach.')
      return
    }

    pushHistory()
    if (tableEl || barRows.length) {
      const element = elements[0]
      element.x = rect.x
      element.y = rect.y
      element.width = rect.width
      element.height = rect.height
      element.z = 30
      clampElement(element)
    }
    const block = page.blocks[selected.id] ?? { dx: 0, dy: 0, hidden: false }
    block.hidden = true
    block.preserve_space = true
    page.blocks[selected.id] = block
    page.elements.push(...elements)
    selected = { kind: 'element', page: selected.page, id: elements[0].id }
    applyLayoutToDom()
    toast.success(message)
  }

  function resetDetached() {
    if (!selected || selected.kind !== 'element') return
    const element = getElement(selected.page, selected.id)
    const source = element?.source_block
    if (!source) return
    const page = getPage(selected.page)
    if (!page) return
    const removed = new Set(
      page.elements.filter((item) => item.source_block === source).map((item) => item.id),
    )
    pushHistory()
    page.elements = page.elements.filter((item) => item.source_block !== source)
    checkedIds = checkedIds.filter((id) => !removed.has(id))
    const current = page.blocks[source]
    if (current?.texts && Object.keys(current.texts).length) {
      page.blocks[source] = { dx: 0, dy: 0, hidden: false, texts: current.texts }
    } else {
      delete page.blocks[source]
    }
    selected = { kind: 'block', page: selected.page, id: source }
    applyLayoutToDom()
    toast.success('Blok dikembalikan ke data otomatis.')
  }

  function showBlock(id: string) {
    const page = getPage(currentPage)
    if (!page) return
    const block = page.blocks[id] ?? { dx: 0, dy: 0, hidden: false }
    block.hidden = false
    page.blocks[id] = block
    selected = { kind: 'block', page: currentPage, id }
    applyLayoutToDom()
  }

  function pickerColor(value: string | undefined, fallback = '#143d30'): string {
    return value && /^#[0-9a-fA-F]{6}$/.test(value) ? value : fallback
  }

  function isTransparentColor(value: string | undefined | null): boolean {
    const normalized = value?.trim().toLowerCase() ?? ''
    return !normalized || normalized === 'transparent' || normalized === 'none'
  }

  let lastRectFill = $state('#ffffff')
  let lastRectBorder = $state('#143d30')

  function toggleRectFill(transparent: boolean) {
    const element = activeElement
    if (!element || element.type !== 'rect') return
    pushHistory()
    if (transparent) {
      if (!isTransparentColor(element.fill)) {
        lastRectFill = pickerColor(element.fill, '#ffffff')
      }
      updateElement({ fill: 'transparent' })
    } else {
      updateElement({ fill: lastRectFill })
    }
  }

  function toggleRectStroke(disabled: boolean) {
    const element = activeElement
    if (!element || element.type !== 'rect') return
    pushHistory()
    if (disabled) {
      if (!isTransparentColor(element.border_color)) {
        lastRectBorder = pickerColor(element.border_color, '#143d30')
      }
      updateElement({ border_color: 'transparent' })
    } else {
      updateElement({
        border_color: lastRectBorder,
        border_width: (element.border_width ?? 0) > 0 ? element.border_width : 0.5,
      })
    }
  }

  function rectFillMode(element: LayoutElement): 'solid' | 'linear' | 'radial' {
    return element.fill_mode === 'linear' || element.fill_mode === 'radial'
      ? element.fill_mode
      : 'solid'
  }

  function gradientColors(element: LayoutElement): string[] {
    const colors = element.gradient_colors ?? []
    if (colors.length >= 2) return colors
    return [pickerColor(element.fill, '#ffffff'), '#b8894b']
  }

  function gradientCss(element: LayoutElement): string {
    const colors = element.gradient_colors ?? []
    const mode = rectFillMode(element)
    if (mode === 'solid' || colors.length < 2) return element.fill ?? '#ffffff'
    const last = colors.length - 1
    const stops = colors
      .map((color, index) => `${color} ${((index / last) * 100).toFixed(1)}%`)
      .join(', ')
    return mode === 'radial'
      ? `radial-gradient(circle, ${stops})`
      : `linear-gradient(${element.gradient_angle ?? 90}deg, ${stops})`
  }

  function setFillMode(mode: 'solid' | 'linear' | 'radial') {
    const element = activeElement
    if (!element || element.type !== 'rect') return
    pushHistory()
    updateElement({ fill_mode: mode, gradient_colors: gradientColors(element) })
  }

  function setGradientColor(index: number, color: string) {
    const element = activeElement
    if (!element || element.type !== 'rect') return
    const colors = [...gradientColors(element)]
    colors[index] = color
    updateElement({ gradient_colors: colors })
  }

  function addGradientColor() {
    const element = activeElement
    if (!element || element.type !== 'rect') return
    const colors = gradientColors(element)
    if (colors.length >= 8) return
    pushHistory()
    const palette = ['#143d30', '#b8894b', '#3f765d', '#5b8fa8', '#a4743d', '#d9b87e', '#7a857f', '#2f5d7c']
    updateElement({ gradient_colors: [...colors, palette[colors.length % palette.length]] })
  }

  function removeGradientColor(index: number) {
    const element = activeElement
    if (!element || element.type !== 'rect') return
    const colors = gradientColors(element)
    if (colors.length <= 2) return
    pushHistory()
    updateElement({ gradient_colors: colors.filter((_, itemIndex) => itemIndex !== index) })
  }

  function numberValue(value: string): number {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : 0
  }

  async function save(silent = false): Promise<boolean> {
    if (!selectedImportId) return false
    saving = true
    error = ''
    const beforeStructure = layoutStructure(layout)
    try {
      const view = await designApi.saveLayout(periodId, selectedImportId, layout)
      const previous = selected
      layout = normalizeLayout(view.layout)
      savedJson = JSON.stringify(layout)
      updatedAt = view.updated_at
      updatedBy = view.updated_by
      if (previous?.kind === 'element' && !getElement(previous.page, previous.id)) {
        selected = null
      } else {
        selected = previous
      }
      applyLayoutToDom()
      if (layoutStructure(layout) !== beforeStructure) void refreshCanvasStructure()
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
      const result = await reportsApi.preview(periodId, selectedImportId, content, layout)
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
    if (selected.kind === 'element') {
      const page = getPage(selected.page)
      const element = getElement(selected.page, selected.id)
      if (!page || !element) return
      const members = groupMembers(page, element)
      let deltaX = dx
      let deltaY = dy
      for (const member of members) {
        deltaX = clamp(deltaX, -member.x, PAGE_WIDTH - member.width - member.x)
        deltaY = clamp(deltaY, -member.y, PAGE_HEIGHT - member.height - member.y)
      }
      pushHistory()
      for (const member of members) {
        member.x = Math.round((member.x + deltaX) * 1000) / 1000
        member.y = Math.round((member.y + deltaY) * 1000) / 1000
      }
    } else {
      const page = getPage(selected.page)
      if (!page) return
      pushHistory()
      const current = page.blocks[selected.id] ?? { dx: 0, dy: 0 }
      page.blocks[selected.id] = { ...current, dx: current.dx + dx, dy: current.dy + dy }
    }
    applyLayoutToDom()
  }

  function handleKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null
    const tag = target?.tagName?.toLowerCase()
    if (tag === 'input' || tag === 'textarea' || tag === 'select' || target?.isContentEditable) return
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
    if (mod && (event.key === '+' || event.key === '=')) {
      event.preventDefault()
      zoomBy(0.1)
      return
    }
    if (mod && event.key === '-') {
      event.preventDefault()
      zoomBy(-0.1)
      return
    }
    if (mod && event.key === '0') {
      event.preventDefault()
      resetZoom()
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
    if (event.key === 'PageUp') {
      event.preventDefault()
      goToPage(currentPage - 1)
    } else if (event.key === 'PageDown') {
      event.preventDefault()
      goToPage(currentPage + 1)
    } else if (event.key === 'ArrowLeft') {
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
        <Button variant="secondary" size="sm" onclick={openShapePicker}><Star size={14} /> Bentuk</Button>
        <Button variant="secondary" size="sm" loading={uploading} onclick={pickImage}><ImagePlus size={14} /> Gambar</Button>
        <Button variant="secondary" size="sm" onclick={addImageUrl}><Link2 size={14} /> URL</Button>
        <Button variant="secondary" size="sm" onclick={addTable}><Table2 size={14} /> Tabel</Button>
        <Button variant="secondary" size="sm" onclick={addChart}><BarChart3 size={14} /> Grafik</Button>
        <Button variant="secondary" size="sm" onclick={addHBarChart}><AlignLeft size={14} /> Bar Samping</Button>
        <Button variant="secondary" size="sm" onclick={addCard}><CreditCard size={14} /> Kartu</Button>
        <Button variant="secondary" size="sm" onclick={openIconPickerForNew}><Shapes size={14} /> Ikon</Button>
        <span class="mx-1 h-6 w-px bg-line"></span>
        <Button variant="ghost" size="sm" disabled={!history.length} onclick={undo}><Undo2 size={14} /> Undo</Button>
        <Button variant="ghost" size="sm" disabled={!redoStack.length} onclick={redo}><Redo2 size={14} /> Redo</Button>
        <input bind:this={fileInput} class="hidden" type="file" accept="image/png,image/jpeg" onchange={handleFile} />
      </div>

      <div class="flex flex-wrap items-center justify-between gap-2 border-b border-line bg-stone-50/60 px-3 py-2">
        <div class="flex shrink-0 items-center gap-2" data-page-nav>
          <Button variant="ghost" size="sm" disabled={currentPage <= 1} onclick={() => goToPage(currentPage - 1)}>
            <ChevronLeft size={14} /> Sebelumnya
          </Button>
          <select class="input max-w-[150px] py-1.5 text-xs" value={currentPage}
            data-page-selector
            onchange={(event) => goToPage(Number((event.currentTarget as HTMLSelectElement).value))}>
            {#each pageNumbers as page}
              <option value={page}>Halaman {page} / {pageNumbers.length}</option>
            {/each}
          </select>
          <Button variant="ghost" size="sm" disabled={currentPage >= pageNumbers.length} onclick={() => goToPage(currentPage + 1)}>
            Berikutnya <ChevronRight size={14} />
          </Button>
        </div>
        <div class="flex shrink-0 items-center gap-1" data-zoom-controls>
          <span class="ml-1 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Zoom</span>
          <Button variant="ghost" size="sm" disabled={!canZoomOut} onclick={() => zoomBy(-0.1)}>
            <span data-zoom-action="out"><Minus size={14} /></span>
          </Button>
          <span class="min-w-[44px] text-center text-[11px] font-semibold text-ink-muted" data-zoom-label>{zoomPercent}%</span>
          <Button variant="ghost" size="sm" disabled={!canZoomIn} onclick={() => zoomBy(0.1)}>
            <span data-zoom-action="in"><Plus size={14} /></span>
          </Button>
          <Button variant="ghost" size="sm" disabled={atFitZoom} onclick={resetZoom}>
            <span data-zoom-action="fit"><Maximize2 size={14} /> Pas</span>
          </Button>
        </div>
      </div>

      <div bind:this={canvasAreaEl} data-canvas-area class="h-[calc(100vh-230px)] min-h-[480px] overflow-auto bg-stone-100">
        <div class="flex min-h-full w-max min-w-full items-center justify-center p-3">
          {#if loading}
            <div class="text-sm text-ink-muted">Menyiapkan kanvas laporan…</div>
          {:else}
            <iframe
              bind:this={iframeEl}
              title="Editor tata letak laporan"
              class="block rounded-lg border border-line bg-white"
              style={`width:${canvasWidth}px;height:${canvasHeight}px`}
              srcdoc={iframeHtml}
              onload={handleIframeLoad}
            ></iframe>
          {/if}
        </div>
      </div>

      <div class="border-t border-line bg-stone-50/60 px-4 py-2.5 text-[10px] leading-relaxed text-ink-muted">
        Satu halaman ditampilkan pada satu waktu. Gunakan Sebelumnya/Berikutnya atau Page Up/Page Down untuk berpindah halaman.
        Tombol −/+ atau Ctrl +/− untuk zoom, dan Pas untuk kembali menampilkan satu halaman penuh. Perubahan baru masuk ke PDF setelah disimpan.
      </div>
    </section>

    <aside class="space-y-4 self-start xl:sticky xl:top-24">
      {#if activeElement}
        <section class="card overflow-hidden">
          <div class="panel-heading">
            <div>
              <span class="eyebrow">ELEMEN TERPILIH</span>
              <h2 class="panel-title mt-1">
                {activeElement.type === 'text' ? 'Teks' : activeElement.type === 'rect' ? shapeLabel(activeElement.shape) : activeElement.type === 'table' ? 'Tabel' : activeElement.type === 'chart' ? 'Grafik' : activeElement.type === 'card' ? 'Kartu' : activeElement.type === 'icon' ? 'Ikon' : 'Gambar'}
              </h2>
            </div>
            <div class="flex flex-col items-end gap-1">
              <span class="badge badge-neutral">Halaman {selected?.page}</span>
              {#if activeElement.hidden}<span class="badge badge-neutral">Tersembunyi</span>{/if}
              {#if activeElement.binding}<span class="badge badge-success">Data</span>{/if}
              {#if activeElement.source_block}<span class="badge badge-progress">Hasil detach</span>{/if}
            </div>
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
                    data-text-font
                    onchange={(event) => updateElement({ font_family: (event.currentTarget as HTMLSelectElement).value }, true)}>
                    <optgroup label="Sistem">
                      {#each systemFontOptions as font}
                        <option value={font.key}>{font.label}</option>
                      {/each}
                    </optgroup>
                    <optgroup label="Google Fonts · Sans">
                      {#each sansFontOptions as font}
                        <option value={font.key}>{font.label}</option>
                      {/each}
                    </optgroup>
                    <optgroup label="Google Fonts · Serif">
                      {#each serifFontOptions as font}
                        <option value={font.key}>{font.label}</option>
                      {/each}
                    </optgroup>
                  </select>
                </label>
                <label class="block">
                  <span class="label">Warna</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.color)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Rata horizontal</span>
                  <select class="input py-1.5 text-xs" value={activeElement.align ?? 'left'}
                    data-text-align
                    onchange={(event) => updateElement({ align: (event.currentTarget as HTMLSelectElement).value as 'left' | 'center' | 'right' }, true)}>
                    <option value="left">Kiri</option>
                    <option value="center">Tengah</option>
                    <option value="right">Kanan</option>
                  </select>
                </label>
                <label class="block">
                  <span class="label">Rata vertikal</span>
                  <select class="input py-1.5 text-xs" value={activeElement.valign ?? 'top'}
                    data-text-valign
                    onchange={(event) => updateElement({ valign: (event.currentTarget as HTMLSelectElement).value as 'top' | 'middle' | 'bottom' }, true)}>
                    <option value="top">Atas</option>
                    <option value="middle">Tengah</option>
                    <option value="bottom">Bawah</option>
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
              <div>
                <span class="label">Bentuk</span>
                <div class="mt-1 grid grid-cols-4 gap-1.5">
                  {#each shapeCatalog as shape (shape.kind)}
                    <button type="button"
                      class="flex min-w-0 flex-col items-center gap-1 rounded-lg border px-1 py-1.5 hover:border-gold-300 hover:bg-gold-50 {shapeDefinition(activeElement.shape).kind === shape.kind ? 'border-gold-300 bg-gold-50 text-forest-800' : 'border-line text-ink-muted'}"
                      title={shape.label} data-shape-choice={shape.kind}
                      onclick={() => updateElement({ shape: shape.kind }, true)}>
                      <svg class="h-4 w-4" viewBox="0 0 100 100" fill="none" stroke="currentColor"
                        stroke-width="6" stroke-linejoin="round" aria-hidden="true">
                        {#if shape.kind === 'rect'}
                          <rect x="4" y="16" width="92" height="68" rx="5" />
                        {:else if shape.ellipse}
                          <ellipse cx="50" cy="50" rx="46" ry="46" />
                        {:else if shape.path}
                          <path d={shape.path} />
                        {:else}
                          <polygon points={shape.points} />
                        {/if}
                      </svg>
                      <span class="w-full truncate text-center text-[8px] leading-tight">{shape.label}</span>
                    </button>
                  {/each}
                </div>
              </div>
              <div class="grid grid-cols-2 gap-3">
                <label class="flex items-center gap-2 text-xs font-medium text-ink">
                  <input type="checkbox" checked={rectNoFill} data-rect-no-fill
                    onchange={(event) => toggleRectFill((event.currentTarget as HTMLInputElement).checked)} />
                  Tanpa isi
                </label>
                <label class="flex items-center gap-2 text-xs font-medium text-ink">
                  <input type="checkbox" checked={rectNoStroke} data-rect-no-stroke
                    onchange={(event) => toggleRectStroke((event.currentTarget as HTMLInputElement).checked)} />
                  Tanpa garis
                </label>
              </div>
              <label class="block">
                <span class="label">Tipe isi</span>
                <select class="input py-1.5 text-xs disabled:opacity-40" value={rectFillMode(activeElement)}
                  data-rect-fill-mode
                  disabled={rectNoFill}
                  onchange={(event) => setFillMode((event.currentTarget as HTMLSelectElement).value as 'solid' | 'linear' | 'radial')}>
                  <option value="solid">Warna solid</option>
                  <option value="linear">Gradien linear</option>
                  <option value="radial">Gradien radial</option>
                </select>
              </label>
              {#if rectFillMode(activeElement) === 'solid'}
                <label class="block">
                  <span class="label">Isi</span>
                  <input class="input h-9 py-1 disabled:opacity-40" type="color" value={pickerColor(activeElement.fill, lastRectFill)}
                    data-rect-fill
                    disabled={rectNoFill}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ fill: (event.currentTarget as HTMLInputElement).value })} />
                </label>
              {:else}
                <div class="space-y-2.5 rounded-xl border border-line p-3">
                  <span class="block h-7 rounded-md border border-line" data-gradient-preview
                    style={`background:${gradientCss(activeElement)}`}></span>
                  <div class="space-y-1.5">
                    {#each gradientColors(activeElement) as color, index (index)}
                      <div class="flex items-center gap-2">
                        <input class="input h-8 min-w-0 flex-1 py-1 disabled:opacity-40" type="color" value={pickerColor(color, '#ffffff')}
                          data-gradient-color={index}
                          disabled={rectNoFill}
                          onfocus={pushHistory}
                          oninput={(event) => setGradientColor(index, (event.currentTarget as HTMLInputElement).value)} />
                        <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100 disabled:opacity-30"
                          title="Hapus warna" data-gradient-remove={index}
                          disabled={rectNoFill || gradientColors(activeElement).length <= 2}
                          onclick={() => removeGradientColor(index)}>
                          <Trash2 size={13} />
                        </button>
                      </div>
                    {/each}
                  </div>
                  <div class="flex flex-wrap items-center gap-2">
                    <span data-gradient-add>
                      <Button variant="secondary" size="sm" disabled={rectNoFill || gradientColors(activeElement).length >= 8}
                        onclick={addGradientColor}>
                        <Plus size={13} /> Tambah warna
                      </Button>
                    </span>
                    {#if rectFillMode(activeElement) === 'linear'}
                      <label class="flex items-center gap-1.5 text-[10px] text-ink-muted">Sudut
                        <input class="input w-16 py-1 text-xs disabled:opacity-40" type="number" min="0" max="360" step="5"
                          value={activeElement.gradient_angle ?? 90}
                          data-gradient-angle
                          disabled={rectNoFill}
                          onfocus={pushHistory}
                          oninput={(event) => updateElement({ gradient_angle: numberValue((event.currentTarget as HTMLInputElement).value) })} />°
                      </label>
                    {/if}
                  </div>
                  <p class="text-[10px] leading-relaxed text-ink-muted">Warna tersebar merata dari 0% sampai 100%. Bisa memakai 2–8 warna.</p>
                </div>
              {/if}
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Garis</span>
                  <input class="input h-9 py-1 disabled:opacity-40" type="color" value={pickerColor(activeElement.border_color, lastRectBorder)}
                    data-rect-border
                    disabled={rectNoStroke}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ border_color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Tebal garis (pt)</span>
                  <input class="input py-1.5 text-xs disabled:opacity-40" type="number" min="0" step="0.25" value={activeElement.border_width ?? 0.5}
                    disabled={rectNoStroke}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ border_width: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
                {#if shapeDefinition(activeElement.shape).kind === 'rect'}
                  <label class="block">
                    <span class="label">Sudut (mm)</span>
                    <input class="input py-1.5 text-xs" type="number" min="0" step="0.5" value={activeElement.radius ?? 1}
                      onfocus={pushHistory}
                      oninput={(event) => updateElement({ radius: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                  </label>
                {/if}
              </div>
              <p class="text-[10px] leading-relaxed text-ink-muted">Tanpa isi membuat latar transparan. Tanpa garis menyembunyikan garis tepi tanpa menghapus warna dan tebalnya.</p>
            {:else if activeElement.type === 'icon'}
              <div class="flex items-center gap-3 rounded-xl border border-line p-3">
                <span class="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg bg-forest-50 text-forest-800">
                  <svg class="h-7 w-7" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                    stroke-width={activeElement.icon_stroke ?? 2} stroke-linecap="round" stroke-linejoin="round">
                    {@html iconByName.get(activeElement.icon_name ?? '')?.svg ?? ''}
                  </svg>
                </span>
                <div class="min-w-0 flex-1">
                  <p class="truncate text-xs font-semibold text-ink">{iconByName.get(activeElement.icon_name ?? '')?.label ?? 'Ikon tidak dikenal'}</p>
                  <p class="truncate text-[10px] text-ink-muted">{activeElement.icon_name ?? ''}</p>
                </div>
                <Button variant="secondary" size="sm" onclick={openIconPickerForReplace}>Ganti</Button>
              </div>
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Warna ikon</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.icon_color)}
                    data-icon-color
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ icon_color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Ketebalan garis</span>
                  <input class="input py-1.5 text-xs" type="number" min="1" max="4" step="0.25" value={activeElement.icon_stroke ?? 2}
                    data-icon-stroke
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ icon_stroke: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
              </div>
              <p class="text-[10px] leading-relaxed text-ink-muted">Ukuran ikon mengikuti kotak elemen. Atur lebar/tinggi pada kolom di atas.</p>
            {:else if activeElement.type === 'card'}
              <label class="block">
                <span class="label">Label</span>
                <input class="input py-1.5 text-xs" value={activeElement.card_label ?? ''}
                  data-card-field="label"
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ card_label: (event.currentTarget as HTMLInputElement).value })} />
              </label>
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Nilai</span>
                  <input class="input py-1.5 text-xs" value={activeElement.card_value ?? ''}
                    data-card-field="value"
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ card_value: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Satuan</span>
                  <input class="input py-1.5 text-xs" value={activeElement.card_unit ?? ''}
                    data-card-field="unit"
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ card_unit: (event.currentTarget as HTMLInputElement).value })} />
                </label>
              </div>
              <label class="block">
                <span class="label">Keterangan tambahan</span>
                <input class="input py-1.5 text-xs" value={activeElement.card_detail ?? ''}
                  data-card-field="detail"
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ card_detail: (event.currentTarget as HTMLInputElement).value })} />
              </label>
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Warna aksen</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.card_accent)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ card_accent: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Latar kartu</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.fill)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ fill: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Warna nilai</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.color)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Sudut (mm)</span>
                  <input class="input py-1.5 text-xs" type="number" min="0" step="0.5" value={activeElement.radius ?? 1.5}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ radius: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
                <label class="block">
                  <span class="label">Ukuran teks (pt)</span>
                  <input class="input py-1.5 text-xs" type="number" min="6" step="0.5" value={activeElement.font_size ?? 9}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ font_size: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
                <label class="block">
                  <span class="label">Tebal aksen (pt)</span>
                  <input class="input py-1.5 text-xs" type="number" min="0" step="0.5" value={activeElement.border_width ?? 2}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ border_width: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
              </div>
            {:else if activeElement.type === 'table'}
              <label class="block">
                <span class="label">Sumber data</span>
                <select class="input py-1.5 text-xs" value={activeElement.binding ?? ''}
                  data-source-field="binding"
                  onchange={(event) => bindSource((event.currentTarget as HTMLSelectElement).value)}>
                  <option value="">Manual — tidak terhubung</option>
                  {#each tableSourceGroups as group}
                    <optgroup label={group.category}>
                      {#each group.items as source}
                        <option value={source.key}>{source.label}</option>
                      {/each}
                    </optgroup>
                  {/each}
                </select>
              </label>
              {#if activeElement.binding}
                <div class="rounded-xl border border-forest-200 bg-forest-50 p-3.5 text-[11px] leading-relaxed text-forest-800">
                  <p>
                    Terhubung ke <strong>{sourceFor(activeElement.binding)?.label ?? activeElement.binding}</strong>.
                    Isi tabel mengikuti data terbaru dan tidak bisa diedit manual.
                  </p>
                  <div class="mt-3 flex flex-wrap gap-2">
                    <Button variant="secondary" size="sm" onclick={() => bindSource('')}>Lepas ikatan</Button>
                    <Button variant="ghost" size="sm" onclick={() => void refreshSources()}>Muat ulang data</Button>
                  </div>
                </div>
              {:else}
                <label class="flex items-center gap-2 text-xs">
                  <input type="checkbox" checked={activeElement.table_header !== false}
                    onchange={(event) => updateElement({ table_header: (event.currentTarget as HTMLInputElement).checked }, true)} />
                  Baris pertama sebagai header
                </label>
                <div class="flex flex-wrap gap-2">
                  <Button variant="secondary" size="sm" onclick={addTableRow}>+ Baris</Button>
                  <Button variant="secondary" size="sm" onclick={removeTableRow}>− Baris</Button>
                  <Button variant="secondary" size="sm" onclick={addTableColumn}>+ Kolom</Button>
                  <Button variant="secondary" size="sm" onclick={removeTableColumn}>− Kolom</Button>
                </div>
                <div class="max-h-64 overflow-auto rounded-lg border border-line">
                  <div class="min-w-max">
                    {#each activeElement.table_rows ?? [] as row, rowIndex}
                      <div class="flex">
                        {#each row as cell, columnIndex}
                          <input class="input w-28 rounded-none border-0 border-b border-r border-line px-2 py-1 text-[11px]" value={cell}
                            data-table-cell={`${rowIndex}-${columnIndex}`}
                            onfocus={pushHistory}
                            oninput={(event) => updateTableCell(rowIndex, columnIndex, (event.currentTarget as HTMLInputElement).value)} />
                        {/each}
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Ukuran teks (pt)</span>
                  <input class="input py-1.5 text-xs" type="number" min="6" step="0.5" value={activeElement.font_size ?? 9}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ font_size: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
                <label class="block">
                  <span class="label">Latar tabel</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.fill)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ fill: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Warna teks</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.color)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Warna border</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.border_color)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ border_color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Tebal border (pt)</span>
                  <input class="input py-1.5 text-xs" type="number" min="0" step="0.25" value={activeElement.border_width ?? 0.5}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ border_width: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
                <label class="block">
                  <span class="label">Latar header</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.table_header_bg)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ table_header_bg: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Teks header</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.table_header_color)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ table_header_color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Baris ganjil</span>
                  <input class="input py-1.5 text-xs" value={activeElement.table_stripe ?? '#f6f8f5'}
                    onfocus={pushHistory}
                    onchange={(event) => updateElement({ table_stripe: (event.currentTarget as HTMLInputElement).value }, true)} />
                </label>
              </div>
            {:else if activeElement.type === 'chart'}
              <label class="block">
                <span class="label">Sumber data</span>
                <select class="input py-1.5 text-xs" value={activeElement.binding ?? ''}
                  data-source-field="binding"
                  onchange={(event) => bindSource((event.currentTarget as HTMLSelectElement).value)}>
                  <option value="">Manual — tidak terhubung</option>
                  {#each chartSourceGroups as group}
                    <optgroup label={group.category}>
                      {#each group.items as source}
                        <option value={source.key}>{source.label}</option>
                      {/each}
                    </optgroup>
                  {/each}
                </select>
              </label>
              {#if activeElement.binding}
                <div class="rounded-xl border border-forest-200 bg-forest-50 p-3.5 text-[11px] leading-relaxed text-forest-800">
                  <p>
                    Terhubung ke <strong>{sourceFor(activeElement.binding)?.label ?? activeElement.binding}</strong>.
                    Label dan nilai mengikuti data terbaru; ubah jenis, judul, dan warna tetap bisa dilakukan.
                  </p>
                  <div class="mt-3 flex flex-wrap gap-2">
                    <Button variant="secondary" size="sm" onclick={() => bindSource('')}>Lepas ikatan</Button>
                    <Button variant="ghost" size="sm" onclick={() => void refreshSources()}>Muat ulang data</Button>
                  </div>
                </div>
              {/if}
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Jenis</span>
                  <select class="input py-1.5 text-xs" value={activeElement.chart_type ?? 'bar'}
                    data-chart-field="type"
                    onchange={(event) => updateElement({ chart_type: (event.currentTarget as HTMLSelectElement).value as 'bar' | 'line' | 'pie' | 'donut' | 'hbar' }, true)}>
                    <option value="bar">Bar</option>
                    <option value="hbar">Bar ke samping</option>
                    <option value="line">Line</option>
                    <option value="pie">Pie</option>
                    <option value="donut">Donut</option>
                  </select>
                </label>
                <label class="block">
                  <span class="label">Ukuran teks (pt)</span>
                  <input class="input py-1.5 text-xs" type="number" min="6" step="0.5" value={activeElement.font_size ?? 9}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ font_size: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                </label>
              </div>
              <label class="block">
                <span class="label">Judul grafik</span>
                <input class="input py-1.5 text-xs" value={activeElement.chart_title ?? ''}
                  data-chart-field="title"
                  onfocus={pushHistory}
                  onchange={(event) => updateElement({ chart_title: (event.currentTarget as HTMLInputElement).value }, true)} />
              </label>
              {#if !activeElement.binding}
                <label class="block">
                  <span class="label">Label (satu per baris)</span>
                  <textarea class="textarea text-xs" rows="4" value={(activeElement.chart_labels ?? []).join('\n')}
                    data-chart-field="labels"
                    onfocus={pushHistory}
                    onchange={(event) => updateChartLabels((event.currentTarget as HTMLTextAreaElement).value)}
                  ></textarea>
                </label>
                <label class="block">
                  <span class="label">Nilai (satu per baris)</span>
                  <textarea class="textarea text-xs" rows="4" value={(activeElement.chart_values ?? []).join('\n')}
                    data-chart-field="values"
                    onfocus={pushHistory}
                    onchange={(event) => updateChartValues((event.currentTarget as HTMLTextAreaElement).value)}
                  ></textarea>
                </label>
              {/if}
              <label class="block">
                <span class="label">Warna (pisahkan koma)</span>
                <input class="input py-1.5 text-xs" value={(activeElement.chart_colors ?? []).join(', ')}
                  data-chart-field="colors"
                  onfocus={pushHistory}
                  onchange={(event) => updateChartColors((event.currentTarget as HTMLInputElement).value)} />
              </label>
              <div class="flex flex-wrap items-center gap-4 text-xs">
                <label class="flex items-center gap-2"><input type="checkbox" checked={activeElement.chart_show_values !== false}
                  onchange={(event) => updateElement({ chart_show_values: (event.currentTarget as HTMLInputElement).checked }, true)} /> Tampilkan nilai</label>
                <label class="flex items-center gap-2"><input type="checkbox" checked={activeElement.chart_show_legend !== false}
                  onchange={(event) => updateElement({ chart_show_legend: (event.currentTarget as HTMLInputElement).checked }, true)} /> Legend</label>
              </div>
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Warna sumbu</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.chart_axis_color)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ chart_axis_color: (event.currentTarget as HTMLInputElement).value })} />
                </label>
                <label class="block">
                  <span class="label">Latar grafik</span>
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.fill)}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ fill: (event.currentTarget as HTMLInputElement).value })} />
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
              {#if activeElement.source_block}
                <Button variant="primary" size="sm" onclick={resetDetached}>Reset ke data</Button>
              {/if}
              <Button variant="secondary" size="sm" onclick={toggleSelectedHidden}>
                {#if activeElement.hidden}<Eye size={14} /> Tampilkan{:else}<EyeOff size={14} /> Sembunyikan{/if}
              </Button>
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
              Blok ini bagian dari laporan otomatis. Geser dengan mouse, atur ukuran/style, atau klik dua kali teksnya untuk edit.
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
              <label class="block">
                <span class="label">Lebar (mm)</span>
                <input class="input py-1.5 text-xs" type="number" min="0" step="0.5" value={activeBlock.width ?? 0}
                  onfocus={pushHistory}
                  oninput={(event) => updateBlock({ width: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Tinggi (mm)</span>
                <input class="input py-1.5 text-xs" type="number" min="0" step="0.5" value={activeBlock.height ?? 0}
                  onfocus={pushHistory}
                  oninput={(event) => updateBlock({ height: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
            </div>
            <p class="text-[10px] text-ink-muted">Isi 0 pada lebar/tinggi untuk ukuran otomatis.</p>
            <div class="grid grid-cols-2 gap-3">
              <label class="block">
                <span class="label">Background</span>
                <input class="input py-1.5 text-xs" value={activeBlock.background ?? ''} placeholder="transparent / #ffffff"
                  onfocus={pushHistory}
                  onchange={(event) => updateBlock({ background: (event.currentTarget as HTMLInputElement).value })} />
              </label>
              <label class="block">
                <span class="label">Warna garis</span>
                <input class="input h-9 py-1" type="color" value={pickerColor(activeBlock.border_color)}
                  onfocus={pushHistory}
                  oninput={(event) => updateBlock({ border_color: (event.currentTarget as HTMLInputElement).value })} />
              </label>
              <label class="block">
                <span class="label">Tebal garis (pt)</span>
                <input class="input py-1.5 text-xs" type="number" min="0" step="0.25" value={activeBlock.border_width ?? 0}
                  onfocus={pushHistory}
                  oninput={(event) => updateBlock({ border_width: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Sudut (mm)</span>
                <input class="input py-1.5 text-xs" type="number" min="0" step="0.5" value={activeBlock.radius ?? 0}
                  onfocus={pushHistory}
                  oninput={(event) => updateBlock({ radius: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
            </div>
            {#if activeBlock.texts && Object.keys(activeBlock.texts).length}
              <span class="badge badge-progress">Ada teks yang diedit</span>
            {/if}
            <p class="break-all text-[10px] text-ink-muted">ID: {selected?.id}</p>
            <div class="flex flex-wrap gap-2 border-t border-line pt-4">
              <Button variant="secondary" size="sm" onclick={resetBlock}>Reset ukuran & gaya</Button>
              <Button variant="danger" size="sm" onclick={hideBlock}>Sembunyikan</Button>
            </div>
            {#if activeBlockDetachKind}
              <div class="rounded-xl border border-gold-200 bg-gold-50/60 p-3.5">
                <p class="text-[11px] leading-relaxed text-gold-800">
                  {#if activeBlockDetachKind === 'cards'}
                    Detach memecah kartu menjadi objek kartu mandiri yang bisa diedit dan digeser. Semua kartu digabung agar bergerak bersama; gunakan Pisahkan di panel Layer untuk memisahkannya.
                  {:else}
                    Detach mengubah isi {activeBlockDetachKind === 'table' ? 'tabel' : 'grafik'} ini menjadi objek manual yang bisa diedit. Blok asli disembunyikan tetapi ruangnya dipertahankan.
                  {/if}
                </p>
                <Button variant="primary" size="sm" class="mt-3" onclick={detachBlock}>{activeBlockDetachKind === 'cards' ? 'Detach & pecah kartu' : 'Detach & edit isi'}</Button>
              </div>
            {/if}
          </div>
        </section>
      {:else}
        <section class="card p-5">
          <span class="eyebrow">CARA PAKAI</span>
          <ul class="mt-3 space-y-2 text-[11px] leading-relaxed text-ink-muted">
            <li>1. Gunakan Sebelumnya/Berikutnya untuk berpindah halaman; kanvas hanya menampilkan satu halaman.</li>
            <li>2. Tambah teks, kotak, gambar, ikon, kartu, tabel, atau grafik.</li>
            <li>3. Tarik elemen untuk menggeser; tarik sudut untuk mengubah ukuran.</li>
            <li>4. Klik blok laporan untuk geser; tarik sudut kanan bawah untuk resize.</li>
            <li>5. Klik dua kali teks blok untuk mengeditnya.</li>
            <li>6. Gunakan panel Layer untuk menyembunyikan, mengurutkan, atau menggabungkan objek.</li>
            <li>7. Simpan desain sebagai template untuk dipakai pada periode lain.</li>
            <li>8. Simpan, lalu Buat PDF.</li>
          </ul>
        </section>
      {/if}

      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow">DOKUMEN</span>
            <h2 class="panel-title mt-1">Halaman & orientasi</h2>
          </div>
          <FilePlus2 size={18} class="text-gold-500" />
        </div>
        <div class="space-y-3 p-4">
          <div class="flex flex-wrap items-center gap-2">
            <span class="badge {isBlank ? 'badge-success' : 'badge-neutral'}">{isBlank ? 'Blank' : 'Laporan 9 halaman'}</span>
            {#if isBlank}
              <span class="text-[10px] text-ink-muted">{layout.pages.length} halaman · A4 {layout.orientation === 'landscape' ? 'landscape' : 'portrait'}</span>
            {/if}
          </div>
          <label class="block">
            <span class="label">Orientasi</span>
            <select class="input py-1.5 text-xs" data-doc-orientation
              value={isBlank ? layout.orientation : blankOrientation}
              onchange={(event) => setDocumentOrientation((event.currentTarget as HTMLSelectElement).value as 'portrait' | 'landscape')}>
              <option value="portrait">Portrait (A4)</option>
              <option value="landscape">Landscape (A4)</option>
            </select>
          </label>
          {#if isBlank}
            <div class="flex flex-wrap gap-2">
              <span data-blank-add-page><Button variant="secondary" size="sm" disabled={layout.pages.length >= MAX_BLANK_PAGES} onclick={addBlankPage}><Plus size={13} /> Tambah halaman</Button></span>
              <span data-blank-remove-page><Button variant="secondary" size="sm" disabled={layout.pages.length <= 1} onclick={removeBlankPage}><Minus size={13} /> Kurangi halaman</Button></span>
            </div>
            <span data-blank-back><Button variant="ghost" size="sm" onclick={backToReport}>Kembali ke laporan</Button></span>
          {:else}
            <label class="block">
              <span class="label">Jumlah halaman awal</span>
              <input class="input py-1.5 text-xs" type="number" min="1" max={MAX_BLANK_PAGES}
                value={blankPageCount}
                data-blank-page-count
                oninput={(event) => (blankPageCount = Math.round(clamp(numberValue((event.currentTarget as HTMLInputElement).value), 1, MAX_BLANK_PAGES)))} />
            </label>
            <span data-blank-start><Button variant="secondary" size="sm" onclick={startBlankDocument}><FilePlus2 size={13} /> Mulai blank</Button></span>
          {/if}
          <p class="text-[10px] leading-relaxed text-ink-muted">Dokumen blank tidak memakai blok laporan otomatis. Simpan sebagai template untuk dipakai lagi di periode berikutnya.</p>
        </div>
      </section>

      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow">TEMPLATE DESAIN</span>
            <h2 class="panel-title mt-1">Gunakan ulang tata letak</h2>
          </div>
          <Layers size={18} class="text-gold-500" />
        </div>
        <div class="space-y-3 p-4">
          <div class="flex gap-2">
            <input class="input min-w-0 flex-1 py-1.5 text-xs" placeholder="Nama template baru" data-template-name bind:value={templateName} />
            <span data-template-save><Button variant="secondary" size="sm" loading={templateBusy} disabled={!templateName.trim()} onclick={saveAsTemplate}>Simpan</Button></span>
          </div>
          {#if templates.length}
            <select class="input py-1.5 text-xs" data-template-select bind:value={selectedTemplateId}>
              <option value="">Pilih template…</option>
              {#each templates as template}
                <option value={template.id}>{template.name}</option>
              {/each}
            </select>
            <div class="flex flex-wrap gap-2">
              <span data-template-apply><Button variant="secondary" size="sm" loading={templateBusy} disabled={!selectedTemplateId} onclick={applySelectedTemplate}>Terapkan</Button></span>
              <span data-template-delete><Button variant="ghost" size="sm" disabled={!selectedTemplateId} onclick={deleteSelectedTemplate}>Hapus</Button></span>
            </div>
          {:else}
            <p class="text-xs text-ink-muted">Belum ada template tersimpan.</p>
          {/if}
          <p class="text-[10px] leading-relaxed text-ink-muted">Template menyimpan semua halaman, mode blank/laporan, orientasi, dan gambar. Menerapkannya akan mengganti desain versi impor ini (bisa di-undo).</p>
        </div>
      </section>

      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow">LAYER · HALAMAN {currentPage}</span>
            <h2 class="panel-title mt-1">Objek desain</h2>
          </div>
          <span class="badge badge-neutral">{pageElements.length}</span>
        </div>
        <div class="space-y-3 p-4">
          <div class="flex flex-wrap gap-2">
            <span data-group-action="group"><Button variant="secondary" size="sm" disabled={checkedIds.length < 2} onclick={groupChecked}>Gabungkan</Button></span>
            <span data-group-action="ungroup"><Button variant="ghost" size="sm" disabled={!checkedHasGroup} onclick={ungroupChecked}>Pisahkan</Button></span>
            <span data-group-action="clear"><Button variant="ghost" size="sm" disabled={!checkedIds.length} onclick={() => (checkedIds = [])}>Bersihkan</Button></span>
          </div>
          {#if pageElements.length}
            <div class="max-h-72 space-y-1.5 overflow-auto pr-1">
              {#each pageElements as element (element.id)}
                <div class="flex items-center gap-1.5 rounded-lg border px-2 py-1.5 {selected?.kind === 'element' && selected?.id === element.id ? 'border-gold-300 bg-gold-50' : 'border-line'}"
                  data-layer-row={element.id}>
                  <input class="h-3.5 w-3.5 shrink-0" type="checkbox" checked={checkedIds.includes(element.id)}
                    data-layer-check={element.id}
                    onchange={() => toggleChecked(element.id)} />
                  <button type="button" class="min-w-0 flex-1 text-left" data-layer-select={element.id} onclick={() => selectLayer(element.id)}>
                    <span class="block truncate text-[11px] font-semibold text-ink">{elementLabel(element)}</span>
                    <span class="block text-[10px] text-ink-muted">z {element.z}{#if element.group} · grup{/if}</span>
                  </button>
                  <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title={element.hidden ? 'Tampilkan' : 'Sembunyikan'}
                    data-layer-hide={element.id}
                    onclick={() => toggleElementHidden(element.id)}>
                    {#if element.hidden}<Eye size={13} />{:else}<EyeOff size={13} />{/if}
                  </button>
                  <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Naikkan layer"
                    data-layer-up={element.id}
                    onclick={() => moveLayerOf(element.id, 1)}><ArrowUp size={13} /></button>
                  <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Turunkan layer"
                    data-layer-down={element.id}
                    onclick={() => moveLayerOf(element.id, -1)}><ArrowDown size={13} /></button>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-xs text-ink-muted">Belum ada objek di halaman ini.</p>
          {/if}
          <p class="text-[10px] leading-relaxed text-ink-muted">Centang dua objek atau lebih lalu klik Gabungkan. Objek dalam satu grup bergerak bersama saat digeser atau dipindah dengan panah.</p>
        </div>
      </section>

      {#if hiddenBlocks.length}
        <section class="card p-5">
          <span class="eyebrow">BLOK TERSEMBUNYI</span>
          <div class="mt-3 space-y-2">
            {#each hiddenBlocks as id}
              <button class="flex w-full items-center justify-between gap-3 rounded-lg border border-line px-3 py-2 text-left text-[11px] text-ink hover:bg-stone-50" onclick={() => showBlock(id)}>
                <span class="truncate">{id}</span>
                <span class="shrink-0 font-semibold text-forest-700">Tampilkan</span>
              </button>
            {/each}
          </div>
        </section>
      {/if}
    </aside>
  </div>

  {#if shapePickerOpen}
    <div use:portal>
      <Modal open={true} title="Pilih bentuk" description="Klik bentuk untuk memasukkannya ke halaman aktif. Isi, garis, dan ukuran dapat diubah setelah ditambahkan." onclose={() => (shapePickerOpen = false)} width="max-w-2xl">
        <div class="grid grid-cols-3 gap-2 sm:grid-cols-4">
          {#each shapeCatalog as shape (shape.kind)}
            <button type="button" class="flex min-w-0 flex-col items-center gap-2 rounded-xl border border-line px-2 py-3 text-ink hover:border-gold-300 hover:bg-gold-50"
              title={shape.label} data-shape-option={shape.kind} onclick={() => chooseShape(shape)}>
              <svg class="h-9 w-9 text-forest-800" viewBox="0 0 100 100" fill="none" stroke="currentColor"
                stroke-width="5" stroke-linejoin="round" aria-hidden="true">
                {#if shape.kind === 'rect'}
                  <rect x="4" y="16" width="92" height="68" rx="5" />
                {:else if shape.ellipse}
                  <ellipse cx="50" cy="50" rx="46" ry="46" />
                {:else if shape.path}
                  <path d={shape.path} />
                {:else}
                  <polygon points={shape.points} />
                {/if}
              </svg>
              <span class="w-full truncate text-center text-[10px] leading-tight text-ink-muted">{shape.label}</span>
            </button>
          {/each}
        </div>
      </Modal>
    </div>
  {/if}

  {#if iconPickerOpen}
    <div use:portal>
      <Modal open={true} title={iconPickerTitle} description="Klik ikon untuk memasukkannya ke halaman aktif. Warna dapat diubah setelah ikon ditambahkan." onclose={() => (iconPickerOpen = false)} width="max-w-4xl">
    <div class="flex flex-wrap gap-2">
      <input class="input min-w-0 flex-1 py-2 text-sm" placeholder="Cari ikon…" data-icon-search bind:value={iconSearch} />
      <select class="input max-w-[220px] py-2 text-sm" data-icon-category bind:value={iconCategory}>
        {#each iconCategories as category}
          <option value={category}>{category}</option>
        {/each}
      </select>
    </div>
    <div class="mt-4 max-h-[55vh] overflow-auto pr-1">
      {#if filteredIcons.length}
        <div class="grid grid-cols-4 gap-2 sm:grid-cols-6 md:grid-cols-8">
          {#each filteredIcons as icon (icon.name)}
            <button type="button" class="flex min-w-0 flex-col items-center gap-1.5 rounded-xl border border-line px-2 py-2.5 text-ink hover:border-gold-300 hover:bg-gold-50"
              title={icon.label} data-icon-option={icon.name} onclick={() => chooseIcon(icon)}>
              <svg class="h-6 w-6 text-forest-800" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{@html icon.svg}</svg>
              <span class="w-full truncate text-center text-[9px] leading-tight text-ink-muted">{icon.label}</span>
            </button>
          {/each}
        </div>
      {:else}
        <p class="py-8 text-center text-sm text-ink-muted">Tidak ada ikon yang cocok.</p>
      {/if}
    </div>
    <p class="mt-3 text-[10px] text-ink-muted">{filteredIcons.length} ikon ditampilkan dari {iconCatalog.length} ikon Lucide.</p>
      </Modal>
    </div>
  {/if}
{/if}
