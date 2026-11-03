//! Ekstraksi EXIF dan OCR untuk analisis bukti dukung gambar.

use std::{collections::HashMap, path::Path, process::Stdio, time::Duration};

use chrono::NaiveDate;
use image::{
    DynamicImage, GenericImageView, GrayImage, ImageFormat, Luma, RgbImage, imageops::FilterType,
};
use tokio::process::Command;

use crate::config::Config;

#[derive(Clone, Debug, Default)]
pub struct ExifData {
    pub datetime: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Membaca EXIF minimal (waktu ambil dan GPS) dari JPEG.
pub fn read_jpeg_exif(bytes: &[u8]) -> ExifData {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return ExifData::default();
    }
    let mut index = 2_usize;
    while index + 4 <= bytes.len() {
        if bytes[index] != 0xFF {
            break;
        }
        let marker = bytes[index + 1];
        if marker == 0xDA || marker == 0xD9 || marker == 0x00 {
            break;
        }
        let length = u16::from_be_bytes([bytes[index + 2], bytes[index + 3]]) as usize;
        if length < 2 || index + 2 + length > bytes.len() {
            break;
        }
        if marker == 0xE1 {
            let payload = &bytes[index + 4..index + 2 + length];
            if payload.len() > 6 && &payload[..6] == b"Exif\0\0" {
                return parse_tiff(&payload[6..]);
            }
        }
        index += 2 + length;
    }
    ExifData::default()
}

fn parse_tiff(data: &[u8]) -> ExifData {
    if data.len() < 8 {
        return ExifData::default();
    }
    let little = match &data[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return ExifData::default(),
    };
    let u16_at = |offset: usize| -> Option<u16> {
        if offset + 2 > data.len() {
            return None;
        }
        Some(if little {
            u16::from_le_bytes([data[offset], data[offset + 1]])
        } else {
            u16::from_be_bytes([data[offset], data[offset + 1]])
        })
    };
    let u32_at = |offset: usize| -> Option<u32> {
        if offset + 4 > data.len() {
            return None;
        }
        Some(if little {
            u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ])
        } else {
            u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ])
        })
    };
    if u16_at(2) != Some(42) {
        return ExifData::default();
    }
    let Some(ifd0) = u32_at(4).map(|value| value as usize) else {
        return ExifData::default();
    };

    let mut result = ExifData::default();
    let mut exif_ifd = None;
    let mut gps_ifd = None;
    for (tag, _kind, _count, value) in entries(ifd0, &u16_at, &u32_at) {
        match tag {
            0x8769 => exif_ifd = Some(value as usize),
            0x8825 => gps_ifd = Some(value as usize),
            _ => {}
        }
    }

    if let Some(offset) = exif_ifd {
        for (tag, _kind, _count, value) in entries(offset, &u16_at, &u32_at) {
            if tag == 0x9003 {
                result.datetime = ascii_at(data, value as usize, 20);
            }
        }
    }

    if let Some(offset) = gps_ifd {
        let mut lat_ref = String::new();
        let mut lon_ref = String::new();
        let mut lat = None;
        let mut lon = None;
        for (tag, kind, count, value) in entries(offset, &u16_at, &u32_at) {
            match tag {
                0x0001 => lat_ref = ascii_inline(value, little),
                0x0003 => lon_ref = ascii_inline(value, little),
                0x0002 if kind == 5 && count >= 3 => lat = rationals(data, value as usize, little),
                0x0004 if kind == 5 && count >= 3 => lon = rationals(data, value as usize, little),
                _ => {}
            }
        }
        if let Some(value) = lat {
            result.latitude = Some(if lat_ref.starts_with('S') {
                -value
            } else {
                value
            });
        }
        if let Some(value) = lon {
            result.longitude = Some(if lon_ref.starts_with('W') {
                -value
            } else {
                value
            });
        }
    }
    result
}

fn entries(
    offset: usize,
    u16_at: &impl Fn(usize) -> Option<u16>,
    u32_at: &impl Fn(usize) -> Option<u32>,
) -> Vec<(u16, u16, u32, u32)> {
    let Some(count) = u16_at(offset) else {
        return Vec::new();
    };
    let mut result = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let base = offset + 2 + index * 12;
        let (Some(tag), Some(kind), Some(items), Some(value)) = (
            u16_at(base),
            u16_at(base + 2),
            u32_at(base + 4),
            u32_at(base + 8),
        ) else {
            break;
        };
        result.push((tag, kind, items, value));
    }
    result
}

