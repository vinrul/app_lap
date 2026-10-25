export type CardTemplateId =
  | 'classic'
  | 'icon_right'
  | 'icon_left'
  | 'centered'
  | 'split'
  | 'minimal'

export type CardIconLayout = 'none' | 'right' | 'left' | 'top' | 'panel'

export interface CardTemplateDefinition {
  id: CardTemplateId
  label: string
  description: string
  layout: CardIconLayout
  fill: string
  accent: string
  color: string
  iconBg: string
  iconColor: string
  borderWidth: number
  radius: number
  icon: string
}

export const cardTemplates: CardTemplateDefinition[] = [
  {
    id: 'classic',
    label: 'Klasik',
    description: 'Aksen kiri, label di atas nilai',
    layout: 'none',
    fill: '#f6f8f5',
    accent: '#b8894b',
    color: '#143d30',
    iconBg: '#f1e6d5',
    iconColor: '#b8894b',
    borderWidth: 2,
    radius: 1.5,
    icon: '',
  },
  {
    id: 'icon_right',
    label: 'Ikon kanan',
    description: 'Nilai besar dengan ikon bulat di kanan',
    layout: 'right',
    fill: '#f9ecec',
    accent: '#b43c3c',
    color: '#a33434',
    iconBg: '#b43c3c',
    iconColor: '#ffffff',
    borderWidth: 0,
    radius: 2.5,
    icon: 'shield',
  },
  {
    id: 'icon_left',
    label: 'Ikon kiri',
    description: 'Ikon di kiri, nilai dan label di kanan',
    layout: 'left',
    fill: '#eef2f7',
    accent: '#c23b3b',
    color: '#14324a',
    iconBg: '#eef2f7',
    iconColor: '#c23b3b',
    borderWidth: 0,
    radius: 2.5,
    icon: 'syringe',
  },
  {
    id: 'centered',
    label: 'Tengah',
    description: 'Ikon di atas, nilai dan label di tengah',
    layout: 'top',
    fill: '#ffffff',
    accent: '#2f6b55',
    color: '#143d30',
    iconBg: '#e8f2ec',
    iconColor: '#2f6b55',
    borderWidth: 0.5,
    radius: 2.5,
    icon: 'chart-column',
  },
  {
    id: 'split',
    label: 'Panel ikon',
    description: 'Panel aksen di kiri dengan ikon besar',
    layout: 'panel',
    fill: '#f7f5ef',
    accent: '#b8894b',
    color: '#143d30',
    iconBg: '#ffffff',
    iconColor: '#b8894b',
    borderWidth: 0,
    radius: 2,
    icon: 'shield-check',
  },
  {
    id: 'minimal',
    label: 'Minimal',
    description: 'Tanpa kotak penuh, garis aksen di bawah',
    layout: 'none',
    fill: '#ffffff',
    accent: '#b8894b',
    color: '#143d30',
    iconBg: '#f1e6d5',
    iconColor: '#b8894b',
    borderWidth: 1,
    radius: 0,
    icon: '',
  },
]

export function cardTemplate(id: string | undefined): CardTemplateDefinition {
  return cardTemplates.find((item) => item.id === id) ?? cardTemplates[0]
}
