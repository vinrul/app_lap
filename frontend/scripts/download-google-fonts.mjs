import fs from 'node:fs'
import path from 'node:path'

// Font Google Fonts yang dibundel. Jalankan ulang dengan:
//   bun scripts/download-google-fonts.mjs
const FONTS = [
  {
    key: 'inter',
    label: 'Inter',
    category: 'Sans',
    stack: "'Inter', 'Segoe UI', Arial, sans-serif",
    query: 'Inter:ital,wght@0,400;0,700;1,400;1,700',
  },
  {
    key: 'roboto',
    label: 'Roboto',
    category: 'Sans',
    stack: "'Roboto', 'Segoe UI', Arial, sans-serif",
    query: 'Roboto:ital,wght@0,400;0,700;1,400;1,700',
  },
  {
    key: 'open-sans',
    label: 'Open Sans',
    category: 'Sans',
    stack: "'Open Sans', 'Segoe UI', Arial, sans-serif",
    query: 'Open+Sans:ital,wght@0,400;0,700;1,400;1,700',
  },
  {
    key: 'montserrat',
    label: 'Montserrat',
    category: 'Sans',
    stack: "'Montserrat', 'Segoe UI', Arial, sans-serif",
    query: 'Montserrat:ital,wght@0,400;0,700;1,400;1,700',
  },
  {
    key: 'lora',
    label: 'Lora',
    category: 'Serif',
    stack: "'Lora', Georgia, 'Times New Roman', serif",
    query: 'Lora:ital,wght@0,400;0,700;1,400;1,700',
  },
  {
    key: 'merriweather',
    label: 'Merriweather',
    category: 'Serif',
    stack: "'Merriweather', Georgia, 'Times New Roman', serif",
    query: 'Merriweather:ital,wght@0,400;0,700;1,400;1,700',
  },
  {
    key: 'playfair-display',
    label: 'Playfair Display',
    category: 'Serif',
    stack: "'Playfair Display', Georgia, 'Times New Roman', serif",
    query: 'Playfair+Display:ital,wght@0,400;0,700;1,400;1,700',
  },
  {
    key: 'roboto-slab',
    label: 'Roboto Slab',
    category: 'Serif',
    stack: "'Roboto Slab', Georgia, 'Times New Roman', serif",
    query: 'Roboto+Slab:wght@400;700',
  },
]

const root = path.resolve(import.meta.dir, '..')
const projectRoot = path.resolve(root, '..')
const fontDir = path.join(projectRoot, 'backend', 'assets', 'fonts')
const backendOut = path.join(projectRoot, 'backend', 'src', 'modules', 'reports', 'fonts.rs')
const frontendOut = path.join(root, 'src', 'lib', 'fonts', 'catalog.ts')

const USER_AGENT =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/155.0.0.0 Safari/537.36'

fs.mkdirSync(fontDir, { recursive: true })
fs.mkdirSync(path.dirname(backendOut), { recursive: true })
fs.mkdirSync(path.dirname(frontendOut), { recursive: true })

async function fetchText(url) {
  const response = await fetch(url, { headers: { 'User-Agent': USER_AGENT } })
  if (!response.ok) throw new Error(`${response.status} ${response.statusText} for ${url}`)
  return response.text()
}

function latinFaces(css, font) {
  const faces = []
  const regex = /\/\*\s*latin\s*\*\/\s*@font-face\s*\{([^}]+)\}/g
  let match
  while ((match = regex.exec(css))) {
    const block = match[1]
    const weight = block.match(/font-weight:\s*(\d+)/)?.[1]
    const style = block.match(/font-style:\s*(normal|italic)/)?.[1]
    const url = block.match(/src:\s*url\((https:[^)]+)\)/)?.[1]
    if (!weight || !style || !url) continue
    faces.push({ weight: Number(weight), style, url })
  }
  if (!faces.length) throw new Error(`no latin faces for ${font.key}`)
  return faces
}

