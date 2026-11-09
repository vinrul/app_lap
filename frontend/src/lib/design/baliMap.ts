import type { QwMapPoint } from '../types'

/**
 * Menggambar peta sebaran titik QW1.1/QW1.2 ke PNG agar dapat dipakai sebagai
 * elemen gambar pada desain laporan. Tile OSM diambil lewat proxy backend agar
 * tidak terkena CORS; bila tile tidak tersedia, siluet Pulau Bali tetap digambar
 * sehingga peta tidak kosong.
 */

const CANVAS_WIDTH = 1260
const CANVAS_HEIGHT = 900
const TILE_SIZE = 256
const ZOOM = 10
const CENTER_LON = 115.08
const CENTER_LAT = -8.47
const TILE_TIMEOUT_MS = 4_000

const FOREST = '#143d30'
const GOLD = '#b8894b'
const LINE = '#9db5a7'
const INK = '#2f3d36'
const MUTED = '#6f7c75'
const PAPER = '#f7faf6'

const WORLD_SIZE = TILE_SIZE * 2 ** ZOOM

/** Siluet ringkas Pulau Bali (lon, lat), cukup untuk peta gaya laporan. */
const BALI_OUTLINE: [number, number][] = [
  [114.44, -8.16],
  [114.50, -8.12],
  [114.56, -8.13],
  [114.63, -8.16],
  [114.70, -8.15],
  [114.77, -8.09],
  [114.85, -8.08],
  [114.93, -8.10],
  [115.00, -8.11],
  [115.08, -8.13],
  [115.16, -8.17],
  [115.24, -8.22],
  [115.31, -8.26],
  [115.40, -8.30],
  [115.48, -8.33],
  [115.56, -8.35],
  [115.63, -8.38],
  [115.65, -8.42],
  [115.62, -8.47],
  [115.58, -8.52],
  [115.53, -8.55],
  [115.45, -8.58],
  [115.38, -8.60],
  [115.32, -8.63],
  [115.26, -8.68],
  [115.23, -8.72],
  [115.20, -8.76],
  [115.16, -8.80],
  [115.11, -8.83],
  [115.06, -8.85],
  [115.05, -8.80],
  [115.09, -8.76],
  [115.13, -8.74],
  [115.16, -8.72],
  [115.15, -8.70],
  [115.12, -8.68],
  [115.09, -8.64],
  [115.05, -8.60],
  [115.00, -8.56],
  [114.95, -8.52],
  [114.90, -8.48],
  [114.85, -8.44],
  [114.80, -8.40],
  [114.74, -8.36],
  [114.69, -8.32],
  [114.63, -8.28],
  [114.57, -8.24],
  [114.51, -8.20],
]

const REGION_LABELS: { label: string; lon: number; lat: number }[] = [
  { label: 'Singaraja', lon: 115.09, lat: -8.12 },
  { label: 'Negara', lon: 114.62, lat: -8.35 },
  { label: 'Denpasar', lon: 115.22, lat: -8.66 },
  { label: 'Klungkung', lon: 115.4, lat: -8.54 },
  { label: 'Karangasem', lon: 115.6, lat: -8.45 },
]

export interface BaliMapSeries {
  code: string
  label: string
  color: string
}

export interface BaliMapOptions {
  title?: string
  series?: BaliMapSeries[]
}

const DEFAULT_SERIES: BaliMapSeries[] = [
  { code: 'QW1.1', label: 'Gatur Lalin (QW1.1)', color: FOREST },
  { code: 'QW1.2', label: 'Polidik (QW1.2)', color: GOLD },
]

interface Projected {
  x: number
  y: number
}

function project(lon: number, lat: number, originX: number, originY: number): Projected {
  const x = ((lon + 180) / 360) * WORLD_SIZE - originX
  const sin = Math.sin((lat * Math.PI) / 180)
  const y =
    (0.5 - Math.log((1 + sin) / (1 - sin)) / (4 * Math.PI)) * WORLD_SIZE - originY
  return { x, y }
}

function origin(): { x: number; y: number } {
  const center = project(CENTER_LON, CENTER_LAT, 0, 0)
  return { x: center.x - CANVAS_WIDTH / 2, y: center.y - CANVAS_HEIGHT / 2 }
}