fn ascii_at(data: &[u8], offset: usize, limit: usize) -> String {
    if offset >= data.len() {
        return String::new();
    }
    let end = (offset + limit).min(data.len());
    String::from_utf8_lossy(&data[offset..end])
        .trim_matches('\0')
        .trim()
        .to_string()
}

fn ascii_inline(value: u32, little: bool) -> String {
    let bytes = if little {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    };
    String::from_utf8_lossy(&bytes)
        .trim_matches('\0')
        .trim()
        .to_string()
}

fn rationals(data: &[u8], offset: usize, little: bool) -> Option<f64> {
    let mut values = [0_f64; 3];
    for (index, slot) in values.iter_mut().enumerate() {
        let base = offset + index * 8;
        if base + 8 > data.len() {
            return None;
        }
        let read = |bytes: [u8; 4]| {
            if little {
                u32::from_le_bytes(bytes)
            } else {
                u32::from_be_bytes(bytes)
            }
        };
        let numerator = read([data[base], data[base + 1], data[base + 2], data[base + 3]]);
        let denominator = read([
            data[base + 4],
            data[base + 5],
            data[base + 6],
            data[base + 7],
        ]);
        if denominator == 0 {
            return None;
        }
        *slot = numerator as f64 / denominator as f64;
    }
    Some(values[0] + values[1] / 60.0 + values[2] / 3600.0)
}

/// Menjalankan Tesseract pada satu berkas gambar.
///
/// Dua mode halaman dipakai karena watermark foto sangat bervariasi: `--psm 11`
/// (teks tersebar) lebih baik untuk tulisan besar, sedangkan `--psm 6` (satu blok
/// teks) lebih baik untuk stempel kecil di sudut foto. Bila keduanya belum
/// menemukan tanggal, jam, dan alamat, gambar dipraproses (crop bawah,
/// perbesaran, masker merah) lalu di-OCR ulang.
pub async fn ocr(config: &Config, path: &Path) -> Result<String, String> {
    let mut texts = Vec::new();
    let mut errors = Vec::new();
    let mut complete = false;

    for psm in ["11", "6"] {
        match run_pass(config, path, psm).await {
            Ok(pass) => {
                tracing::debug!(
                    psm,
                    confidence = pass.confidence,
                    chars = pass.text.chars().count(),
                    "OCR pass selesai"
                );
                complete = watermark_fields(&pass.text);
                if !pass.text.trim().is_empty() {
                    texts.push(pass.text);
                }
                if complete {
                    break;
                }
            }
            Err(error) => errors.push(format!("psm {psm}: {error}")),
        }
    }

    // Watermark kecil atau berwarna merah sering tidak terbaca pada gambar
    // penuh. Coba versi crop bawah yang diperbesar dan masker merah.
    if !complete {
        for variant in prepare_async(path).await {
            match run_pass(config, variant.path(), "6").await {
                Ok(pass) => {
                    tracing::debug!(
                        mode = variant.label(),
                        confidence = pass.confidence,
                        chars = pass.text.chars().count(),
                        "OCR pass praproses selesai"
                    );
                    complete = watermark_fields(&pass.text);
                    if !pass.text.trim().is_empty() {
                        texts.push(pass.text);
                    }
                    if complete {
                        break;
                    }
                }
                Err(error) => errors.push(format!("{}: {error}", variant.label())),
            }
        }
    }

    let merged = merge_texts(&texts);
    if merged.trim().is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(merged)
}

fn watermark_fields(text: &str) -> bool {
    find_date(text).is_some() && has_time(text) && has_address(text)
}

/// Berkas gambar sementara hasil praproses untuk satu mode OCR.
struct PreparedImage {
    file: tempfile::NamedTempFile,
    red: bool,
}

impl PreparedImage {
    fn path(&self) -> &Path {
        self.file.path()
    }

    fn label(&self) -> &'static str {
        if self.red {
            "masker merah"
        } else {
            "crop bawah"
        }
    }
}

async fn prepare_async(path: &Path) -> Vec<PreparedImage> {
    let source = path.to_path_buf();
    let task_path = source.clone();
    match tokio::task::spawn_blocking(move || preprocess(&task_path)).await {
        Ok(Ok(variants)) => variants,
        Ok(Err(error)) => {
            tracing::warn!(file = %source.display(), %error, "praproses OCR gagal");
            Vec::new()
        }
        Err(error) => {
            tracing::warn!(file = %source.display(), %error, "praproses OCR gagal dijalankan");
            Vec::new()
        }
    }
}