const downloaded = []

for (const font of FONTS) {
  const cssUrl = `https://fonts.googleapis.com/css2?family=${font.query}&display=swap`
  const css = await fetchText(cssUrl)
  const faces = latinFaces(css, font)
  const wanted = [
    { weight: 400, style: 'normal', suffix: '400' },
    { weight: 700, style: 'normal', suffix: '700' },
    { weight: 400, style: 'italic', suffix: '400i' },
    { weight: 700, style: 'italic', suffix: '700i' },
  ]
  const buffers = {}
  for (const target of wanted) {
    const face = faces.find(
      (item) => item.weight === target.weight && item.style === target.style,
    )
    if (!face) {
      buffers[target.suffix] = null
      continue
    }
    const response = await fetch(face.url, { headers: { 'User-Agent': USER_AGENT } })
    if (!response.ok) throw new Error(`font download failed: ${face.url}`)
    buffers[target.suffix] = Buffer.from(await response.arrayBuffer())
    console.log(`${font.key} ${target.suffix}: ${buffers[target.suffix].length} bytes`)
  }
  font.buffers = buffers
}

for (const font of FONTS) {
  font.variable = Boolean(
    font.buffers['400'] && font.buffers['700'] && font.buffers['400'].equals(font.buffers['700']),
  )
}

for (const font of FONTS) {
  font.files = {}
  const save = (suffix, filename, bytes) => {
    fs.writeFileSync(path.join(fontDir, filename), bytes)
    font.files[suffix] = filename
    downloaded.push({ font: font.key, file: filename, bytes: bytes.length })
  }
  for (const [regularSuffix, boldSuffix, base] of [
    ['400', '700', '400'],
    ['400i', '700i', '400i'],
  ]) {
    const regular = font.buffers[regularSuffix]
    const bold = font.buffers[boldSuffix]
    if (!regular) continue
    const filename = `${font.key}-${base}.woff2`
    save(regularSuffix, filename, regular)
    if (bold && !regular.equals(bold)) {
      save(boldSuffix, `${font.key}-${base.replace('400', '700')}.woff2`, bold)
    } else {
      font.files[boldSuffix] = filename
    }
  }
}

const bytesConst = (filename) =>
  filename
    ? `include_bytes!("../../../assets/fonts/${filename}")`
    : '&[]'