function loadImage(url: string): Promise<HTMLImageElement | null> {
  return new Promise((resolve) => {
    const image = new Image()
    let settled = false
    const finish = (value: HTMLImageElement | null) => {
      if (settled) return
      settled = true
      resolve(value)
    }
    const timer = window.setTimeout(() => finish(null), TILE_TIMEOUT_MS)
    image.onload = () => {
      window.clearTimeout(timer)
      finish(image)
    }
    image.onerror = () => {
      window.clearTimeout(timer)
      finish(null)
    }
    image.src = url
  })
}

function roundedRect(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
): void {
  context.beginPath()
  context.moveTo(x + radius, y)
  context.lineTo(x + width - radius, y)
  context.quadraticCurveTo(x + width, y, x + width, y + radius)
  context.lineTo(x + width, y + height - radius)
  context.quadraticCurveTo(x + width, y + height, x + width - radius, y + height)
  context.lineTo(x + radius, y + height)
  context.quadraticCurveTo(x, y + height, x, y + height - radius)
  context.lineTo(x, y + radius)
  context.quadraticCurveTo(x, y, x + radius, y)
  context.closePath()
}

function drawOutline(
  context: CanvasRenderingContext2D,
  originPoint: Projected,
  withFill: boolean,
): void {
  context.beginPath()
  BALI_OUTLINE.forEach(([lon, lat], index) => {
    const point = project(lon, lat, originPoint.x, originPoint.y)
    if (index === 0) context.moveTo(point.x, point.y)
    else context.lineTo(point.x, point.y)
  })
  context.closePath()
  if (withFill) {
    context.fillStyle = 'rgba(224, 236, 226, 0.62)'
    context.fill()
  }
  context.strokeStyle = LINE
  context.lineWidth = 3
  context.stroke()
}

function drawRegionLabels(
  context: CanvasRenderingContext2D,
  originPoint: Projected,
): void {
  context.font = '500 22px Inter, "Segoe UI", Arial, sans-serif'
  context.textAlign = 'center'
  context.textBaseline = 'middle'
  context.lineJoin = 'round'
  context.lineWidth = 5
  context.strokeStyle = 'rgba(255, 255, 255, 0.9)'
  context.fillStyle = MUTED
  for (const region of REGION_LABELS) {
    const point = project(region.lon, region.lat, originPoint.x, originPoint.y)
    context.strokeText(region.label, point.x, point.y)
    context.fillText(region.label, point.x, point.y)
  }
}

function drawPoints(
  context: CanvasRenderingContext2D,
  originPoint: Projected,
  points: QwMapPoint[],
  colors: Map<string, string>,
): void {
  context.lineWidth = 2.4
  context.strokeStyle = 'rgba(255, 255, 255, 0.92)'
  for (const point of points) {
    const projected = project(point.longitude, point.latitude, originPoint.x, originPoint.y)
    if (
      projected.x < -20 ||
      projected.y < -20 ||
      projected.x > CANVAS_WIDTH + 20 ||
      projected.y > CANVAS_HEIGHT + 20
    ) {
      continue
    }
    context.beginPath()
    context.arc(projected.x, projected.y, 8, 0, Math.PI * 2)
    context.fillStyle = colors.get(point.kode) ?? MUTED
    context.globalAlpha = 0.82
    context.fill()
    context.globalAlpha = 0.9
    context.stroke()
  }
  context.globalAlpha = 1
}

function drawLegend(
  context: CanvasRenderingContext2D,
  series: BaliMapSeries[],
  counts: Map<string, number>,
  title: string,
): void {
  const x = 28
  const y = 28
  const width = 340
  const rowHeight = 26
  const height = 84 + series.length * rowHeight
  context.save()
  roundedRect(context, x, y, width, height, 14)
  context.fillStyle = 'rgba(255, 255, 255, 0.93)'
  context.fill()
  context.strokeStyle = 'rgba(157, 181, 167, 0.7)'
  context.lineWidth = 2
  context.stroke()

  context.fillStyle = MUTED
  context.font = '700 16px Inter, "Segoe UI", Arial, sans-serif'
  context.textAlign = 'left'
  context.textBaseline = 'middle'
  context.fillText('TITIK KOORDINAT', x + 22, y + 30)
  context.fillStyle = INK
  context.font = '600 21px Inter, "Segoe UI", Arial, sans-serif'
  context.fillText(title, x + 22, y + 60)

  series.forEach((item, index) => {
    const rowY = y + 92 + index * rowHeight
    context.beginPath()
    context.arc(x + 30, rowY, 7, 0, Math.PI * 2)
    context.fillStyle = item.color
    context.fill()
    context.fillStyle = INK
    context.font = '500 18px Inter, "Segoe UI", Arial, sans-serif'
    context.fillText(item.label, x + 46, rowY)
    context.fillStyle = MUTED
    context.font = '700 18px Inter, "Segoe UI", Arial, sans-serif'
    context.fillText(String(counts.get(item.code) ?? 0), x + width - 28, rowY)
  })
  context.restore()
}

