import type { ReportContent } from '../types'

export const quickWinsLabels: Record<string, string> = {
  QW1: 'Pelayanan prima & kegiatan rutin',
  QW2: 'Berantas narkoba',
  QW3: 'Perang digital',
  QW4: 'Pamapta hadir & respons cepat',
  QW5: 'Pelayanan reskrim',
  QW6: 'Propam presisi & antipungli',
  QW7: 'Bhabinkamtibmas & polsek siaga',
  QW8: 'Kejahatan ekonomi & KUHP baru',
}

export const qwGroupTitles: Record<string, string> = {
  QW1: 'Ruang Publik Aman',
  QW2: 'Pemulihan Kawasan Narkoba',
  QW3: 'Bersih Judi Daring dan Penipuan Digital',
  QW4: '110 Respons Cepat, SPKT Ramah Korban',
  QW5: 'Perkara Terlacak dan SP2HP Pasti',
  QW6: 'Bersih ke Dalam',
  QW7: 'Polres Kuat, Polsek Tangguh',
  QW8: 'KUHP/KUHAP Siap dan Scientific Crime Investigation',
}

export const qwCodeTitles: Record<string, string> = {
  'QW1.1': 'Gatur Pagi (Pengaturan Lalu Lintas Senin Pagi oleh Ka/Waka)',
  'QW1.2': 'Polisi Peduli Pendidikan (Polidik)',
  'QW1.3': 'Polisi Peduli Pengangguran (Poliran)',
  'QW1.4': 'Kumpul Bersama Masyarakat',
  'QW1.5': 'Green Policing',
  'QW1.6': 'Jumat Keliling (Jumling)',
  'QW1.7': 'Subuh Keliling Bersama Masyarakat (Suling)',
  'QW1.8': 'Minggu Kasih',
  'QW1.9': 'Patroli Siaga Reserse (Pagar)',
  'QW1.10': 'Patroli Sabhara',
  'QW1.11': 'Patroli Polisi Perairan',
  'QW1.12': 'Pencegahan ekstremisme pada anak',
  'QW1.13': 'Polisi Peduli Pangan',
  'QW1.14': 'DIMTAQ (Disiplin Iman dan Taqwa)',
  'QW2.1': 'Pemberantasan dan pemulihan kawasan narkoba',
  'QW2.2': 'Pembinaan kawasan bersama BNN dan Pemda',
  'QW2.3': 'Rujukan rehabilitasi pengguna',
  'QW2.4': 'Penyuluhan bahaya narkoba',
  'QW3.1': 'Penegakan hukum judi dan penipuan di ruang digital',
  'QW3.2': 'Layanan aduan korban penipuan daring',
  'QW3.3': 'Edukasi anti judi daring dan penipuan',
  'QW3.4': 'Patroli siber',
  'QW4.1': 'Pamapta Hadir dan Penanganan ke TKP',
  'QW4.2': 'Petugas Yanmas Melayani Laporan dan Aduan di SPKT',
  'QW4.3': 'Respons Cepat Panggilan 110',
  'QW4.4': 'Layanan Ramah Perempuan dan Anak',
  'QW5.1': 'SP2HP Berkala kepada Pelapor',
  'QW5.2': 'Layanan Informasi Perkara',
  'QW5.3': 'Gelar Perkara atas Pengaduan Warga',
  'QW5.4': 'Keadilan Restoratif Perkara Ringan',
  'QW6.1': 'Layanan SIM, SKCK, dan Tilang tanpa Pungli',
  'QW6.2': 'Copot Hari Itu Juga',
  'QW6.3': 'Kanal Pengaduan Propam/Dumas',
  'QW7.1': 'Satu Desa Satu Bhabin',
  'QW7.2': 'Penataan Personel',
  'QW7.3': 'Polsek Siaga 24 Jam',
  'QW7.4': 'Perkara Ringan dan Rembuk Warga',
  'QW8.1': 'Penanganan BBM Ilegal dan Kejahatan Ekonomi',
  'QW8.2': 'Penanganan TPPO, Korupsi, Penyelundupan dan Pencucian Uang',
  'QW8.3': 'Penyuluhan KUHP Baru kepada Masyarakat',
  'QW8.4': 'Olah TKP Ilmiah dan Identifikasi Korban',
}

export function normalizeQuickWinCode(value: string): string {
  return value
    .trim()
    .toUpperCase()
    .replace(/\s+/g, '')
    .replace(/—/g, '.')
}

export function quickWinCodeParts(value: string): { code: string; title: string } {
  const raw = value.trim()
  const separator = raw.indexOf('·')
  const rawCode = separator >= 0 ? raw.slice(0, separator) : raw
  const inlineTitle = separator >= 0 ? raw.slice(separator + 1).trim() : ''
  const normalized = normalizeQuickWinCode(rawCode)
  const code = /^QW\d+(?:\.\d+)?$/.test(normalized) ? normalized : rawCode.trim()
  return {
    code,
    title: inlineTitle || qwCodeTitles[code] || '',
  }
}

export function isQuickWinCode(value: string): boolean {
  return /^QW\d+\.\d+$/.test(quickWinCodeParts(value).code)
}

export const emptyContent: ReportContent = {
  summary: '',
  narkoba: '',
  green_policing: '',
  jumat_keliling: '',
  subuh_keliling: '',
  minggu_kasih: '',
  aspirations: '',
  follow_up: '',
  media: '',
  closing: '',
  signatory_name: '',
  signatory_rank: '',
  signatory_nip: '',
  report_city: 'Denpasar',
  report_date: '',
}

export const navItems = [
  { id: 'dashboard', label: 'Ringkasan', path: '/dashboard', icon: 'dashboard' },
  { id: 'periods', label: 'Periode laporan', path: '/periods', icon: 'periods' },
  { id: 'users', label: 'Pengguna & akses', path: '/users', icon: 'users', adminOnly: true },
] as const
