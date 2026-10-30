import { fontStack } from '../fonts/catalog'
import { iconByName } from '../icons/catalog'
import type { LayoutElement } from '../types'
import { cardTemplate } from './cardTemplates'

const SVG_NS = 'http://www.w3.org/2000/svg'

export type CardContent = {
  label: string
  value: string
  unit: string
  detail: string
}

type CardPart = 'label' | 'value' | 'unit' | 'detail'

function clampNumber(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

function cardPartStyle(
  element: LayoutElement,
  part: CardPart,
): { fontFamily: string; fontSize: string; color: string } {
  const base = element.font_size ?? 9
  const family =
    part === 'label'
      ? element.card_label_font_family
      : part === 'value'
        ? element.card_value_font_family
        : part === 'unit'
          ? element.card_unit_font_family
          : element.card_detail_font_family
  const explicitSize =
    part === 'label'
      ? element.card_label_font_size
      : part === 'value'
        ? element.card_value_font_size
        : part === 'unit'
          ? element.card_unit_font_size
          : element.card_detail_font_size
  const color =
    (part === 'label'
      ? element.card_label_color
      : part === 'value'
        ? element.card_value_color
        : part === 'unit'
          ? element.card_unit_color
          : element.card_detail_color) || element.color || '#143d30'
  const fallbackSize =
    part === 'value'
      ? clampNumber(base * 2.35, 14, 40)
      : part === 'unit'
        ? base * 0.82
        : part === 'detail'
          ? base * 0.78
          : base
  const size = explicitSize && explicitSize > 0 ? explicitSize : fallbackSize
  return {
    fontFamily: fontStack(family || element.font_family || 'sans'),
    fontSize: `${size}pt`,
    color,
  }
}

function applyPartStyle(node: HTMLElement, element: LayoutElement, part: CardPart): void {
  const style = cardPartStyle(element, part)
  node.style.fontFamily = style.fontFamily
  node.style.fontSize = style.fontSize
  node.style.color = style.color
}

function createCardIconNode(
  doc: Document,
  element: LayoutElement,
  sizeMm: number,
): HTMLElement | null {
  const icon = iconByName.get(element.card_icon ?? '')
  if (!icon) return null
  const badge = doc.createElement('span')
  badge.style.display = 'inline-flex'
  badge.style.alignItems = 'center'
  badge.style.justifyContent = 'center'
  badge.style.flex = '0 0 auto'
  badge.style.width = `${sizeMm}mm`
  badge.style.height = `${sizeMm}mm`
  badge.style.borderRadius = '50%'
  badge.style.background = element.card_icon_bg ?? '#f1e6d5'
  badge.style.color = element.card_icon_color ?? '#b8894b'
  const svg = doc.createElementNS(SVG_NS, 'svg')
  svg.setAttribute('viewBox', '0 0 24 24')
  svg.setAttribute('fill', 'none')
  svg.setAttribute('stroke', 'currentColor')
  svg.setAttribute('stroke-width', '2')
  svg.setAttribute('stroke-linecap', 'round')
  svg.setAttribute('stroke-linejoin', 'round')
  svg.style.width = '56%'
  svg.style.height = '56%'
  svg.innerHTML = icon.svg
  badge.appendChild(svg)
  return badge
}

export function createCardNode(
  doc: Document,
  element: LayoutElement,
  content?: CardContent,
): HTMLElement {
  const template = cardTemplate(element.card_template)
  const cardLabel = content?.label ?? element.card_label ?? ''
  const cardValue = content?.value ?? element.card_value ?? ''
  const cardUnit = content?.unit ?? element.card_unit ?? ''
  const cardDetail = content?.detail ?? element.card_detail ?? ''
  const accent = element.card_accent ?? template.accent
  const fill = element.fill ?? template.fill
  const color = element.color ?? template.color
  const minSide = Math.min(element.width, element.height)
  const iconSize =
    Math.max(
      8,
      Math.min(
        template.id === 'centered' ? 14 : 18,
        minSide * (template.id === 'centered' ? 0.24 : 0.34),
      ),
    ) * ((element.card_icon_scale ?? 100) / 100)
  const iconNode = createCardIconNode(doc, element, iconSize)
  const label = doc.createElement('span')
  label.className = 'layout-card-label'
  label.textContent = cardLabel
  applyPartStyle(label, element, 'label')
  const value = doc.createElement('strong')
  value.className = 'layout-card-value'
  value.textContent = cardValue
  applyPartStyle(value, element, 'value')
  const unit = doc.createElement('span')
  unit.className = 'layout-card-unit'
  unit.textContent = cardUnit
  applyPartStyle(unit, element, 'unit')
  const card = doc.createElement('div')
  card.style.boxSizing = 'border-box'
  card.style.overflow = 'hidden'
  card.style.background = fill
  card.style.color = color
  card.style.display = 'flex'
  const appendDetail = (parent: HTMLElement) => {
    if (!cardDetail.trim()) return
    const detail = doc.createElement('span')
    detail.className = 'layout-card-detail'
    detail.textContent = cardDetail
    applyPartStyle(detail, element, 'detail')
    parent.appendChild(detail)
  }

  if (template.id === 'icon_right') {
    card.style.flexDirection = 'column'
    card.style.padding = '3mm 3.5mm'
    card.style.borderRadius = `${element.radius ?? template.radius}mm`
    const top = doc.createElement('div')
    top.style.display = 'flex'
    top.style.alignItems = 'flex-start'
    top.style.justifyContent = 'space-between'
    top.style.gap = '2mm'
    top.style.width = '100%'
    const inner = doc.createElement('div')
    inner.style.minWidth = '0'
    inner.style.flex = '1'
    inner.append(value, unit)
    top.appendChild(inner)
    if (iconNode) top.appendChild(iconNode)
    card.append(top, label)
    appendDetail(card)
  } else if (template.id === 'icon_left') {
    card.style.flexDirection = 'row'
    card.style.alignItems = 'center'
    card.style.gap = '3mm'
    card.style.padding = '3mm 3.5mm'
    card.style.borderRadius = `${element.radius ?? template.radius}mm`
    if (iconNode) card.appendChild(iconNode)
    const inner = doc.createElement('div')
    inner.style.minWidth = '0'
    inner.style.flex = '1'
    inner.append(value, label, unit)
    appendDetail(inner)
    card.appendChild(inner)
  } else if (template.id === 'centered') {
    card.style.flexDirection = 'column'
    card.style.alignItems = 'center'
    card.style.justifyContent = 'center'
    card.style.textAlign = 'center'
    card.style.padding = '3mm'
    card.style.border = `${element.border_width ?? template.borderWidth}pt solid ${accent}`
    card.style.borderRadius = `${element.radius ?? template.radius}mm`
    if (iconNode) card.appendChild(iconNode)
    card.append(value, label, unit)
    appendDetail(card)
  } else if (template.id === 'split') {
    card.style.flexDirection = 'row'
    card.style.alignItems = 'stretch'
    card.style.borderRadius = `${element.radius ?? template.radius}mm`
    const panel = doc.createElement('div')
    panel.style.width = '32%'
    panel.style.flex = '0 0 auto'
    panel.style.display = 'flex'
    panel.style.alignItems = 'center'
    panel.style.justifyContent = 'center'
    panel.style.background = accent
    if (iconNode) panel.appendChild(iconNode)
    const inner = doc.createElement('div')
    inner.style.minWidth = '0'
    inner.style.flex = '1'
    inner.style.display = 'flex'
    inner.style.flexDirection = 'column'
    inner.style.justifyContent = 'center'
    inner.style.padding = '3mm 3.5mm'
    inner.append(value, label, unit)
    appendDetail(inner)
    card.append(panel, inner)
  } else if (template.id === 'minimal') {
    card.style.flexDirection = 'column'
    card.style.padding = '2mm 0'
    card.style.borderBottom = `${Math.max(element.border_width ?? template.borderWidth, 1)}pt solid ${accent}`
    const row = doc.createElement('div')
    row.style.display = 'flex'
    row.style.alignItems = 'baseline'
    row.style.gap = '2mm'
    row.append(value, unit)
    card.append(row, label)
    appendDetail(card)
  } else {
    card.style.flexDirection = 'column'
    card.style.padding = '3mm 3.5mm'
    card.style.borderLeft = `${element.border_width ?? template.borderWidth}pt solid ${accent}`
    card.style.borderRadius = `${element.radius ?? template.radius}mm`
    card.append(label, value, unit)
    appendDetail(card)
  }
  return card
}