async function drawTiles(
  context: CanvasRenderingContext2D,
  originPoint: Projected,
): Promise<boolean> {
  const startX = Math.floor(originPoint.x / TILE_SIZE)
  const startY = Math.floor(originPoint.y / TILE_SIZE)
  const endX = Math.floor((originPoint.x + CANVAS_WIDTH) / TILE_SIZE)
  const endY = Math.floor((originPoint.y + CANVAS_HEIGHT) / TILE_SIZE)
  const requests: Promise<void>[] = []
  let loaded = 0
  let total = 0
  for (let tileX = startX; tileX <= endX; tileX += 1) {
    for (let tileY = startY; tileY <= endY; tileY += 1) {
      total += 1
      requests.push(
        loadImage(`/api/map/tiles/${ZOOM}/${tileX}/${tileY}`).then((image) => {
          if (!image) return
          loaded += 1
          context.drawImage(
            image,
            tileX * TILE_SIZE - originPoint.x,
            tileY * TILE_SIZE - originPoint.y,
            TILE_SIZE,
            TILE_SIZE,
          )
        }),
      )
    }
  }
  await Promise.all(requests)
  return total > 0 && loaded / total >= 0.4
}

export async function buildBaliMapFile(
  points: QwMapPoint[],
  options: BaliMapOptions = {},
): Promise<File | null> {
  if (typeof document === 'undefined') return null
  const series = options.series?.length ? options.series : DEFAULT_SERIES
  const codes = new Set(series.map((item) => item.code))
  const visiblePoints = points.filter((point) => codes.has(point.kode))
  const counts = new Map<string, number>()
  for (const item of series) {
    counts.set(item.code, visiblePoints.filter((point) => point.kode === item.code).length)
  }
  const seriesColors = new Map(series.map((item) => [item.code, item.color]))
  const title = options.title ?? 'Peta Sebaran Gatur & Polidik'
  const canvas = document.createElement('canvas')
  canvas.width = CANVAS_WIDTH
  canvas.height = CANVAS_HEIGHT
  const context = canvas.getContext('2d')
  if (!context) return null
  const originPoint = origin()

  context.fillStyle = PAPER
  context.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT)

  let hasTiles = false
  try {
    hasTiles = await drawTiles(context, originPoint)
  } catch {
    hasTiles = false
  }

  if (hasTiles) {
    // Tile OSM dibiarkan redup agar warna laporan tetap dominan.
    context.fillStyle = 'rgba(247, 250, 246, 0.28)'
    context.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT)
    drawOutline(context, originPoint, false)
  } else {
    context.fillStyle = '#eef5ed'
    context.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT)
    context.strokeStyle = 'rgba(157, 181, 167, 0.25)'
    context.lineWidth = 1
    for (let x = 0; x <= CANVAS_WIDTH; x += 90) {
      context.beginPath()
      context.moveTo(x, 0)
      context.lineTo(x, CANVAS_HEIGHT)
      context.stroke()
    }
    for (let y = 0; y <= CANVAS_HEIGHT; y += 90) {
      context.beginPath()
      context.moveTo(0, y)
      context.lineTo(CANVAS_WIDTH, y)
      context.stroke()
    }
    drawOutline(context, originPoint, true)
  }

  drawRegionLabels(context, originPoint)
  drawPoints(context, originPoint, visiblePoints, seriesColors)
  drawLegend(context, series, counts, title)

  context.strokeStyle = 'rgba(20, 61, 48, 0.18)'
  context.lineWidth = 3
  context.strokeRect(1.5, 1.5, CANVAS_WIDTH - 3, CANVAS_HEIGHT - 3)

  const blob = await new Promise<Blob | null>((resolve) => {
    canvas.toBlob((value) => resolve(value), 'image/png')
  })
  if (!blob) return null
  return new File([blob], 'peta-gatur-polidik.png', { type: 'image/png' })
}
