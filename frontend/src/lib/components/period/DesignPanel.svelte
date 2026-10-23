<script lang="ts">
  import { tick, untrack } from 'svelte'
  import {
    AlertCircle,
    BarChart3,
    Check,
    Copy,
    Eye,
    FileDown,
    ImagePlus,
    Link2,
    Redo2,
    Save,
    Square,
    Table2,
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
  const defaultTexts = new Map<string, string>()

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
  const hiddenBlocks = $derived.by(() => {
    const page = getPage(currentPage)
    if (!page) return []
    return Object.entries(page.blocks)
      .filter(([, block]) => block.hidden)
      .map(([id]) => id)
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
      .layout-table table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      .layout-table th, .layout-table td { border: var(--table-border, .5pt solid #d7ddd6); padding: 1.2mm 1.5mm; vertical-align: top; word-break: break-word; }
      .layout-table th { font-weight: 700; }
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
    } else if (element.type === 'table') {
      const wrapper = doc.createElement('div')
      wrapper.style.background = element.fill ?? '#ffffff'
      const table = doc.createElement('table')
      table.style.fontSize = `${element.font_size ?? 9}pt`
      table.style.color = element.color ?? '#143d30'
      table.style.setProperty(
        '--table-border',
        `${element.border_width ?? 0.5}pt solid ${element.border_color ?? '#d7ddd6'}`,
      )
      const rows = element.table_rows ?? []
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
    } else if (element.type === 'chart') {
      const wrapper = doc.createElement('div')
      wrapper.appendChild(createChartSvg(doc, element))
      node = wrapper
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
    const top = element.chart_title?.trim() ? 50 : 18
    if (element.chart_title?.trim()) {
      svgText(doc, svg, 300, 30, element.chart_title, {
        fill: element.color ?? '#143d30',
        size: base + 4,
        anchor: 'middle',
        weight: 700,
      })
    }
    if (element.chart_type === 'line') {
      appendLineChart(doc, svg, element, top, base)
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

  function updateTableCell(rowIndex: number, columnIndex: number, value: string) {
    if (!activeElement || activeElement.type !== 'table') return
    const rows = activeElement.table_rows
    if (!rows?.[rowIndex]) return
    rows[rowIndex][columnIndex] = value
    activeElement.table_rows = rows
    applyLayoutToDom()
  }

  function addTableRow() {
    if (!activeElement || activeElement.type !== 'table') return
    pushHistory()
    const rows = activeElement.table_rows ?? []
    const columns = Math.max(1, ...rows.map((row) => row.length))
    activeElement.table_rows = [...rows, Array.from({ length: columns }, () => '')]
    applyLayoutToDom()
  }

  function removeTableRow() {
    if (!activeElement || activeElement.type !== 'table') return
    const rows = activeElement.table_rows ?? []
    if (rows.length <= 1) return
    pushHistory()
    activeElement.table_rows = rows.slice(0, -1)
    applyLayoutToDom()
  }

  function addTableColumn() {
    if (!activeElement || activeElement.type !== 'table') return
    pushHistory()
    const rows = (activeElement.table_rows ?? []).map((row) => [...row, ''])
    activeElement.table_rows = rows
    applyLayoutToDom()
  }

  function removeTableColumn() {
    if (!activeElement || activeElement.type !== 'table') return
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
    if (!activeElement || activeElement.type !== 'chart') return
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
    if (!activeElement || activeElement.type !== 'chart') return
    normalizeChart(parseLabelList(value), activeElement.chart_values ?? [])
  }

  function updateChartValues(value: string) {
    if (!activeElement || activeElement.type !== 'chart') return
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

  function showBlock(id: string) {
    const page = getPage(currentPage)
    if (!page) return
    const block = page.blocks[id] ?? { dx: 0, dy: 0, hidden: false }
    block.hidden = false
    page.blocks[id] = block
    selected = { kind: 'block', page: currentPage, id }
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
        <Button variant="secondary" size="sm" onclick={addTable}><Table2 size={14} /> Tabel</Button>
        <Button variant="secondary" size="sm" onclick={addChart}><BarChart3 size={14} /> Grafik</Button>
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
            {:else if activeElement.type === 'table'}
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
              <div class="grid grid-cols-2 gap-3">
                <label class="block">
                  <span class="label">Jenis</span>
                  <select class="input py-1.5 text-xs" value={activeElement.chart_type ?? 'bar'}
                    data-chart-field="type"
                    onchange={(event) => updateElement({ chart_type: (event.currentTarget as HTMLSelectElement).value as 'bar' | 'line' | 'pie' | 'donut' }, true)}>
                    <option value="bar">Bar</option>
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
          </div>
        </section>
      {:else}
        <section class="card p-5">
          <span class="eyebrow">CARA PAKAI</span>
          <ul class="mt-3 space-y-2 text-[11px] leading-relaxed text-ink-muted">
            <li>1. Klik halaman yang ingin ditata.</li>
            <li>2. Tambah teks, kotak, gambar, URL, tabel, atau grafik.</li>
            <li>3. Tarik elemen untuk menggeser; tarik sudut untuk mengubah ukuran.</li>
            <li>4. Klik blok laporan untuk geser; tarik sudut kanan bawah untuk resize.</li>
            <li>5. Klik dua kali teks blok untuk mengeditnya.</li>
            <li>6. Simpan, lalu Buat PDF.</li>
          </ul>
        </section>
      {/if}

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
{/if}
