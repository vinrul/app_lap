export interface ShapeDefinition {
  kind: string
  label: string
  points?: string
  path?: string
  ellipse?: boolean
}

export const shapeCatalog: ShapeDefinition[] = [
  { kind: 'rect', label: 'Kotak' },
  { kind: 'ellipse', label: 'Lingkaran / Elips', ellipse: true },
  { kind: 'triangle', label: 'Segitiga', points: '50,2 98,98 2,98' },
  { kind: 'right-triangle', label: 'Segitiga siku', points: '2,2 98,98 2,98' },
  { kind: 'diamond', label: 'Belah ketupat', points: '50,1 99,50 50,99 1,50' },
  { kind: 'parallelogram', label: 'Jajar genjang', points: '25,2 100,2 75,98 0,98' },
  { kind: 'trapezoid', label: 'Trapesium', points: '22,4 78,4 99,96 1,96' },
  { kind: 'pentagon', label: 'Segi lima', points: '50,2 95.65,35.16 78.2,88.83 21.8,88.83 4.35,35.16' },
  { kind: 'hexagon', label: 'Segi enam', points: '50,2 91.6,26 91.6,74 50,98 8.4,74 8.4,26' },
  {
    kind: 'star',
    label: 'Bintang',
    points:
      '50,0 61.81,38.2 100,38.2 69.1,61.8 80.92,100 50,76.4 19.09,100 30.9,61.8 0,38.2 38.19,38.2',
  },
  { kind: 'arrow', label: 'Panah', points: '0,30 60,30 60,8 100,50 60,92 60,70 0,70' },
  {
    kind: 'plus',
    label: 'Plus',
    points: '35,0 65,0 65,35 100,35 100,65 65,65 65,100 35,100 35,65 0,65 0,35 35,35',
  },
  {
    kind: 'heart',
    label: 'Hati',
    path: 'M50 88 C20 68 4 50 4 30 C4 14 16 4 30 4 C39 4 46 9 50 16 C54 9 61 4 70 4 C84 4 96 14 96 30 C96 50 80 68 50 88 Z',
  },
]

const shapeByKind = new Map(shapeCatalog.map((shape) => [shape.kind, shape]))

export function shapeDefinition(kind: string | undefined): ShapeDefinition {
  return shapeByKind.get(kind ?? '') ?? shapeCatalog[0]
}

export function shapeLabel(kind: string | undefined): string {
  return shapeDefinition(kind).label
}
