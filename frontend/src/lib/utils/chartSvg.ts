import { fontStack } from '../fonts/catalog'
import type { LayoutElement } from '../types'

const SVG_NS = 'http://www.w3.org/2000/svg'

export const CHART_PALETTE = [
  '#143d30',
  '#b8894b',
  '#3f765d',
  '#5b8fa8',
  '#a4743d',
  '#d9b87e',
  '#7a857f',
  '#2f5d7c',
]

export function chartColor(element: LayoutElement, index: number): string {
  return element.chart_colors?.[index] ?? CHART_PALETTE[index % CHART_PALETTE.length]
}

export function chartValueText(value: number): string {
  return Number.isInteger(value)
    ? new Intl.NumberFormat('id-ID').format(value)
    : value.toFixed(1)
}

function chartTextSize(value: number | undefined, fallback: number): number {
  return value && value > 0 ? Math.min(48, Math.max(6, value * 1.6)) : fallback
}

function chartFontFamily(value: string | undefined): string {
  return fontStack(value || 'sans')
}

function truncateText(value: string, max: number): string {
  return value.length <= max ? value : `${value.slice(0, max - 1)}…`
}

function wrapChartText(value: string, maxChars: number, maxLines: number): string[] {
  const words = value.split(/\s+/).filter(Boolean)
  const lines: string[] = []
  let current = ''
  let consumed = 0
  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word
    if (candidate.length <= maxChars) {
      current = candidate
      consumed += 1
      continue
    }
    if (current) {
      lines.push(current)
      current = ''
      if (lines.length >= maxLines) break
    }
    current = word
    consumed += 1
  }
  if (current && lines.length < maxLines) lines.push(current)
  if (consumed < words.length && lines.length) {
    const last = lines.length - 1
    lines[last] = truncateText(lines[last], Math.max(4, maxChars - 1))
  }
  return lines.slice(0, maxLines).map((line) => truncateText(line, maxChars))
}

type ChartTitleLayout = {
  lines: string[]
  size: number
  lineHeight: number
}

function chartTitleLayout(
  element: LayoutElement,
  base: number,
  isHbar: boolean,
): ChartTitleLayout {
  const title = element.chart_title?.trim() ?? ''
  if (!title) return { lines: [], size: 0, lineHeight: 0 }
  const size = chartTextSize(element.chart_title_font_size, base + 4)
  const maxWidth = isHbar ? 552 : 540
  const maxChars = Math.max(8, Math.floor(maxWidth / (size * 0.54)))
  const lines = element.chart_title_wrap
    ? wrapChartText(title, maxChars, 3)
    : [truncateText(title, maxChars)]
  return { lines, size, lineHeight: size * 1.18 }
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
  options: {
    fill?: string
    size?: number
    anchor?: string
    weight?: number
    family?: string
    rotate?: number
  } = {},
): void {
  const attributes: Record<string, string | number> = {
    x,
    y,
    fill: options.fill ?? '#143d30',
    'font-size': options.size ?? 14,
    'text-anchor': options.anchor ?? 'start',
    'font-weight': options.weight ?? 400,
  }
  if (options.family) attributes['font-family'] = options.family
  if (options.rotate) attributes.transform = `rotate(${options.rotate} ${x} ${y})`
  const node = svgNode(doc, 'text', attributes)
  node.textContent = text
  svg.appendChild(node)
}

export function createChartSvg(doc: Document, element: LayoutElement): SVGElement {
  const svg = svgNode(doc, 'svg', {
    viewBox: '0 0 600 360',
    preserveAspectRatio: 'xMidYMid meet',
    width: '100%',
    height: '100%',
    'font-family': chartFontFamily('sans'),
  })
  const fill = element.fill ?? '#ffffff'
  svg.appendChild(svgNode(doc, 'rect', { x: 0, y: 0, width: 600, height: 360, fill }))
  const base = Math.min(22, Math.max(10, (element.font_size ?? 9) * 1.6))
  const isHbar = element.chart_type === 'hbar'
  const title = chartTitleLayout(element, base, isHbar)
  const top = title.lines.length
    ? (isHbar ? 58 : 50) + (title.lines.length - 1) * title.lineHeight
    : isHbar
      ? 20
      : 18
  if (title.lines.length) {
    const x = isHbar ? 24 : 300
    const anchor = isHbar ? 'start' : 'middle'
    title.lines.forEach((line, index) => {
      svgText(doc, svg, x, (isHbar ? 32 : 30) + index * title.lineHeight, line, {
        fill: element.color ?? '#143d30',
        size: title.size,
        anchor,
        weight: 700,
        family: chartFontFamily(element.chart_title_font_family),
      })
    })
  }
  if (element.chart_type === 'line') {
    appendLineChart(doc, svg, element, top, base)
  } else if (element.chart_type === 'hbar') {
    appendHorizontalBarChart(doc, svg, element, top, base)
  } else if (element.chart_type === 'pie' || element.chart_type === 'donut') {
    appendPieChart(doc, svg, element, top, base, element.chart_type === 'donut')
  } else if (element.chart_series?.length) {
    appendGroupedBarChart(doc, svg, element, top, base)
  } else {
    appendBarChart(doc, svg, element, top, base)
  }
  return svg
}

