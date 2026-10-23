import type { ReportContent } from '../types'

export interface ReportSectionDefinition {
  key: keyof ReportContent
  number: string
  label: string
  hint: string
  placeholder: string
  rows: number
}

export const reportSections: ReportSectionDefinition[] = [
  {
    key: 'summary',
    number: '01',
    label: 'Ringkasan minggu',
    hint: 'Gambaran singkat pelaksanaan dan sorotan utama periode.',
    placeholder: 'Tuliskan ringkasan pelaksanaan Program Prioritas dan Quick Wins minggu ini…',
    rows: 4,
  },
  {
    key: 'narkoba',
    number: '02',
    label: 'Berantas narkoba',
    hint: 'Konteks penindakan, hasil, dan rencana pemulihan kawasan.',
    placeholder: 'Contoh: Penindakan difokuskan pada kawasan rawan di …',
    rows: 4,
  },
  {
    key: 'green_policing',
    number: '03',
    label: 'Green policing',
    hint: 'Lokasi prioritas, jenis tanaman, kolaborasi, dan pemeliharaan.',
    placeholder: 'Uraikan kegiatan penghijauan dan kerja bakti yang menonjol…',
    rows: 4,
  },
  {
    key: 'jumat_keliling',
    number: '04',
    label: 'Jumat keliling',
    hint: 'Pesan kamtibmas, tokoh yang ditemui, dan aspirasi warga.',
    placeholder: 'Catat masjid utama, jumlah jemaah, dan pesan yang disampaikan…',
    rows: 4,
  },
  {
    key: 'subuh_keliling',
    number: '05',
    label: 'Subuh keliling',
    hint: 'Rangkuman kegiatan subuh dan aspirasi yang diserap.',
    placeholder: 'Catat pelaksanaan Subuh Keliling dan tindak lanjut awal…',
    rows: 4,
  },
  {
    key: 'minggu_kasih',
    number: '06',
    label: 'Minggu kasih',
    hint: 'Keragaman tempat ibadah, jemaat, dan tali asih.',
    placeholder: 'Uraikan kegiatan Minggu Kasih lintas agama dan pesan kamtibmas…',
    rows: 4,
  },
  {
    key: 'aspirations',
    number: '07',
    label: 'Aspirasi masyarakat',
    hint: 'Tema aspirasi dan rencana tindak lanjut prioritas.',
    placeholder: 'Rangkum tema aspirasi yang muncul dan rencana tindak lanjutnya…',
    rows: 4,
  },
  {
    key: 'follow_up',
    number: '08',
    label: 'Tindak lanjut / KRYD',
    hint: 'Kegiatan gabungan lintas fungsi dan instansi.',
    placeholder: 'Uraikan kegiatan gabungan, KRYD, dan koordinasi lintas instansi…',
    rows: 4,
  },
  {
    key: 'media',
    number: '09',
    label: 'Amplifikasi media',
    hint: 'Kanal pemberitaan dan strategi publikasi.',
    placeholder: 'Catat kanal dominan dan kegiatan yang paling banyak diberitakan…',
    rows: 3,
  },
  {
    key: 'closing',
    number: '10',
    label: 'Penutup',
    hint: 'Kalimat penutup laporan dan arahan tindak lanjut.',
    placeholder: 'Demikian laporan mingguan ini disusun sebagai bahan evaluasi pimpinan…',
    rows: 3,
  },
]

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
