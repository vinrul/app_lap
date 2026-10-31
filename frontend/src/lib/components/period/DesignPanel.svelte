<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte'
  import {
    AlertCircle,
    AlignLeft,
    ArrowDown,
    ArrowUp,
    BarChart3,
    CalendarDays,
    Check,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    Copy,
    CreditCard,
    Eye,
    EyeOff,
    FileDown,
    FilePlus2,
    FileText,
    ImagePlus,
    Layers,
    Link2,
    Lock,
    LockOpen,
    Maximize2,
    Minus,
    Plus,
    Redo2,
    RefreshCw,
    Save,
    Share2,
    Shapes,
    SlidersHorizontal,
    Star,
    Table2,
    Trash2,
    Type,
    Undo2,
  } from 'lucide-svelte'
  import { designApi, importsApi, reportsApi } from '../../api'
  import { errorMessage } from '../../api/client'
  import type {
    CoverageCode,
    CoverageGroup,
    CoverageResponse,
    CoverageUnit,
    CoverageUnitCount,
    DesignSource,
    DesignTemplateView,
    LayoutBlockOffset,
    LayoutElement,
    LayoutPage,
    PeriodDetail,
    ReportContent,
    ReportLayout,
    ReportView,
    UploadedDesignImage,
  } from '../../types'
  import { iconByName, iconCatalog, type IconDefinition } from '../../icons/catalog'
  import { fontOptions, fontStack } from '../../fonts/catalog'
  import { shapeCatalog, shapeDefinition, shapeLabel, type ShapeDefinition } from '../../shapes/catalog'
  import { emptyContent } from '../../utils/constants'
  import { cardTemplate, type CardTemplateId } from '../../utils/cardTemplates'
  import { createCardNode } from '../../utils/cardNode'
  import { CHART_PALETTE, createChartSvg } from '../../utils/chartSvg'
  import { createTableNode } from '../../utils/tableNode'
  import {
    clearDesignDraft,
    layoutsMatch,
    parseDesignDraftLayout,
    readDesignDraft,
    writeDesignDraft,
  } from '../../utils/designDraft'
  import { formatDate, formatDateTime, formatMonthYear, formatNumber, formatRange } from '../../utils/format'
  import { toast } from '../../stores/toast.svelte'
  import { dialog } from '../../stores/dialog.svelte'
  import { auth } from '../../stores/auth.svelte'
  import Button from '../ui/Button.svelte'
  import EmptyState from '../ui/EmptyState.svelte'
  import Modal from '../ui/Modal.svelte'
  import CardTemplatePicker from './CardTemplatePicker.svelte'
  import CardEditorDialog from './CardEditorDialog.svelte'
  import ChartEditorDialog from './ChartEditorDialog.svelte'
  import TableEditorDialog from './TableEditorDialog.svelte'
  import MediaLibrary from './MediaLibrary.svelte'
  import ShareReportDialog from './ShareReportDialog.svelte'

  type Props = {
    periodId: string
    detail: PeriodDetail
    ongenerated: () => Promise<void>
  }

  type Selection =
    | { kind: 'element'; page: number; id: string }
    | { kind: 'block'; page: number; id: string }
    | null

  type SidebarTab = 'properties' | 'layers' | 'coverage' | 'page'

  type ContextMenuState = {
    x: number
    y: number
    page: number
    kind: 'element' | 'block' | 'canvas'
    id: string | null
  }

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

  type CakupanNode =
    | { kind: 'cards' }
    | { kind: 'cardGroup'; group: string }
    | { kind: 'card'; code: string }
    | { kind: 'cardMetric'; code: string; metric: string }
    | { kind: 'cardBreakdown'; code: string; breakdown: string }
    | { kind: 'cardTable'; code: string; table: string }
    | { kind: 'cardTableSeries'; code: string; table: string }
    | { kind: 'cardTableColumn'; code: string; table: string; column: number }
    | { kind: 'groups' }
    | { kind: 'group'; code: string }
    | { kind: 'code'; code: string }
    | { kind: 'programs' }
    | { kind: 'program'; key: string }
    | { kind: 'metric'; program: string; metric: string }
    | {
        kind: 'units'
        scope: 'polres' | 'satker'
        group?: string
        code?: string
        program?: string
      }

  type CakupanItem = {
    label: string
    value: number
    unit: string
    detail: string
    icon: string
  }

  type CakupanSeries = { label: string; values: number[]; color: string }

  type CakupanDataset = {
    title: string
    summaryLabel: string
    summaryValue: number
    summaryDetail: string
    summaryIcon: string
    tableColumns: string[]
    tableRows: string[][]
    chartLabels: string[]
    chartValues: number[]
    chartTitle: string
    chartColor?: string
    chartSeries?: CakupanSeries[]
    items: CakupanItem[]
  }

  const PAGE_PORTRAIT_WIDTH_PX = 793.7
  const PAGE_PORTRAIT_HEIGHT_PX = 1122.5
  const MIN_ZOOM = 0.25
  const MAX_ZOOM = 2
  const MAX_HISTORY = 40
  const MAX_BLANK_PAGES = 30
  const AUTOSAVE_DEBOUNCE_MS = 2_000
  const AUTOSAVE_MAX_WAIT_MS = 30_000
  const AUTOSAVE_RETRY_MS = 15_000

  let {
    periodId,
    detail,
    ongenerated,
  }: Props = $props()

  let iframeEl = $state<HTMLIFrameElement | null>(null)
  let canvasAreaEl = $state<HTMLDivElement | null>(null)
  let asideEl = $state<HTMLElement | null>(null)
  let panelOpen = $state(false)
  let iframeHtml = $state('')
  let canvasWidth = $state(PAGE_PORTRAIT_WIDTH_PX)
  let canvasHeight = $state(PAGE_PORTRAIT_HEIGHT_PX)
  let canvasZoom = $state(1)
  let fitZoom = $state(1)
  let zoomOverride = $state<number | null>(null)
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
  let hasSavedDesign = $state(false)
  let choosingTemplate = $state(false)
  let loading = $state(true)
  let saving = $state(false)
  let autosaveReady = $state(false)
  let autosaveState = $state<'idle' | 'saved' | 'error'>('idle')
  let autosaveError = $state('')
  let lastSaveMode = $state<'manual' | 'auto' | null>(null)
  let generating = $state(false)
  let sharing = $state(false)
  let shareTarget = $state<ReportView | null>(null)
  let previewing = $state(false)
  let mediaOpen = $state(false)
  let mediaTarget = $state<'new' | 'replace' | 'background'>('new')
  let error = $state('')
  let currentPage = $state(1)
  let selected = $state<Selection>(null)
  let contextMenu = $state<ContextMenuState | null>(null)
  let sidebarTab = $state<SidebarTab>('properties')
  let chartEditorTarget = $state<{ page: number; id: string } | null>(null)
  let cardEditorTarget = $state<{ page: number; id: string } | null>(null)
  let tableEditorTarget = $state<{ page: number; id: string } | null>(null)
  let clipboard = $state<LayoutElement | null>(null)
  let checkedIds = $state<string[]>([])
  let history = $state<ReportLayout[]>([])
  let redoStack = $state<ReportLayout[]>([])
  let templates = $state<DesignTemplateView[]>([])
  let templateName = $state('')
  let selectedTemplateId = $state('')
  let templateBusy = $state(false)
  let iconPickerOpen = $state(false)
  let shapePickerOpen = $state(false)
  let textPickerOpen = $state(false)
  let chartPickerOpen = $state(false)
  let iconPickerTarget = $state<'new' | 'replace' | 'card'>('new')
  let iconSearch = $state('')
  let iconCategory = $state('Semua')
  let coverage = $state<CoverageResponse | null>(null)
  let cakupanExpanded = $state<string[]>([])
  let cakupanTarget = $state<CakupanNode | null>(null)
  let cakupanDialogOpen = $state(false)
  let cakupanDropPoint = $state<{ x: number; y: number } | null>(null)
  let cakupanDragActive = $state(false)
  let cakupanDragNode = $state<CakupanNode | null>(null)
  let cakupanCardStep = $state(false)
  let cakupanCardScope = $state<'summary' | 'all' | 'pick'>('all')
  let cakupanCardPicks = $state<number[]>([])
  let cakupanCardGroup = $state(false)

  let loadedKey = ''
  let canvasRequest = 0
  let boundDoc: Document | null = null
  let drag: DragState | null = null
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null
  let autosaveMaxTimer: ReturnType<typeof setTimeout> | null = null
  let autosaveRetryTimer: ReturnType<typeof setTimeout> | null = null
  let autosaveQueued = false
  let activeSavePromise: Promise<boolean> | null = null
  let autosaveErrorNotified = false
  let localDraftAt = $state<string | null>(null)
  let draftUnavailable = $state(false)
  const defaultTexts = new Map<string, string>()

  const hasImport = $derived(detail.imports.length > 0)
  const draftOwner = $derived(auth.user?.id ?? 'anon')
  const dirty = $derived(JSON.stringify(layout) !== savedJson)
  const pageNumbers = $derived(Array.from({ length: layout.pages.length }, (_, index) => index + 1))
  const periodDateText = $derived.by(() => {
    const period = detail.period
    if (period.period_type === 'monthly') return formatMonthYear(period.start_date)
    if (period.period_type === 'daily' || period.start_date === period.end_date) {
      return formatDate(period.start_date)
    }
    return formatRange(period.start_date, period.end_date)
  })
  const zoomPercent = $derived(Math.round((zoomOverride ?? fitZoom) * 100))
  const atFitZoom = $derived(zoomOverride === null)
  const canZoomOut = $derived((zoomOverride ?? fitZoom) > MIN_ZOOM + 0.001)
  const canZoomIn = $derived((zoomOverride ?? fitZoom) < MAX_ZOOM - 0.001)
  const iconCategories = ['Semua', ...new Set(iconCatalog.map((icon) => icon.category))]
  const systemFontOptions = fontOptions.filter((font) => font.category === 'Sistem')
  const sansFontOptions = fontOptions.filter((font) => font.category === 'Sans')
  const serifFontOptions = fontOptions.filter((font) => font.category === 'Serif')
  const iconPickerTitle = $derived(
    iconPickerTarget === 'replace'
      ? 'Ganti ikon'
      : iconPickerTarget === 'card'
        ? 'Ikon kartu'
        : 'Tambah ikon',
  )
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
  const activePage = $derived(getPage(currentPage))
  const pageBackgroundMode = $derived(
    activePage?.background_mode === 'linear' ||
      activePage?.background_mode === 'radial' ||
      activePage?.background_mode === 'image'
      ? activePage.background_mode
      : 'solid',
  )
  const activeCardTemplate = $derived(
    cardTemplate(activeElement?.type === 'card' ? activeElement.card_template : undefined),
  )
  const activeCardIcon = $derived(
    activeElement?.type === 'card' ? iconByName.get(activeElement.card_icon ?? '') : undefined,
  )
  const activeBlock = $derived.by(() => {
    if (!selected || selected.kind !== 'block') return null
    return getPage(selected.page)?.blocks[selected.id] ?? { dx: 0, dy: 0, hidden: false }
  })
  const contextElement = $derived.by(() => {
    if (!contextMenu || contextMenu.kind !== 'element' || !contextMenu.id) return null
    return getElement(contextMenu.page, contextMenu.id) ?? null
  })
  const contextBlock = $derived.by(() => {
    if (!contextMenu || contextMenu.kind !== 'block' || !contextMenu.id) return null
    return getPage(contextMenu.page)?.blocks[contextMenu.id] ?? { dx: 0, dy: 0, hidden: false }
  })

  const chartEditorElement = $derived.by(() => {
    const target = chartEditorTarget
    if (!target) return null
    const element = getElement(target.page, target.id)
    if (!element || element.type !== 'chart') return null
    const source = element.binding ? sourceFor(element.binding) : null
    if (source && source.can_chart && source.chart_labels.length) {
      return {
        ...element,
        chart_labels: [...source.chart_labels],
        chart_values: [...source.chart_values],
        chart_title: element.chart_title?.trim() ? element.chart_title : source.chart_title,
      }
    }
    return { ...element }
  })

  const chartEditorLocked = $derived.by(() => {
    const target = chartEditorTarget
    if (!target) return false
    return Boolean(getElement(target.page, target.id)?.binding)
  })

  const chartEditorSourceLabel = $derived.by(() => {
    const target = chartEditorTarget
    if (!target) return ''
    const binding = getElement(target.page, target.id)?.binding
    return binding ? (sourceFor(binding)?.label ?? binding) : ''
  })

  const cardEditorElement = $derived.by(() => {
    const target = cardEditorTarget
    if (!target) return null
    const element = getElement(target.page, target.id)
    if (!element || element.type !== 'card') return null
    const source = element.binding ? sourceFor(element.binding) : null
    if (source && source.can_card) {
      return {
        ...element,
        card_label: source.card_label ?? '',
        card_value: source.card_value ?? '',
        card_unit: source.card_unit ?? '',
        card_detail: source.card_detail ?? '',
      }
    }
    return { ...element }
  })

  const cardEditorLocked = $derived.by(() => {
    const target = cardEditorTarget
    if (!target) return false
    return Boolean(getElement(target.page, target.id)?.binding)
  })

  const cardEditorSourceLabel = $derived.by(() => {
    const target = cardEditorTarget
    if (!target) return ''
    const binding = getElement(target.page, target.id)?.binding
    return binding ? (sourceFor(binding)?.label ?? binding) : ''
  })

  const tableEditorElement = $derived.by(() => {
    const target = tableEditorTarget
    if (!target) return null
    const element = getElement(target.page, target.id)
    if (!element || element.type !== 'table') return null
    const source = element.binding ? sourceFor(element.binding) : null
    if (source && source.table_rows.length) {
      return {
        ...element,
        table_rows: source.table_rows.map((row) => [...row]),
      }
    }
    return { ...element }
  })

  const tableEditorLocked = $derived.by(() => {
    const target = tableEditorTarget
    if (!target) return false
    return Boolean(getElement(target.page, target.id)?.binding)
  })

  const tableEditorSourceLabel = $derived.by(() => {
    const target = tableEditorTarget
    if (!target) return ''
    const binding = getElement(target.page, target.id)?.binding
    return binding ? (sourceFor(binding)?.label ?? binding) : ''
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
  const tableSourceGroups = $derived(groupSources(sources.filter((source) => source.can_table !== false)))
  const chartSourceGroups = $derived(groupSources(sources.filter((source) => source.can_chart)))
  const cardSourceGroups = $derived(groupSources(sources.filter((source) => source.can_card === true)))
  const pageElements = $derived.by(() => {
    const page = getPage(currentPage)
    if (!page) return []
    return [...page.elements].sort((left, right) => right.z - left.z)
  })
  const checkedHasGroup = $derived.by(() =>
    pageElements.some((element) => checkedIds.includes(element.id) && Boolean(element.group)),
  )
  const cakupanGroups = $derived(coverage?.groups ?? [])
  const cakupanTotalRows = $derived(
    cakupanGroups.reduce((total, group) => total + group.rows, 0),
  )
  const cakupanCards = $derived(coverage?.codes ?? [])
  const cakupanCardGroups = $derived(
    cakupanGroups
      .map((group) => ({
        code: group.code,
        title: group.title,
        cards: cakupanCards.filter((card) => card.code.startsWith(`${group.code}.`)),
      }))
      .filter((group) => group.cards.length > 0),
  )
  const cakupanCardTotal = $derived(
    cakupanCards.reduce(
      (total, card) => total + card.metrics.length + card.breakdowns.length,
      0,
    ),
  )
  const cakupanDataset = $derived(cakupanTarget ? buildCakupanDataset(cakupanTarget) : null)
  const selectionKey = $derived(
    selected ? `${selected.kind}:${selected.page}:${selected.id}` : '',
  )

  $effect(() => {
    const key = periodId
    if (key === loadedKey) return
    loadedKey = key
    untrack(() => void load())
  })

  $effect(() => {
    const json = JSON.stringify(layout)
    if (!autosaveReady || loading || choosingTemplate || !hasImport) return
    if (json === savedJson) {
      clearAutosaveTimers()
      return
    }
    scheduleAutosave()
  })

  $effect(() => {
    const onVisibility = () => {
      if (document.visibilityState === 'hidden') {
        if (dirty && autosaveReady) void autosaveNow()
      } else if (dirty) {
        scheduleAutosave()
      }
    }
    const onPageHide = () => flushOnPageHide()
    document.addEventListener('visibilitychange', onVisibility)
    window.addEventListener('pagehide', onPageHide)
    return () => {
      document.removeEventListener('visibilitychange', onVisibility)
      window.removeEventListener('pagehide', onPageHide)
    }
  })

  onDestroy(() => {
    clearAutosaveTimers()
    flushOnPageHide()
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
    const key = selectionKey
    if (!key) return
    panelOpen = true
    const currentTab = untrack(() => sidebarTab)
    if (currentTab !== 'coverage') sidebarTab = 'properties'
    untrack(() => asideEl?.scrollTo({ top: 0, behavior: 'smooth' }))
  })

  function switchSidebarTab(tab: SidebarTab): void {
    sidebarTab = tab
    void tick().then(() => asideEl?.scrollTo({ top: 0, behavior: 'smooth' }))
  }

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
        ...pageStyle(page),
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
          ...pageStyle(found),
        }
      }),
    }
  }

  function pageStyle(
    page: LayoutPage | undefined,
  ): Pick<LayoutPage, 'background' | 'background_mode' | 'gradient_angle' | 'gradient_colors'> {
    return {
      background: page?.background ?? '',
      background_mode:
        page?.background_mode === 'linear' ||
        page?.background_mode === 'radial' ||
        page?.background_mode === 'image'
          ? page.background_mode
          : 'solid',
      gradient_angle: page?.gradient_angle ?? 90,
      gradient_colors: [...(page?.gradient_colors ?? [])],
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
      syncTableColumnStyles(element)
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
      return
    }
    if (element.type === 'card' && source.can_card) {
      element.card_label = source.card_label ?? element.card_label
      element.card_value = source.card_value ?? element.card_value
      element.card_unit = source.card_unit ?? element.card_unit
      element.card_detail = source.card_detail ?? element.card_detail
      if (!element.card_icon?.trim() && source.card_icon) {
        element.card_icon = source.card_icon
      }
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
    if (!hasImport) return
    try {
      const [sourceList, coverageData] = await Promise.all([
        designApi.sources(periodId),
        importsApi.coverage(periodId).catch(() => null),
      ])
      sources = sourceList
      if (coverageData) {
        coverage = coverageData
        if (!cakupanExpanded.includes('cards')) cakupanExpanded = ['cards', ...cakupanExpanded]
      }
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

  function closeContextMenu(): void {
    contextMenu = null
  }

  function handleWindowPointerDown(event: PointerEvent): void {
    if (!contextMenu) return
    const target = event.target
    if (target instanceof Element && target.closest('[data-design-context-menu]')) return
    closeContextMenu()
  }

  function onContextMenu(event: MouseEvent): void {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    const target = event.target as HTMLElement | null
    if (!target) return
    if (target.isContentEditable || target.closest('[contenteditable="true"]')) return
    const pageEl = target.closest<HTMLElement>('.page')
    if (!pageEl) {
      closeContextMenu()
      return
    }

    event.preventDefault()
    event.stopPropagation()

    const page = pageIndexOf(pageEl)
    if (page !== currentPage) checkedIds = []
    currentPage = page

    const handleEl = target.closest<HTMLElement>('[data-handle]')
    let kind: ContextMenuState['kind'] = 'canvas'
    let id: string | null = null
    if (handleEl) {
      if (selected?.kind === 'element' && selected.page === page) {
        kind = 'element'
        id = selected.id
      } else if (selected?.kind === 'block' && selected.page === page) {
        kind = 'block'
        id = selected.id
      }
    }
    if (kind === 'canvas') {
      const elementEl = target.closest<HTMLElement>('[data-element-id]')
      const elementId = elementEl?.dataset.elementId
      if (elementId) {
        kind = 'element'
        id = elementId
      } else {
        const blockEl = target.closest<HTMLElement>('[data-block]')
        const blockId = blockEl?.dataset.block
        if (blockId) {
          kind = 'block'
          id = blockId
        }
      }
    }

    selected =
      (kind === 'element' || kind === 'block') && id
        ? { kind, page, id }
        : null
    applyPageVisibility()
    renderSelection()

    const frameRect = iframeEl?.getBoundingClientRect()
    const anchorX = (frameRect?.left ?? 0) + event.clientX
    const anchorY = (frameRect?.top ?? 0) + event.clientY
    const menuWidth = 256
    const menuHeight = kind === 'element' ? 424 : kind === 'block' ? 186 : 128
    const margin = 8
    contextMenu = {
      x: Math.round(clamp(anchorX, margin, Math.max(margin, window.innerWidth - menuWidth - margin))),
      y: Math.round(clamp(anchorY, margin, Math.max(margin, window.innerHeight - menuHeight - margin))),
      page,
      kind,
      id,
    }
  }

  function copySelected(silent = false): boolean {
    if (!selected || selected.kind !== 'element') return false
    const element = getElement(selected.page, selected.id)
    if (!element) return false
    clipboard = JSON.parse(JSON.stringify(element)) as LayoutElement
    if (!silent) toast.success('Objek disalin. Tekan Ctrl+V untuk menempel.')
    return true
  }

  function pasteClipboard(): void {
    if (!clipboard) return
    const page = getPage(currentPage)
    if (!page) return
    pushHistory()
    const copy: LayoutElement = {
      ...(JSON.parse(JSON.stringify(clipboard)) as LayoutElement),
      id: newId(),
      x: clipboard.x + 8,
      y: clipboard.y + 8,
      z: clipboard.z + 1,
      source_block: '',
      group: '',
    }
    clampElement(copy)
    page.elements.push(copy)
    selected = { kind: 'element', page: currentPage, id: copy.id }
    closeContextMenu()
    applyLayoutToDom()
  }

  function contextMoveLayer(delta: number): void {
    const menu = contextMenu
    if (!menu || menu.kind !== 'element' || !menu.id) return
    moveLayerOf(menu.id, delta)
    closeContextMenu()
  }

  function contextToggleHidden(): void {
    const menu = contextMenu
    if (!menu || menu.kind !== 'element' || !menu.id) return
    toggleElementHidden(menu.id)
    closeContextMenu()
  }

  function contextToggleLocked(): void {
    const menu = contextMenu
    if (!menu || menu.kind !== 'element' || !menu.id) return
    toggleElementLocked(menu.id)
    closeContextMenu()
  }

  function contextDuplicate(): void {
    if (!contextElement) return
    duplicateSelected()
    closeContextMenu()
  }

  function contextCopy(): void {
    if (!copySelected()) return
    closeContextMenu()
  }

  function contextDelete(): void {
    if (!contextElement) return
    deleteSelected()
    closeContextMenu()
  }

  function contextToggleBlockHidden(): void {
    const menu = contextMenu
    if (!menu || menu.kind !== 'block' || !menu.id) return
    const page = getPage(menu.page)
    if (!page) return
    pushHistory()
    const block = page.blocks[menu.id] ?? { dx: 0, dy: 0, hidden: false }
    block.hidden = !block.hidden
    page.blocks[menu.id] = block
    if (block.hidden) selected = null
    closeContextMenu()
    applyLayoutToDom()
  }

  function contextResetBlock(): void {
    if (!contextBlock) return
    resetBlock()
    closeContextMenu()
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

  function toggleElementLocked(id: string): void {
    const element = getElement(currentPage, id)
    if (!element) return
    pushHistory()
    element.locked = !element.locked
    applyLayoutToDom()
    renderSelection()
  }

  function toggleSelectedLocked(): void {
    if (!selected || selected.kind !== 'element') return
    toggleElementLocked(selected.id)
  }

  function elementOrGroupLocked(page: LayoutPage, element: LayoutElement): boolean {
    if (element.locked) return true
    if (!element.group) return false
    return page.elements.some((item) => item.group === element.group && item.locked)
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
    if (!hasImport || !selectedTemplateId) return
    const template = templates.find((item) => item.id === selectedTemplateId)
    const label = template?.name ?? 'ini'
    const confirmed = await dialog.confirm({
      title: 'Terapkan template?',
      message: `Desain saat ini akan diganti dengan template "${label}". Perubahan masih bisa di-undo.`,
      confirmLabel: 'Terapkan',
    })
    if (!confirmed) return
    if (!(await flushAutosave())) return
    templateBusy = true
    const before = cloneLayout(layout)
    try {
      const view = await designApi.applyTemplate(periodId, selectedTemplateId)
      pushSnapshot(before)
      layout = normalizeLayout(view.layout)
      savedJson = JSON.stringify(layout)
      updatedAt = view.updated_at
      updatedBy = view.updated_by
      hasSavedDesign = true
      autosaveReady = true
      autosaveState = 'idle'
      autosaveError = ''
      lastSaveMode = null
      clearAutosaveTimers()
      clearDesignDraft(periodId, draftOwner)
      localDraftAt = null
      draftUnavailable = false
      choosingTemplate = false
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
      locked: false,
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
      table_header_font_family: 'sans',
      table_header_font_size: 0,
      table_column_backgrounds: [],
      table_column_colors: [],
      table_column_font_families: [],
      table_column_font_sizes: [],
      table_column_aligns: [],
      chart_type: 'bar',
      chart_title: '',
      chart_labels: ['A', 'B', 'C', 'D'],
      chart_values: [10, 20, 15, 25],
      chart_colors: ['#143d30', '#b8894b', '#3f765d', '#5b8fa8'],
      chart_show_values: true,
      chart_show_legend: true,
      chart_axis_color: '#6d7872',
      chart_title_font_family: 'sans',
      chart_title_font_size: 0,
      chart_title_wrap: false,
      chart_label_font_size: 0,
      chart_label_rotation: 0,
      chart_value_font_family: 'sans',
      chart_value_font_size: 0,
      chart_legend_font_size: 0,
      card_label: 'Label kartu',
      card_value: '0',
      card_unit: 'unit',
      card_detail: '',
      card_accent: '#b8894b',
      card_label_font_family: 'sans',
      card_label_font_size: 0,
      card_label_color: '',
      card_value_font_family: 'sans',
      card_value_font_size: 0,
      card_value_color: '',
      card_unit_font_family: 'sans',
      card_unit_font_size: 0,
      card_unit_color: '',
      card_detail_font_family: 'sans',
      card_detail_font_size: 0,
      card_detail_color: '',
      card_template: type === 'card' ? 'classic' : '',
      card_icon: '',
      card_icon_color: type === 'card' ? '#b8894b' : '',
      card_icon_bg: type === 'card' ? '#f1e6d5' : '',
      card_icon_scale: 100,
      icon_name: '',
      icon_color: '#143d30',
      icon_stroke: 2,
    }
  }

  async function load() {
    const targetPeriod = periodId
    clearAutosaveTimers()
    loading = true
    autosaveReady = false
    autosaveState = 'idle'
    autosaveError = ''
    autosaveQueued = false
    autosaveErrorNotified = false
    lastSaveMode = null
    localDraftAt = null
    draftUnavailable = false
    error = ''
    selected = null
    contextMenu = null
    clipboard = null
    drag = null
    currentPage = 1
    history = []
    redoStack = []
    sources = []
    coverage = null
    checkedIds = []
    selectedTemplateId = ''
    zoomOverride = null
    if (!hasImport) {
      content = { ...emptyContent }
      layout = emptyLayout()
      savedJson = ''
      updatedAt = null
      updatedBy = null
      hasSavedDesign = false
      choosingTemplate = false
      iframeHtml = ''
      autosaveReady = false
      loading = false
      return
    }
    try {
      const [draft, layoutView, sourceList, templateList, coverageData] = await Promise.all([
        reportsApi.getDraft(targetPeriod),
        designApi.getLayout(targetPeriod),
        designApi.sources(targetPeriod),
        designApi.templates(),
        importsApi.coverage(targetPeriod).catch(() => null),
      ])
      if (targetPeriod !== periodId) return
      content = { ...emptyContent, ...draft.content }
      sources = sourceList
      templates = templateList
      coverage = coverageData
      if (coverageData) cakupanExpanded = ['cards']

      const serverLayout = normalizeLayout(layoutView.layout)
      const serverSaved = layoutView.updated_at !== null
      const serverJson = serverSaved ? JSON.stringify(serverLayout) : ''
      let restoredLayout: ReportLayout | null = null
      let restoredAt: string | null = null

      const localDraft = readDesignDraft(targetPeriod, draftOwner)
      if (localDraft) {
        const draftLayout = parseDesignDraftLayout(localDraft)
        if (!draftLayout) {
          clearDesignDraft(targetPeriod, draftOwner)
        } else {
          const normalizedDraft = normalizeLayout(draftLayout)
          if (serverSaved && layoutsMatch(normalizedDraft, serverLayout)) {
            clearDesignDraft(targetPeriod, draftOwner)
          } else {
            const draftTime = formatDateTime(localDraft.savedAt)
            const serverInfo = serverSaved
              ? `Versi server terakhir disimpan ${formatDateTime(layoutView.updated_at)}${layoutView.updated_by ? ` oleh ${layoutView.updated_by}` : ''}.`
              : 'Desain ini belum pernah disimpan di server.'
            const changedOnServer =
              serverSaved && localDraft.baseUpdatedAt !== layoutView.updated_at
            const message = [
              `Draft lokal disimpan ${draftTime}.`,
              serverInfo,
              changedOnServer
                ? 'Desain di server sudah berubah sejak draft dibuat; memulihkan draft akan menimpa versi server.'
                : '',
              'Pulihkan draft lokal?',
            ]
              .filter(Boolean)
              .join(' ')
            const restore = await dialog.confirm({
              title: 'Draft lokal ditemukan',
              message,
              confirmLabel: 'Pulihkan draft',
              cancelLabel: serverSaved ? 'Pakai versi server' : 'Buang draft',
            })
            if (targetPeriod !== periodId) return
            if (restore) {
              restoredLayout = normalizedDraft
              restoredAt = localDraft.savedAt
            } else {
              clearDesignDraft(targetPeriod, draftOwner)
            }
          }
        }
      }

      layout = restoredLayout ?? serverLayout
      hasSavedDesign = serverSaved
      choosingTemplate = !serverSaved && !restoredLayout
      savedJson = serverJson
      updatedAt = layoutView.updated_at
      updatedBy = layoutView.updated_by
      localDraftAt = restoredAt
      autosaveReady = true
      autosaveState = 'idle'
      autosaveError = ''
      lastSaveMode = null
      if (restoredLayout) toast.success('Draft lokal dipulihkan.')
      if (hasSavedDesign || restoredLayout) {
        const preview = await reportsApi.preview(targetPeriod, content, layout)
        if (targetPeriod !== periodId) return
        iframeHtml = preview.html
      } else {
        iframeHtml = ''
      }
    } catch (caught) {
      if (targetPeriod !== periodId) return
      error = errorMessage(caught)
      autosaveReady = false
    } finally {
      if (targetPeriod === periodId) loading = false
    }
  }

  async function refreshCanvasStructure(): Promise<void> {
    if (!hasImport) return
    const request = ++canvasRequest
    try {
      const preview = await reportsApi.preview(periodId, content, layout)
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
      doc.addEventListener('contextmenu', onContextMenu)
    }
    applyLayoutToDom()
    applyPageBackgroundToDom()
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
        node = createTableNode(doc, element, rows)
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
      const bound = element.binding ? sourceFor(element.binding) : undefined
      node = createCardNode(
        doc,
        element,
        bound?.can_card
          ? {
              label: bound.card_label ?? '',
              value: bound.card_value ?? '',
              unit: bound.card_unit ?? '',
              detail: bound.card_detail ?? '',
            }
          : undefined,
      )
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
    if (element.locked) {
      node.dataset.locked = 'true'
      node.style.cursor = 'not-allowed'
    }
    return node
  }

  const SVG_NS = 'http://www.w3.org/2000/svg'

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

      pageEl.style.background = pageBackgroundCss(page)

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
    if (element.locked) {
      frame.dataset.locked = 'true'
      frame.style.cursor = 'not-allowed'
    } else {
      for (const [handle, [left, top]] of Object.entries(handles)) {
        const node = doc.createElement('div')
        node.className = 'editor-handle'
        node.dataset.handle = handle
        node.style.left = left
        node.style.top = top
        node.style.cursor = handle === 'nw' || handle === 'se' ? 'nwse-resize' : 'nesw-resize'
        frame.appendChild(node)
      }
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
    closeContextMenu()
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
      if (element.locked) return
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
      selected = { kind: 'element', page, id }
      renderSelection()
      event.preventDefault()
      if (pageData && elementOrGroupLocked(pageData, element)) return
      const groupStarts =
        element.group && pageData
          ? pageData.elements
              .filter((item) => item.group === element.group)
              .map((item) => ({ id: item.id, x: item.x, y: item.y }))
          : []
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
    scheduleAutosave()
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
    if (!(await flushAutosave())) return
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
    clearDesignDraft(periodId, draftOwner)
    localDraftAt = null
    draftUnavailable = false
  }

  function chooseBlank() {
    const pages = Math.round(clamp(blankPageCount, 1, MAX_BLANK_PAGES))
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
    history = []
    redoStack = []
    savedJson = ''
    choosingTemplate = false
    autosaveReady = true
    autosaveState = 'idle'
    autosaveError = ''
    lastSaveMode = null
    clearDesignDraft(periodId, draftOwner)
    localDraftAt = null
    draftUnavailable = false
    toast.success('Dokumen kosong siap disusun.')
    void refreshCanvasStructure()
    scheduleAutosave()
  }

  async function useTemplateFromChooser(templateId: string) {
    if (!hasImport || templateBusy) return
    const template = templates.find((item) => item.id === templateId)
    templateBusy = true
    selectedTemplateId = templateId
    error = ''
    try {
      const view = await designApi.applyTemplate(periodId, templateId)
      layout = normalizeLayout(view.layout)
      savedJson = JSON.stringify(layout)
      updatedAt = view.updated_at
      updatedBy = view.updated_by
      hasSavedDesign = true
      autosaveReady = true
      autosaveState = 'idle'
      autosaveError = ''
      lastSaveMode = null
      clearAutosaveTimers()
      clearDesignDraft(periodId, draftOwner)
      localDraftAt = null
      draftUnavailable = false
      choosingTemplate = false
      selected = null
      checkedIds = []
      history = []
      redoStack = []
      applyLayoutToDom()
      void refreshCanvasStructure()
      toast.success(`Template "${template?.name ?? 'desain'}" diterapkan.`)
    } catch (caught) {
      error = errorMessage(caught)
    } finally {
      templateBusy = false
    }
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

  function duplicateBlankPage() {
    if (!isBlank || layout.pages.length >= MAX_BLANK_PAGES) return
    const source = getPage(currentPage)
    if (!source) return
    pushHistory()
    const groups = new Map<string, string>()
    const elements = source.elements.map((element) => {
      const copy = JSON.parse(JSON.stringify(element)) as LayoutElement
      copy.id = newId()
      copy.source_block = ''
      if (copy.group) {
        let group = groups.get(copy.group)
        if (!group) {
          group = newId()
          groups.set(copy.group, group)
        }
        copy.group = group
      }
      return copy
    })
    layout.pages.splice(currentPage, 0, {
      ...source,
      page: currentPage + 1,
      blocks: {},
      elements,
    })
    layout.pages = layout.pages.map((page, index) => ({ ...page, page: index + 1 }))
    currentPage += 1
    selected = null
    checkedIds = []
    void refreshCanvasStructure()
    toast.success('Halaman diduplikasi.')
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

  function addText(kind: 'period' | 'date' | 'plain') {
    const element = baseElement('text')
    if (kind === 'period') {
      element.text = detail.period.label
      element.width = 160
      element.height = 18
      element.font_size = 16
      element.bold = true
      element.align = 'center'
      element.valign = 'middle'
    } else if (kind === 'date') {
      element.text = periodDateText
      element.width = 150
      element.height = 12
      element.font_size = 11
      element.align = 'center'
      element.valign = 'middle'
      element.color = '#52635b'
    } else {
      element.text = 'Teks baru'
      element.width = 80
      element.height = 15
    }
    textPickerOpen = false
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

  function addChart(series: 'single' | 'multi' = 'single') {
    const element = baseElement('chart')
    element.width = 120
    element.height = 80
    element.font_size = 9
    element.chart_title = series === 'multi' ? 'Grafik multi-seri' : 'Grafik baru'
    element.chart_type = 'bar'
    element.chart_labels = ['A', 'B', 'C', 'D']
    element.chart_colors = ['#143d30', '#b8894b', '#3f765d', '#5b8fa8']
    if (series === 'multi') {
      element.chart_series = [
        { label: 'Seri 1', values: [10, 20, 15, 25], color: CHART_PALETTE[0] },
        { label: 'Seri 2', values: [8, 14, 12, 18], color: CHART_PALETTE[1] },
      ]
      element.chart_values = [18, 34, 27, 43]
      element.chart_show_legend = true
    } else {
      element.chart_values = [10, 20, 15, 25]
    }
    chartPickerOpen = false
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

  const CAKUPAN_GROUP_ICONS: Record<string, string> = {
    QW1: 'shield',
    QW2: 'shield-alert',
    QW3: 'smartphone',
    QW4: 'siren',
    QW5: 'file-text',
    QW6: 'lock',
    QW7: 'building-2',
    QW8: 'microscope',
  }

  function cakupanGroupIcon(code: string): string {
    return CAKUPAN_GROUP_ICONS[code] ?? 'clipboard-list'
  }

  const CAKUPAN_PROGRAM_ICONS: Record<string, string> = {
    green_policing: 'sprout',
    jumat_keliling: 'users',
    subuh_keliling: 'sunrise',
    minggu_kasih: 'heart-handshake',
    narkoba: 'shield-alert',
    media: 'megaphone',
  }

  function cakupanProgramIcon(key: string): string {
    return CAKUPAN_PROGRAM_ICONS[key] ?? 'chart-column'
  }

  const KERAWANAN_COLORS: Record<string, string> = {
    'Rawan kemacetan': '#2f6db0',
    'Rawan keramaian': '#3f765d',
    'Rawan pelanggaran': '#b8894b',
    'Rawan kecelakaan': '#b0413e',
    'Rawan banjir': '#2f5d7c',
    'Rawan kriminalitas': '#7a857f',
    'Rawan narkoba': '#6b4c8a',
    Lainnya: '#a4743d',
  }

  const FEEDBACK_COLORS: Record<string, string> = {
    Apresiasi: '#3f765d',
    Keluhan: '#b0413e',
    'Tindak lanjut': '#b8894b',
  }

  function tableSeriesColor(label: string, index: number): string {
    return (
      KERAWANAN_COLORS[label] ??
      FEEDBACK_COLORS[label] ??
      CHART_PALETTE[index % CHART_PALETTE.length]
    )
  }

  function cakupanProgram(key: string) {
    return coverage?.programs.find((program) => program.key === key)
  }

  function cakupanValue(value: number, unit: string): string {
    if (unit === 'm²' || !Number.isInteger(value)) {
      return value.toLocaleString('id-ID', { maximumFractionDigits: 2 })
    }
    return formatNumber(value)
  }

  function cakupanGroup(code: string): CoverageGroup | undefined {
    return coverage?.groups.find((group) => group.code === code)
  }

  function cakupanCode(code: string): CoverageCode | undefined {
    for (const group of coverage?.groups ?? []) {
      const found = group.codes.find((item) => item.code === code)
      if (found) return found
    }
    return undefined
  }

  function cakupanCodeGroup(code: string): CoverageGroup | undefined {
    return coverage?.groups.find((group) => group.codes.some((item) => item.code === code))
  }

  function cakupanCard(code: string) {
    return cakupanCards.find((item) => item.code === code)
  }

  function isCakupanExpanded(key: string): boolean {
    return cakupanExpanded.includes(key)
  }

  function toggleCakupan(key: string): void {
    cakupanExpanded = isCakupanExpanded(key)
      ? cakupanExpanded.filter((item) => item !== key)
      : [...cakupanExpanded, key]
  }

  function cakupanNodeLabel(node: CakupanNode): string {
    if (node.kind === 'cards') return 'Semua kartu cakupan Quick Wins'
    if (node.kind === 'cardGroup') {
      const group = cakupanGroup(node.group)
      return group ? `${group.code} · ${group.title}` : node.group
    }
    if (node.kind === 'card') {
      const card = cakupanCard(node.code)
      return card ? `${card.code} · ${card.title}` : node.code
    }
    if (node.kind === 'cardMetric') {
      const card = cakupanCard(node.code)
      const metric = card?.metrics.find((item) => item.key === node.metric)
      return [card?.code, metric?.label].filter(Boolean).join(' · ') || node.metric
    }
    if (node.kind === 'cardBreakdown') {
      const card = cakupanCard(node.code)
      const breakdown = card?.breakdowns.find((item) => item.key === node.breakdown)
      return [card?.code, breakdown?.label].filter(Boolean).join(' · ') || node.breakdown
    }
    if (node.kind === 'cardTable') {
      const card = cakupanCard(node.code)
      const table = card?.tables.find((item) => item.key === node.table)
      return [card?.code, table?.label].filter(Boolean).join(' · ') || node.table
    }
    if (node.kind === 'cardTableSeries') {
      const card = cakupanCard(node.code)
      const table = card?.tables.find((item) => item.key === node.table)
      return [card?.code, table ? `Grafik ${table.label}` : 'Grafik tabel']
        .filter(Boolean)
        .join(' · ')
    }
    if (node.kind === 'cardTableColumn') {
      const card = cakupanCard(node.code)
      const table = card?.tables.find((item) => item.key === node.table)
      const column = table?.columns[node.column]
      return [card?.code, column ? `Grafik ${column} per polres` : 'Grafik per polres']
        .filter(Boolean)
        .join(' · ')
    }
    if (node.kind === 'groups') return 'Semua kelompok Quick Wins'
    if (node.kind === 'programs') return 'Program prioritas'
    if (node.kind === 'group') {
      const group = cakupanGroup(node.code)
      return group ? `${group.code} · ${group.title}` : node.code
    }
    if (node.kind === 'code') {
      const code = cakupanCode(node.code)
      return code ? `${code.code} · ${code.title}` : node.code
    }
    if (node.kind === 'program') {
      const program = cakupanProgram(node.key)
      return program ? program.title : node.key
    }
    if (node.kind === 'metric') {
      const program = cakupanProgram(node.program)
      const metric = program?.metrics.find((item) => item.key === node.metric)
      return [program?.title, metric?.label].filter(Boolean).join(' · ') || node.metric
    }
    const scope = node.scope === 'polres' ? 'Polres' : 'Satker Polda'
    const context = node.program
      ? cakupanNodeLabel({ kind: 'program', key: node.program })
      : node.group
        ? cakupanNodeLabel({ kind: 'group', code: node.group })
        : node.code
          ? cakupanNodeLabel({ kind: 'code', code: node.code })
          : ''
    return context ? `${scope} · ${context}` : scope
  }

  function buildCakupanDataset(node: CakupanNode): CakupanDataset | null {
    const groups = coverage?.groups ?? []
    const codeColumns = ['Kode', 'Deskripsi', 'Kegiatan', 'Polres', 'Satker']
    const cardColumns = ['Kode', 'Sub-kode', 'Kartu', 'Nilai', 'Satuan', 'Terisi']
    if (node.kind === 'cards' || node.kind === 'cardGroup') {
      const cards =
        node.kind === 'cards'
          ? cakupanCards
          : (cakupanCardGroups.find((group) => group.code === node.group)?.cards ?? [])
      const entries = cards.flatMap((card) =>
        card.metrics.map((metric) => ({ card, metric })),
      )
      if (!entries.length) return null
      return {
        title:
          node.kind === 'cards'
            ? 'Kartu cakupan Quick Wins'
            : `${node.group} · Kartu cakupan Quick Wins`,
        summaryLabel: 'Kartu cakupan Quick Wins',
        summaryValue: entries.length,
        summaryDetail: `${cards.length} sub-kode · ${entries.length} kartu`,
        summaryIcon: 'clipboard-list',
        tableColumns: cardColumns,
        tableRows: [
          [...cardColumns],
          ...entries.map(({ card, metric }) => [
            card.code,
            card.title,
            metric.label,
            cakupanValue(metric.total, metric.unit),
            metric.unit,
            `${metric.filled_rows}/${metric.filled_rows + metric.missing_rows}`,
          ]),
        ],
        chartLabels: cards.map((card) => card.code),
        chartValues: cards.map(
          (card) => card.metrics.length + card.breakdowns.length,
        ),
        chartTitle: 'Jumlah kartu per sub-kode',
        items: entries.map(({ card, metric }) => ({
          label: `${card.code} · ${metric.label}`,
          value: metric.total,
          unit: metric.unit,
          detail: `${card.title} · ${metric.filled_rows} laporan terisi`,
          icon: cakupanGroupIcon(card.code.split('.')[0] ?? ''),
        })),
      }
    }
    if (node.kind === 'card') {
      const card = cakupanCard(node.code)
      if (!card) return null
      return {
        title: `${card.code} · ${card.title}`,
        summaryLabel: card.title,
        summaryValue: card.rows,
        summaryDetail: `${card.code} · ${card.metrics.length} kartu · ${card.breakdowns.length} rincian`,
        summaryIcon: cakupanGroupIcon(card.code.split('.')[0] ?? ''),
        tableColumns: cardColumns,
        tableRows: [
          [...cardColumns],
          ...card.metrics.map((metric) => [
            card.code,
            card.title,
            metric.label,
            cakupanValue(metric.total, metric.unit),
            metric.unit,
            `${metric.filled_rows}/${metric.filled_rows + metric.missing_rows}`,
          ]),
        ],
        chartLabels: card.metrics.map((metric) => metric.label),
        chartValues: card.metrics.map((metric) => metric.total),
        chartTitle: `${card.code} · kartu cakupan`,
        items: card.metrics.map((metric) => ({
          label: metric.label,
          value: metric.total,
          unit: metric.unit,
          detail: `${card.code} · ${metric.filled_rows} dari ${metric.filled_rows + metric.missing_rows} laporan mengisi`,
          icon: cakupanGroupIcon(card.code.split('.')[0] ?? ''),
        })),
      }
    }
    if (node.kind === 'cardMetric') {
      const card = cakupanCard(node.code)
      const metric = card?.metrics.find((item) => item.key === node.metric)
      if (!card || !metric) return null
      const detail = `${metric.filled_rows}/${metric.filled_rows + metric.missing_rows} laporan mengisi`
      return {
        title: `${card.code} · ${metric.label}`,
        summaryLabel: metric.label,
        summaryValue: metric.total,
        summaryDetail: detail,
        summaryIcon: cakupanGroupIcon(card.code.split('.')[0] ?? ''),
        tableColumns: cardColumns,
        tableRows: [
          [...cardColumns],
          [
            card.code,
            card.title,
            metric.label,
            cakupanValue(metric.total, metric.unit),
            metric.unit,
            `${metric.filled_rows}/${metric.filled_rows + metric.missing_rows}`,
          ],
        ],
        chartLabels: [metric.label],
        chartValues: [metric.total],
        chartTitle: `${card.code} · ${metric.label}`,
        items: [
          {
            label: metric.label,
            value: metric.total,
            unit: metric.unit,
            detail: `${card.code} · ${detail}`,
            icon: cakupanGroupIcon(card.code.split('.')[0] ?? ''),
          },
        ],
      }
    }
    if (node.kind === 'cardBreakdown') {
      const card = cakupanCard(node.code)
      const breakdown = card?.breakdowns.find((item) => item.key === node.breakdown)
      if (!card || !breakdown) return null
      const columns = ['Kategori', 'Jumlah']
      return {
        title: `${card.code} · ${breakdown.label}`,
        summaryLabel: breakdown.label,
        summaryValue: breakdown.values.reduce((total, item) => total + item.count, 0),
        summaryDetail: `${breakdown.values.length} kategori · ${breakdown.filled_rows} laporan`,
        summaryIcon: 'chart-column',
        tableColumns: columns,
        tableRows: [
          [...columns],
          ...breakdown.values.map((item) => [item.label, formatNumber(item.count)]),
        ],
        chartLabels: breakdown.values.map((item) => item.label),
        chartValues: breakdown.values.map((item) => item.count),
        chartTitle: `${card.code} · ${breakdown.label}`,
        items: breakdown.values.map((item) => ({
          label: item.label,
          value: item.count,
          unit: 'laporan',
          detail: `${card.code} · ${breakdown.label}`,
          icon: 'chart-column',
        })),
      }
    }
    if (node.kind === 'cardTable') {
      const card = cakupanCard(node.code)
      const table = card?.tables.find((item) => item.key === node.table)
      if (!card || !table) return null
      const totals = table.rows.map((row) =>
        row.values.reduce((sum, value) => sum + value, 0),
      )
      return {
        title: `${card.code} · ${table.label}`,
        summaryLabel: table.label,
        summaryValue: totals.reduce((sum, value) => sum + value, 0),
        summaryDetail: `${table.rows.length} polres · ${table.columns.length - 1} jenis`,
        summaryIcon: 'clipboard-list',
        tableColumns: table.columns,
        tableRows: [
          [...table.columns],
          ...table.rows.map((row) => [
            row.label,
            ...row.values.map((value) => formatNumber(value)),
          ]),
        ],
        chartLabels: table.rows.map((row) => row.label),
        chartValues: totals,
        chartTitle: `${card.code} · ${table.label}`,
        items: table.rows.map((row, index) => ({
          label: row.label,
          value: totals[index] ?? 0,
          unit: 'laporan',
          detail: `${card.code} · ${table.label}`,
          icon: 'chart-column',
        })),
      }
    }
    if (node.kind === 'cardTableColumn') {
      const card = cakupanCard(node.code)
      const table = card?.tables.find((item) => item.key === node.table)
      const column = table?.columns[node.column]
      if (!card || !table || !column || node.column < 1) return null
      const labels = table.rows.map((row) => row.label)
      const values = table.rows.map((row) => row.values[node.column - 1] ?? 0)
      return {
        title: `${card.code} · ${column} per polres`,
        summaryLabel: `${column} per polres`,
        summaryValue: values.reduce((sum, value) => sum + value, 0),
        summaryDetail: `${labels.length} polres`,
        summaryIcon: 'chart-column',
        tableColumns: ['Polres', column],
        tableRows: [
          ['Polres', column],
          ...table.rows.map((row) => [row.label, formatNumber(row.values[node.column - 1] ?? 0)]),
        ],
        chartLabels: labels,
        chartValues: values,
        chartTitle: `${column} per polres`,
        chartColor: tableSeriesColor(column, node.column - 1),
        items: table.rows.map((row, index) => ({
          label: row.label,
          value: values[index] ?? 0,
          unit: 'kegiatan',
          detail: `${card.code} · ${column}`,
          icon: 'chart-column',
        })),
      }
    }
    if (node.kind === 'cardTableSeries') {
      const card = cakupanCard(node.code)
      const table = card?.tables.find((item) => item.key === node.table)
      if (!card || !table) return null
      const labels = table.rows.map((row) => row.label)
      const series: CakupanSeries[] = table.columns.slice(1).map((column, index) => ({
        label: column,
        values: table.rows.map((row) => row.values[index] ?? 0),
        color: tableSeriesColor(column, index),
      }))
      if (!series.length) return null
      const totals = labels.map((_, index) =>
        series.reduce((sum, item) => sum + (item.values[index] ?? 0), 0),
      )
      return {
        title: `${card.code} · ${table.label}`,
        summaryLabel: table.label,
        summaryValue: totals.reduce((sum, value) => sum + value, 0),
        summaryDetail: `${labels.length} polres · ${series.length} seri`,
        summaryIcon: 'chart-column',
        tableColumns: table.columns,
        tableRows: [
          [...table.columns],
          ...table.rows.map((row) => [
            row.label,
            ...row.values.map((value) => formatNumber(value)),
          ]),
        ],
        chartLabels: labels,
        chartValues: totals,
        chartTitle: table.label,
        chartSeries: series,
        items: labels.map((label, index) => ({
          label,
          value: totals[index] ?? 0,
          unit: 'laporan',
          detail: `${card.code} · ${series.length} jenis kerawanan`,
          icon: 'chart-column',
        })),
      }
    }
    if (node.kind === 'groups') {
      return {
        title: 'Cakupan kelompok Quick Wins',
        summaryLabel: 'Semua kelompok Quick Wins',
        summaryValue: groups.reduce((total, group) => total + group.rows, 0),
        summaryDetail: `${groups.length} kelompok`,
        summaryIcon: 'clipboard-list',
        tableColumns: codeColumns,
        tableRows: [
          [...codeColumns],
          ...groups.map((group) => [
            group.code,
            group.title,
            formatNumber(group.rows),
            String(group.polres),
            String(group.satker),
          ]),
        ],
        chartLabels: groups.map((group) => group.code),
        chartValues: groups.map((group) => group.rows),
        chartTitle: 'Kegiatan per kelompok Quick Wins',
        items: groups.map((group) => ({
          label: group.title,
          value: group.rows,
          unit: 'kegiatan',
          detail: `${group.code} · ${group.polres} polres · ${group.satker} satker`,
          icon: cakupanGroupIcon(group.code),
        })),
      }
    }
    if (node.kind === 'group') {
      const group = cakupanGroup(node.code)
      if (!group) return null
      return {
        title: `${group.code} · ${group.title}`,
        summaryLabel: group.title,
        summaryValue: group.rows,
        summaryDetail: `${group.code} · ${group.codes.length} sub-kode`,
        summaryIcon: cakupanGroupIcon(group.code),
        tableColumns: codeColumns,
        tableRows: [
          [...codeColumns],
          ...group.codes.map((code) => [
            code.code,
            code.title,
            formatNumber(code.rows),
            String(code.polres),
            String(code.satker),
          ]),
        ],
        chartLabels: group.codes.map((code) => code.code),
        chartValues: group.codes.map((code) => code.rows),
        chartTitle: `${group.code} · ${group.title}`,
        items: group.codes.map((code) => ({
          label: code.title,
          value: code.rows,
          unit: 'kegiatan',
          detail: `${code.code} · ${code.polres} polres · ${code.satker} satker`,
          icon: cakupanGroupIcon(group.code),
        })),
      }
    }
    if (node.kind === 'code') {
      const code = cakupanCode(node.code)
      if (!code) return null
      const group = cakupanCodeGroup(code.code)
      return {
        title: `${code.code} · ${code.title}`,
        summaryLabel: code.title,
        summaryValue: code.rows,
        summaryDetail: code.code,
        summaryIcon: cakupanGroupIcon(group?.code ?? ''),
        tableColumns: codeColumns,
        tableRows: [
          [...codeColumns],
          [
            code.code,
            code.title,
            formatNumber(code.rows),
            String(code.polres),
            String(code.satker),
          ],
        ],
        chartLabels: [code.code],
        chartValues: [code.rows],
        chartTitle: `${code.code} · ${code.title}`,
        items: [
          {
            label: code.title,
            value: code.rows,
            unit: 'kegiatan',
            detail: `${code.code} · ${code.polres} polres · ${code.satker} satker`,
            icon: cakupanGroupIcon(group?.code ?? ''),
          },
        ],
      }
    }

    if (node.kind === 'programs') {
      const programs = coverage?.programs ?? []
      const columns = ['Program', 'Kegiatan', 'Polres', 'Satker']
      return {
        title: 'Cakupan program prioritas',
        summaryLabel: 'Program prioritas',
        summaryValue: coverage?.total_rows ?? 0,
        summaryDetail: `${programs.length} program`,
        summaryIcon: 'clipboard-list',
        tableColumns: columns,
        tableRows: [
          [...columns],
          ...programs.map((program) => [
            program.title,
            formatNumber(program.rows),
            String(program.polres),
            String(program.satker),
          ]),
        ],
        chartLabels: programs.map((program) => program.title),
        chartValues: programs.map((program) => program.rows),
        chartTitle: 'Kegiatan program prioritas',
        items: programs.map((program) => ({
          label: program.title,
          value: program.rows,
          unit: 'kegiatan',
          detail: `${program.polres}/${program.polres_total} polres · ${program.satker}/${program.satker_total} satker`,
          icon: cakupanProgramIcon(program.key),
        })),
      }
    }
    if (node.kind === 'program') {
      const program = cakupanProgram(node.key)
      if (!program) return null
      const columns = [
        'Indikator',
        'Total',
        'Satuan',
        'Baris terisi',
        'Baris kosong',
        'Polres terisi',
        'Satker terisi',
      ]
      return {
        title: program.title,
        summaryLabel: program.title,
        summaryValue: program.rows,
        summaryDetail: `${program.polres}/${program.polres_total} polres · ${program.satker}/${program.satker_total} satker`,
        summaryIcon: cakupanProgramIcon(program.key),
        tableColumns: columns,
        tableRows: [
          [...columns],
          ...program.metrics.map((metric) => [
            metric.label,
            cakupanValue(metric.total, metric.unit),
            metric.unit,
            String(metric.filled_rows),
            String(metric.missing_rows),
            String(metric.filled_polres),
            String(metric.filled_satker),
          ]),
        ],
        chartLabels: program.metrics.map((metric) => metric.label),
        chartValues: program.metrics.map((metric) => metric.total),
        chartTitle: `${program.title} · indikator`,
        items: program.metrics.map((metric) => ({
          label: metric.label,
          value: metric.total,
          unit: metric.unit,
          detail: `${metric.filled_rows}/${metric.filled_rows + metric.missing_rows} baris terisi · ${metric.filled_polres} polres · ${metric.filled_satker} satker`,
          icon: cakupanProgramIcon(program.key),
        })),
      }
    }
    if (node.kind === 'metric') {
      const program = cakupanProgram(node.program)
      const metric = program?.metrics.find((item) => item.key === node.metric)
      if (!program || !metric) return null
      const columns = [
        'Indikator',
        'Total',
        'Satuan',
        'Baris terisi',
        'Baris kosong',
        'Polres terisi',
        'Satker terisi',
      ]
      const detail = `${metric.filled_rows}/${metric.filled_rows + metric.missing_rows} baris terisi · ${metric.filled_polres} polres · ${metric.filled_satker} satker`
      return {
        title: `${program.title} · ${metric.label}`,
        summaryLabel: `${program.title} · ${metric.label}`,
        summaryValue: metric.total,
        summaryDetail: detail,
        summaryIcon: cakupanProgramIcon(program.key),
        tableColumns: columns,
        tableRows: [
          [...columns],
          [
            metric.label,
            cakupanValue(metric.total, metric.unit),
            metric.unit,
            String(metric.filled_rows),
            String(metric.missing_rows),
            String(metric.filled_polres),
            String(metric.filled_satker),
          ],
        ],
        chartLabels: [metric.label],
        chartValues: [metric.total],
        chartTitle: `${program.title} · ${metric.label}`,
        items: [
          {
            label: metric.label,
            value: metric.total,
            unit: metric.unit,
            detail: `${program.title} · ${detail}`,
            icon: cakupanProgramIcon(program.key),
          },
        ],
      }
    }

    if (node.kind === 'units' && node.program) {
      const program = cakupanProgram(node.program)
      if (!program) return null
      const scopeLabel = node.scope === 'polres' ? 'Polres' : 'Satker Polda'
      const units = (node.scope === 'polres' ? program.by_polres : program.by_satker) ?? []
      const columns = [
        scopeLabel,
        'Kegiatan',
        ...program.metrics.map((metric) =>
          metric.unit ? `${metric.label} (${metric.unit})` : metric.label,
        ),
      ]
      return {
        title: `${scopeLabel} · ${program.title}`,
        summaryLabel: scopeLabel,
        summaryValue: units.reduce((total, unit) => total + unit.rows, 0),
        summaryDetail: program.title,
        summaryIcon: node.scope === 'polres' ? 'building-2' : 'landmark',
        tableColumns: columns,
        tableRows: [
          [...columns],
          ...units.map((unit) => [
            unit.label,
            formatNumber(unit.rows),
            ...unit.values.map((value, index) => {
              const metric = program.metrics[index]
              return value.filled ? cakupanValue(value.value, metric?.unit ?? '') : '—'
            }),
          ]),
        ],
        chartLabels: units.map((unit) => unit.label),
        chartValues: units.map((unit) => unit.rows),
        chartTitle: `${scopeLabel} · ${program.title}`,
        items: units.map((unit) => ({
          label: unit.label,
          value: unit.rows,
          unit: 'kegiatan',
          detail: `${program.title}${unit.missing.length ? ` · ${unit.missing.length} indikator kosong` : ' · semua indikator terisi'}`,
          icon: node.scope === 'polres' ? 'building-2' : 'landmark',
        })),
      }
    }

    let units: CoverageUnitCount[] | undefined
    let contextTitle = ''
    if (node.group) {
      const group = cakupanGroup(node.group)
      units = node.scope === 'polres' ? group?.by_polres : group?.by_satker
      contextTitle = group ? `${group.code} · ${group.title}` : node.group
    } else {
      const code = cakupanCode(node.code ?? '')
      units = node.scope === 'polres' ? code?.by_polres : code?.by_satker
      contextTitle = code ? `${code.code} · ${code.title}` : (node.code ?? '')
    }
    const scopeLabel = node.scope === 'polres' ? 'Polres' : 'Satker Polda'
    const list = units ?? []
    return {
      title: contextTitle ? `${scopeLabel} · ${contextTitle}` : scopeLabel,
      summaryLabel: scopeLabel,
      summaryValue: list.reduce((total, unit) => total + unit.rows, 0),
      summaryDetail: contextTitle,
      summaryIcon: node.scope === 'polres' ? 'building-2' : 'landmark',
      tableColumns: [scopeLabel, 'Kegiatan'],
      tableRows: [
        [scopeLabel, 'Kegiatan'],
        ...list.map((unit) => [unit.label, formatNumber(unit.rows)]),
      ],
      chartLabels: list.map((unit) => unit.label),
      chartValues: list.map((unit) => unit.rows),
      chartTitle: contextTitle
        ? `${scopeLabel} · ${contextTitle}`
        : `Kegiatan per ${scopeLabel.toLowerCase()}`,
      items: list.map((unit) => ({
        label: unit.label,
        value: unit.rows,
        unit: 'kegiatan',
        detail: contextTitle,
        icon: node.scope === 'polres' ? 'building-2' : 'landmark',
      })),
    }
  }

  function startCakupanDrag(event: DragEvent, node: CakupanNode): void {
    cakupanDragNode = node
    cakupanDragActive = true
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'copy'
      event.dataTransfer.setData('text/plain', cakupanNodeLabel(node))
    }
  }

  function endCakupanDrag(): void {
    cakupanDragActive = false
    cakupanDragNode = null
  }

  function openCakupanDialog(
    node: CakupanNode,
    point: { x: number; y: number } | null = null,
  ): void {
    cakupanTarget = node
    cakupanDropPoint = point
    cakupanDialogOpen = true
  }

  function closeCakupanDialog(): void {
    cakupanDialogOpen = false
    cakupanTarget = null
    cakupanDropPoint = null
    cakupanCardStep = false
    cakupanCardScope = 'all'
    cakupanCardPicks = []
    cakupanCardGroup = false
  }

  function onCakupanDrop(event: DragEvent): void {
    event.preventDefault()
    const node = cakupanDragNode
    const rect = iframeEl?.getBoundingClientRect()
    cakupanDragActive = false
    cakupanDragNode = null
    if (!node) return
    let point: { x: number; y: number } | null = null
    if (rect && rect.width > 0 && rect.height > 0) {
      point = {
        x: clamp(((event.clientX - rect.left) / rect.width) * PAGE_WIDTH, 2, PAGE_WIDTH - 2),
        y: clamp(((event.clientY - rect.top) / rect.height) * PAGE_HEIGHT, 2, PAGE_HEIGHT - 2),
      }
    }
    openCakupanDialog(node, point)
  }

  function insertCakupan(mode: 'table' | 'chart' | 'hbar' | 'cards'): void {
    if (!cakupanTarget) return
    const dataset = buildCakupanDataset(cakupanTarget)
    const page = getPage(currentPage)
    if (!dataset || !page) return
    if (!dataset.items.length) {
      toast.error('Belum ada data cakupan untuk pilihan ini.')
      return
    }
    if (mode === 'cards') {
      openCakupanCardStep()
      return
    }
    pushHistory()
    addCakupanBlock(page, dataset, mode, cakupanDropPoint)
    closeCakupanDialog()
    applyLayoutToDom()
    toast.success(
      `Data cakupan ditambahkan sebagai ${mode === 'table' ? 'tabel' : 'grafik'}.`,
    )
  }

  function openCakupanCardStep(): void {
    const dataset = cakupanDataset
    if (!dataset) return
    if (dataset.items.length <= 1) {
      createCakupanCards(dataset.items)
      return
    }
    cakupanCardScope = 'all'
    cakupanCardPicks = dataset.items.map((_, index) => index)
    cakupanCardStep = true
  }

  function toggleCakupanPick(index: number): void {
    cakupanCardPicks = cakupanCardPicks.includes(index)
      ? cakupanCardPicks.filter((item) => item !== index)
      : [...cakupanCardPicks, index]
  }

  function selectAllCakupanPicks(): void {
    const dataset = cakupanDataset
    if (!dataset) return
    cakupanCardPicks = dataset.items.map((_, index) => index)
  }

  function clearCakupanPicks(): void {
    cakupanCardPicks = []
  }

  function applyCakupanCards(): void {
    const dataset = cakupanDataset
    if (!dataset) return
    if (cakupanCardScope === 'summary') {
      createCakupanCards([
        {
          label: dataset.summaryLabel,
          value: dataset.summaryValue,
          unit: 'kegiatan',
          detail: dataset.summaryDetail,
          icon: dataset.summaryIcon,
        },
      ])
      return
    }
    const items =
      cakupanCardScope === 'pick'
        ? cakupanCardPicks
            .map((index) => dataset.items[index])
            .filter((item): item is CakupanItem => Boolean(item))
        : dataset.items
    createCakupanCards(items)
  }

  function createCakupanCards(items: CakupanItem[], groupCards = cakupanCardGroup): void {
    const page = getPage(currentPage)
    if (!page) return
    if (!items.length) {
      toast.error('Pilih minimal satu kartu.')
      return
    }
    pushHistory()
    addCakupanCards(page, items, cakupanDropPoint, groupCards)
    closeCakupanDialog()
    applyLayoutToDom()
    toast.success(`${items.length} kartu cakupan ditambahkan.`)
  }

  function addCakupanBlock(
    page: LayoutPage,
    dataset: CakupanDataset,
    mode: 'table' | 'chart' | 'hbar',
    point: { x: number; y: number } | null,
  ): void {
    const element = baseElement(mode === 'table' ? 'table' : 'chart')
    if (mode === 'table') {
      element.width = Math.min(180, PAGE_WIDTH - 24)
      element.height = Math.min(PAGE_HEIGHT - 20, 12 + dataset.tableRows.length * 5.5)
      element.font_size = 8.5
      element.table_rows = dataset.tableRows.map((row) => [...row])
      element.table_header = true
    } else {
      element.width = Math.min(190, PAGE_WIDTH - 24)
      element.height = Math.min(160, Math.max(70, 40 + dataset.chartLabels.length * 6))
      element.font_size = 9
      element.chart_type = mode === 'hbar' ? 'hbar' : 'bar'
      element.chart_title = dataset.chartTitle
      element.chart_labels = [...dataset.chartLabels]
      element.chart_values = [...dataset.chartValues]
      element.chart_colors = dataset.chartColor
        ? dataset.chartLabels.map(() => dataset.chartColor as string)
        : dataset.chartLabels.map(
            (_, index) => CHART_PALETTE[index % CHART_PALETTE.length],
          )
      element.chart_series = dataset.chartSeries
        ? dataset.chartSeries.map((item) => ({
            label: item.label,
            values: [...item.values],
            color: item.color,
          }))
        : []
      element.chart_show_values = true
      element.chart_show_legend = dataset.chartSeries?.length
        ? true
        : dataset.chartLabels.length <= 8
      element.chart_label_rotation =
        dataset.chartLabels.length > 6 &&
        dataset.chartLabels.some((label) => label.length > 10)
          ? -35
          : 0
    }
    if (point) {
      element.x = clamp(point.x, 2, PAGE_WIDTH - element.width - 2)
      element.y = clamp(point.y, 2, PAGE_HEIGHT - element.height - 2)
    } else {
      element.x = (PAGE_WIDTH - element.width) / 2
      element.y = (PAGE_HEIGHT - element.height) / 2
    }
    clampElement(element)
    page.elements.push(element)
    selected = { kind: 'element', page: currentPage, id: element.id }
  }

  function addCakupanCards(
    page: LayoutPage,
    items: CakupanItem[],
    point: { x: number; y: number } | null,
    groupCards: boolean,
  ): void {
    const count = items.length
    const columns =
      count <= 1
        ? 1
        : count === 2
          ? 2
          : count === 3
            ? 3
            : count === 4
              ? 2
              : count <= 6
                ? 3
                : count <= 12
                  ? 4
                  : 5
    const gap = 4
    const available = PAGE_WIDTH - 24
    const cardWidth = Math.min(64, (available - gap * (columns - 1)) / columns)
    const cardHeight = count > 12 ? 24 : count > 6 ? 27 : 30
    const rows = Math.ceil(count / columns)
    const gridWidth = columns * cardWidth + gap * (columns - 1)
    const gridHeight = rows * cardHeight + gap * (rows - 1)
    const startX = clamp(
      point ? point.x : (PAGE_WIDTH - gridWidth) / 2,
      6,
      Math.max(6, PAGE_WIDTH - gridWidth - 6),
    )
    const startY = clamp(
      point ? point.y : (PAGE_HEIGHT - gridHeight) / 2,
      6,
      Math.max(6, PAGE_HEIGHT - gridHeight - 6),
    )
    const template = cardTemplate('icon_right')
    const groupId = groupCards && count > 1 ? newId() : ''
    items.forEach((item, index) => {
      const element = baseElement('card')
      element.width = cardWidth
      element.height = cardHeight
      element.x = startX + (index % columns) * (cardWidth + gap)
      element.y = startY + Math.floor(index / columns) * (cardHeight + gap)
      element.z = 10 + index
      element.group = groupId
      element.card_template = template.id
      element.fill = template.fill
      element.card_accent = template.accent
      element.color = template.color
      element.card_icon_bg = template.iconBg
      element.card_icon_color = template.iconColor
      element.border_width = template.borderWidth
      element.radius = template.radius
      element.card_icon = item.icon
      element.card_icon_scale = 80
      element.card_label = item.label
      element.card_value = cakupanValue(item.value, item.unit)
      element.card_unit = item.unit
      element.card_detail = item.detail
      element.font_size = count > 12 ? 8 : 9
      page.elements.push(element)
    })
    const last = page.elements[page.elements.length - 1]
    selected = last ? { kind: 'element', page: currentPage, id: last.id } : null
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

  function openCardIconPicker() {
    if (!activeElement || activeElement.type !== 'card') return
    iconPickerTarget = 'card'
    iconSearch = ''
    iconPickerOpen = true
  }

  function applyCardTemplate(templateId: CardTemplateId) {
    if (!activeElement || activeElement.type !== 'card') return
    const template = cardTemplate(templateId)
    const patch: Partial<LayoutElement> = {
      card_template: template.id,
      fill: template.fill,
      card_accent: template.accent,
      color: template.color,
      card_icon_color: template.iconColor,
      card_icon_bg: template.iconBg,
      border_width: template.borderWidth,
      radius: template.radius,
      card_label_font_family: 'sans',
      card_label_font_size: 0,
      card_label_color: '',
      card_value_font_family: 'sans',
      card_value_font_size: 0,
      card_value_color: '',
      card_unit_font_family: 'sans',
      card_unit_font_size: 0,
      card_unit_color: '',
      card_detail_font_family: 'sans',
      card_detail_font_size: 0,
      card_detail_color: '',
    }
    if (template.icon && !activeElement.card_icon?.trim()) {
      patch.card_icon = template.icon
    }
    updateElement(patch, true)
  }

  function chooseIcon(icon: IconDefinition) {
    if (iconPickerTarget === 'card' && activeElement?.type === 'card') {
      updateElement({ card_icon: icon.name }, true)
      iconPickerOpen = false
      return
    }
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

  function syncTableColumnStyles(element: LayoutElement): void {
    const columns = Math.max(1, ...(element.table_rows ?? []).map((row) => row.length))
    element.table_column_backgrounds = Array.from(
      { length: columns },
      (_, index) => element.table_column_backgrounds?.[index] ?? '',
    )
    element.table_column_colors = Array.from(
      { length: columns },
      (_, index) => element.table_column_colors?.[index] ?? '',
    )
    element.table_column_font_families = Array.from(
      { length: columns },
      (_, index) => element.table_column_font_families?.[index] ?? '',
    )
    element.table_column_font_sizes = Array.from(
      { length: columns },
      (_, index) => element.table_column_font_sizes?.[index] ?? 0,
    )
    element.table_column_aligns = Array.from({ length: columns }, (_, index) => {
      const value = element.table_column_aligns?.[index]
      return value === 'left' || value === 'center' || value === 'right' ? value : ''
    })
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
    syncTableColumnStyles(activeElement)
    applyLayoutToDom()
  }

  function removeTableColumn() {
    if (!activeElement || activeElement.type !== 'table' || activeElement.binding) return
    const rows = activeElement.table_rows ?? []
    const columns = Math.max(1, ...rows.map((row) => row.length))
    if (columns <= 1) return
    pushHistory()
    activeElement.table_rows = rows.map((row) => row.slice(0, -1))
    syncTableColumnStyles(activeElement)
    applyLayoutToDom()
  }

  function openChartEditor(): void {
    if (!selected || selected.kind !== 'element') return
    const element = getElement(selected.page, selected.id)
    if (!element || element.type !== 'chart') return
    chartEditorTarget = { page: selected.page, id: selected.id }
  }

  function contextEditChart(): void {
    const menu = contextMenu
    if (!menu || menu.kind !== 'element' || !menu.id) return
    const element = getElement(menu.page, menu.id)
    if (!element || element.type !== 'chart') {
      closeContextMenu()
      return
    }
    selected = { kind: 'element', page: menu.page, id: menu.id }
    chartEditorTarget = { page: menu.page, id: menu.id }
    closeContextMenu()
  }

  function saveChartEditor(patch: Partial<LayoutElement>): void {
    const target = chartEditorTarget
    if (!target) return
    const element = getElement(target.page, target.id)
    if (!element) {
      chartEditorTarget = null
      return
    }
    pushHistory()
    Object.assign(element, patch)
    clampElement(element)
    applyLayoutToDom()
    chartEditorTarget = null
    toast.success('Grafik diperbarui.')
  }

  function openCardEditor(): void {
    if (!selected || selected.kind !== 'element') return
    const element = getElement(selected.page, selected.id)
    if (!element || element.type !== 'card') return
    cardEditorTarget = { page: selected.page, id: selected.id }
  }

  function contextEditCard(): void {
    const menu = contextMenu
    if (!menu || menu.kind !== 'element' || !menu.id) return
    const element = getElement(menu.page, menu.id)
    if (!element || element.type !== 'card') {
      closeContextMenu()
      return
    }
    selected = { kind: 'element', page: menu.page, id: menu.id }
    cardEditorTarget = { page: menu.page, id: menu.id }
    closeContextMenu()
  }

  function saveCardEditor(patch: Partial<LayoutElement>): void {
    const target = cardEditorTarget
    if (!target) return
    const element = getElement(target.page, target.id)
    if (!element) {
      cardEditorTarget = null
      return
    }
    pushHistory()
    Object.assign(element, patch)
    clampElement(element)
    applyLayoutToDom()
    cardEditorTarget = null
    toast.success('Kartu diperbarui.')
  }

  function openTableEditor(): void {
    if (!selected || selected.kind !== 'element') return
    const element = getElement(selected.page, selected.id)
    if (!element || element.type !== 'table') return
    tableEditorTarget = { page: selected.page, id: selected.id }
  }

  function contextEditTable(): void {
    const menu = contextMenu
    if (!menu || menu.kind !== 'element' || !menu.id) return
    const element = getElement(menu.page, menu.id)
    if (!element || element.type !== 'table') {
      closeContextMenu()
      return
    }
    selected = { kind: 'element', page: menu.page, id: menu.id }
    tableEditorTarget = { page: menu.page, id: menu.id }
    closeContextMenu()
  }

  function saveTableEditor(patch: Partial<LayoutElement>): void {
    const target = tableEditorTarget
    if (!target) return
    const element = getElement(target.page, target.id)
    if (!element) {
      tableEditorTarget = null
      return
    }
    pushHistory()
    Object.assign(element, patch)
    clampElement(element)
    applyLayoutToDom()
    tableEditorTarget = null
    toast.success('Tabel diperbarui.')
  }

  function openMediaLibrary() {
    mediaTarget = 'new'
    mediaOpen = true
  }

  function openMediaReplace() {
    mediaTarget = 'replace'
    mediaOpen = true
  }

  function insertMediaImage(uploaded: UploadedDesignImage, filename: string) {
    if (mediaTarget === 'background') {
      const page = activePage
      if (!page) return
      pushHistory()
      page.background = `upload:${uploaded.id}`
      page.background_mode = 'image'
      applyPageBackgroundToDom()
      toast.success(`Gambar ${filename} dipakai sebagai latar halaman.`)
      return
    }
    if (mediaTarget === 'replace' && activeElement?.type === 'image') {
      updateElement({ src: `upload:${uploaded.id}`, alt: filename }, true)
      toast.success('Gambar berhasil diganti.')
      return
    }
    const element = baseElement('image')
    element.src = `upload:${uploaded.id}`
    element.alt = filename
    addElement(element)
    toast.success('Gambar ditambahkan ke halaman.')
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

  function pageGradientColors(page: LayoutPage | undefined): string[] {
    const colors = page?.gradient_colors ?? []
    if (colors.length >= 2) return colors
    return [pickerColor(page?.background, '#ffffff'), '#b8894b']
  }

  function pageBackgroundImageSrc(page: LayoutPage | undefined): string {
    const src = (page?.background ?? '').trim()
    if (!src) return ''
    if (src.startsWith('upload:')) {
      const id = src.slice(7)
      return id ? designApi.imageUrl(periodId, id) : ''
    }
    return /^https?:\/\//i.test(src) ? src : ''
  }

  function cssUrl(value: string): string {
    return value
      .replace(/'/g, '%27')
      .replace(/"/g, '%22')
      .replace(/\(/g, '%28')
      .replace(/\)/g, '%29')
      .replace(/\\/g, '%5C')
  }

  function pageBackgroundCss(page: LayoutPage | undefined): string {
    if (!page) return '#ffffff'
    if (page.background_mode === 'image') {
      const src = pageBackgroundImageSrc(page)
      return src ? `url("${cssUrl(src)}") center / cover no-repeat` : '#ffffff'
    }
    const colors = page.gradient_colors ?? []
    const mode = page.background_mode
    if ((mode === 'linear' || mode === 'radial') && colors.length >= 2) {
      const last = colors.length - 1
      const stops = colors
        .map((color, index) => `${color} ${((index / last) * 100).toFixed(1)}%`)
        .join(', ')
      return mode === 'radial'
        ? `radial-gradient(circle, ${stops})`
        : `linear-gradient(${page.gradient_angle ?? 90}deg, ${stops})`
    }
    return pickerColor(page.background, '#ffffff')
  }

  function applyPageBackgroundToDom() {
    const doc = iframeEl?.contentDocument
    if (!doc) return
    Array.from(doc.querySelectorAll<HTMLElement>('.page')).forEach((pageEl, index) => {
      const page = getPage(index + 1)
      if (page) pageEl.style.background = pageBackgroundCss(page)
    })
  }

  function setPageBackgroundMode(mode: 'solid' | 'linear' | 'radial' | 'image') {
    const page = activePage
    if (!page) return
    pushHistory()
    const wasImage = page.background_mode === 'image'
    page.background_mode = mode
    if (mode === 'image') {
      if (!pageBackgroundImageSrc(page)) page.background = ''
      applyPageBackgroundToDom()
      return
    }
    if (wasImage) page.background = '#ffffff'
    if (mode !== 'solid' && (page.gradient_colors?.length ?? 0) < 2) {
      page.gradient_colors = pageGradientColors(page)
    }
    applyPageBackgroundToDom()
  }

  function openPageBackgroundMedia() {
    mediaTarget = 'background'
    mediaOpen = true
  }

  function clearPageBackgroundImage() {
    const page = activePage
    if (!page) return
    pushHistory()
    page.background = '#ffffff'
    page.background_mode = 'solid'
    page.gradient_colors = []
    applyPageBackgroundToDom()
  }

  function setPageBackgroundColor(color: string) {
    const page = activePage
    if (!page) return
    page.background = color
    applyPageBackgroundToDom()
  }

  function setPageGradientColor(index: number, color: string) {
    const page = activePage
    if (!page) return
    const colors = [...pageGradientColors(page)]
    colors[index] = color
    page.gradient_colors = colors
    applyPageBackgroundToDom()
  }

  function addPageGradientColor() {
    const page = activePage
    if (!page) return
    const colors = pageGradientColors(page)
    if (colors.length >= 8) return
    pushHistory()
    const palette = ['#143d30', '#b8894b', '#3f765d', '#5b8fa8', '#a4743d', '#d9b87e', '#7a857f', '#2f5d7c']
    page.gradient_colors = [...colors, palette[colors.length % palette.length]]
    applyPageBackgroundToDom()
  }

  function removePageGradientColor(index: number) {
    const page = activePage
    if (!page) return
    const colors = pageGradientColors(page)
    if (colors.length <= 2) return
    pushHistory()
    page.gradient_colors = colors.filter((_, itemIndex) => itemIndex !== index)
    applyPageBackgroundToDom()
  }

  function setPageGradientAngle(value: number) {
    const page = activePage
    if (!page) return
    page.gradient_angle = value
    applyPageBackgroundToDom()
  }

  function resetPageBackground() {
    const page = activePage
    if (!page) return
    pushHistory()
    page.background = '#ffffff'
    page.background_mode = 'solid'
    page.gradient_angle = 90
    page.gradient_colors = []
    applyPageBackgroundToDom()
  }

  function applyPageBackgroundToAll() {
    const source = activePage
    if (!source) return
    pushHistory()
    const background = source.background ?? ''
    const mode = source.background_mode ?? 'solid'
    const angle = source.gradient_angle ?? 90
    const colors = [...(source.gradient_colors ?? [])]
    for (const page of layout.pages) {
      page.background = background
      page.background_mode = mode
      page.gradient_angle = angle
      page.gradient_colors = [...colors]
    }
    applyPageBackgroundToDom()
    toast.success('Latar diterapkan ke semua halaman.')
  }

  function numberValue(value: string): number {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : 0
  }

  function clearAutosaveTimers(): void {
    if (autosaveTimer !== null) {
      clearTimeout(autosaveTimer)
      autosaveTimer = null
    }
    if (autosaveMaxTimer !== null) {
      clearTimeout(autosaveMaxTimer)
      autosaveMaxTimer = null
    }
    if (autosaveRetryTimer !== null) {
      clearTimeout(autosaveRetryTimer)
      autosaveRetryTimer = null
    }
  }

  function canAutosave(): boolean {
    return (
      hasImport &&
      autosaveReady &&
      !loading &&
      !choosingTemplate &&
      !drag &&
      !generating &&
      !sharing &&
      !previewing
    )
  }

  function scheduleAutosave(delay = AUTOSAVE_DEBOUNCE_MS): void {
    if (!canAutosave() || !dirty) return
    if (autosaveTimer !== null) clearTimeout(autosaveTimer)
    if (autosaveRetryTimer !== null) {
      clearTimeout(autosaveRetryTimer)
      autosaveRetryTimer = null
    }
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null
      void autosaveNow()
    }, delay)
    if (autosaveMaxTimer === null) {
      autosaveMaxTimer = setTimeout(() => {
        autosaveMaxTimer = null
        if (autosaveTimer !== null) {
          clearTimeout(autosaveTimer)
          autosaveTimer = null
        }
        void autosaveNow()
      }, AUTOSAVE_MAX_WAIT_MS)
    }
  }

  function scheduleAutosaveRetry(): void {
    if (autosaveRetryTimer !== null) clearTimeout(autosaveRetryTimer)
    autosaveRetryTimer = setTimeout(() => {
      autosaveRetryTimer = null
      void autosaveNow()
    }, AUTOSAVE_RETRY_MS)
  }

  async function autosaveNow(): Promise<void> {
    if (!canAutosave() || !dirty) {
      if (!dirty) clearAutosaveTimers()
      return
    }
    if (saving) {
      autosaveQueued = true
      return
    }

    const targetPeriod = periodId
    clearAutosaveTimers()
    autosaveQueued = false
    autosaveError = ''
    const task = performSave({ silent: true, auto: true })
    activeSavePromise = task
    const ok = await task
    if (activeSavePromise === task) activeSavePromise = null
    if (targetPeriod !== periodId) return

    if (ok) {
      autosaveError = ''
      autosaveErrorNotified = false
      autosaveState = 'saved'
      lastSaveMode = 'auto'
      if (dirty) scheduleAutosave()
    } else {
      autosaveState = 'error'
      if (!autosaveErrorNotified) {
        autosaveErrorNotified = true
        toast.error(
          localDraftAt
            ? 'Auto-save gagal. Draft tersimpan di browser.'
            : 'Auto-save gagal. Perubahan belum tersimpan.',
        )
      }
      scheduleAutosaveRetry()
    }

    if (autosaveQueued) {
      autosaveQueued = false
      if (dirty) scheduleAutosave()
    }
  }

  async function performSave(options: { silent: boolean; auto: boolean }): Promise<boolean> {
    if (!hasImport) return false
    const targetPeriod = periodId
    const snapshot = cloneLayout(layout)
    const snapshotJson = JSON.stringify(snapshot)
    if (snapshotJson === savedJson) {
      if (localDraftAt) {
        clearDesignDraft(targetPeriod, draftOwner)
        localDraftAt = null
      }
      if (!options.auto && !options.silent) toast.success('Layout desain tersimpan.')
      return true
    }

    const draft = writeDesignDraft(targetPeriod, snapshot, updatedAt, draftOwner)
    if (draft) {
      localDraftAt = draft.savedAt
      draftUnavailable = false
    } else {
      localDraftAt = null
      draftUnavailable = true
    }

    saving = true
    if (!options.auto) {
      error = ''
      autosaveState = 'idle'
      autosaveError = ''
    }
    const beforeStructure = layoutStructure(snapshot)

    try {
      const view = await designApi.saveLayout(targetPeriod, snapshot)
      if (targetPeriod !== periodId) return false
      clearDesignDraft(targetPeriod, draftOwner)
      localDraftAt = null
      draftUnavailable = false
      const unchanged = JSON.stringify(layout) === snapshotJson
      if (!options.auto && unchanged) {
        const previous = selected
        const normalized = normalizeLayout(view.layout)
        layout = normalized
        savedJson = JSON.stringify(normalized)
        if (previous?.kind === 'element' && !getElement(previous.page, previous.id)) {
          selected = null
        } else {
          selected = previous
        }
        applyLayoutToDom()
        if (layoutStructure(normalized) !== beforeStructure) void refreshCanvasStructure()
      } else {
        // Autosave tidak mengganti layout/DOM agar fokus edit teks dan drag
        // tidak terganggu. Simpan hanya penanda versi yang sudah tersimpan.
        savedJson = snapshotJson
      }
      updatedAt = view.updated_at
      updatedBy = view.updated_by
      hasSavedDesign = true
      autosaveReady = true
      lastSaveMode = options.auto ? 'auto' : 'manual'
      autosaveState = options.auto ? 'saved' : 'idle'
      autosaveError = ''
      autosaveErrorNotified = false
      if (!options.silent) toast.success('Layout desain tersimpan.')
      return true
    } catch (caught) {
      const message = errorMessage(caught)
      if (options.auto) {
        autosaveError = message
      } else {
        error = message
      }
      return false
    } finally {
      saving = false
    }
  }

  async function save(silent = false): Promise<boolean> {
    if (!hasImport) return false
    const targetPeriod = periodId
    clearAutosaveTimers()
    autosaveQueued = false

    if (saving) {
      const current = activeSavePromise
      if (current) await current
      if (targetPeriod !== periodId) return false
      if (!dirty) return true
    }

    const task = performSave({ silent, auto: false })
    activeSavePromise = task
    const ok = await task
    if (activeSavePromise === task) activeSavePromise = null
    if (targetPeriod !== periodId) return false
    if (ok && dirty) scheduleAutosave()
    if (autosaveQueued) {
      autosaveQueued = false
      if (dirty) scheduleAutosave()
    }
    return ok
  }

  async function flushAutosave(): Promise<boolean> {
    clearAutosaveTimers()
    for (let attempt = 0; attempt < 2; attempt += 1) {
      if (saving) {
        const current = activeSavePromise
        if (current) await current
      }
      if (!dirty) return true
      const ok = await save(true)
      if (!ok) return false
      if (!dirty) return true
    }
    if (dirty) {
      error = 'Perubahan desain belum selesai disimpan. Coba lagi.'
      return false
    }
    return true
  }

  function flushOnPageHide(): void {
    if (!hasImport || !autosaveReady || !dirty) return
    clearAutosaveTimers()
    const snapshot = cloneLayout(layout)
    const draft = writeDesignDraft(periodId, snapshot, updatedAt, draftOwner)
    if (draft) localDraftAt = draft.savedAt
    void designApi.saveLayout(periodId, snapshot, true).catch(() => {})
  }

  async function discardLocalDraft(): Promise<void> {
    if (!localDraftAt && !draftUnavailable) return
    const confirmed = await dialog.confirm({
      title: 'Buang draft lokal?',
      message:
        'Salinan lokal periode ini akan dihapus. Desain yang sedang diedit tetap ada di layar dan akan dicoba disimpan lagi.',
      confirmLabel: 'Buang draft',
      tone: 'danger',
    })
    if (!confirmed) return
    clearDesignDraft(periodId, draftOwner)
    localDraftAt = null
    draftUnavailable = false
    toast.success('Draft lokal dibuang.')
  }

  async function preview() {
    if (!hasImport) return
    if (!(await flushAutosave())) return
    const target = window.open('', '_blank')
    if (!target) {
      toast.error('Izinkan popup untuk membuka pratinjau.')
      return
    }
    previewing = true
    try {
      const result = await reportsApi.preview(periodId, content, layout)
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

  async function sharePresentation() {
    if (!hasImport || generating || sharing) return
    if (!(await flushAutosave())) return
    sharing = true
    error = ''
    try {
      const report = await reportsApi.generate(periodId, content)
      await ongenerated()
      shareTarget = report
    } catch (caught) {
      error = errorMessage(caught)
      toast.error(errorMessage(caught))
    } finally {
      sharing = false
    }
  }

  async function generate() {
    if (!hasImport || sharing) return
    if (!(await flushAutosave())) return
    generating = true
    error = ''
    try {
      const report = await reportsApi.generate(periodId, content)
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
      if (members.some((member) => member.locked)) return
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
    if (event.key === 'Escape' && contextMenu) {
      event.preventDefault()
      closeContextMenu()
      return
    }
    if (contextMenu) closeContextMenu()
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
    if (mod && event.key.toLowerCase() === 'c' && selected?.kind === 'element') {
      event.preventDefault()
      copySelected(true)
      return
    }
    if (mod && event.key.toLowerCase() === 'v' && clipboard) {
      event.preventDefault()
      pasteClipboard()
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

<svelte:window
  onkeydown={handleKeydown}
  onpointerdown={handleWindowPointerDown}
  onblur={closeContextMenu}
  onresize={closeContextMenu}
/>

{#if !hasImport}
  <section class="card">
    <EmptyState
      title="Belum ada versi data"
      description="Impor workbook terlebih dahulu sebelum menata desain laporan."
    >
      <Button onclick={() => (window.location.hash = '/imports')}>Impor workbook</Button>
    </EmptyState>
  </section>
{:else if loading}
  <section class="card p-6">
    <div class="space-y-3">
      <div class="skeleton h-7 w-56"></div>
      <div class="skeleton h-4 w-80"></div>
      <div class="grid gap-4 sm:grid-cols-2">
        <div class="skeleton h-48"></div>
        <div class="skeleton h-48"></div>
      </div>
    </div>
  </section>
{:else if choosingTemplate}
  <section class="card overflow-hidden">
    <div class="panel-heading">
      <div>
        <span class="eyebrow">MULAI DESAIN</span>
        <h2 class="panel-title mt-1">Pilih titik awal desain</h2>
        <p class="mt-1 text-xs text-ink-muted">Belum ada desain tersimpan untuk periode ini. Mulai dari halaman kosong atau gunakan template desain yang sudah dibuat.</p>
      </div>
      <Layers size={18} class="text-gold-500" />
    </div>

    {#if error}
      <div class="m-5 flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
        <AlertCircle size={17} class="mt-0.5 shrink-0" /> <span>{error}</span>
      </div>
    {/if}

    <div class="grid gap-4 border-b border-line p-5 xl:grid-cols-[minmax(0,0.85fr)_minmax(0,1.6fr)]">
      <div class="rounded-2xl border border-line bg-stone-50/60 p-5">
        <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-white text-forest-700 shadow-sm"><FilePlus2 size={18} /></span>
        <h3 class="mt-3 font-display text-lg font-semibold text-forest-900">Blank template</h3>
        <p class="mt-1 text-[11px] leading-relaxed text-ink-muted">Mulai dari halaman kosong tanpa blok laporan otomatis. Atur orientasi dan jumlah halaman sesuai kebutuhan.</p>
        <div class="mt-4 grid gap-3 sm:grid-cols-2">
          <label class="block">
            <span class="label">Orientasi</span>
            <select class="input py-1.5 text-xs" bind:value={blankOrientation}>
              <option value="portrait">Portrait (A4)</option>
              <option value="landscape">Landscape (A4)</option>
            </select>
          </label>
          <label class="block">
            <span class="label">Jumlah halaman</span>
            <input class="input py-1.5 text-xs" type="number" min="1" max={MAX_BLANK_PAGES}
              value={blankPageCount}
              oninput={(event) => (blankPageCount = Math.round(clamp(numberValue((event.currentTarget as HTMLInputElement).value), 1, MAX_BLANK_PAGES)))} />
          </label>
        </div>
        <Button class="mt-4 w-full" onclick={chooseBlank}><FilePlus2 size={14} /> Mulai dari blank</Button>
      </div>

      <div>
        <div class="mb-3 flex flex-wrap items-end justify-between gap-2">
          <div>
            <h3 class="text-xs font-bold tracking-[0.12em] text-ink-muted uppercase">Template desain</h3>
            <p class="mt-0.5 text-[11px] text-ink-muted">Template yang pernah disimpan dari desain laporan.</p>
          </div>
          <span class="badge badge-neutral">{templates.length} template</span>
        </div>
        {#if templates.length}
          <div class="grid gap-3 sm:grid-cols-2">
            {#each templates as template}
              <article class="flex flex-col rounded-2xl border border-line bg-white p-4">
                <div class="flex flex-wrap items-center justify-between gap-2">
                  <span class="badge {template.layout.mode === 'blank' ? 'badge-neutral' : 'badge-progress'}">
                    {template.layout.mode === 'blank' ? 'Blank' : 'Laporan'}
                  </span>
                  <span class="text-[10px] text-ink-muted">{template.layout.pages.length} halaman · {template.layout.orientation === 'landscape' ? 'Landscape' : 'Portrait'}</span>
                </div>
                <h4 class="mt-2 text-sm font-semibold text-ink">{template.name}</h4>
                <p class="mt-1 text-[10px] text-ink-muted">Dibuat {formatDateTime(template.created_at)}{#if template.created_by} oleh {template.created_by}{/if}</p>
                <div class="mt-auto pt-3">
                  <Button variant="secondary" size="sm" class="w-full" loading={templateBusy && selectedTemplateId === template.id}
                    disabled={templateBusy && selectedTemplateId !== template.id}
                    onclick={() => useTemplateFromChooser(template.id)}>
                    Gunakan template
                  </Button>
                </div>
              </article>
            {/each}
          </div>
        {:else}
          <div class="flex min-h-40 flex-col items-center justify-center rounded-2xl border border-dashed border-line px-5 py-8 text-center">
            <Layers size={24} class="text-ink-muted" />
            <p class="mt-2 text-xs font-semibold text-ink">Belum ada template desain</p>
            <p class="mt-1 max-w-sm text-[11px] text-ink-muted">Mulai dari blank, lalu simpan desain sebagai template agar bisa dipakai pada periode berikutnya.</p>
          </div>
        {/if}
      </div>
    </div>

    <div class="bg-stone-50/60 px-5 py-3 text-[11px] leading-relaxed text-ink-muted">
      Pilihan ini hanya muncul selama desain periode belum pernah disimpan. Setelah disimpan, tab Desain akan langsung menampilkan kanvas terakhir.
    </div>
  </section>
{:else}
  <section class="card mb-4 flex flex-wrap items-center justify-between gap-3 px-5 py-4">
    <div>
      <div class="flex flex-wrap items-center gap-2">
        <span class="eyebrow">EDITOR DESAIN</span>
        {#if saving}
          <span class="badge badge-progress">Menyimpan…</span>
        {:else if autosaveState === 'error' && dirty}
          <span class="badge badge-danger">Gagal autosave</span>
        {:else if dirty}
          <span class="badge badge-progress">Belum disimpan</span>
        {:else if lastSaveMode === 'auto'}
          <span class="badge badge-success"><Check size={11} /> Tersimpan otomatis</span>
        {:else}
          <span class="badge badge-success"><Check size={11} /> Tersimpan</span>
        {/if}
      </div>
      {#if autosaveState === 'error' && dirty && autosaveError}
        <p class="mt-1 max-w-2xl text-xs text-red-700">Auto-save gagal: {autosaveError}</p>
      {/if}
      {#if updatedAt}
        <p class="mt-1 text-xs text-ink-muted">
          Terakhir disimpan {formatDateTime(updatedAt)}{#if updatedBy} oleh {updatedBy}{/if}
        </p>
      {:else}
        <p class="mt-1 text-xs text-ink-muted">Belum pernah disimpan di server</p>
      {/if}
      {#if dirty && localDraftAt}
        <p class="mt-1 text-xs text-ink-muted">Draft lokal tersimpan {formatDateTime(localDraftAt)}</p>
      {/if}
      {#if draftUnavailable && dirty}
        <p class="mt-1 max-w-2xl text-xs text-amber-700">
          Draft lokal tidak tersedia (penyimpanan browser penuh atau desain terlalu besar).
        </p>
      {/if}
    </div>
    <div class="flex flex-wrap items-center gap-2">
      {#if autosaveState === 'error' && dirty}
        <Button variant="secondary" size="sm" loading={saving} onclick={() => autosaveNow()}><RefreshCw size={14} /> Coba lagi</Button>
        {#if localDraftAt}
          <Button variant="ghost" size="sm" onclick={discardLocalDraft}><Trash2 size={14} /> Buang draft</Button>
        {/if}
      {/if}
      <Button variant="secondary" size="sm" loading={previewing} onclick={preview}><Eye size={14} /> Pratinjau</Button>
      <Button variant="secondary" size="sm" loading={saving} onclick={() => save()}><Save size={14} /> Simpan</Button>
      <Button size="sm" loading={generating} disabled={sharing} onclick={generate}><FileDown size={14} /> Buat PDF</Button>
      <Button variant="secondary" size="sm" loading={sharing} disabled={generating} onclick={sharePresentation}><Share2 size={14} /> Bagikan & presentasi</Button>
    </div>
  </section>

  {#if error}
    <div class="mb-4 flex items-start gap-2.5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertCircle size={17} class="mt-0.5 shrink-0" /> <span>{error}</span>
    </div>
  {/if}

  <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px] xl:grid-cols-[minmax(0,1fr)_340px]">
    <section class="card overflow-hidden">
      <div class="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2.5">
        <Button variant="secondary" size="sm" onclick={() => (textPickerOpen = true)}><Type size={14} /> Teks</Button>
        <Button variant="secondary" size="sm" onclick={openShapePicker}><Star size={14} /> Bentuk</Button>
        <Button variant="secondary" size="sm" onclick={openMediaLibrary}><ImagePlus size={14} /> Gambar</Button>
        <Button variant="secondary" size="sm" onclick={addImageUrl}><Link2 size={14} /> URL</Button>
        <Button variant="secondary" size="sm" onclick={addTable}><Table2 size={14} /> Tabel</Button>
        <Button variant="secondary" size="sm" onclick={() => (chartPickerOpen = true)}><BarChart3 size={14} /> Grafik</Button>
        <Button variant="secondary" size="sm" onclick={addCard}><CreditCard size={14} /> Kartu</Button>
        <Button variant="secondary" size="sm" onclick={openIconPickerForNew}><Shapes size={14} /> Ikon</Button>
        <span class="mx-1 h-6 w-px bg-line"></span>
        <Button variant="ghost" size="sm" disabled={!history.length} onclick={undo}><Undo2 size={14} /> Undo</Button>
        <Button variant="ghost" size="sm" disabled={!redoStack.length} onclick={redo}><Redo2 size={14} /> Redo</Button>
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

      <div bind:this={canvasAreaEl} data-canvas-area class="h-[calc(100vh-230px)] min-h-[480px] overflow-auto bg-stone-100" onscroll={closeContextMenu}>
        <div class="flex min-h-full w-max min-w-full items-center justify-center p-3">
          {#if loading}
            <div class="text-sm text-ink-muted">Menyiapkan kanvas laporan…</div>
          {:else}
            <div class="relative">
              <iframe
                bind:this={iframeEl}
                title="Editor tata letak laporan"
                class="block rounded-lg border border-line bg-white"
                style={`width:${canvasWidth}px;height:${canvasHeight}px`}
                srcdoc={iframeHtml}
                onload={handleIframeLoad}
              ></iframe>
              {#if cakupanDragActive}
                <button
                  type="button"
                  class="absolute inset-0 z-20 flex items-start justify-center rounded-lg border-2 border-dashed border-gold-400 bg-gold-50/40 p-3"
                  data-cakupan-drop
                  aria-label="Lepas data cakupan di kanvas"
                  ondragover={(event) => {
                    event.preventDefault()
                    if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy'
                  }}
                  ondrop={onCakupanDrop}
                >
                  <span class="rounded-full bg-white/95 px-3 py-1.5 text-[11px] font-semibold text-forest-800 shadow">
                    Lepas untuk menambahkan data cakupan
                  </span>
                </button>
              {/if}
            </div>
          {/if}
        </div>
      </div>

      <div class="border-t border-line bg-stone-50/60 px-4 py-2.5 text-[10px] leading-relaxed text-ink-muted">
        Satu halaman ditampilkan pada satu waktu. Gunakan Sebelumnya/Berikutnya atau Page Up/Page Down untuk berpindah halaman.
        Tombol −/+ atau Ctrl +/− untuk zoom, dan Pas untuk kembali menampilkan satu halaman penuh. Klik kanan pada objek atau kanvas untuk menu layer, sembunyikan, kunci, salin, dan tempel. Perubahan baru masuk ke PDF setelah disimpan.
      </div>
    </section>

    <aside bind:this={asideEl} class="space-y-4 self-start lg:sticky lg:top-24 lg:max-h-[calc(100vh-7rem)] lg:overflow-y-auto lg:pr-1 max-lg:fixed max-lg:inset-y-20 max-lg:right-3 max-lg:z-40 max-lg:w-[min(92vw,340px)] max-lg:overflow-y-auto max-lg:rounded-2xl max-lg:bg-canvas max-lg:p-2 max-lg:shadow-2xl {panelOpen ? '' : 'max-lg:hidden'}">
      <button class="btn btn-secondary btn-sm w-full lg:hidden" onclick={() => (panelOpen = false)}>Tutup panel</button>

      <nav
        class="sticky top-0 z-20 -mx-1 grid grid-cols-4 gap-0.5 rounded-xl border border-line bg-white/95 p-1 shadow-sm backdrop-blur"
        aria-label="Panel editor desain"
      >
        <button
          type="button"
          class="flex flex-col items-center gap-1 rounded-lg px-1 py-2 text-[10px] font-semibold transition-colors {sidebarTab === 'properties' ? 'bg-forest-800 text-white shadow-sm' : 'text-ink-soft hover:bg-stone-100'}"
          aria-pressed={sidebarTab === 'properties'}
          onclick={() => switchSidebarTab('properties')}
        >
          <SlidersHorizontal size={15} /> Properti
        </button>
        <button
          type="button"
          class="flex flex-col items-center gap-1 rounded-lg px-1 py-2 text-[10px] font-semibold transition-colors {sidebarTab === 'layers' ? 'bg-forest-800 text-white shadow-sm' : 'text-ink-soft hover:bg-stone-100'}"
          aria-pressed={sidebarTab === 'layers'}
          onclick={() => switchSidebarTab('layers')}
        >
          <Layers size={15} /> Layer
        </button>
        <button
          type="button"
          class="flex flex-col items-center gap-1 rounded-lg px-1 py-2 text-[10px] font-semibold transition-colors {sidebarTab === 'coverage' ? 'bg-forest-800 text-white shadow-sm' : 'text-ink-soft hover:bg-stone-100'}"
          aria-pressed={sidebarTab === 'coverage'}
          onclick={() => switchSidebarTab('coverage')}
        >
          <BarChart3 size={15} /> Cakupan
        </button>
        <button
          type="button"
          class="flex flex-col items-center gap-1 rounded-lg px-1 py-2 text-[10px] font-semibold transition-colors {sidebarTab === 'page' ? 'bg-forest-800 text-white shadow-sm' : 'text-ink-soft hover:bg-stone-100'}"
          aria-pressed={sidebarTab === 'page'}
          onclick={() => switchSidebarTab('page')}
        >
          <FileText size={15} /> Halaman
        </button>
      </nav>

      {#if sidebarTab === 'properties'}
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
              {#if activeElement.locked}<span class="badge badge-progress">Terkunci</span>{/if}
              {#if activeElement.binding}<span class="badge badge-success">Data</span>{/if}
              {#if activeElement.source_block}<span class="badge badge-progress">Hasil detach</span>{/if}
            </div>
          </div>

          <div class="space-y-4 p-5">
            <div class="grid grid-cols-2 gap-3">
              <label class="block">
                <span class="label">X (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.x}
                  disabled={activeElement.locked}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ x: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Y (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.y}
                  disabled={activeElement.locked}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ y: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Lebar (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.width}
                  disabled={activeElement.locked}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ width: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Tinggi (mm)</span>
                <input class="input py-1.5 text-xs" type="number" step="0.5" value={activeElement.height}
                  disabled={activeElement.locked}
                  onfocus={pushHistory}
                  oninput={(event) => updateElement({ height: numberValue((event.currentTarget as HTMLInputElement).value) })} />
              </label>
              <label class="block">
                <span class="label">Rotasi (°)</span>
                <input class="input py-1.5 text-xs" type="number" step="1" value={activeElement.rotation}
                  disabled={activeElement.locked}
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
              <div>
                <span class="label">Template kartu</span>
                <div class="mt-1.5">
                  <CardTemplatePicker value={activeElement.card_template ?? 'classic'} onselect={applyCardTemplate} />
                </div>
              </div>
              <div class="rounded-xl border border-forest-200 bg-forest-50/70 p-3.5">
                <p class="text-[11px] leading-relaxed text-forest-800">
                  Atur font, ukuran, dan warna label, nilai, unit, dan detail secara terpisah lewat editor kartu.
                </p>
                <Button variant="primary" size="sm" class="mt-2.5 w-full" onclick={openCardEditor}>
                  <SlidersHorizontal size={14} /> Buka editor kartu
                </Button>
              </div>
              {#if activeCardTemplate.layout !== 'none'}
                <div class="rounded-xl border border-line p-3">
                  <div class="flex items-center gap-3">
                    <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full"
                      style:background={activeElement.card_icon_bg ?? activeCardTemplate.iconBg}
                      style:color={activeElement.card_icon_color ?? activeCardTemplate.iconColor}>
                      {#if activeCardIcon}
                        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{@html activeCardIcon.svg}</svg>
                      {:else}
                        <Shapes size={15} />
                      {/if}
                    </span>
                    <div class="min-w-0 flex-1">
                      <p class="truncate text-xs font-semibold text-ink">{activeCardIcon?.label ?? 'Belum ada ikon'}</p>
                      <p class="truncate text-[10px] text-ink-muted">{activeElement.card_icon || 'Pilih ikon untuk template ini.'}</p>
                    </div>
                    <Button variant="secondary" size="sm" onclick={openCardIconPicker}>{activeElement.card_icon ? 'Ganti' : 'Pilih'}</Button>
                    {#if activeElement.card_icon}
                      <button class="btn btn-ghost btn-sm" onclick={() => updateElement({ card_icon: '' }, true)}>Hapus</button>
                    {/if}
                  </div>
                  <div class="mt-3 grid grid-cols-2 gap-3">
                    <label class="block">
                      <span class="label">Warna ikon</span>
                      <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.card_icon_color ?? activeCardTemplate.iconColor)}
                        onfocus={pushHistory}
                        oninput={(event) => updateElement({ card_icon_color: (event.currentTarget as HTMLInputElement).value })} />
                    </label>
                    <label class="block">
                      <span class="label">Latar ikon</span>
                      <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.card_icon_bg ?? activeCardTemplate.iconBg)}
                        onfocus={pushHistory}
                        oninput={(event) => updateElement({ card_icon_bg: (event.currentTarget as HTMLInputElement).value })} />
                    </label>
                  </div>
                  <div class="mt-3">
                    <div class="flex items-center justify-between gap-2">
                      <span class="label mb-0">Skala ikon</span>
                      <span class="text-[11px] font-semibold text-forest-800">{Math.round(activeElement.card_icon_scale ?? 100)}%</span>
                    </div>
                    <input class="mt-1 w-full accent-forest-700" type="range" min="30" max="300" step="5"
                      value={activeElement.card_icon_scale ?? 100}
                      aria-label="Skala ikon kartu"
                      onfocus={pushHistory}
                      oninput={(event) => updateElement({ card_icon_scale: numberValue((event.currentTarget as HTMLInputElement).value) })} />
                    <div class="flex justify-between text-[9px] text-ink-muted"><span>Kecil</span><span>Besar</span></div>
                  </div>
                </div>
              {/if}
              <label class="block">
                <span class="label">Sumber data</span>
                <select class="input py-1.5 text-xs" value={activeElement.binding ?? ''}
                  data-card-source
                  onchange={(event) => bindSource((event.currentTarget as HTMLSelectElement).value)}>
                  <option value="">Manual — tidak terhubung</option>
                  {#each cardSourceGroups as group}
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
                    Nilai kartu mengikuti data Ringkasan/Analisis terbaru dan tidak bisa diedit manual.
                  </p>
                  <p class="mt-2 text-forest-700">
                    {sourceFor(activeElement.binding)?.card_label ?? ''}:
                    <strong>{sourceFor(activeElement.binding)?.card_value ?? '—'}</strong>
                    {sourceFor(activeElement.binding)?.card_unit ?? ''}
                  </p>
                  <div class="mt-3 flex flex-wrap gap-2">
                    <Button variant="secondary" size="sm" onclick={() => bindSource('')}>Lepas ikatan</Button>
                    <Button variant="ghost" size="sm" onclick={() => void refreshSources()}>Muat ulang data</Button>
                  </div>
                </div>
              {:else}
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
              {/if}
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
              <div class="rounded-xl border border-forest-200 bg-forest-50/70 p-3.5">
                <p class="text-[11px] leading-relaxed text-forest-800">
                  Atur isi tabel, header, font, dan warna latar/teks tiap kolom secara terpisah lewat editor tabel.
                </p>
                <Button variant="primary" size="sm" class="mt-2.5 w-full" onclick={openTableEditor}>
                  <SlidersHorizontal size={14} /> Buka editor tabel
                </Button>
              </div>
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
                  <input class="input h-9 py-1" type="color" value={pickerColor(activeElement.table_stripe, '#f6f8f5')}
                    onfocus={pushHistory}
                    oninput={(event) => updateElement({ table_stripe: (event.currentTarget as HTMLInputElement).value })} />
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
              <div class="rounded-xl border border-forest-200 bg-forest-50/70 p-3.5">
                <p class="text-[11px] leading-relaxed text-forest-800">
                  Atur font judul, wrap judul, ukuran &amp; sudut label, font nilai, warna seri, serta legend lewat editor grafik.
                </p>
                <Button variant="primary" size="sm" class="mt-2.5 w-full" onclick={openChartEditor}>
                  <SlidersHorizontal size={14} /> Buka editor grafik
                </Button>
              </div>
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
              <Button variant="secondary" size="sm" class="w-full" onclick={openMediaReplace}>
                <ImagePlus size={14} /> Pilih dari media library
              </Button>
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
              <Button variant="secondary" size="sm" onclick={toggleSelectedLocked}>
                {#if activeElement.locked}<LockOpen size={14} /> Buka kunci{:else}<Lock size={14} /> Kunci posisi{/if}
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
            <li>3. Tarik data dari panel Cakupan ke kanvas, lalu pilih grafik, tabel, atau kartu otomatis.</li>
            <li>4. Tarik elemen untuk menggeser; tarik sudut untuk mengubah ukuran.</li>
            <li>5. Klik blok laporan untuk geser; tarik sudut kanan bawah untuk resize.</li>
            <li>6. Klik dua kali teks blok untuk mengeditnya.</li>
            <li>7. Gunakan panel Layer untuk menyembunyikan, mengunci posisi, mengurutkan, atau menggabungkan objek.</li>
            <li>8. Simpan desain sebagai template untuk dipakai pada periode lain.</li>
            <li>9. Simpan, lalu Buat PDF.</li>
          </ul>
        </section>
      {/if}
      {/if}

      {#snippet cakupanUnits(options: {
        key: string
        scope: 'polres' | 'satker'
        units: CoverageUnitCount[]
        node: CakupanNode
      })}
        <div class="ml-6 flex items-center gap-1 rounded-lg border border-line bg-stone-50/70 px-1.5 py-1.5">
          <button type="button" class="rounded p-0.5 text-ink-muted hover:bg-stone-100"
            onclick={() => toggleCakupan(options.key)}>
            {#if isCakupanExpanded(options.key)}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
          </button>
          <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing" onclick={() => toggleCakupan(options.key)}
            draggable="true"
            ondragstart={(event) => startCakupanDrag(event, options.node)}
            ondragend={endCakupanDrag}>
            <span class="block truncate text-[11px] font-medium text-ink">{options.scope === 'polres' ? 'Polres' : 'Satker Polda'}</span>
            <span class="block text-[10px] text-ink-muted">
              {options.units.length} unit · {formatNumber(options.units.reduce((total, unit) => total + unit.rows, 0))} kegiatan
            </span>
          </button>
          <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
            onclick={() => openCakupanDialog(options.node)}>
            <Plus size={12} />
          </button>
        </div>
        {#if isCakupanExpanded(options.key)}
          {#each options.units as unit (unit.label)}
            <div class="ml-10 flex items-center justify-between gap-2 px-2 py-0.5 text-[10px] text-ink-muted">
              <span class="min-w-0 truncate">{unit.label}</span>
              <span class="shrink-0 font-semibold text-ink">{formatNumber(unit.rows)}</span>
            </div>
          {/each}
          {#if !options.units.length}
            <p class="ml-10 px-2 py-0.5 text-[10px] text-ink-muted">Belum ada unit.</p>
          {/if}
        {/if}
      {/snippet}

      {#snippet cakupanProgramUnits(options: {
        key: string
        scope: 'polres' | 'satker'
        units: CoverageUnit[]
        program: string
      })}
        <div class="ml-6 flex items-center gap-1 rounded-lg border border-line bg-stone-50/70 px-1.5 py-1.5">
          <button type="button" class="rounded p-0.5 text-ink-muted hover:bg-stone-100"
            onclick={() => toggleCakupan(options.key)}>
            {#if isCakupanExpanded(options.key)}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
          </button>
          <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing"
            onclick={() => toggleCakupan(options.key)}
            draggable="true"
            ondragstart={(event) => startCakupanDrag(event, { kind: 'units', scope: options.scope, program: options.program })}
            ondragend={endCakupanDrag}>
            <span class="block truncate text-[11px] font-medium text-ink">{options.scope === 'polres' ? 'Polres' : 'Satker Polda'}</span>
            <span class="block text-[10px] text-ink-muted">
              {options.units.length} unit · {formatNumber(options.units.reduce((total, unit) => total + unit.rows, 0))} kegiatan
            </span>
          </button>
          <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
            onclick={() => openCakupanDialog({ kind: 'units', scope: options.scope, program: options.program })}>
            <Plus size={12} />
          </button>
        </div>
        {#if isCakupanExpanded(options.key)}
          {#each options.units as unit (unit.label)}
            <div class="ml-10 flex items-center justify-between gap-2 px-2 py-0.5 text-[10px] text-ink-muted">
              <span class="min-w-0 truncate">{unit.label}</span>
              <span class="shrink-0 font-semibold text-ink">{formatNumber(unit.rows)}</span>
              {#if unit.missing.length}
                <span class="shrink-0 rounded bg-red-50 px-1 py-0.5 text-[9px] font-bold text-red-700">{unit.missing.length} kosong</span>
              {/if}
            </div>
          {/each}
          {#if !options.units.length}
            <p class="ml-10 px-2 py-0.5 text-[10px] text-ink-muted">Belum ada unit.</p>
          {/if}
        {/if}
      {/snippet}

      {#if sidebarTab === 'coverage'}
      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow">CAKUPAN</span>
            <h2 class="panel-title mt-1">Tarik data ke kanvas</h2>
          </div>
          <span class="badge badge-neutral">{cakupanGroups.length} QW</span>
        </div>
        <div class="space-y-2 p-3">
          <p class="text-[10px] leading-relaxed text-ink-muted">
            Tarik kartu cakupan, kelompok, sub-kode, polres, atau satker ke kanvas lalu pilih grafik, tabel, atau kartu. Tombol + menambahkan tanpa drag.
          </p>
          {#if !hasImport}
            <p class="text-xs text-ink-muted">Impor data terlebih dahulu untuk melihat cakupan.</p>
          {:else if !coverage}
            <p class="text-xs text-ink-muted">Cakupan belum tersedia. Coba muat ulang data dari panel elemen.</p>
          {:else}
            <div class="max-h-[420px] space-y-1 overflow-auto pr-1" data-cakupan-tree>
              <div class="flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                <button type="button" class="rounded p-0.5 text-ink-muted hover:bg-stone-100"
                  onclick={() => toggleCakupan('cards')}>
                  {#if isCakupanExpanded('cards')}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
                </button>
                <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing" onclick={() => toggleCakupan('cards')}
                  draggable="true"
                  data-cakupan-node="cards"
                  ondragstart={(event) => startCakupanDrag(event, { kind: 'cards' })}
                  ondragend={endCakupanDrag}>
                  <span class="block truncate text-[11px] font-semibold text-ink">Kartu cakupan Quick Wins</span>
                  <span class="block text-[10px] text-ink-muted">
                    {cakupanCardTotal} kartu · {cakupanCards.length} sub-kode
                  </span>
                </button>
                <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                  onclick={() => openCakupanDialog({ kind: 'cards' })}>
                  <Plus size={12} />
                </button>
              </div>
              {#if isCakupanExpanded('cards')}
                {#each cakupanCardGroups as group (group.code)}
                  <div class="ml-2 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                    <button type="button" class="rounded p-0.5 text-ink-muted hover:bg-stone-100"
                      onclick={() => toggleCakupan(`cardGroup:${group.code}`)}>
                      {#if isCakupanExpanded(`cardGroup:${group.code}`)}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
                    </button>
                    <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing"
                      onclick={() => toggleCakupan(`cardGroup:${group.code}`)}
                      draggable="true"
                      data-cakupan-node={`cardGroup:${group.code}`}
                      ondragstart={(event) => startCakupanDrag(event, { kind: 'cardGroup', group: group.code })}
                      ondragend={endCakupanDrag}>
                      <span class="block truncate text-[11px] font-semibold text-ink">{group.code} · {group.title}</span>
                      <span class="block text-[10px] text-ink-muted">
                        {group.cards.length} sub-kode · {group.cards.reduce((total, card) => total + card.metrics.length + card.breakdowns.length, 0)} kartu
                      </span>
                    </button>
                    <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                      onclick={() => openCakupanDialog({ kind: 'cardGroup', group: group.code })}>
                      <Plus size={12} />
                    </button>
                  </div>
                  {#if isCakupanExpanded(`cardGroup:${group.code}`)}
                    {#each group.cards as card (card.code)}
                      <div class="ml-4 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                        <button type="button" class="rounded p-0.5 text-ink-muted hover:bg-stone-100"
                          onclick={() => toggleCakupan(`card:${card.code}`)}>
                          {#if isCakupanExpanded(`card:${card.code}`)}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
                        </button>
                        <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing"
                          onclick={() => toggleCakupan(`card:${card.code}`)}
                          draggable="true"
                          data-cakupan-node={`card:${card.code}`}
                          ondragstart={(event) => startCakupanDrag(event, { kind: 'card', code: card.code })}
                          ondragend={endCakupanDrag}>
                          <span class="block truncate text-[11px] font-medium text-ink">{card.code} · {card.title}</span>
                          <span class="block text-[10px] text-ink-muted">
                            {card.metrics.length} kartu · {card.breakdowns.length} rincian
                          </span>
                        </button>
                        <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                          onclick={() => openCakupanDialog({ kind: 'card', code: card.code })}>
                          <Plus size={12} />
                        </button>
                      </div>
                      {#if isCakupanExpanded(`card:${card.code}`)}
                        {#each card.metrics as metric (metric.key)}
                          <div class="ml-6 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                            <span class="flex w-4 shrink-0 justify-center"><span class="h-1.5 w-1.5 rounded-full bg-forest-400"></span></span>
                            <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing"
                              draggable="true"
                              data-cakupan-node={`cardMetric:${card.code}:${metric.key}`}
                              ondragstart={(event) => startCakupanDrag(event, { kind: 'cardMetric', code: card.code, metric: metric.key })}
                              ondragend={endCakupanDrag}>
                              <span class="block truncate text-[11px] font-medium text-ink">{metric.label}</span>
                              <span class="block text-[10px] text-ink-muted">
                                {cakupanValue(metric.total, metric.unit)} {metric.unit} · {metric.filled_rows}/{metric.filled_rows + metric.missing_rows} laporan
                              </span>
                            </button>
                            <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                              onclick={() => openCakupanDialog({ kind: 'cardMetric', code: card.code, metric: metric.key })}>
                              <Plus size={12} />
                            </button>
                          </div>
                        {/each}
                        {#each card.breakdowns as breakdown (breakdown.key)}
                          <div class="ml-6 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                            <span class="flex w-4 shrink-0 justify-center"><span class="h-1.5 w-1.5 rounded-full bg-gold-400"></span></span>
                            <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing"
                              draggable="true"
                              data-cakupan-node={`cardBreakdown:${card.code}:${breakdown.key}`}
                              ondragstart={(event) => startCakupanDrag(event, { kind: 'cardBreakdown', code: card.code, breakdown: breakdown.key })}
                              ondragend={endCakupanDrag}>
                              <span class="block truncate text-[11px] font-medium text-ink">{breakdown.label}</span>
                              <span class="block text-[10px] text-ink-muted">
                                {breakdown.values.length} kategori · {breakdown.filled_rows} laporan
                              </span>
                            </button>
                            <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                              onclick={() => openCakupanDialog({ kind: 'cardBreakdown', code: card.code, breakdown: breakdown.key })}>
                              <Plus size={12} />
                            </button>
                          </div>
                        {/each}
                        {#each card.tables as table (table.key)}
                          <div class="ml-6 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                            <span class="flex w-4 shrink-0 justify-center"><span class="h-1.5 w-1.5 rounded-full bg-forest-700"></span></span>
                            <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing"
                              draggable="true"
                              data-cakupan-node={`cardTable:${card.code}:${table.key}`}
                              ondragstart={(event) => startCakupanDrag(event, { kind: 'cardTable', code: card.code, table: table.key })}
                              ondragend={endCakupanDrag}>
                              <span class="block truncate text-[11px] font-medium text-ink">{table.label}</span>
                              <span class="block text-[10px] text-ink-muted">
                                {table.rows.length} polres · {table.columns.length - 1} jenis
                              </span>
                            </button>
                            <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                              onclick={() => openCakupanDialog({ kind: 'cardTable', code: card.code, table: table.key })}>
                              <Plus size={12} />
                            </button>
                          </div>
                          <div class="ml-8 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                            <span class="flex w-4 shrink-0 justify-center">
                              <span class="h-1.5 w-1.5 rounded-full bg-forest-700"></span>
                            </span>
                            <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing"
                              draggable="true"
                              data-cakupan-node={`cardTableSeries:${card.code}:${table.key}`}
                              ondragstart={(event) => startCakupanDrag(event, { kind: 'cardTableSeries', code: card.code, table: table.key })}
                              ondragend={endCakupanDrag}>
                              <span class="block truncate text-[11px] font-medium text-ink">Grafik {table.label}</span>
                              <span class="block text-[10px] text-ink-muted">{table.rows.length} polres · {table.columns.length - 1} seri</span>
                            </button>
                            <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                              onclick={() => openCakupanDialog({ kind: 'cardTableSeries', code: card.code, table: table.key })}>
                              <Plus size={12} />
                            </button>
                          </div>
                          {#each table.columns.slice(1) as column, columnIndex (column)}
                            <div class="ml-10 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                              <span class="flex w-4 shrink-0 justify-center"><span class="h-1.5 w-1.5 rounded-full bg-gold-400"></span></span>
                              <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing"
                                draggable="true"
                                data-cakupan-node={`cardTableColumn:${card.code}:${table.key}:${columnIndex + 1}`}
                                ondragstart={(event) => startCakupanDrag(event, { kind: 'cardTableColumn', code: card.code, table: table.key, column: columnIndex + 1 })}
                                ondragend={endCakupanDrag}>
                                <span class="block truncate text-[11px] font-medium text-ink">Grafik {column} per polres</span>
                                <span class="block text-[10px] text-ink-muted">1 seri · {table.rows.length} polres</span>
                              </button>
                              <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                                onclick={() => openCakupanDialog({ kind: 'cardTableColumn', code: card.code, table: table.key, column: columnIndex + 1 })}>
                                <Plus size={12} />
                              </button>
                            </div>
                          {/each}
                        {/each}
                      {/if}
                    {/each}
                  {/if}
                {/each}
              {/if}

              <div class="mt-2 border-t border-line pt-2"></div>
              <p class="px-1 pb-1 text-[10px] font-bold tracking-[0.12em] text-ink-muted uppercase">Pelaksanaan per kelompok</p>
              <div class="flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                <button type="button" class="rounded p-0.5 text-ink-muted hover:bg-stone-100"
                  onclick={() => toggleCakupan('groups')}>
                  {#if isCakupanExpanded('groups')}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
                </button>
                <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing" onclick={() => toggleCakupan('groups')}
                  draggable="true"
                  data-cakupan-node="groups"
                  ondragstart={(event) => startCakupanDrag(event, { kind: 'groups' })}
                  ondragend={endCakupanDrag}>
                  <span class="block truncate text-[11px] font-semibold text-ink">Semua kelompok</span>
                  <span class="block text-[10px] text-ink-muted">
                    {formatNumber(cakupanTotalRows)} kegiatan · {cakupanGroups.length} kelompok
                  </span>
                </button>
                <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                  onclick={() => openCakupanDialog({ kind: 'groups' })}>
                  <Plus size={12} />
                </button>
              </div>
              {#if isCakupanExpanded('groups')}
                {#each cakupanGroups as group (group.code)}
                  <div class="ml-2 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                    <button type="button" class="rounded p-0.5 text-ink-muted hover:bg-stone-100"
                      onclick={() => toggleCakupan(`group:${group.code}`)}>
                      {#if isCakupanExpanded(`group:${group.code}`)}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
                    </button>
                    <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing" onclick={() => toggleCakupan(`group:${group.code}`)}
                      draggable="true"
                      data-cakupan-node={group.code}
                      ondragstart={(event) => startCakupanDrag(event, { kind: 'group', code: group.code })}
                      ondragend={endCakupanDrag}>
                      <span class="block truncate text-[11px] font-semibold text-ink">{group.code} · {group.title}</span>
                      <span class="block text-[10px] text-ink-muted">
                        {formatNumber(group.rows)} kegiatan · {group.polres} polres · {group.satker} satker
                      </span>
                    </button>
                    <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                      onclick={() => openCakupanDialog({ kind: 'group', code: group.code })}>
                      <Plus size={12} />
                    </button>
                  </div>
                  {#if isCakupanExpanded(`group:${group.code}`)}
                    {#each group.codes as code (code.code)}
                      <div class="ml-4 flex items-center gap-1 rounded-lg border border-line bg-white px-1.5 py-1.5">
                        <button type="button" class="rounded p-0.5 text-ink-muted hover:bg-stone-100"
                          onclick={() => toggleCakupan(`code:${code.code}`)}>
                          {#if isCakupanExpanded(`code:${code.code}`)}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
                        </button>
                        <button type="button" class="min-w-0 flex-1 cursor-grab text-left active:cursor-grabbing" onclick={() => toggleCakupan(`code:${code.code}`)}
                          draggable="true"
                          data-cakupan-node={code.code}
                          ondragstart={(event) => startCakupanDrag(event, { kind: 'code', code: code.code })}
                          ondragend={endCakupanDrag}>
                          <span class="block truncate text-[11px] font-medium text-ink">{code.code} · {code.title}</span>
                          <span class="block text-[10px] text-ink-muted">
                            {formatNumber(code.rows)} kegiatan · {code.polres} polres · {code.satker} satker
                          </span>
                        </button>
                        <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title="Tambahkan ke kanvas"
                          onclick={() => openCakupanDialog({ kind: 'code', code: code.code })}>
                          <Plus size={12} />
                        </button>
                      </div>
                      {#if isCakupanExpanded(`code:${code.code}`)}
                        {@render cakupanUnits({
                          key: `code-polres:${code.code}`,
                          scope: 'polres',
                          units: code.by_polres,
                          node: { kind: 'units', scope: 'polres', code: code.code },
                        })}
                        {@render cakupanUnits({
                          key: `code-satker:${code.code}`,
                          scope: 'satker',
                          units: code.by_satker,
                          node: { kind: 'units', scope: 'satker', code: code.code },
                        })}
                      {/if}
                    {/each}
                    {@render cakupanUnits({
                      key: `group-polres:${group.code}`,
                      scope: 'polres',
                      units: group.by_polres,
                      node: { kind: 'units', scope: 'polres', group: group.code },
                    })}
                    {@render cakupanUnits({
                      key: `group-satker:${group.code}`,
                      scope: 'satker',
                      units: group.by_satker,
                      node: { kind: 'units', scope: 'satker', group: group.code },
                    })}
                  {/if}
                {/each}
              {/if}
            </div>
          {/if}
        </div>
      </section>
      {/if}

      {#if sidebarTab === 'page'}
      <section class="card overflow-hidden">
        <div class="panel-heading">
          <div>
            <span class="eyebrow">LATAR HALAMAN</span>
            <h2 class="panel-title mt-1">Halaman {currentPage}</h2>
          </div>
          <Shapes size={18} class="text-gold-500" />
        </div>
        <div class="space-y-3 p-4">
          <label class="block">
            <span class="label">Tipe latar</span>
            <select class="input py-1.5 text-xs" value={pageBackgroundMode}
              data-page-background-mode
              onchange={(event) => setPageBackgroundMode((event.currentTarget as HTMLSelectElement).value as 'solid' | 'linear' | 'radial' | 'image')}>
              <option value="solid">Warna solid</option>
              <option value="linear">Gradien linear</option>
              <option value="radial">Gradien radial</option>
              <option value="image">Gambar dari media library</option>
            </select>
          </label>
          {#if pageBackgroundMode === 'solid'}
            <label class="block">
              <span class="label">Warna latar</span>
              <input class="input h-9 py-1" type="color" value={pickerColor(activePage?.background, '#ffffff')}
                data-page-background
                onfocus={pushHistory}
                oninput={(event) => setPageBackgroundColor((event.currentTarget as HTMLInputElement).value)} />
            </label>
          {:else if pageBackgroundMode === 'image'}
            <div class="space-y-2.5 rounded-xl border border-line p-3">
              {#if pageBackgroundImageSrc(activePage)}
                <img src={pageBackgroundImageSrc(activePage)} alt="Latar halaman"
                  class="h-28 w-full rounded-md border border-line object-cover"
                  data-page-background-preview />
              {:else}
                <div class="flex h-28 items-center justify-center rounded-md border border-dashed border-line text-[11px] text-ink-muted">
                  Belum ada gambar latar
                </div>
              {/if}
              <div class="flex flex-wrap gap-2">
                <Button variant="secondary" size="sm" onclick={openPageBackgroundMedia}>
                  <ImagePlus size={13} /> {pageBackgroundImageSrc(activePage) ? 'Ganti gambar' : 'Pilih dari media library'}
                </Button>
                {#if pageBackgroundImageSrc(activePage)}
                  <Button variant="ghost" size="sm" onclick={clearPageBackgroundImage}>Hapus gambar</Button>
                {/if}
              </div>
              <p class="text-[10px] leading-relaxed text-ink-muted">
                Gambar menutupi seluruh halaman (cover), mengikuti ukuran dan orientasi halaman, serta tidak bisa digeser. Pilih dari media library agar gambar ikut tersimpan di layout dan template.
              </p>
            </div>
          {:else}
            <div class="space-y-2.5 rounded-xl border border-line p-3">
              <span class="block h-9 rounded-md border border-line" data-page-gradient-preview
                style={`background:${pageBackgroundCss(activePage)}`}></span>
              <div class="space-y-1.5">
                {#each pageGradientColors(activePage) as color, index (index)}
                  <div class="flex items-center gap-2">
                    <input class="input h-8 min-w-0 flex-1 py-1" type="color" value={pickerColor(color, '#ffffff')}
                      data-page-gradient-color={index}
                      onfocus={pushHistory}
                      oninput={(event) => setPageGradientColor(index, (event.currentTarget as HTMLInputElement).value)} />
                    <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100 disabled:opacity-30"
                      title="Hapus warna" data-page-gradient-remove={index}
                      disabled={pageGradientColors(activePage).length <= 2}
                      onclick={() => removePageGradientColor(index)}>
                      <Trash2 size={13} />
                    </button>
                  </div>
                {/each}
              </div>
              <div class="flex flex-wrap items-center gap-2">
                <Button variant="secondary" size="sm" disabled={pageGradientColors(activePage).length >= 8}
                  onclick={addPageGradientColor}>
                  <Plus size={13} /> Tambah warna
                </Button>
                {#if pageBackgroundMode === 'linear'}
                  <label class="flex items-center gap-1.5 text-[10px] text-ink-muted">Sudut
                    <input class="input w-16 py-1 text-xs" type="number" min="0" max="360" step="5"
                      value={activePage?.gradient_angle ?? 90}
                      data-page-gradient-angle
                      onfocus={pushHistory}
                      oninput={(event) => setPageGradientAngle(numberValue((event.currentTarget as HTMLInputElement).value))} />°
                  </label>
                {/if}
              </div>
              <p class="text-[10px] leading-relaxed text-ink-muted">Warna tersebar merata dari 0% sampai 100%. Bisa memakai 2–8 warna.</p>
            </div>
          {/if}
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" onclick={resetPageBackground}>Putih</Button>
            <Button variant="ghost" size="sm" onclick={applyPageBackgroundToAll}>Terapkan ke semua halaman</Button>
          </div>
          <p class="text-[10px] leading-relaxed text-ink-muted">Latar halaman tampil di kanvas, pratinjau, dan PDF. Duplikat halaman menyalin latar halaman aktif.</p>
        </div>
      </section>

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
              <span data-blank-duplicate-page><Button variant="secondary" size="sm" disabled={layout.pages.length >= MAX_BLANK_PAGES} onclick={duplicateBlankPage}><Copy size={13} /> Duplikat halaman</Button></span>
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
      {/if}

      {#if sidebarTab === 'layers'}
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
                    <span class="block text-[10px] text-ink-muted">z {element.z}{#if element.group} · grup{/if}{#if element.locked} · terkunci{/if}</span>
                  </button>
                  <button type="button" class="rounded p-1 text-ink-muted hover:bg-stone-100" title={element.hidden ? 'Tampilkan' : 'Sembunyikan'}
                    data-layer-hide={element.id}
                    onclick={() => toggleElementHidden(element.id)}>
                    {#if element.hidden}<Eye size={13} />{:else}<EyeOff size={13} />{/if}
                  </button>
                  <button type="button" class="rounded p-1 hover:bg-stone-100 {element.locked ? 'text-gold-700' : 'text-ink-muted'}" title={element.locked ? 'Buka kunci posisi' : 'Kunci posisi'}
                    data-layer-lock={element.id}
                    onclick={() => toggleElementLocked(element.id)}>
                    {#if element.locked}<Lock size={13} />{:else}<LockOpen size={13} />{/if}
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
      {/if}
    </aside>
  </div>

  <MediaLibrary
    open={mediaOpen}
    {periodId}
    onclose={() => (mediaOpen = false)}
    onuse={insertMediaImage}
  />

  <ShareReportDialog
    open={shareTarget !== null}
    report={shareTarget}
    onclose={() => (shareTarget = null)}
  />

  {#if chartEditorTarget && chartEditorElement}
    <ChartEditorDialog
      element={chartEditorElement}
      dataLocked={chartEditorLocked}
      sourceLabel={chartEditorSourceLabel}
      onclose={() => (chartEditorTarget = null)}
      onsave={saveChartEditor}
    />
  {/if}

  {#if cardEditorTarget && cardEditorElement}
    <CardEditorDialog
      element={cardEditorElement}
      dataLocked={cardEditorLocked}
      sourceLabel={cardEditorSourceLabel}
      onclose={() => (cardEditorTarget = null)}
      onsave={saveCardEditor}
    />
  {/if}

  {#if tableEditorTarget && tableEditorElement}
    <TableEditorDialog
      element={tableEditorElement}
      dataLocked={tableEditorLocked}
      sourceLabel={tableEditorSourceLabel}
      onclose={() => (tableEditorTarget = null)}
      onsave={saveTableEditor}
    />
  {/if}

  {#if contextMenu}
    <div use:portal>
      <div
        class="fixed z-[70] max-h-[calc(100vh-16px)] w-64 max-w-[calc(100vw-16px)] overflow-y-auto rounded-xl border border-line bg-white py-1.5 shadow-2xl"
        style={`left:${contextMenu.x}px;top:${contextMenu.y}px`}
        data-design-context-menu
        role="menu"
      >
        {#if contextMenu.kind === 'element' && contextElement}
          <p class="truncate px-3 pt-1 pb-1.5 text-[9px] font-bold tracking-[0.16em] text-ink-muted uppercase">{elementLabel(contextElement)}</p>
          {#if contextElement.type === 'chart' || contextElement.type === 'card' || contextElement.type === 'table'}
            <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-forest-800 hover:bg-forest-50" onclick={contextElement.type === 'chart' ? contextEditChart : contextElement.type === 'card' ? contextEditCard : contextEditTable}>
              <SlidersHorizontal size={14} class="shrink-0" /> {contextElement.type === 'chart' ? 'Edit grafik' : contextElement.type === 'card' ? 'Edit kartu' : 'Edit tabel'}
            </button>
            <div class="my-1 h-px bg-line"></div>
          {/if}
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100" onclick={() => contextMoveLayer(1)}>
            <ArrowUp size={14} class="shrink-0 text-ink-muted" /> Naikkan layer
          </button>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100" onclick={() => contextMoveLayer(-1)}>
            <ArrowDown size={14} class="shrink-0 text-ink-muted" /> Turunkan layer
          </button>
          <div class="my-1 h-px bg-line"></div>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100" onclick={contextToggleHidden}>
            {#if contextElement.hidden}<Eye size={14} class="shrink-0 text-ink-muted" /> Tampilkan objek{:else}<EyeOff size={14} class="shrink-0 text-ink-muted" /> Sembunyikan objek{/if}
          </button>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100" onclick={contextToggleLocked}>
            {#if contextElement.locked}<LockOpen size={14} class="shrink-0 text-ink-muted" /> Buka kunci posisi{:else}<Lock size={14} class="shrink-0 text-ink-muted" /> Kunci posisi{/if}
          </button>
          <div class="my-1 h-px bg-line"></div>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100" onclick={contextDuplicate}>
            <Layers size={14} class="shrink-0 text-ink-muted" /> Duplikat
          </button>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100" onclick={contextCopy}>
            <Copy size={14} class="shrink-0 text-ink-muted" /> <span class="flex-1">Salin</span><span class="text-[10px] text-ink-muted">Ctrl+C</span>
          </button>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-40" disabled={!clipboard} onclick={pasteClipboard}>
            <FilePlus2 size={14} class="shrink-0 text-ink-muted" /> <span class="flex-1">Tempel</span><span class="text-[10px] text-ink-muted">Ctrl+V</span>
          </button>
          <div class="my-1 h-px bg-line"></div>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-red-700 hover:bg-red-50" onclick={contextDelete}>
            <Trash2 size={14} class="shrink-0" /> Hapus objek
          </button>
        {:else if contextMenu.kind === 'block' && contextBlock}
          <p class="px-3 pt-1 pb-1.5 text-[9px] font-bold tracking-[0.16em] text-ink-muted uppercase">Blok laporan</p>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100" onclick={contextResetBlock}>
            <Undo2 size={14} class="shrink-0 text-ink-muted" /> Reset ukuran & gaya
          </button>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100" onclick={contextToggleBlockHidden}>
            {#if contextBlock.hidden}<Eye size={14} class="shrink-0 text-ink-muted" /> Tampilkan blok{:else}<EyeOff size={14} class="shrink-0 text-ink-muted" /> Sembunyikan blok{/if}
          </button>
          <div class="my-1 h-px bg-line"></div>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-40" disabled={!clipboard} onclick={pasteClipboard}>
            <FilePlus2 size={14} class="shrink-0 text-ink-muted" /> Tempel objek
          </button>
        {:else}
          <p class="px-3 pt-1 pb-1.5 text-[9px] font-bold tracking-[0.16em] text-ink-muted uppercase">Kanvas</p>
          <button type="button" role="menuitem" class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs font-medium text-ink hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-40" disabled={!clipboard} onclick={pasteClipboard}>
            <FilePlus2 size={14} class="shrink-0 text-ink-muted" /> <span class="flex-1">Tempel objek</span><span class="text-[10px] text-ink-muted">Ctrl+V</span>
          </button>
          {#if !clipboard}
            <p class="px-3 pt-1 pb-1.5 text-[10px] leading-relaxed text-ink-muted">Salin objek terlebih dahulu untuk menempelkannya di sini.</p>
          {/if}
        {/if}
      </div>
    </div>
  {/if}

  {#if !panelOpen}
    <button class="btn btn-primary fixed right-5 bottom-5 z-40 shadow-lift lg:hidden" onclick={() => (panelOpen = true)}>Panel</button>
  {/if}

  {#if textPickerOpen}
    <div use:portal>
      <Modal
        open={true}
        title="Tambah teks"
        description="Pilih jenis teks yang akan ditempatkan di halaman aktif. Isi dan gaya tetap bisa diedit setelah ditambahkan."
        onclose={() => (textPickerOpen = false)}
        width="max-w-2xl"
      >
        <div class="grid gap-2 sm:grid-cols-3">
          <button
            type="button"
            class="flex h-full flex-col items-start rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
            data-text-option="period"
            onclick={() => addText('period')}
          >
            <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><FileText size={17} /></span>
            <span class="mt-3 text-sm font-semibold text-ink">Judul periode</span>
            <span class="mt-1 line-clamp-2 break-words text-[10px] leading-relaxed text-ink-muted">{detail.period.label}</span>
          </button>

          <button
            type="button"
            class="flex h-full flex-col items-start rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
            data-text-option="date"
            onclick={() => addText('date')}
          >
            <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-gold-50 text-gold-700"><CalendarDays size={17} /></span>
            <span class="mt-3 text-sm font-semibold text-ink">Tanggal laporan</span>
            <span class="mt-1 line-clamp-2 break-words text-[10px] leading-relaxed text-ink-muted">{periodDateText}</span>
          </button>

          <button
            type="button"
            class="flex h-full flex-col items-start rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
            data-text-option="plain"
            onclick={() => addText('plain')}
          >
            <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-100 text-ink-soft"><Type size={17} /></span>
            <span class="mt-3 text-sm font-semibold text-ink">Teks biasa</span>
            <span class="mt-1 text-[10px] leading-relaxed text-ink-muted">Tambah teks kosong untuk diisi dan diatur secara bebas.</span>
          </button>
        </div>
      </Modal>
    </div>
  {/if}

  {#if chartPickerOpen}
    <div use:portal>
      <Modal
        open={true}
        title="Tambah grafik"
        description="Pilih jenis data grafik. Keduanya memakai editor grafik yang sama dan bisa diubah kapan saja."
        onclose={() => (chartPickerOpen = false)}
        width="max-w-2xl"
      >
        <div class="grid gap-2 sm:grid-cols-2">
          <button
            type="button"
            class="flex h-full flex-col items-start rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
            data-chart-option="single"
            onclick={() => addChart('single')}
          >
            <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><BarChart3 size={17} /></span>
            <span class="mt-3 text-sm font-semibold text-ink">Satu seri</span>
            <span class="mt-1 text-[10px] leading-relaxed text-ink-muted">
              Satu nilai per kategori. Cocok untuk perbandingan sederhana antar polres atau kegiatan.
            </span>
          </button>

          <button
            type="button"
            class="flex h-full flex-col items-start rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
            data-chart-option="multi"
            onclick={() => addChart('multi')}
          >
            <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-gold-50 text-gold-700"><Layers size={17} /></span>
            <span class="mt-3 text-sm font-semibold text-ink">Multi seri</span>
            <span class="mt-1 text-[10px] leading-relaxed text-ink-muted">
              Beberapa seri per kategori sebagai bar berkelompok, lengkap dengan legenda. Nilai dan warna tiap seri diatur di editor grafik.
            </span>
          </button>
        </div>
      </Modal>
    </div>
  {/if}

  {#if cakupanDialogOpen && cakupanTarget}
    <div use:portal>
      <Modal
        open={true}
        title={cakupanCardStep ? 'Pilih kartu cakupan' : 'Tambahkan data cakupan'}
        description={cakupanCardStep
          ? `${cakupanNodeLabel(cakupanTarget)} · pilih kartu yang ingin dibuat`
          : `${cakupanNodeLabel(cakupanTarget)}${cakupanDataset ? ` · ${cakupanDataset.items.length} data` : ''}`}
        onclose={closeCakupanDialog}
        width="max-w-2xl"
      >
        {#if cakupanCardStep && cakupanDataset}
          <div class="space-y-3">
            <div class="grid gap-2 sm:grid-cols-3">
              <button
                type="button"
                class="rounded-2xl border p-3 text-left transition-colors hover:border-gold-300 {cakupanCardScope === 'summary' ? 'border-gold-300 bg-gold-50/50' : 'border-line bg-white'}"
                data-cakupan-card-scope="summary"
                onclick={() => (cakupanCardScope = 'summary')}
              >
                <span class="block text-sm font-semibold text-ink">1 kartu ringkasan</span>
                <span class="mt-1 block text-[10px] leading-relaxed text-ink-muted">
                  Total {formatNumber(cakupanDataset.summaryValue)} kegiatan dari seluruh data.
                </span>
                <span class="mt-1 block truncate text-[10px] font-medium text-forest-700">{cakupanDataset.summaryLabel}</span>
              </button>
              <button
                type="button"
                class="rounded-2xl border p-3 text-left transition-colors hover:border-gold-300 {cakupanCardScope === 'all' ? 'border-gold-300 bg-gold-50/50' : 'border-line bg-white'}"
                data-cakupan-card-scope="all"
                onclick={() => (cakupanCardScope = 'all')}
              >
                <span class="block text-sm font-semibold text-ink">Semua kartu</span>
                <span class="mt-1 block text-[10px] leading-relaxed text-ink-muted">
                  {cakupanDataset.items.length} kartu, satu kartu untuk setiap data.
                </span>
              </button>
              <button
                type="button"
                class="rounded-2xl border p-3 text-left transition-colors hover:border-gold-300 {cakupanCardScope === 'pick' ? 'border-gold-300 bg-gold-50/50' : 'border-line bg-white'}"
                data-cakupan-card-scope="pick"
                onclick={() => (cakupanCardScope = 'pick')}
              >
                <span class="block text-sm font-semibold text-ink">Pilih sebagian</span>
                <span class="mt-1 block text-[10px] leading-relaxed text-ink-muted">
                  Centang hanya kartu yang ingin ditampilkan.
                </span>
              </button>
            </div>

            {#if cakupanCardScope === 'pick'}
              <div class="overflow-hidden rounded-xl border border-line">
                <div class="flex flex-wrap items-center justify-between gap-2 border-b border-line bg-stone-50/60 px-3 py-2">
                  <span class="text-[10px] text-ink-muted">{cakupanCardPicks.length} dari {cakupanDataset.items.length} dipilih</span>
                  <div class="flex gap-1">
                    <Button variant="ghost" size="sm" onclick={selectAllCakupanPicks}>Pilih semua</Button>
                    <Button variant="ghost" size="sm" onclick={clearCakupanPicks}>Kosongkan</Button>
                  </div>
                </div>
                <div class="max-h-56 overflow-auto p-2">
                  {#each cakupanDataset.items as item, index (index)}
                    <label class="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 text-[11px] hover:bg-stone-50">
                      <input
                        type="checkbox"
                        class="h-3.5 w-3.5"
                        checked={cakupanCardPicks.includes(index)}
                        data-cakupan-card-pick={index}
                        onchange={() => toggleCakupanPick(index)}
                      />
                      <span class="min-w-0 flex-1 truncate">{item.label}</span>
                      <span class="shrink-0 font-semibold text-ink">{formatNumber(item.value)}</span>
                    </label>
                  {/each}
                </div>
              </div>
            {/if}

            <label class="flex items-center gap-2 text-[11px] text-ink-muted">
              <input
                type="checkbox"
                class="h-3.5 w-3.5"
                checked={cakupanCardGroup}
                data-cakupan-card-group
                onchange={(event) => (cakupanCardGroup = (event.currentTarget as HTMLInputElement).checked)}
              />
              Gabungkan kartu agar bergerak bersama (opsional)
            </label>

            <div class="flex flex-wrap items-center justify-end gap-2 border-t border-line pt-3">
              <Button variant="ghost" size="sm" onclick={() => (cakupanCardStep = false)}>Kembali</Button>
              <Button
                variant="primary"
                size="sm"
                disabled={cakupanCardScope === 'pick' && !cakupanCardPicks.length}
                onclick={applyCakupanCards}
              >
                Buat kartu
              </Button>
            </div>
          </div>
        {:else}
          <div class="grid gap-2 sm:grid-cols-2">
            <button
              type="button"
              class="flex items-start gap-3 rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
              data-cakupan-mode="chart"
              onclick={() => insertCakupan('chart')}
            >
              <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><BarChart3 size={17} /></span>
              <span>
                <span class="block text-sm font-semibold text-ink">Grafik batang</span>
                <span class="mt-1 block text-[10px] leading-relaxed text-ink-muted">Bandingkan jumlah kegiatan antar data.</span>
              </span>
            </button>
            <button
              type="button"
              class="flex items-start gap-3 rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
              data-cakupan-mode="hbar"
              onclick={() => insertCakupan('hbar')}
            >
              <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-gold-50 text-gold-700"><AlignLeft size={17} /></span>
              <span>
                <span class="block text-sm font-semibold text-ink">Grafik bar samping</span>
                <span class="mt-1 block text-[10px] leading-relaxed text-ink-muted">Cocok untuk label panjang seperti nama polres atau satker.</span>
              </span>
            </button>
            <button
              type="button"
              class="flex items-start gap-3 rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
              data-cakupan-mode="table"
              onclick={() => insertCakupan('table')}
            >
              <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-stone-100 text-ink-soft"><Table2 size={17} /></span>
              <span>
                <span class="block text-sm font-semibold text-ink">Tabel</span>
                <span class="mt-1 block text-[10px] leading-relaxed text-ink-muted">Tampilkan rincian angka dalam baris dan kolom.</span>
              </span>
            </button>
            <button
              type="button"
              class="flex items-start gap-3 rounded-2xl border border-line bg-white p-4 text-left transition-colors hover:border-gold-300 hover:bg-gold-50/40"
              data-cakupan-mode="cards"
              onclick={() => insertCakupan('cards')}
            >
              <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-forest-50 text-forest-700"><CreditCard size={17} /></span>
              <span>
                <span class="block text-sm font-semibold text-ink">Kartu</span>
                <span class="mt-1 block text-[10px] leading-relaxed text-ink-muted">
                  {cakupanDataset?.items.length ?? 0} data tersedia. Pilih ringkasan, semua, atau sebagian.
                </span>
              </span>
            </button>
          </div>
          {#if cakupanDataset && !cakupanDataset.items.length}
            <p class="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-[11px] text-amber-800">
              Belum ada data untuk pilihan ini.
            </p>
          {/if}
          <p class="mt-3 text-[10px] leading-relaxed text-ink-muted">
            Elemen dibuat dari data cakupan periode ini dan bisa diedit setelah ditambahkan. Kartu tidak digabung secara default sehingga setiap kartu bisa digeser sendiri; centang Gabungkan bila ingin bergerak bersama.
          </p>
        {/if}
      </Modal>
    </div>
  {/if}

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
      <Modal open={true} title={iconPickerTitle} description={iconPickerTarget === 'card' ? 'Pilih ikon untuk kartu terpilih. Warna ikon dan latar bulat dapat diubah di panel kartu.' : 'Klik ikon untuk memasukkannya ke halaman aktif. Warna dapat diubah setelah ikon ditambahkan.'} onclose={() => (iconPickerOpen = false)} width="max-w-4xl">
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