/// Menyiapkan variasi gambar untuk watermark yang sulit dibaca: crop bagian
/// bawah, perbesaran, dan masker merah.
fn preprocess(path: &Path) -> Result<Vec<PreparedImage>, String> {
    let image = image::ImageReader::open(path)
        .map_err(|error| format!("gambar tidak dapat dibuka ({error})"))?
        .with_guessed_format()
        .map_err(|error| format!("format gambar tidak dapat dibaca ({error})"))?
        .decode()
        .map_err(|error| format!("gambar tidak dapat di-decode ({error})"))?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err("gambar kosong".to_string());
    }

    let crop_height = (height / 2).max(1);
    let cropped = image
        .crop_imm(0, height - crop_height, width, crop_height)
        .to_rgb8();
    let scale: u32 = if u64::from(width) * u64::from(crop_height) * 4 <= 24_000_000 {
        2
    } else {
        1
    };
    let resized = DynamicImage::ImageRgb8(cropped).resize_exact(
        width * scale,
        crop_height * scale,
        FilterType::Lanczos3,
    );
    let rgb = resized.to_rgb8();
    let gray = resized.to_luma8();

    let mut variants = Vec::new();
    if let Some(mask) = red_mask(&rgb) {
        variants.push(PreparedImage {
            file: write_png(&DynamicImage::ImageLuma8(mask))?,
            red: true,
        });
    }
    variants.push(PreparedImage {
        file: write_png(&DynamicImage::ImageLuma8(gray))?,
        red: false,
    });
    Ok(variants)
}

fn write_png(image: &DynamicImage) -> Result<tempfile::NamedTempFile, String> {
    let mut file = tempfile::Builder::new()
        .prefix("laporan-ocr-")
        .suffix(".png")
        .tempfile()
        .map_err(|error| format!("berkas sementara gagal dibuat ({error})"))?;
    image
        .write_to(&mut file.as_file_mut(), ImageFormat::Png)
        .map_err(|error| format!("gambar praproses gagal disimpan ({error})"))?;
    Ok(file)
}

/// Membuat citra hitam-putih dari piksel yang didominasi merah. Teks dibuat
/// hitam di atas latar putih agar mudah dibaca Tesseract.
fn red_mask(image: &RgbImage) -> Option<GrayImage> {
    let total = u64::from(image.width()) * u64::from(image.height());
    if total == 0 {
        return None;
    }
    let mut mask = GrayImage::new(image.width(), image.height());
    let mut red_pixels = 0_u64;
    for (x, y, pixel) in image.enumerate_pixels() {
        let red = i16::from(pixel[0]);
        let green = i16::from(pixel[1]);
        let blue = i16::from(pixel[2]);
        if red - green.max(blue) > 40 {
            mask.put_pixel(x, y, Luma([0]));
            red_pixels += 1;
        } else {
            mask.put_pixel(x, y, Luma([255]));
        }
    }
    // Kurang dari 0,05% piksel: kemungkinan bukan stempel merah.
    if red_pixels * 2000 < total {
        return None;
    }
    Some(mask)
}

/// Menjalankan satu mode halaman, dengan fallback bahasa bila paket `ind`
/// belum terpasang.
async fn run_pass(config: &Config, path: &Path, psm: &str) -> Result<OcrPass, String> {
    match run_tesseract(config, path, "ind+eng", psm).await {
        Ok(pass) => Ok(pass),
        // Kedua pesan error digabung agar penyebab kegagalan tidak tertutup
        // oleh percobaan terakhir.
        Err(primary) => run_tesseract(config, path, "eng", psm)
            .await
            .map_err(|fallback| format!("ind+eng: {primary}; eng: {fallback}")),
    }
}

/// Menggabungkan hasil beberapa mode OCR tanpa mengulang baris yang sama.
fn merge_texts(texts: &[String]) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut lines = Vec::new();
    for text in texts {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if seen.insert(line.to_lowercase()) {
                lines.push(line.to_string());
            }
        }
    }
    lines.join("\n")
}

