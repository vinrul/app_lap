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