const backend = `// Generated by frontend/scripts/download-google-fonts.mjs. Jangan edit manual.
use std::sync::OnceLock;

use base64::{Engine as _, engine::general_purpose::STANDARD};

pub struct FontDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub stack: &'static str,
    pub variable: bool,
    pub regular: &'static [u8],
    pub bold: &'static [u8],
    pub italic: &'static [u8],
    pub bold_italic: &'static [u8],
}

pub const FONTS: &[FontDefinition] = &[
${FONTS.map(
  (font) => `    FontDefinition {
        key: ${JSON.stringify(font.key)},
        label: ${JSON.stringify(font.label)},
        category: ${JSON.stringify(font.category)},
        stack: ${JSON.stringify(font.stack)},
        variable: ${font.variable},
        regular: ${bytesConst(font.files['400'])},
        bold: ${bytesConst(font.files['700'])},
        italic: ${bytesConst(font.files['400i'])},
        bold_italic: ${bytesConst(font.files['700i'])},
    },`,
).join('\n')}
];

pub fn find(key: &str) -> Option<&'static FontDefinition> {
    FONTS.iter().find(|font| font.key == key)
}

pub fn stack(key: &str) -> &'static str {
    match key {
        "serif" => "Georgia, 'Times New Roman', serif",
        "sans" => "'Segoe UI', Arial, sans-serif",
        _ => find(key)
            .map(|font| font.stack)
            .unwrap_or("'Segoe UI', Arial, sans-serif"),
    }
}

pub fn face_css() -> &'static str {
    static CSS: OnceLock<String> = OnceLock::new();
    CSS.get_or_init(|| {
        let mut css = String::new();
        for font in FONTS {
            css.push_str(&format!("/* {} - {} */\\n", font.category, font.label));
            for (style, regular, bold) in [
                ("normal", font.regular, font.bold),
                ("italic", font.italic, font.bold_italic),
            ] {
                if regular.is_empty() {
                    continue;
                }
                if font.variable {
                    css.push_str(&format!(
                        "@font-face {{ font-family: '{}'; src: url(data:font/woff2;base64,{}) format('woff2'); font-weight: 100 900; font-style: {}; font-display: swap; }}\n",
                        font.label,
                        STANDARD.encode(regular),
                        style
                    ));
                } else if !bold.is_empty() {
                    css.push_str(&format!(
                        "@font-face {{ font-family: '{}'; src: url(data:font/woff2;base64,{}) format('woff2'); font-weight: 400; font-style: {}; font-display: swap; }}\n",
                        font.label,
                        STANDARD.encode(regular),
                        style
                    ));
                    css.push_str(&format!(
                        "@font-face {{ font-family: '{}'; src: url(data:font/woff2;base64,{}) format('woff2'); font-weight: 700; font-style: {}; font-display: swap; }}\n",
                        font.label,
                        STANDARD.encode(bold),
                        style
                    ));
                } else {
                    css.push_str(&format!(
                        "@font-face {{ font-family: '{}'; src: url(data:font/woff2;base64,{}) format('woff2'); font-weight: 400; font-style: {}; font-display: swap; }}\n",
                        font.label,
                        STANDARD.encode(regular),
                        style
                    ));
                }
            }
        }
        css
    })
}
`
fs.writeFileSync(backendOut, backend)

const frontend = `// Generated by scripts/download-google-fonts.mjs. Jangan edit manual.
export interface FontOption {
  key: string
  label: string
  category: 'Sistem' | 'Sans' | 'Serif'
  stack: string
}

export const fontOptions: FontOption[] = [
  { key: 'sans', label: 'Sans (sistem)', category: 'Sistem', stack: "'Segoe UI', Arial, sans-serif" },
  { key: 'serif', label: 'Serif (sistem)', category: 'Sistem', stack: "Georgia, 'Times New Roman', serif" },
${FONTS.map(
  (font) =>
    `  { key: ${JSON.stringify(font.key)}, label: ${JSON.stringify(font.label)}, category: ${JSON.stringify(font.category)}, stack: ${JSON.stringify(font.stack)} },`,
).join('\n')}
]

export const fontByKey = new Map(fontOptions.map((font) => [font.key, font]))

export function fontStack(key: string | undefined): string {
  return fontByKey.get(key ?? '')?.stack ?? fontOptions[0].stack
}
`
fs.writeFileSync(frontendOut, frontend)

const license = `# Bundled Google Fonts

This directory contains WOFF2 files downloaded from Google Fonts for offline
report rendering. Font families and licenses:

- Inter — SIL Open Font License 1.1
- Roboto — Apache License 2.0
- Open Sans — SIL Open Font License 1.1
- Montserrat — SIL Open Font License 1.1
- Lora — SIL Open Font License 1.1
- Merriweather — SIL Open Font License 1.1
- Playfair Display — SIL Open Font License 1.1
- Roboto Slab — Apache License 2.0

Source: https://fonts.google.com

The SIL Open Font License 1.1 text is available at
https://openfontlicense.org/open-font-license-official-text/
and the Apache License 2.0 at https://www.apache.org/licenses/LICENSE-2.0.
`
fs.writeFileSync(path.join(fontDir, 'README.md'), license)

const total = downloaded.reduce((sum, item) => sum + item.bytes, 0)
console.log(`Downloaded ${downloaded.length} files, ${total} bytes total.`)
console.log(backendOut)
console.log(frontendOut)