function labelOptions(element: LayoutElement, fallback: number) {
  return {
    size: chartTextSize(element.chart_label_font_size, fallback),
    rotate: Math.min(90, Math.max(-90, element.chart_label_rotation ?? 0)),
  }
}

function valueOptions(element: LayoutElement, fallback: number) {
  return {
    size: chartTextSize(element.chart_value_font_size, fallback),
    family: chartFontFamily(element.chart_value_font_family),
  }
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
  const label = labelOptions(element, base - 2)
  const value = valueOptions(element, base - 2)
  svg.appendChild(svgNode(doc, 'line', { x1: left, y1: bottom, x2: right, y2: bottom, stroke: element.chart_axis_color ?? '#6d7872', 'stroke-width': 1.2 }))
  svgText(doc, svg, left - 8, top + base, chartValueText(max), { fill: element.chart_axis_color ?? '#6d7872', size: value.size, anchor: 'end', family: value.family })
  values.forEach((item, index) => {
    const height = (item / max) * plotHeight
    const x = left + step * index + (step - barWidth) / 2
    const y = bottom - height
    const color = chartColor(element, index)
    svg.appendChild(svgNode(doc, 'rect', { x, y, width: barWidth, height, rx: 2, fill: color }))
    if (element.chart_show_values !== false) {
      svgText(doc, svg, x + barWidth / 2, Math.max(top + base, y - 4), chartValueText(item), { fill: element.color ?? '#143d30', size: value.size, anchor: 'middle', family: value.family })
    }
    const text = element.chart_labels?.[index]
    if (text) {
      svgText(doc, svg, x + barWidth / 2, bottom + base + 8, truncateText(text, 14), { fill: element.chart_axis_color ?? '#6d7872', size: label.size, anchor: 'middle', rotate: label.rotate })
    }
  })
}