/// Memeriksa ketersediaan Tesseract sekali saat startup agar kegagalan OCR
/// terlihat di log sebelum pemindaian pertama berjalan.
pub async fn tesseract_version(config: &Config) -> Result<String, String> {
    let output = Command::new(&config.tesseract_path)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| format!("Tesseract tidak dapat dijalankan ({error})"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Tesseract --version gagal: {}", stderr.trim()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().next().unwrap_or("").trim().to_string())
}

/// Hasil satu mode OCR: teks yang sudah terurut dan rata-rata confidence.
struct OcrPass {
    text: String,
    confidence: f64,
}

/// Kata dengan confidence di bawah ambang ini biasanya berupa pecahan garis
/// atau latar foto, bukan tulisan watermark.
const MIN_WORD_CONFIDENCE: f64 = 30.0;

struct TsvLine {
    top: u32,
    left: u32,
    words: Vec<(u32, String, f64)>,
}

async fn run_tesseract(
    config: &Config,
    path: &Path,
    language: &str,
    psm: &str,
) -> Result<OcrPass, String> {
    let output = tokio::time::timeout(
        Duration::from_secs(40),
        Command::new(&config.tesseract_path)
            .arg(path)
            .arg("stdout")
            .arg("-l")
            .arg(language)
            .arg("--psm")
            .arg(psm)
            .arg("tsv")
            .stdin(Stdio::null())
            .output(),
    )
    .await
    .map_err(|_| "OCR melewati batas waktu".to_string())?
    .map_err(|error| format!("Tesseract tidak dapat dijalankan ({error})"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Tesseract gagal: {}", stderr.trim()));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(parse_tsv(&raw))
}

/// Mengubah keluaran TSV Tesseract menjadi teks yang terurut dari atas ke bawah
/// dan kiri ke kanan. Kata dengan confidence sangat rendah dibuang, tetapi bila
/// seluruh teks terkena ambang, teks asli tetap dipakai.
fn parse_tsv(raw: &str) -> OcrPass {
    let mut lines: Vec<TsvLine> = Vec::new();
    let mut positions: HashMap<(u32, u32, u32), usize> = HashMap::new();
    for row in raw.lines().skip(1) {
        let columns: Vec<&str> = row.split('\t').collect();
        if columns.len() < 12 {
            continue;
        }
        let text = columns[11].trim();
        if text.is_empty() {
            continue;
        }
        let Ok(confidence) = columns[10].parse::<f64>() else {
            continue;
        };
        if confidence < 0.0 {
            continue;
        }
        let number = |index: usize| columns[index].parse::<u32>().unwrap_or(0);
        let key = (number(2), number(3), number(4));
        let left = number(6);
        let top = number(7);
        let position = *positions.entry(key).or_insert_with(|| {
            lines.push(TsvLine {
                top,
                left,
                words: Vec::new(),
            });
            lines.len() - 1
        });
        lines[position].top = lines[position].top.min(top);
        lines[position].left = lines[position].left.min(left);
        lines[position]
            .words
            .push((left, text.to_string(), confidence));
    }
    lines.sort_by_key(|line| (line.top, line.left));

    let (text, confidence) = render_tsv(&mut lines, MIN_WORD_CONFIDENCE);
    if text.is_empty() {
        let (text, confidence) = render_tsv(&mut lines, 0.0);
        return OcrPass { text, confidence };
    }
    OcrPass { text, confidence }
}

fn render_tsv(lines: &mut [TsvLine], threshold: f64) -> (String, f64) {
    let mut output = Vec::new();
    let mut confidence_sum = 0.0;
    let mut confidence_count = 0_u32;
    for line in lines {
        line.words.sort_by_key(|(left, _, _)| *left);
        let kept: Vec<&(u32, String, f64)> = line
            .words
            .iter()
            .filter(|(_, _, confidence)| *confidence >= threshold)
            .collect();
        if kept.is_empty() {
            continue;
        }
        let mut words = Vec::with_capacity(kept.len());
        for (_, text, confidence) in kept {
            words.push(text.as_str());
            confidence_sum += confidence;
            confidence_count += 1;
        }
        output.push(words.join(" "));
    }
    let confidence = if confidence_count == 0 {
        0.0
    } else {
        confidence_sum / f64::from(confidence_count)
    };
    (output.join("\n"), confidence)
}

/// Apakah OCR menghasilkan teks yang bisa dipakai sama sekali.
pub fn has_text(text: &str) -> bool {
    text.chars().any(|character| character.is_alphanumeric())
}

/// Apakah teks OCR cukup panjang untuk dianggap tulisan watermark, bukan
/// pecahan huruf dari latar foto.
pub fn meaningful(text: &str) -> bool {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .count()
        >= 12
}

pub fn has_address(text: &str) -> bool {
    let lower = text.to_lowercase();
    const KEYWORDS: [&str; 20] = [
        "jl",
        "jalan",
        "desa",
        "kel.",
        "kelurahan",
        "kec.",
        "kecamatan",
        "kab.",
        "kabupaten",
        "kota",
        "banjar",
        "dusun",
        "br.",
        "provinsi",
        "kp.",
        "kampung",
        "gang",
        "gg.",
        "lingkungan",
        "lkr.",
    ];
    KEYWORDS.iter().any(|keyword| lower.contains(keyword))
}

pub fn has_time(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let cleaned = token.trim_matches(|character: char| {
            !character.is_ascii_digit() && character != ':' && character != '.'
        });
        let parts: Vec<&str> = cleaned.split([':', '.']).collect();
        parts.len() >= 2
            && parts[0].parse::<u8>().is_ok_and(|hour| hour < 24)
            && parts[1].parse::<u8>().is_ok_and(|minute| minute < 60)
    })
}

/// Mencari tanggal pertama pada teks watermark: `dd/mm/yyyy`, `dd-mm-yy`,
/// `dd.mm.yyyy`, `yyyy-mm-dd`, atau `dd NamaBulan yyyy` (Indonesia/Inggris).
pub fn find_date(text: &str) -> Option<NaiveDate> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for (index, token) in tokens.iter().enumerate() {
        let numeric = token.trim_matches(|character: char| {
            !character.is_ascii_digit() && character != '/' && character != '-' && character != '.'
        });
        if let Some(date) = numeric_date(numeric) {
            return Some(date);
        }

        let Some(day) = token
            .trim_matches(|character: char| !character.is_ascii_digit())
            .parse::<u32>()
            .ok()
        else {
            continue;
        };
        if !(1..=31).contains(&day) {
            continue;
        }
        let Some(month) = tokens.get(index + 1).and_then(|value| month_number(value)) else {
            continue;
        };
        let Some(year) = tokens.get(index + 2).and_then(|value| {
            parse_year(value.trim_matches(|character: char| !character.is_ascii_digit()))
        }) else {
            continue;
        };
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            return Some(date);
        }
    }
    None
}

