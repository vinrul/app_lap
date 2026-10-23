import fs from 'node:fs'
import path from 'node:path'

// Katalog ikon Lucide yang dibundel. Tambah entri di sini lalu jalankan:
//   bun scripts/generate-icon-catalog.mjs
const ICONS = [
  // Umum
  ['shield', 'Perisai', 'Umum'],
  ['shield-check', 'Perisai terverifikasi', 'Umum'],
  ['shield-alert', 'Perisai peringatan', 'Umum'],
  ['shield-half', 'Perisai separuh', 'Umum'],
  ['badge-check', 'Lencana centang', 'Umum'],
  ['award', 'Penghargaan', 'Umum'],
  ['flag', 'Bendera', 'Umum'],
  ['target', 'Sasaran', 'Umum'],
  ['star', 'Bintang', 'Umum'],
  ['check', 'Centang', 'Umum'],
  ['circle-check-big', 'Centang lingkaran', 'Umum'],
  ['circle-x', 'Silang lingkaran', 'Umum'],
  ['triangle-alert', 'Peringatan', 'Umum'],
  ['info', 'Informasi', 'Umum'],
  ['badge-info', 'Info lencana', 'Umum'],
  ['scale', 'Timbangan', 'Umum'],
  ['handshake', 'Jabat tangan', 'Umum'],
  ['calendar', 'Kalender', 'Umum'],
  ['clock', 'Jam', 'Umum'],
  ['map-pin', 'Pin lokasi', 'Umum'],
  ['map-pinned', 'Lokasi tersemat', 'Umum'],
  ['users', 'Pengguna', 'Umum'],
  ['user-round', 'Profil', 'Umum'],
  ['users-round', 'Grup pengguna', 'Umum'],
  ['building-2', 'Gedung', 'Umum'],
  ['landmark', 'Markah tanah', 'Umum'],
  ['file-text', 'Dokumen', 'Umum'],
  ['files', 'Berkas', 'Umum'],
  ['clipboard-list', 'Daftar tugas', 'Umum'],
  ['clipboard-check', 'Daftar selesai', 'Umum'],
  ['folder', 'Folder', 'Umum'],
  ['mail', 'Surel', 'Umum'],
  ['phone', 'Telepon', 'Umum'],
  ['link', 'Tautan', 'Umum'],
  ['external-link', 'Tautan luar', 'Umum'],
  ['search', 'Pencarian', 'Umum'],
  ['settings', 'Pengaturan', 'Umum'],
  ['plus', 'Tambah', 'Umum'],
  ['minus', 'Kurang', 'Umum'],
  ['x', 'Tutup', 'Umum'],
  ['arrow-right', 'Panah kanan', 'Umum'],
  ['arrow-up-right', 'Panah naik kanan', 'Umum'],
  ['chevron-right', 'Chevron kanan', 'Umum'],
  ['eye', 'Mata', 'Umum'],
  ['lock', 'Gembok', 'Umum'],
  ['key', 'Kunci', 'Umum'],
  ['menu', 'Menu', 'Umum'],
  ['ellipsis', 'Lainnya', 'Umum'],

  // Statistik
  ['chart-bar', 'Grafik batang', 'Statistik'],
  ['chart-column', 'Grafik kolom', 'Statistik'],
  ['chart-line', 'Grafik garis', 'Statistik'],
  ['chart-pie', 'Grafik pie', 'Statistik'],
  ['chart-area', 'Grafik area', 'Statistik'],
  ['trending-up', 'Tren naik', 'Statistik'],
  ['trending-down', 'Tren turun', 'Statistik'],
  ['activity', 'Aktivitas', 'Statistik'],
  ['gauge', 'Pengukur', 'Statistik'],
  ['percent', 'Persen', 'Statistik'],
  ['calculator', 'Kalkulator', 'Statistik'],

  // Lingkungan
  ['leaf', 'Daun', 'Lingkungan'],
  ['sprout', 'Tunas', 'Lingkungan'],
  ['tree-pine', 'Pohon pinus', 'Lingkungan'],
  ['flower-2', 'Bunga', 'Lingkungan'],
  ['recycle', 'Daur ulang', 'Lingkungan'],
  ['droplet', 'Tetesan air', 'Lingkungan'],
  ['flame', 'Api', 'Lingkungan'],
  ['cloud-rain', 'Hujan', 'Lingkungan'],
  ['wind', 'Angin', 'Lingkungan'],
  ['mountain', 'Gunung', 'Lingkungan'],
  ['waves', 'Gelombang', 'Lingkungan'],
  ['sun', 'Matahari', 'Lingkungan'],
  ['sunrise', 'Matahari terbit', 'Lingkungan'],
  ['sunset', 'Matahari terbenam', 'Lingkungan'],
  ['moon', 'Bulan', 'Lingkungan'],
  ['globe', 'Bumi', 'Lingkungan'],

  // Sosial & media
  ['hand-heart', 'Tangan hati', 'Sosial & Media'],
  ['heart-handshake', 'Hati berjabat', 'Sosial & Media'],
  ['heart', 'Hati', 'Sosial & Media'],
  ['heart-pulse', 'Denyut jantung', 'Sosial & Media'],
  ['smile', 'Senyum', 'Sosial & Media'],
  ['thumbs-up', 'Jempol', 'Sosial & Media'],
  ['megaphone', 'Megafon', 'Sosial & Media'],
  ['newspaper', 'Koran', 'Sosial & Media'],
  ['radio', 'Radio', 'Sosial & Media'],
  ['tv', 'Televisi', 'Sosial & Media'],
  ['smartphone', 'Ponsel', 'Sosial & Media'],
  ['camera', 'Kamera', 'Sosial & Media'],
  ['image', 'Gambar', 'Sosial & Media'],
  ['video', 'Video', 'Sosial & Media'],
  ['headphones', 'Headphone', 'Sosial & Media'],
  ['presentation', 'Presentasi', 'Sosial & Media'],
  ['mic', 'Mikrofon', 'Sosial & Media'],
  ['message-circle', 'Pesan', 'Sosial & Media'],
  ['message-square', 'Pesan kotak', 'Sosial & Media'],
  ['send', 'Kirim', 'Sosial & Media'],
  ['share-2', 'Bagikan', 'Sosial & Media'],
  ['book-open', 'Buku terbuka', 'Sosial & Media'],
  ['graduation-cap', 'Topi wisuda', 'Sosial & Media'],
  ['church', 'Gereja', 'Sosial & Media'],
  ['person-standing', 'Orang berdiri', 'Sosial & Media'],
  ['baby', 'Bayi', 'Sosial & Media'],
  ['accessibility', 'Aksesibilitas', 'Sosial & Media'],

  // Transportasi & lokasi
  ['car', 'Mobil', 'Transportasi & Lokasi'],
  ['bike', 'Sepeda', 'Transportasi & Lokasi'],
  ['ship', 'Kapal', 'Transportasi & Lokasi'],
  ['plane', 'Pesawat', 'Transportasi & Lokasi'],
  ['truck', 'Truk', 'Transportasi & Lokasi'],
  ['route', 'Rute', 'Transportasi & Lokasi'],
  ['compass', 'Kompas', 'Transportasi & Lokasi'],
  ['map', 'Peta', 'Transportasi & Lokasi'],
  ['navigation', 'Navigasi', 'Transportasi & Lokasi'],
  ['house', 'Rumah', 'Transportasi & Lokasi'],
  ['warehouse', 'Gudang', 'Transportasi & Lokasi'],
  ['factory', 'Pabrik', 'Transportasi & Lokasi'],

  // Kesehatan & keselamatan
  ['siren', 'Sirene', 'Kesehatan & Keselamatan'],
  ['hospital', 'Rumah sakit', 'Kesehatan & Keselamatan'],
  ['stethoscope', 'Stetoskop', 'Kesehatan & Keselamatan'],
  ['pill', 'Pil', 'Kesehatan & Keselamatan'],
  ['syringe', 'Suntikan', 'Kesehatan & Keselamatan'],
  ['microscope', 'Mikroskop', 'Kesehatan & Keselamatan'],
  ['dna', 'DNA', 'Kesehatan & Keselamatan'],
  ['brain', 'Otak', 'Kesehatan & Keselamatan'],

  // Keuangan & data
  ['banknote', 'Uang', 'Keuangan & Data'],
  ['coins', 'Koin', 'Keuangan & Data'],
  ['wallet', 'Dompet', 'Keuangan & Data'],
  ['piggy-bank', 'Celengan', 'Keuangan & Data'],
  ['package', 'Paket', 'Keuangan & Data'],
  ['lightbulb', 'Lampu ide', 'Keuangan & Data'],
  ['zap', 'Petir', 'Keuangan & Data'],
  ['plug', 'Steker', 'Keuangan & Data'],
  ['wifi', 'Sinyal WiFi', 'Keuangan & Data'],
  ['signal', 'Sinyal', 'Keuangan & Data'],
  ['scan', 'Pemindaian', 'Keuangan & Data'],
  ['crosshair', 'Bidik', 'Keuangan & Data'],
  ['focus', 'Fokus', 'Keuangan & Data'],
  ['rocket', 'Roket', 'Keuangan & Data'],
  ['radar', 'Radar', 'Keuangan & Data'],
]

