import { fontStack } from '../fonts/catalog'
import type { LayoutElement } from '../types'

function isNumericCell(value: string): boolean {
  const normalized = value.trim().replace(/\./g, '').replace(',', '.')
  return normalized !== '' && Number.isFinite(Number(normalized))
}

function columnValue<T>(values: T[] | undefined, index: number, fallback: T): T {
  return values?.[index] ?? fallback
}

export function createTableNode(
  doc: Document,
  element: LayoutElement,
  rowsOverride?: string[][],
): HTMLElement {
  const rows = rowsOverride ?? element.table_rows ?? []
  const wrapper = doc.createElement('div')
  wrapper.style.background = element.fill ?? '#ffffff'
  wrapper.style.width = '100%'
  wrapper.style.height = '100%'
  wrapper.style.boxSizing = 'border-box'
  wrapper.style.overflow = 'hidden'
  if (!rows.length) {
    wrapper.style.display = 'flex'
    wrapper.style.alignItems = 'center'
    wrapper.style.justifyContent = 'center'
    wrapper.style.padding = '2mm'
    wrapper.style.background = '#f6f8f5'
    wrapper.style.color = '#6d7872'
    wrapper.style.fontSize = '8pt'
    wrapper.style.textAlign = 'center'
    wrapper.textContent = 'Belum ada baris tabel.'
    return wrapper
  }

  const baseSize = element.font_size ?? 9
  const baseFamily = fontStack(element.font_family || 'sans')
  const headerSize =
    element.table_header_font_size && element.table_header_font_size > 0
      ? element.table_header_font_size
      : baseSize
  const headerFamily = fontStack(
    element.table_header_font_family || element.font_family || 'sans',
  )
  const table = doc.createElement('table')
  table.style.width = '100%'
  table.style.borderCollapse = 'collapse'
  table.style.tableLayout = 'fixed'
  table.style.fontFamily = baseFamily
  table.style.fontSize = `${baseSize}pt`
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
    headerRow.forEach((cell) => {
      const th = doc.createElement('th')
      th.textContent = cell
      th.style.background = element.table_header_bg ?? '#143d30'
      th.style.color = element.table_header_color ?? '#ffffff'
      th.style.fontFamily = headerFamily
      th.style.fontSize = `${headerSize}pt`
      th.style.textAlign = isNumericCell(cell) ? 'right' : 'left'
      tr.appendChild(th)
    })
    thead.appendChild(tr)
    table.appendChild(thead)
  }
  const tbody = doc.createElement('tbody')
  bodyRows.forEach((row, rowIndex) => {
    const tr = doc.createElement('tr')
    row.forEach((cell, columnIndex) => {
      const td = doc.createElement('td')
      td.textContent = cell
      const columnBackground = columnValue(
        element.table_column_backgrounds,
        columnIndex,
        '',
      )
      const columnColor = columnValue(element.table_column_colors, columnIndex, '')
      const columnFamily = columnValue(
        element.table_column_font_families,
        columnIndex,
        '',
      )
      const columnSize = columnValue(element.table_column_font_sizes, columnIndex, 0)
      const columnAlign = columnValue(element.table_column_aligns, columnIndex, '')
      if (columnBackground) {
        td.style.background = columnBackground
      } else if (
        rowIndex % 2 === 1 &&
        element.table_stripe &&
        element.table_stripe !== 'transparent'
      ) {
        td.style.background = element.table_stripe
      }
      if (columnColor) td.style.color = columnColor
      if (columnFamily) td.style.fontFamily = fontStack(columnFamily)
      if (columnSize > 0) td.style.fontSize = `${columnSize}pt`
      td.style.textAlign = columnAlign || (isNumericCell(cell) ? 'right' : 'left')
      tr.appendChild(td)
    })
    tbody.appendChild(tr)
  })
  table.appendChild(tbody)
  wrapper.appendChild(table)
  return wrapper
}