fn numeric_date(token: &str) -> Option<NaiveDate> {
    let separator = ['/', '-', '.']
        .into_iter()
        .find(|separator| token.contains(*separator))?;
    let parts: Vec<&str> = token.split(separator).collect();
    if parts.len() != 3 {
        return None;
    }
    // Pemisah titik dipakai juga oleh jam (`07.07.09`). Anggap tanggal hanya
    // bila salah satu ujungnya berupa tahun empat digit.
    if separator == '.' && parts[0].len() != 4 && parts[2].len() != 4 {
        return None;
    }
    let (year, month, day) = if parts[0].len() == 4 {
        (
            parse_year(parts[0])?,
            parts[1].parse::<u32>().ok()?,
            parts[2].parse::<u32>().ok()?,
        )
    } else {
        (
            parse_year(parts[2])?,
            parts[1].parse::<u32>().ok()?,
            parts[0].parse::<u32>().ok()?,
        )
    };
    NaiveDate::from_ymd_opt(year, month, day)
}

fn parse_year(raw: &str) -> Option<i32> {
    let year = raw.parse::<i32>().ok()?;
    match raw.len() {
        2 => Some(2000 + year),
        4 if (1900..=2100).contains(&year) => Some(year),
        _ => None,
    }
}

fn month_number(raw: &str) -> Option<u32> {
    let key = raw
        .trim_matches(|character: char| !character.is_alphabetic())
        .to_lowercase();
    match key.as_str() {
        "jan" | "januari" | "january" => Some(1),
        "feb" | "februari" | "february" => Some(2),
        "mar" | "maret" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "mei" | "may" => Some(5),
        "jun" | "juni" | "june" => Some(6),
        "jul" | "juli" | "july" => Some(7),
        "agu" | "agt" | "agustus" | "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "okt" | "oktober" | "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "des" | "desember" | "dec" | "december" => Some(12),
        _ => None,
    }
}