const root = path.resolve(import.meta.dir, '..')
const iconsDir = path.join(root, 'node_modules', 'lucide-svelte', 'dist', 'icons')
const frontendOut = path.join(root, 'src', 'lib', 'icons', 'catalog.ts')
const backendOut = path.resolve(
  root,
  '..',
  'backend',
  'src',
  'modules',
  'reports',
  'icons.rs',
)

const allowedTags = new Set(['path', 'circle', 'line', 'polyline', 'polygon', 'rect', 'ellipse'])
const entries = []
const missing = []

for (const [name, label, category] of ICONS) {
  const file = path.join(iconsDir, `${name}.svelte`)
  if (!fs.existsSync(file)) {
    missing.push(name)
    continue
  }
  const source = fs.readFileSync(file, 'utf8')
  const match = source.match(/const iconNode = (\[[\s\S]*?\]);/)
  if (!match) {
    missing.push(`${name} (no iconNode)`)
    continue
  }
  let nodes
  try {
    nodes = Function(`return (${match[1]})`)()
  } catch (error) {
    missing.push(`${name} (parse: ${error.message})`)
    continue
  }
  const svg = nodes
    .map(([tag, attrs]) => {
      if (!allowedTags.has(tag)) throw new Error(`${name}: unsupported tag ${tag}`)
      const attributes = Object.entries(attrs)
        .map(
          ([key, value]) =>
            `${key}="${String(value)
              .replaceAll('&', '&amp;')
              .replaceAll('"', '&quot;')}"`,
        )
        .join(' ')
      return `<${tag}${attributes ? ` ${attributes}` : ''}/>`
    })
    .join('')
  entries.push({ name, label, category, svg })
}