function appendGroupedBarChart(
  doc: Document,
  svg: SVGElement,
  element: LayoutElement,
  top: number,
  base: number,
): void {
  const left = 55
  const right = 575
  const bottom = 316
  const labels = element.chart_labels ?? []
  const count = Math.max(1, labels.length)
  const series = element.chart_series ?? []
  if (!series.length) {
    appendBarChart(doc, svg, element, top, base)
    return
  }
  const legendSize = chartTextSize(element.chart_legend_font_size, base - 4)
  const legendHeight = element.chart_show_legend !== false ? legendSize + 10 : 0
  const plotTop = top + legendHeight
  const plotHeight = Math.max(40, bottom - plotTop)
  const max = Math.max(1, ...series.flatMap((item) => item.values))
  const step = (right - left) / count
  const groupWidth = step * 0.78
  const barWidth = Math.max(2, groupWidth / Math.max(1, series.length))
  const label = labelOptions(element, base - 2)
  const value = valueOptions(element, base - 2)

  if (element.chart_show_legend !== false) {
    let x = left
    for (const item of series) {
      svg.appendChild(
        svgNode(doc, 'rect', {
          x,
          y: top + 1,
          width: 10,
          height: 10,
          rx: 2,
          fill: item.color,
        }),
      )
      svgText(doc, svg, x + 14, top + legendSize, truncateText(item.label, 28), {
        fill: element.color ?? '#143d30',
        size: legendSize,
      })
      const advance = item.label.length * legendSize * 0.55 + 26
      if (x + advance > right) break
      x += advance
    }
  }

  svg.appendChild(
    svgNode(doc, 'line', {
      x1: left,
      y1: bottom,
      x2: right,
      y2: bottom,
      stroke: element.chart_axis_color ?? '#6d7872',
      'stroke-width': 1.2,
    }),
  )
  svgText(doc, svg, left - 8, plotTop + base, chartValueText(max), {
    fill: element.chart_axis_color ?? '#6d7872',
    size: value.size,
    anchor: 'end',
    family: value.family,
  })

  labels.forEach((text, index) => {
    const groupX = left + step * index + (step - groupWidth) / 2
    series.forEach((item, seriesIndex) => {
      const valueItem = item.values[index] ?? 0
      const height = (valueItem / max) * plotHeight
      const x = groupX + seriesIndex * barWidth
      const y = bottom - height
      svg.appendChild(
        svgNode(doc, 'rect', {
          x,
          y,
          width: barWidth,
          height,
          rx: 1.5,
          fill: item.color,
        }),
      )
      if (element.chart_show_values !== false && series.length <= 2) {
        svgText(doc, svg, x + barWidth / 2, Math.max(plotTop + base, y - 3), chartValueText(valueItem), {
          fill: element.color ?? '#143d30',
          size: value.size,
          anchor: 'middle',
          family: value.family,
        })
      }
    })
    svgText(doc, svg, groupX + groupWidth / 2, bottom + base + 8, truncateText(text, 14), {
      fill: element.chart_axis_color ?? '#6d7872',
      size: label.size,
      anchor: 'middle',
      rotate: label.rotate,
    })
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
  const fallbackLabelSize = count > 12 ? Math.max(7, base - 4) : Math.max(8, base - 3)
  const label = labelOptions(element, fallbackLabelSize)
  const value = valueOptions(element, Math.max(8, base - 2))
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
  values.forEach((item, index) => {
    const center = top + rowHeight * (index + 0.5)
    const baseline = center + label.size * 0.34
    const text = labels[index]
    if (text) {
      svgText(doc, svg, left, baseline, truncateText(text, 26), {
        fill: '#46534b',
        size: label.size,
        rotate: label.rotate,
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
    const width = trackWidth * Math.min(1, Math.max(0, item / max))
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
      svgText(doc, svg, right, baseline, chartValueText(item), {
        fill: element.color ?? '#143d30',
        size: value.size,
        anchor: 'end',
        weight: 700,
        family: value.family,
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
  const label = labelOptions(element, base - 2)
  const value = valueOptions(element, base - 2)
  svg.appendChild(svgNode(doc, 'line', { x1: left, y1: bottom, x2: right, y2: bottom, stroke: element.chart_axis_color ?? '#6d7872', 'stroke-width': 1.2 }))
  const coordinates = values.map((item, index) => {
    const x = count > 1 ? left + step * index : (left + right) / 2
    const y = bottom - (item / max) * plotHeight
    return { x, y, value: item }
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
      svgText(doc, svg, point.x, Math.max(top + base, point.y - 8), chartValueText(point.value), { fill: element.color ?? '#143d30', size: value.size, anchor: 'middle', family: value.family })
    }
    const text = element.chart_labels?.[index]
    if (text) {
      svgText(doc, svg, point.x, bottom + base + 8, truncateText(text, 14), { fill: element.chart_axis_color ?? '#6d7872', size: label.size, anchor: 'middle', rotate: label.rotate })
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
  const total = values.reduce((sum, item) => sum + item, 0)
  const value = valueOptions(element, base - 2)
  if (total <= 0) {
    svgText(doc, svg, 300, 180, 'Belum ada data', { fill: element.chart_axis_color ?? '#6d7872', size: value.size, anchor: 'middle', family: value.family })
    return
  }
  const showLegend = element.chart_show_legend !== false
  const cx = showLegend ? 205 : 300
  const cy = top + (316 - top) / 2
  const outer = Math.min(125, Math.max(40, (316 - top) / 2 - 12))
  const inner = donut ? outer * 0.55 : 0
  const positives = values.filter((item) => item > 0).length
  let angle = -Math.PI / 2
  values.forEach((item, index) => {
    if (item <= 0) return
    const sweep = (item / total) * Math.PI * 2
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
      svgText(doc, svg, cx + radius * Math.cos(mid), cy + radius * Math.sin(mid) + base * 0.35, `${Math.round((sweep / (Math.PI * 2)) * 100)}%`, { fill: '#ffffff', size: value.size, anchor: 'middle', weight: 700, family: value.family })
    }
    angle = end
  })
  if (donut) {
    svg.appendChild(svgNode(doc, 'circle', { cx, cy, r: inner, fill: element.fill ?? '#ffffff' }))
  }
  if (showLegend) {
    const legendSize = chartTextSize(element.chart_legend_font_size, base - 3)
    let y = top + base
    element.chart_labels?.slice(0, 12).forEach((text, index) => {
      svg.appendChild(svgNode(doc, 'rect', { x: 355, y: y - 10, width: 12, height: 12, rx: 2, fill: chartColor(element, index) }))
      svgText(doc, svg, 374, y, truncateText(text, 20), { fill: element.color ?? '#143d30', size: legendSize })
      y += legendSize + 7
    })
  }
}