/// Mencari pasangan koordinat pada teks watermark. Mendukung desimal titik
/// maupun koma, derajat, serta arah `S/W` (negatif) dan `N/E` (positif).
pub fn find_coords(text: &str) -> Option<(f64, f64)> {
    let normalized = normalize_coordinate_separators(text);
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    let mut numbers = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let Some((value, mut suffix)) = coordinate_number(tokens[index]) else {
            index += 1;
            continue;
        };
        if suffix.is_none() {
            if let Some(found) = tokens.get(index + 1).and_then(|token| hemisphere(token)) {
                suffix = Some(found);
                index += 1;
            }
        }
        numbers.push((value, suffix));
        index += 1;
    }

    for pair in numbers.windows(2) {
        let (first, second) = if pair[0].0.abs() > 90.0 && pair[1].0.abs() <= 90.0 {
            (pair[1], pair[0])
        } else {
            (pair[0], pair[1])
        };
        let lat = apply_hemisphere(first.0, first.1);
        let lon = apply_hemisphere(second.0, second.1);
        if lat.abs() <= 90.0 && lon.abs() <= 180.0 && (lat != 0.0 || lon != 0.0) {
            return Some((lat, lon));
        }
    }
    None
}

/// Mengubah desimal koma (`8,5431`) menjadi titik tanpa mengubah koma yang
/// berfungsi sebagai pemisah pasangan (`-8.65, 115.22`).
fn normalize_coordinate_separators(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut output = String::with_capacity(text.len());
    for (index, character) in characters.iter().enumerate() {
        if *character == ','
            && index > 0
            && index + 1 < characters.len()
            && characters[index - 1].is_ascii_digit()
            && characters[index + 1].is_ascii_digit()
        {
            output.push('.');
        } else {
            output.push(*character);
        }
    }
    output
}

fn coordinate_number(raw: &str) -> Option<(f64, Option<char>)> {
    let token = raw.trim_matches(|character: char| {
        character == ','
            || character == ';'
            || character == '('
            || character == ')'
            || character == '|'
    });
    let token = token.trim_matches(|character: char| {
        !character.is_ascii_digit()
            && character != '.'
            && character != '-'
            && character != 'S'
            && character != 'N'
            && character != 'E'
            && character != 'W'
    });
    let (number, suffix) = match token.chars().last() {
        Some(last @ ('S' | 'N' | 'E' | 'W')) => (&token[..token.len() - 1], Some(last)),
        _ => (token, None),
    };
    // Koordinat selalu memakai desimal; angka bulat seperti tahun diabaikan.
    if !number.contains('.') {
        return None;
    }
    let value = number.parse::<f64>().ok()?;
    if !value.is_finite() || value.abs() > 180.0 {
        return None;
    }
    Some((value, suffix))
}

fn hemisphere(raw: &str) -> Option<char> {
    let token = raw.trim_matches(|character: char| !character.is_alphabetic());
    let mut characters = token.chars();
    let value = characters.next()?.to_ascii_uppercase();
    if characters.next().is_some() {
        return None;
    }
    matches!(value, 'S' | 'N' | 'E' | 'W').then_some(value)
}

fn apply_hemisphere(value: f64, hemisphere: Option<char>) -> f64 {
    match hemisphere {
        Some('S' | 'W') => -value.abs(),
        Some('N' | 'E') => value.abs(),
        _ => value,
    }
}