if (missing.length) {
  console.error('Ikon tidak ditemukan/tidak valid:', missing.join(', '))
  process.exit(1)
}

fs.mkdirSync(path.dirname(frontendOut), { recursive: true })
fs.mkdirSync(path.dirname(backendOut), { recursive: true })

const categories = [...new Set(entries.map((entry) => entry.category))]
const frontend = `// Generated by scripts/generate-icon-catalog.mjs. Jangan edit manual.
export interface IconDefinition {
  name: string
  label: string
  category: (${categories.map((value) => JSON.stringify(value)).join(' | ')})
  svg: string
}

export const iconCatalog: IconDefinition[] = [
${entries
  .map(
    (entry) =>
      `  { name: ${JSON.stringify(entry.name)}, label: ${JSON.stringify(entry.label)}, category: ${JSON.stringify(entry.category)}, svg: ${JSON.stringify(entry.svg)} },`,
  )
  .join('\n')}
]

export const iconByName = new Map(iconCatalog.map((icon) => [icon.name, icon]))
`
fs.writeFileSync(frontendOut, frontend)

const backend = `// Generated by frontend/scripts/generate-icon-catalog.mjs. Jangan edit manual.
pub struct IconDefinition {
    pub name: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub svg: &'static str,
}

pub const ICONS: &[IconDefinition] = &[
${entries
  .map(
    (entry) =>
      `    IconDefinition {\n        name: ${JSON.stringify(entry.name)},\n        label: ${JSON.stringify(entry.label)},\n        category: ${JSON.stringify(entry.category)},\n        svg: r##"${entry.svg}"##,\n    },`,
  )
  .join('\n')}
];

pub fn find(name: &str) -> Option<&'static IconDefinition> {
    ICONS.iter().find(|icon| icon.name == name)
}
`
fs.writeFileSync(backendOut, backend)

console.log(`Generated ${entries.length} icons in ${categories.length} categories.`)
console.log(frontendOut)
console.log(backendOut)