pub fn haversine_meters(left: (f64, f64), right: (f64, f64)) -> f64 {
    let radius = 6_371_000.0_f64;
    let lat1 = left.0.to_radians();
    let lat2 = right.0.to_radians();
    let delta_lat = (right.0 - left.0).to_radians();
    let delta_lon = (right.1 - left.1).to_radians();
    let a =
        (delta_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
    2.0 * radius * a.sqrt().atan2((1.0 - a).sqrt())
}

/// Mengubah `YYYY:MM:DD HH:MM:SS` (EXIF) menjadi `YYYY-MM-DD`.
pub fn exif_date(raw: &str) -> Option<NaiveDate> {
    let date = raw.split_whitespace().next()?;
    NaiveDate::parse_from_str(date, "%Y:%m:%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn parses_date_time_address_and_coords() {
        let text = "Kuta, 19/10/2026 08:14 WIB\n-8.657100, 115.221300";
        assert_eq!(find_date(text).unwrap().to_string(), "2026-10-19");
        assert!(has_time(text));
        assert!(has_address("Jl. Raya Kuta No. 1"));
        assert_eq!(find_coords(text), Some((-8.6571, 115.2213)));
    }

    #[test]
    fn rejects_incomplete_watermark() {
        let text = "Gambar kegiatan";
        assert!(!has_time(text));
        assert!(!has_address(text));
        assert!(find_date(text).is_none());
    }

    #[test]
    fn computes_distance() {
        assert!(haversine_meters((-8.6571, 115.2213), (-8.6571, 115.2213)) < 1.0);
        let other = haversine_meters((-8.6571, 115.2213), (-8.6671, 115.2213));
        assert!(other > 900.0 && other < 1200.0);
    }

    #[test]
    fn reads_exif_datetime_format() {
        assert_eq!(
            exif_date("2026:10:19 08:14:00").unwrap().to_string(),
            "2026-10-19"
        );
    }

    #[test]
    fn parses_indonesian_month_names() {
        assert_eq!(
            find_date("Senin, 02 November 2026 07:13")
                .unwrap()
                .to_string(),
            "2026-11-02"
        );
        assert_eq!(find_date("19 Okt 2026").unwrap().to_string(), "2026-10-19");
        assert_eq!(find_date("26/10/26").unwrap().to_string(), "2026-10-26");
        assert_eq!(
            find_date("26 Oktober 2026").unwrap().to_string(),
            "2026-10-26"
        );
        assert_eq!(find_date("2026-10-26").unwrap().to_string(), "2026-10-26");
        assert_eq!(find_date("26.10.2026").unwrap().to_string(), "2026-10-26");
        assert!(find_date("8.6819744").is_none());
        assert!(find_date("07.07.09").is_none());
        assert!(find_date("26/13/2026").is_none());
    }

    #[test]
    fn parses_dot_time_separator() {
        assert!(has_time("26 Okt 2026 07.07.09"));
        assert!(has_time("26/10/26 07:01 AM"));
        assert!(!has_time("8.6819744"));
        assert!(!has_time("26.10.2026"));
    }

    #[test]
    fn parses_coordinates_with_hemispheres() {
        let (lat, lon) = find_coords("8.6819744° S, 115.4884660° E").unwrap();
        assert!((lat + 8.6819744).abs() < 1e-6, "lat: {lat}");
        assert!((lon - 115.4884660).abs() < 1e-6, "lon: {lon}");

        let (lat, lon) = find_coords("8,5431S 115,2142E").unwrap();
        assert!((lat + 8.5431).abs() < 1e-6, "lat: {lat}");
        assert!((lon - 115.2142).abs() < 1e-6, "lon: {lon}");

        let (lat, lon) = find_coords("Lat -8.5410946 Long 115.3472174").unwrap();
        assert!((lat + 8.5410946).abs() < 1e-6, "lat: {lat}");
        assert!((lon - 115.3472174).abs() < 1e-6, "lon: {lon}");

        let (lat, lon) = find_coords("115.4884660, -8.6819744").unwrap();
        assert!((lat + 8.6819744).abs() < 1e-6, "lat: {lat}");
        assert!((lon - 115.4884660).abs() < 1e-6, "lon: {lon}");

        assert!(find_coords("Kode Foto: EW8QY6P2HK4C3H").is_none());
        assert!(find_coords("Senin, 02 November 2026 07:13").is_none());
    }

    #[test]
    fn recognises_extended_address_keywords() {
        assert!(has_address("Kp. Toyapakeh, Kec. Nusa Penida"));
        assert!(has_address("Jalan Pendet, Kecamatan Abiansemal"));
        assert!(has_address("Gang Melati No. 3"));
    }

    #[test]
    fn recognises_watermarks_from_real_photos() {
        let timemark = "Senin, 02 November 2026 07:13\nKp. Toyapakeh, Kec. Nusa Penida\nKabupaten Klungkung, Bali\n8.6819744° S, 115.4884660° E";
        assert_eq!(find_date(timemark).unwrap().to_string(), "2026-11-02");
        assert!(has_time(timemark));
        assert!(has_address(timemark));
        let (lat, lon) = find_coords(timemark).unwrap();
        assert!((lat + 8.6819744).abs() < 1e-6);
        assert!((lon - 115.4884660).abs() < 1e-6);

        let timefix = "19 Okt 2026\n07:19\nJalan Raya Darmasaba No. 2, Kabupaten Badung, Bali";
        assert_eq!(find_date(timefix).unwrap().to_string(), "2026-10-19");
        assert!(has_time(timefix));
        assert!(has_address(timefix));

        let red_stamp = "26 Okt 2026 07.07.09\n8,5431S 115,2142E\nJalan Pendet\nKecamatan Abiansemal\nKabupaten Badung";
        assert_eq!(find_date(red_stamp).unwrap().to_string(), "2026-10-26");
        assert!(has_time(red_stamp));
        assert!(has_address(red_stamp));
        let (lat, lon) = find_coords(red_stamp).unwrap();
        assert!((lat + 8.5431).abs() < 1e-6);
        assert!((lon - 115.2142).abs() < 1e-6);

        let gps_camera = "Kecamatan Gianyar, Bali, Indonesia\nLat -8.5410946 Long 115.3472174\n26/10/26 07:01 AM";
        assert_eq!(find_date(gps_camera).unwrap().to_string(), "2026-10-26");
        assert!(has_time(gps_camera));
        assert!(has_address(gps_camera));
        let (lat, lon) = find_coords(gps_camera).unwrap();
        assert!((lat + 8.5410946).abs() < 1e-6);
        assert!((lon - 115.3472174).abs() < 1e-6);
    }

    #[test]
    fn merges_ocr_passes_without_duplicates() {
        let merged = merge_texts(&[
            "19 Okt 2026\n07:19".to_string(),
            "07:19\nJalan Raya Darmasaba".to_string(),
        ]);
        assert_eq!(merged, "19 Okt 2026\n07:19\nJalan Raya Darmasaba");
    }

    #[test]
    fn detects_empty_ocr_text() {
        assert!(!has_text(""));
        assert!(!has_text("   \n --- "));
        assert!(has_text("07:19"));
    }

    #[test]
    fn detects_short_ocr_text() {
        assert!(!meaningful("ab 1"));
        assert!(meaningful("07:19 19 Okt 2026"));
    }

    #[test]
    fn orders_tsv_lines_and_drops_noise() {
        let raw = concat!(
            "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n",
            "5\t1\t1\t1\t1\t3\t150\t20\t40\t10\t92\tOkt\n",
            "5\t1\t1\t1\t1\t2\t70\t20\t40\t10\t96\t19\n",
            "5\t1\t1\t1\t1\t1\t10\t20\t40\t10\t95\tSenin,\n",
            "5\t1\t1\t2\t1\t3\t200\t60\t40\t10\t20\tzzz\n",
            "5\t1\t1\t2\t1\t2\t100\t60\t40\t10\t96\t2026\n",
            "5\t1\t1\t2\t1\t1\t10\t60\t40\t10\t88\tJalan\n",
        );
        let pass = parse_tsv(raw);
        assert_eq!(pass.text, "Senin, 19 Okt\nJalan 2026");
        assert!(pass.confidence > 90.0, "confidence: {}", pass.confidence);
    }

    #[test]
    fn keeps_low_confidence_text_when_everything_is_low() {
        let raw = concat!(
            "header\n",
            "5\t1\t1\t1\t1\t1\t10\t10\t10\t10\t12\tJalan\n",
            "5\t1\t1\t1\t1\t2\t60\t10\t10\t10\t18\tPendet\n",
        );
        let pass = parse_tsv(raw);
        assert_eq!(pass.text, "Jalan Pendet");
        assert!(pass.confidence > 10.0, "confidence: {}", pass.confidence);
    }

    #[test]
    fn ignores_images_without_red_watermark() {
        let source = RgbImage::from_pixel(20, 20, Rgb([180, 180, 180]));
        assert!(red_mask(&source).is_none());
    }

    #[test]
    fn preprocesses_bottom_crop_and_red_mask() {
        let mut source = RgbImage::new(40, 40);
        for pixel in source.pixels_mut() {
            *pixel = Rgb([25, 25, 25]);
        }
        for x in 0..10 {
            source.put_pixel(x, 35, Rgb([220, 30, 30]));
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("contoh.png");
        source.save(&path).unwrap();

        let variants = preprocess(&path).unwrap();
        assert!(variants.iter().any(|variant| variant.red));
        assert!(variants.iter().any(|variant| !variant.red));
        for variant in &variants {
            let decoded = image::ImageReader::open(variant.path())
                .unwrap()
                .decode()
                .unwrap();
            assert!(decoded.width() >= 40);
        }
    }

    #[tokio::test]
    async fn ocr_error_mentions_both_language_attempts() {
        let mut config = Config::test_config();
        config.tesseract_path = "tesseract-tidak-terpasang".to_string();
        let error = ocr(&config, Path::new("gambar.jpg")).await.unwrap_err();
        assert!(error.contains("ind+eng"), "pesan: {error}");
        assert!(error.contains("eng"), "pesan: {error}");
    }
}
