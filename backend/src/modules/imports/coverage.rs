use std::collections::{BTreeMap, BTreeSet};

use crate::{
    modules::imports::{
        models::{
            CoverageBreakdown, CoverageCode, CoverageGroup, CoverageMetric, CoverageProgram,
            CoverageResponse, CoverageTable, CoverageTableRow, CoverageUnit, CoverageUnitCount,
            CoverageUnitValue, RecordView,
        },
        qw,
    },
    shared::{LabeledCount, extract_links, is_noise, parse_count},
};

use super::duties::{self, DutyLevel};
use super::service::{record_is_polda, record_satker_label};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    Satker,
    Polres,
}

/// Cakupan unit yang dipakai saat mengelompokkan baris.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitMode {
    /// Seluruh Polda: dimensi polres dan satker Polda.
    Polda,
    /// Satu polres: dimensi polsek dan satker polres.
    Polres,
}

struct KnownUnits {
    polres: BTreeMap<String, String>,
    satker: BTreeMap<String, String>,
}

impl KnownUnits {
    fn from_rows(rows: &[RecordView], mode: UnitMode) -> Self {
        let mut polres = BTreeMap::new();
        let mut satker = BTreeMap::new();
        for row in rows {
            let Some((scope, label)) = row_unit(row, mode) else {
                continue;
            };
            let key = label.to_lowercase();
            match scope {
                Scope::Polres => {
                    polres.entry(key).or_insert(label);
                }
                Scope::Satker => {
                    satker.entry(key).or_insert(label);
                }
            }
        }
        Self { polres, satker }
    }

    fn set(&self, scope: Scope) -> &BTreeMap<String, String> {
        match scope {
            Scope::Polres => &self.polres,
            Scope::Satker => &self.satker,
        }
    }
}

/// Peran yang menentukan tanggung jawab pada dimensi polres/polsek.
fn unit_role(mode: UnitMode, scope: Scope) -> Option<&'static str> {
    match (mode, scope) {
        (UnitMode::Polda, Scope::Polres) => Some("KAPOLRES"),
        (UnitMode::Polres, Scope::Polres) => Some("KAPOLSEK"),
        _ => None,
    }
}

/// Unit yang seharusnya melaksanakan satu kode, termasuk unit yang sudah melapor.
fn responsible_units(
    known: &KnownUnits,
    scope: Scope,
    mode: UnitMode,
    code: &str,
    performed: &BTreeSet<String>,
) -> BTreeSet<String> {
    let labels = known.set(scope);
    let mut responsible: BTreeSet<String> = match unit_role(mode, scope) {
        Some(role) => {
            if !duties::is_assigned(code) || duties::role_assigned(code, role) {
                labels.keys().cloned().collect()
            } else {
                BTreeSet::new()
            }
        }
        None => {
            let level = match mode {
                UnitMode::Polda => DutyLevel::Polda,
                UnitMode::Polres => DutyLevel::Polres,
            };
            match duties::satker_keywords(code, level) {
                None => labels.keys().cloned().collect(),
                Some(keywords) => {
                    // Cukup salah satu: bila pejabat pelaksana tingkat ini sudah
                    // melapor, kode dianggap terlaksana dan tidak ada satker yang
                    // dicatat belum melaksanakan.
                    let pejabat = duties::pelaksana_keywords(code, level).unwrap_or_default();
                    let covered = labels.iter().any(|(key, label)| {
                        performed.contains(key) && duties::label_matches(label, &pejabat)
                    });
                    if covered {
                        BTreeSet::new()
                    } else {
                        labels
                            .iter()
                            .filter(|(_, label)| duties::label_matches(label, &keywords))
                            .map(|(key, _)| key.clone())
                            .collect()
                    }
                }
            }
        }
    };
    responsible.extend(performed.iter().cloned());
    responsible
}

/// Unit bertanggung jawab yang belum melapor, diurutkan seperti daftar unit.
fn missing_units(
    known: &KnownUnits,
    scope: Scope,
    mode: UnitMode,
    code: &str,
    performed: &BTreeSet<String>,
) -> Vec<String> {
    let responsible = responsible_units(known, scope, mode, code, performed);
    known
        .set(scope)
        .iter()
        .filter(|(key, _)| responsible.contains(*key) && !performed.contains(*key))
        .map(|(_, label)| label.clone())
        .collect()
}

/// Unit bertanggung jawab yang belum melapor untuk satu kode, sesuai daftar tugas.
pub(crate) fn missing_units_for_code(
    rows: &[RecordView],
    code: &str,
    polres: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let filter = polres.map(str::trim).filter(|value| !value.is_empty());
    let mode = if filter.is_some() {
        UnitMode::Polres
    } else {
        UnitMode::Polda
    };
    let scoped: Vec<RecordView> = match filter {
        Some(label) => rows
            .iter()
            .filter(|row| row.polres.trim().eq_ignore_ascii_case(label))
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    let rows: &[RecordView] = if filter.is_some() { &scoped } else { rows };
    let known = KnownUnits::from_rows(rows, mode);
    let performed_polres = performed_units(rows, code, mode, Scope::Polres);
    let performed_satker = performed_units(rows, code, mode, Scope::Satker);
    (
        missing_units(&known, Scope::Polres, mode, code, &performed_polres),
        missing_units(&known, Scope::Satker, mode, code, &performed_satker),
    )
}

fn performed_units(
    rows: &[RecordView],
    code: &str,
    mode: UnitMode,
    scope: Scope,
) -> BTreeSet<String> {
    let normalized = qw::normalize(code);
    let is_group = !normalized.contains('.');
    let prefix = format!("{normalized}.");
    let matching: Vec<&RecordView> = rows
        .iter()
        .filter(|row| {
            let row_code = qw::normalize(&row.kode);
            if is_group {
                row_code == normalized || row_code.starts_with(&prefix)
            } else {
                row_code == normalized
            }
        })
        .collect();
    units_of(&matching, mode, scope)
}

fn units_of(matching: &[&RecordView], mode: UnitMode, scope: Scope) -> BTreeSet<String> {
    matching
        .iter()
        .filter_map(|row| row_unit(row, mode))
        .filter(|(item_scope, _)| *item_scope == scope)
        .map(|(_, label)| label.to_lowercase())
        .collect()
}

/// Jumlah baris per unit, tanpa nilai metrik.
fn unit_row_counts(matching: &[&RecordView], mode: UnitMode, scope: Scope) -> Vec<CoverageUnit> {
    let mut counts: BTreeMap<String, (String, i64)> = BTreeMap::new();
    for row in matching {
        let Some((item_scope, label)) = row_unit(row, mode) else {
            continue;
        };
        if item_scope != scope {
            continue;
        }
        let entry = counts.entry(label.to_lowercase()).or_insert((label, 0));
        entry.1 += 1;
    }
    let mut units: Vec<CoverageUnit> = counts
        .into_values()
        .map(|(label, rows)| CoverageUnit {
            label,
            rows,
            values: Vec::new(),
            missing: Vec::new(),
        })
        .collect();
    units.sort_by(|left, right| {
        right
            .rows
            .cmp(&left.rows)
            .then_with(|| left.label.cmp(&right.label))
    });
    units
}

// ===== Kartu cakupan per sub-kode =====

/// Field administratif yang tidak dijadikan kartu.
const CARD_SKIP: &[&str] = &[
    "pelaksana",
    "nama dan pangkat",
    "dokumentasi",
    "link posting",
    "inovasi",
    "saran masukan",
    "problem, masalah",
    "kegiatan lain",
    "bukti dukung",
    "status kegiatan",
    "kendala",
    "pelapor",
    "tentang",
    "uraian",
    "sebutkan",
    "rincian",
    "daftar",
    "tautan",
    "unggah",
    "sprint",
    "foto",
    "peta kerawanan",
    "titik koordinat",
    "alamat",
];

fn is_card_skip(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    CARD_SKIP.iter().any(|skip| lower.contains(skip)) || lower.contains("nama ")
}

/// Kartu jumlah tempat/mitra unik untuk field nama, rute, atau kerja sama.
fn place_card(key: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let lower = key.to_ascii_lowercase();
    if [
        "pertimbangan",
        "alasan",
        "uraian",
        "rincian",
        "daftar",
        "link",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return None;
    }
    if ["mitra", "kerja sama", "kerjasama"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Some(("mitra", "Jumlah mitra", "mitra"));
    }
    if !lower.contains("nama ") && !lower.contains("rute") {
        return None;
    }
    if lower.contains("sekolah") {
        return Some(("sekolah", "Jumlah sekolah", "sekolah"));
    }
    if lower.contains("masjid") {
        return Some(("masjid", "Jumlah masjid", "masjid"));
    }
    if lower.contains("ibadah") {
        return Some(("ibadah", "Jumlah tempat ibadah", "lokasi"));
    }
    if lower.contains("rute") {
        return Some(("rute", "Jumlah rute patroli", "rute"));
    }
    if lower.contains("lokasi") || lower.contains("tempat") {
        return Some(("lokasi", "Jumlah lokasi", "lokasi"));
    }
    None
}

fn card_unit(key: &str) -> &'static str {
    let lower = key.to_ascii_lowercase();
    if [
        "personel", "peserta", "orang", "warga", "jamaah", "siswa", "anggota",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return "orang";
    }
    if ["lokasi", "titik", "sekolah", "masjid", "ibadah", "kawasan"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return "lokasi";
    }
    ""
}

fn card_slug(key: &str) -> String {
    let mut slug = String::new();
    for character in key.chars() {
        if character.is_alphanumeric() {
            slug.extend(character.to_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn card_value_label(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.to_ascii_lowercase().starts_with("lainnya") {
        return "Lainnya".to_string();
    }
    trimmed.to_string()
}

fn jenjang_of(value: &str) -> Option<&'static str> {
    let upper = value.to_ascii_uppercase();
    let tokens: Vec<&str> = upper
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.iter().any(|token| token.starts_with("SMK")) {
        return Some("SMK");
    }
    if tokens
        .iter()
        .any(|token| token.starts_with("SMA") || *token == "MA")
    {
        return Some("SMA/MA");
    }
    if tokens
        .iter()
        .any(|token| token.starts_with("SMP") || token.starts_with("MTS"))
    {
        return Some("SMP/MTs");
    }
    if tokens
        .iter()
        .any(|token| token.starts_with("SD") || *token == "MI")
    {
        return Some("SD/MI");
    }
    None
}

fn card_metric(
    key: &str,
    label: &str,
    unit: &str,
    total: f64,
    filled: usize,
    rows: usize,
) -> CoverageMetric {
    CoverageMetric {
        key: key.to_string(),
        label: label.to_string(),
        unit: unit.to_string(),
        total,
        filled_rows: filled as i64,
        missing_rows: rows as i64 - filled as i64,
        filled_polres: 0,
        filled_satker: 0,
    }
}

/// Nilai yang dianggap kosong saat menyusun kartu. Angka `0` tetap dihitung.
fn is_card_noise(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.is_empty()
        || matches!(
            lower.as_str(),
            "-" | "—" | "null" | "none" | "nihil" | "tidak" | "tidak ada" | "belum ada"
        )
        || lower.starts_with("nihil")
        || lower.starts_with("tidak ada")
}

/// Label personel/peserta yang biasa ditulis sebagai baris tersendiri,
/// mis. `Polri : 5 pers` atau `Warga: 40 orang`.
fn personnel_label(key: &str) -> Option<&'static str> {
    let lower = key.trim().to_ascii_lowercase();
    if lower.is_empty() || lower.len() > 40 {
        return None;
    }
    // Field berlabel "Jumlah …" sudah ditangani sebagai kartu angka biasa.
    if lower.starts_with("jumlah") || lower.contains("jumlah ") {
        return None;
    }
    if lower.contains("polri") || lower.contains("polisi") {
        return Some("Polri");
    }
    if lower.contains("bnn") {
        return Some("BNN");
    }
    if lower.contains("tni") {
        return Some("TNI");
    }
    if lower.contains("warga") || lower.contains("masyarakat") {
        return Some("Warga/masyarakat");
    }
    if lower.contains("peserta") {
        return Some("Peserta");
    }
    if lower.contains("pemuda") || lower.contains("karang taruna") {
        return Some("Pemuda");
    }
    if lower.contains("pecalang") {
        return Some("Pecalang");
    }
    if lower.contains("prajuru") || lower.contains("adat") {
        return Some("Prajuru adat");
    }
    if lower.contains("keluarga") {
        return Some("Keluarga");
    }
    if lower.contains("pemda") || lower.contains("instansi") {
        return Some("Pemda/instansi");
    }
    None
}

/// Field angka yang mencatat jumlah personel Polri.
fn is_polri_personnel_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if [
        "keluhan",
        "problem",
        "komplain",
        "saran",
        "pendapat",
        "tindak lanjut",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return false;
    }
    if [
        "bnn",
        "tni",
        "warga",
        "masyarakat",
        "siswa",
        "guru",
        "sekolah",
        "pemda",
        "instansi",
        "keluarga",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return false;
    }
    ["polri", "polisi", "personel", "personil"]
        .iter()
        .any(|word| lower.contains(word))
}

fn polri_field_count(value: &str) -> Option<i64> {
    let lower = value.to_ascii_lowercase();
    if [
        "keluhan",
        "ditindaklanjuti",
        "tindak lanjut",
        "problem",
        "saran",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return None;
    }
    parse_count(value)
}

/// Field teks yang wajar memuat rincian personel/peserta kegiatan.
fn is_personnel_text_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if [
        "pertimbangan",
        "alasan",
        "saran",
        "problem",
        "keluhan",
        "komplain",
        "tindak lanjut",
        "pendapat",
        "dokumentasi",
        "link",
        "nama dan pangkat",
        "materi",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return false;
    }
    [
        "peserta",
        "cara bertindak",
        "kegiatan",
        "uraian",
        "personel",
        "personil",
        "hadir",
    ]
    .iter()
    .any(|word| lower.contains(word))
}

/// Membuang keterangan dalam kurung, mis. `Warga (anggota organisasi)`.
fn without_parenthetical(text: &str) -> String {
    let mut output = String::new();
    let mut depth = 0_i32;
    for character in text.chars() {
        match character {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if depth == 0 => output.push(character),
            _ => {}
        }
    }
    output
}

/// Kategori personel dari label pendek, mis. `Polri`, `Guru dan Staf`, `Siswa kelas VII`.
fn personnel_category(label: &str) -> Option<&'static str> {
    let lower = without_parenthetical(&label.trim().to_ascii_lowercase());
    let lower = lower.strip_prefix("jumlah ").unwrap_or(&lower);
    let cleaned = lower
        .trim_matches(|character: char| !character.is_alphanumeric() && character != '/')
        .trim();
    if cleaned.is_empty() {
        return None;
    }
    let normalized = cleaned.replace('/', " ");
    let words: Vec<&str> = normalized.split_whitespace().collect();
    if words.is_empty() || words.len() > 4 {
        return None;
    }
    let banned = [
        "problem",
        "keluhan",
        "komplain",
        "saran",
        "masukan",
        "tindak",
        "lanjut",
        "menghimbau",
        "mengikuti",
        "hadir",
        "diikuti",
        "terdiri",
        "melaksanakan",
        "memberikan",
        "menyampaikan",
        "berkoordinasi",
        "melalui",
        "polsek",
        "polres",
    ];
    if words.iter().any(|word| banned.contains(word)) {
        return None;
    }
    if words
        .iter()
        .any(|word| word.chars().all(|character| character.is_ascii_digit()))
    {
        return None;
    }

    let exact = |allowed: &[&str]| {
        words.len() == allowed.len() && words.iter().zip(allowed).all(|(a, b)| a == b)
    };

    if exact(&["polri"])
        || exact(&["polisi"])
        || exact(&["personel"])
        || exact(&["personil"])
        || exact(&["anggota", "polri"])
        || exact(&["anggota", "polisi"])
        || exact(&["personel", "polri"])
        || exact(&["personil", "polri"])
        || exact(&["personel", "polisi"])
        || exact(&["personil", "polisi"])
    {
        return Some("Polri");
    }
    if exact(&["bnn"]) || exact(&["personel", "bnn"]) || exact(&["personil", "bnn"]) {
        return Some("BNN");
    }
    if exact(&["tni"]) || exact(&["personel", "tni"]) || exact(&["personil", "tni"]) {
        return Some("TNI");
    }
    if exact(&["guru"])
        || exact(&["staf"])
        || exact(&["staff"])
        || exact(&["guru", "staf"])
        || exact(&["guru", "staff"])
        || exact(&["guru", "dan", "staf"])
        || exact(&["guru", "dan", "staff"])
        || exact(&["guru", "&", "staf"])
        || exact(&["guru", "&", "staff"])
    {
        return Some("Guru/staf");
    }
    if exact(&["siswa"])
        || exact(&["siswi"])
        || exact(&["pelajar"])
        || exact(&["murid"])
        || exact(&["siswa", "siswi"])
        || exact(&["siswa", "dan", "siswi"])
        || exact(&["peserta", "didik"])
        || (words.len() <= 3
            && words
                .first()
                .is_some_and(|word| matches!(*word, "siswa" | "siswi" | "pelajar" | "murid")))
    {
        return Some("Siswa/pelajar");
    }
    if exact(&["mahasiswa"]) || exact(&["mahasiswi"]) {
        return Some("Mahasiswa");
    }
    if exact(&["warga"]) || exact(&["masyarakat"]) || exact(&["warga", "masyarakat"]) {
        return Some("Warga/masyarakat");
    }
    if exact(&["pemuda"]) || exact(&["karang", "taruna"]) {
        return Some("Pemuda");
    }
    if exact(&["pecalang"]) {
        return Some("Pecalang");
    }
    if exact(&["prajuru"]) || exact(&["prajuru", "adat"]) {
        return Some("Prajuru adat");
    }
    if exact(&["keluarga"]) {
        return Some("Keluarga");
    }
    if exact(&["pemda"]) || exact(&["instansi"]) || exact(&["pemda", "instansi"]) {
        return Some("Pemda/instansi");
    }
    if exact(&["dinas"]) || exact(&["dinas", "terkait"]) {
        return Some("Pemda/instansi");
    }
    if exact(&["jamaah"])
        || exact(&["jemaah"])
        || exact(&["warga", "jamaah"])
        || exact(&["warga", "jemaah"])
        || exact(&["jamaah", "masjid"])
        || exact(&["jemaah", "masjid"])
    {
        return Some("Jamaah");
    }
    if exact(&["jemaat"]) || exact(&["umat"]) || exact(&["jemaat", "ibadah"]) {
        return Some("Jemaat");
    }
    if exact(&["komunitas"]) || exact(&["paguyuban"]) || exact(&["komunitas", "paguyuban"]) {
        return Some("Komunitas");
    }
    if exact(&["tokoh", "masyarakat"])
        || exact(&["tokoh", "masyarakat", "adat"])
        || exact(&["tokoh", "adat"])
        || exact(&["tokoh", "agama"])
    {
        return Some("Tokoh masyarakat/adat");
    }
    if exact(&["nelayan"]) || exact(&["abk"]) || exact(&["abk", "nelayan"]) {
        return Some("Nelayan");
    }
    if exact(&["ojek", "online"])
        || exact(&["ojol"])
        || exact(&["driver", "online"])
        || exact(&["pengemudi", "online"])
    {
        return Some("Ojek online/driver");
    }
    if exact(&["petani"]) || exact(&["kelompok", "tani"]) {
        return Some("Petani");
    }
    if exact(&["peternak"]) || exact(&["kelompok", "peternak"]) {
        return Some("Peternak");
    }
    if exact(&["pedagang"]) || exact(&["pedagang", "pasar"]) {
        return Some("Pedagang");
    }
    if exact(&["buruh"]) || exact(&["karyawan"]) || exact(&["pekerja"]) {
        return Some("Buruh/pekerja");
    }
    if exact(&["lansia"]) {
        return Some("Lansia");
    }
    None
}

fn digits_before(text: &str) -> Option<i64> {
    let trimmed = text.trim_end();
    let token: String = trimmed
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit() || matches!(character, '.' | ','))
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    if token.is_empty() {
        return None;
    }
    parse_count(&token)
}

/// Menjumlahkan angka sebelum frasa, mis. `1 saran atau keluhan`.
fn sum_number_before(text: &str, needle: &str) -> i64 {
    let lower = text.to_ascii_lowercase();
    let mut total = 0_i64;
    let mut start = 0;
    while let Some(offset) = lower[start..].find(needle) {
        let position = start + offset;
        if let Some(count) = digits_before(&text[..position]) {
            total += count;
        }
        start = position + needle.len();
    }
    total
}

fn strip_list_marker(text: &str) -> &str {
    let mut trimmed = text.trim();
    for _ in 0..2 {
        trimmed = trimmed
            .trim_start_matches(['-', '•', '*', '–', '—'])
            .trim_start();
        let digits = trimmed
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .count();
        if digits > 0 {
            let rest = &trimmed[digits..];
            if let Some(rest) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')) {
                trimmed = rest.trim_start();
                continue;
            }
        }
        let mut characters = trimmed.chars();
        if let (Some(first), Some(second)) = (characters.next(), characters.next()) {
            if first.is_ascii_alphabetic()
                && matches!(second, '.' | ')')
                && characters.next().is_some_and(char::is_whitespace)
            {
                trimmed = &trimmed[first.len_utf8() + second.len_utf8()..];
                trimmed = trimmed.trim_start();
                continue;
            }
        }
        break;
    }
    trimmed
}

fn leading_count(text: &str) -> Option<i64> {
    let trimmed = text.trim_start_matches(|character: char| {
        character.is_whitespace() || matches!(character, ':' | '-' | '=' | '(')
    });
    let token: String = trimmed
        .chars()
        .take_while(|character| character.is_ascii_digit() || matches!(character, '.' | ','))
        .collect();
    if token.is_empty() {
        return None;
    }
    parse_count(&token)
}

fn leading_count_and_rest(text: &str) -> Option<(i64, &str)> {
    let trimmed = text.trim_start();
    let mut digits = String::new();
    let mut end = 0;
    for (index, character) in trimmed.char_indices() {
        if character.is_ascii_digit() || (matches!(character, '.' | ',') && !digits.is_empty()) {
            digits.push(character);
            end = index + character.len_utf8();
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    let rest = trimmed[end..].trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((parse_count(&digits)?, rest))
}

/// Memecah teks menjadi baris angka, termasuk daftar `1. Polri : 10 Personel. 2. ...`.
fn personnel_segments(value: &str) -> Vec<String> {
    let mut segments = Vec::new();
    for line in value.split(['\n', ';']) {
        let characters: Vec<char> = line.chars().collect();
        let mut current = String::new();
        let mut index = 0;
        while index < characters.len() {
            if characters[index] == '.'
                && index + 1 < characters.len()
                && characters[index + 1].is_whitespace()
            {
                let mut next = index + 1;
                while next < characters.len() && characters[next].is_whitespace() {
                    next += 1;
                }
                let mut digits_end = next;
                while digits_end < characters.len() && characters[digits_end].is_ascii_digit() {
                    digits_end += 1;
                }
                if digits_end > next
                    && digits_end + 1 < characters.len()
                    && matches!(characters[digits_end], '.' | ')')
                    && characters[digits_end + 1].is_whitespace()
                {
                    segments.push(current.trim().to_string());
                    current.clear();
                    index = digits_end + 1;
                    continue;
                }
            }
            current.push(characters[index]);
            index += 1;
        }
        segments.push(current.trim().to_string());
    }
    segments
}

fn personnel_segment_count(segment: &str) -> Option<(&'static str, i64)> {
    let text = strip_list_marker(segment);
    if text.is_empty() {
        return None;
    }
    if let Some((label, rest)) = text.split_once(':') {
        if let Some(count) = leading_count(rest) {
            if let Some(category) = personnel_category(label) {
                if count > 0 {
                    return Some((category, count));
                }
            }
        }
    }
    if let Some((label, rest)) = text.split_once('=') {
        if let Some(count) = leading_count(rest) {
            if let Some(category) = personnel_category(label) {
                if count > 0 {
                    return Some((category, count));
                }
            }
        }
    }
    let lower = text.to_ascii_lowercase();
    if let Some(position) = lower.find("sebanyak") {
        let label = &text[..position];
        let rest = &text[position + "sebanyak".len()..];
        if let Some(count) = leading_count(rest) {
            if let Some(category) = personnel_category(label) {
                if count > 0 {
                    return Some((category, count));
                }
            }
        }
    }
    if let Some((count, rest)) = leading_count_and_rest(text) {
        if let Some(category) = personnel_category(rest) {
            if count > 0 {
                return Some((category, count));
            }
        }
    }
    None
}

/// Rincian personel yang ditulis dalam teks, mis. `Polri : 10 Personel`.
fn embedded_personnel(value: &str) -> BTreeMap<&'static str, i64> {
    let mut counts: BTreeMap<&'static str, i64> = BTreeMap::new();
    for segment in personnel_segments(value) {
        if let Some((category, count)) = personnel_segment_count(&segment) {
            *counts.entry(category).or_default() += count;
        }
    }
    counts
}

/// Membaca pasangan label/nilai pada satu baris, mis. `Polri = 9 Dinas = 0`.
fn scan_labeled_counts(value: &str) -> BTreeMap<&'static str, i64> {
    let characters: Vec<char> = value.chars().collect();
    let mut counts: BTreeMap<&'static str, i64> = BTreeMap::new();
    let mut label_start = 0;
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if matches!(character, '\n' | ';') {
            label_start = index + 1;
            index += 1;
            continue;
        }
        if matches!(character, ':' | '=') {
            let label: String = characters[label_start..index].iter().collect();
            let mut next = index + 1;
            while next < characters.len() && characters[next].is_whitespace() {
                next += 1;
            }
            let mut end = next;
            while end < characters.len()
                && (characters[end].is_ascii_digit() || matches!(characters[end], '.' | ','))
            {
                end += 1;
            }
            if end > next {
                let token: String = characters[next..end].iter().collect();
                if let Some(count) = parse_count(&token) {
                    if let Some(category) = personnel_category(strip_list_marker(&label)) {
                        *counts.entry(category).or_default() += count;
                    }
                }
                label_start = end;
                index = end;
                continue;
            }
            label_start = index + 1;
        }
        index += 1;
    }
    counts
}

fn number_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (index, character) in text.char_indices() {
        if character.is_ascii_digit() || (matches!(character, '.' | ',') && start.is_some()) {
            if start.is_none() {
                start = Some(index);
            }
        } else if let Some(begin) = start.take() {
            spans.push((begin, index));
        }
    }
    if let Some(begin) = start {
        spans.push((begin, text.len()));
    }
    spans
}

/// Jumlah jamaah dari teks, mis. `jamaah sekitar 450 orang` atau `450 jamaah`.
fn count_near(value: &str, anchors: &[&str]) -> Option<i64> {
    let mut best: Option<(usize, i64)> = None;
    for line in value.split(['\n', ';']) {
        let lower = line.to_ascii_lowercase();
        for anchor_word in anchors {
            let Some(anchor) = lower.find(anchor_word) else {
                continue;
            };
            let anchor_end = anchor + anchor_word.len();
            for (start, end) in number_spans(line) {
                let Some(count) = parse_count(&line[start..end]) else {
                    continue;
                };
                if count <= 0 || count > 1_000_000 {
                    continue;
                }
                let distance = if end <= anchor {
                    anchor - end
                } else if start >= anchor_end {
                    start - anchor_end
                } else {
                    0
                };
                if distance <= 20 && best.map_or(true, |(closest, _)| distance < closest) {
                    best = Some((distance, count));
                }
            }
        }
    }
    best.map(|(_, count)| count)
}

fn jamaah_from_text(value: &str) -> Option<i64> {
    count_near(value, &["jamaah", "jemaah"])
}

fn jemaat_from_text(value: &str) -> Option<i64> {
    count_near(value, &["jemaat", "umat"])
}

fn merge_best(best: &mut Option<i64>, count: i64) {
    *best = Some(best.map_or(count, |current| current.max(count)));
}

#[derive(Default)]
struct Attendance {
    polri: Option<i64>,
    jamaah: Option<i64>,
    jemaat: Option<i64>,
    masyarakat: Option<i64>,
}

/// Jumlah personel Polri, jamaah/jemaat, dan masyarakat yang hadir pada satu laporan.
fn row_attendance(row: &RecordView) -> Attendance {
    let mut polri: Option<i64> = None;
    let mut jamaah: Option<i64> = None;
    let mut jemaat: Option<i64> = None;
    for (key, value) in &row.fields {
        if is_polri_personnel_key(key) {
            if let Some(count) = polri_field_count(value) {
                merge_best(&mut polri, count);
            }
        }
        if is_jamaah_key(key) {
            if let Some(count) = first_count(value) {
                merge_best(&mut jamaah, count);
            }
        }
        if is_jemaat_key(key) {
            if let Some(count) = first_count(value) {
                merge_best(&mut jemaat, count);
            }
        }
    }

    let mut counts: BTreeMap<&'static str, i64> = BTreeMap::new();
    let scan_pertimbangan = row
        .fields
        .keys()
        .any(|key| key.trim().eq_ignore_ascii_case("peran") || is_jamaah_key(key));
    for (key, value) in &row.fields {
        let mut per_field: BTreeMap<&'static str, i64> = BTreeMap::new();
        if is_personnel_text_key(key) {
            for (category, count) in embedded_personnel(value) {
                per_field
                    .entry(category)
                    .and_modify(|current| *current = (*current).max(count))
                    .or_insert(count);
            }
        }
        if is_attendance_scan_key(key) {
            let lower = key.trim().to_ascii_lowercase();
            let allowed = !lower.contains("pertimbangan") || scan_pertimbangan;
            if allowed {
                for (category, count) in scan_labeled_counts(value) {
                    per_field
                        .entry(category)
                        .and_modify(|current| *current = (*current).max(count))
                        .or_insert(count);
                }
                if let Some(count) = jamaah_from_text(value) {
                    per_field
                        .entry("Jamaah")
                        .and_modify(|current| *current = (*current).max(count))
                        .or_insert(count);
                }
                if let Some(count) = jemaat_from_text(value) {
                    per_field
                        .entry("Jemaat")
                        .and_modify(|current| *current = (*current).max(count))
                        .or_insert(count);
                }
            }
        }
        for (category, count) in per_field {
            let entry = counts.entry(category).or_default();
            *entry = (*entry).max(count);
        }
    }
    if let Some(count) = counts.get("Polri") {
        merge_best(&mut polri, *count);
    }
    if let Some(count) = counts.get("Jamaah") {
        merge_best(&mut jamaah, *count);
    }
    if let Some(count) = counts.get("Jemaat") {
        merge_best(&mut jemaat, *count);
    }

    let masyarakat: i64 = counts
        .iter()
        .filter(|(category, _)| **category != "Polri" && **category != "Pemda/instansi")
        .map(|(_, count)| *count)
        .sum();
    Attendance {
        polri,
        jamaah,
        jemaat,
        masyarakat: (masyarakat > 0).then_some(masyarakat),
    }
}

fn is_area_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    (lower.contains("luas area")
        || lower.contains("luas kegiatan")
        || lower.contains("area kegiatan"))
        && !lower.contains("uraian")
        && !lower.contains("rincian")
}

fn is_bibit_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if lower.contains("uraian") || lower.contains("rincian") {
        return false;
    }
    lower.contains("bibit") || (lower.contains("jumlah pohon") && lower.contains("ditanam"))
}

fn is_jamaah_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if ["uraian", "rincian", "keluhan", "problem", "saran"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return false;
    }
    lower.contains("jamaah") || lower.contains("jemaah")
}

fn is_jemaat_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if ["uraian", "rincian", "keluhan", "problem", "saran"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return false;
    }
    lower.contains("jemaat")
        || (lower.contains("jumlah") && (lower.contains("ibadah") || lower.contains("umat")))
}

fn is_ibadah_place_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    lower.contains("tempat ibadah")
        && !lower.contains("jumlah")
        && !lower.contains("uraian")
        && !lower.contains("rincian")
}

/// Jenis tempat ibadah dari namanya. `None` bila nama bukan tempat ibadah.
fn ibadah_kind(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("gereja") || lower.contains("kapel") {
        return Some("Gereja");
    }
    let tokens: Vec<&str> = lower
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.iter().any(|token| {
        matches!(
            *token,
            "gpi" | "gkii" | "gki" | "hkbp" | "gpib" | "gbi" | "bethel"
        )
    }) {
        return Some("Gereja");
    }
    if lower.contains("pura")
        || lower.contains("pure")
        || lower.contains("kuil")
        || lower.contains("penglukatan")
    {
        return Some("Pura");
    }
    if lower.contains("vihara") || lower.contains("wihara") {
        return Some("Wihara");
    }
    if lower.contains("klenteng") {
        return Some("Klenteng");
    }
    if lower.contains("candi") {
        return Some("Candi");
    }
    if lower.contains("sinagog") {
        return Some("Sinagog");
    }
    if lower.contains("ashram") {
        return Some("Ashram");
    }
    if lower.contains("rumah ibadah") || lower.contains("tempat ibadah") {
        return Some("Lainnya");
    }
    None
}

/// QW1.8 hanya untuk agama non-Islam; laporan bertema masjid tidak dihitung.
fn is_masjid_place(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("masjid")
        || lower.contains("mesjid")
        || lower.contains("musholla")
        || lower.contains("musala")
        || lower.contains("mushola")
}

fn row_is_masjid(row: &RecordView) -> bool {
    row.fields
        .iter()
        .any(|(key, value)| is_ibadah_place_key(key) && is_masjid_place(value))
}

fn ibadah_kind_counts(matching: &[&RecordView]) -> (Vec<LabeledCount>, i64) {
    let mut counts: BTreeMap<&'static str, i64> = BTreeMap::new();
    let mut filled = 0_i64;
    for row in matching {
        for (key, value) in &row.fields {
            if is_ibadah_place_key(key) && !is_card_noise(value) {
                if let Some(kind) = ibadah_kind(value) {
                    *counts.entry(kind).or_default() += 1;
                    filled += 1;
                    break;
                }
            }
        }
    }
    let mut values: Vec<LabeledCount> = counts
        .into_iter()
        .map(|(label, count)| LabeledCount::new(label, count))
        .collect();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    (values, filled)
}

/// Nama tempat tanpa koordinat di ujung, dipakai saat menghitung lokasi unik.
fn place_name(value: &str) -> String {
    let mut name = value.trim().to_ascii_lowercase();
    if name.ends_with(')') {
        if let Some(position) = name.rfind('(') {
            let inner = &name[position + 1..name.len() - 1];
            if inner.contains(',') || inner.chars().any(|character| character.is_ascii_digit()) {
                name.truncate(position);
            }
        }
    }
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Field teks yang boleh dipindai untuk rincian kehadiran.
fn is_attendance_scan_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if [
        "dokumentasi",
        "link",
        "nama dan pangkat",
        "pelaksana",
        "pelapor",
        "status",
        "saran",
        "problem",
        "keluhan",
        "materi",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return false;
    }
    lower.contains("peran")
        || lower.contains("pertimbangan pilihan lokasi")
        || lower.contains("kegiatan lain")
}

/// Angka pertama pada nilai, mis. `50 Pohon, terdiri dari ...` -> 50.
fn first_count(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    let token: String = trimmed
        .chars()
        .take_while(|character| character.is_ascii_digit() || matches!(character, '.' | ','))
        .collect();
    if token.is_empty() {
        parse_count(trimmed)
    } else {
        parse_count(&token)
    }
}

fn is_green_text_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if [
        "dokumentasi",
        "link",
        "nama dan pangkat",
        "pelaksana",
        "pelapor",
        "status",
        "saran",
        "problem",
        "keluhan",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return false;
    }
    lower.contains("nama lokasi")
        || lower.contains("lokasi kegiatan")
        || lower.contains("nama tempat")
        || lower.contains("pertimbangan pilihan lokasi")
        || lower.contains("rincian")
        || lower == "kegiatan"
        || lower.contains("kegiatan lain")
}

fn green_text(row: &RecordView) -> String {
    let mut text = String::new();
    for (key, value) in &row.fields {
        if is_green_text_key(key) && !is_card_noise(value) {
            text.push_str(value);
            text.push(' ');
        }
    }
    text
}

/// Jenis area kegiatan dari lokasi, pertimbangan, atau rincian pohon.
fn area_kind(text: &str) -> Option<&'static str> {
    let lower = text.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }
    if lower.contains("mangrove") || lower.contains("bakau") {
        return Some("Hutan mangrove");
    }
    if ["pantai", "pesisir", "segara", "laut", "pulau"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Some("Pantai/pesisir");
    }
    if [
        "taman",
        "lapangan",
        "alun",
        "gor ",
        "shelter",
        "parkir",
        "ruang terbuka",
        "ruang publik",
        "pedestrian",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return Some("Ruang publik");
    }
    if lower.contains("hutan") {
        return Some("Hutan");
    }
    if ["sungai", "bantaran", "saluran", "selokan"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Some("Bantaran sungai");
    }
    if ["danau", "embung", "waduk", "bendungan", "telaga"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Some("Danau/embung");
    }
    if ["trotoar", "simpang", "jalan"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Some("Jalan/trotoar");
    }
    if ["pura", "masjid", "gereja", "vihara", "klenteng"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Some("Tempat ibadah");
    }
    if ["pasar", "terminal", "sekolah", "kampus", "kantor"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Some("Fasilitas umum");
    }
    Some("Lainnya")
}

/// Jumlah lokasi titik gatur per polres dari field "Jumlah lokasi gatur pagi".
fn gatur_location_breakdown(matching: &[&RecordView]) -> Option<CoverageBreakdown> {
    let mut counts: BTreeMap<String, (String, i64)> = BTreeMap::new();
    let mut filled = 0_i64;
    for row in matching {
        let mut count: Option<i64> = None;
        for (key, value) in &row.fields {
            let lower = key.trim().to_ascii_lowercase();
            if lower.contains("jumlah lokasi") && lower.contains("gatur") {
                if let Some(parsed) = first_count(value) {
                    count = Some(parsed);
                    break;
                }
            }
        }
        let Some(count) = count else {
            continue;
        };
        let label = if !row.polres.trim().is_empty() {
            row.polres.trim().to_string()
        } else if !row.satuan.trim().is_empty() {
            row.satuan.trim().to_string()
        } else {
            continue;
        };
        filled += 1;
        let entry = counts
            .entry(label.to_ascii_lowercase())
            .or_insert((label, 0));
        entry.1 += count;
    }
    if counts.is_empty() {
        return None;
    }
    let mut values: Vec<LabeledCount> = counts
        .into_values()
        .map(|(label, count)| LabeledCount::new(label, count))
        .collect();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    Some(CoverageBreakdown {
        key: "lokasi-gatur-per-polres".to_string(),
        label: "Jumlah lokasi titik gatur per polres".to_string(),
        filled_rows: filled,
        values,
    })
}

/// Jumlah personel Polri yang bertugas per polres pada laporan gatur.
fn personnel_breakdown(matching: &[&RecordView]) -> Option<CoverageBreakdown> {
    let mut counts: BTreeMap<String, (String, i64)> = BTreeMap::new();
    let mut filled = 0_i64;
    for row in matching {
        let Some(count) = row_attendance(row).polri else {
            continue;
        };
        if count <= 0 {
            continue;
        }
        let label = if !row.polres.trim().is_empty() {
            row.polres.trim().to_string()
        } else if !row.satuan.trim().is_empty() {
            row.satuan.trim().to_string()
        } else {
            continue;
        };
        filled += 1;
        let entry = counts
            .entry(label.to_ascii_lowercase())
            .or_insert((label, 0));
        entry.1 += count;
    }
    if counts.is_empty() {
        return None;
    }
    let mut values: Vec<LabeledCount> = counts
        .into_values()
        .map(|(label, count)| LabeledCount::new(label, count))
        .collect();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    Some(CoverageBreakdown {
        key: "personel-bertugas-per-polres".to_string(),
        label: "Jumlah personel bertugas per polres".to_string(),
        filled_rows: filled,
        values,
    })
}

const KERAWANAN_KINDS: &[&str] = &[
    "Rawan kemacetan",
    "Rawan keramaian",
    "Rawan pelanggaran",
    "Rawan kecelakaan",
    "Rawan banjir",
    "Rawan kriminalitas",
    "Rawan narkoba",
    "Lainnya",
];

fn kerawanan_kinds(value: &str) -> BTreeSet<&'static str> {
    let mut text = value.trim();
    if let Some(rest) = text
        .strip_prefix("Lainnya:")
        .or_else(|| text.strip_prefix("lainnya:"))
    {
        text = rest.trim();
    }
    let lower = text.to_ascii_lowercase();
    let mut kinds = BTreeSet::new();
    if is_card_noise(text)
        || [
            "tidak dilaksanakan",
            "tidak melaksanakan",
            "belum melakukan",
            "tidak ada giat",
            "nihil kegiatan",
            "sesuai jadwal tidak",
        ]
        .iter()
        .any(|phrase| lower.contains(phrase))
    {
        return kinds;
    }
    if lower.contains("kemacetan") || lower.contains("macet") {
        kinds.insert("Rawan kemacetan");
    }
    if lower.contains("keramaian") {
        kinds.insert("Rawan keramaian");
    }
    if lower.contains("pelanggaran") {
        kinds.insert("Rawan pelanggaran");
    }
    if lower.contains("laka") || lower.contains("kecelakaan") {
        kinds.insert("Rawan kecelakaan");
    }
    if lower.contains("banjir") {
        kinds.insert("Rawan banjir");
    }
    if lower.contains("kriminal") || lower.contains("kamtibmas") {
        kinds.insert("Rawan kriminalitas");
    }
    if lower.contains("narkoba") {
        kinds.insert("Rawan narkoba");
    }
    if kinds.is_empty() && lower.contains("rawan") {
        kinds.insert("Lainnya");
    }
    kinds
}

/// Rekap jenis kerawanan per polres dari "Pertimbangan pilihan lokasi".
fn kerawanan_table(matching: &[&RecordView]) -> Option<CoverageTable> {
    let mut counts: BTreeMap<String, (String, BTreeMap<&'static str, i64>)> = BTreeMap::new();
    let mut filled = 0_i64;
    for row in matching {
        let mut kinds: BTreeSet<&'static str> = BTreeSet::new();
        for (key, value) in &row.fields {
            let lower = key.trim().to_ascii_lowercase();
            if (lower == "pertimbangan pilihan lokasi"
                || lower == "pertimbangan pilihan lokasi (alasan utama)")
                && !is_card_noise(value)
            {
                kinds.extend(kerawanan_kinds(value));
            }
        }
        if kinds.is_empty() {
            continue;
        }
        let label = if !row.polres.trim().is_empty() {
            row.polres.trim().to_string()
        } else if !row.satuan.trim().is_empty() {
            row.satuan.trim().to_string()
        } else {
            continue;
        };
        filled += 1;
        let entry = counts
            .entry(label.to_ascii_lowercase())
            .or_insert((label, BTreeMap::new()));
        for kind in kinds {
            *entry.1.entry(kind).or_default() += 1;
        }
    }
    if counts.is_empty() {
        return None;
    }

    let columns: Vec<&'static str> = KERAWANAN_KINDS
        .iter()
        .copied()
        .filter(|kind| {
            counts
                .values()
                .any(|(_, items)| items.get(kind).is_some_and(|count| *count > 0))
        })
        .collect();
    if columns.is_empty() {
        return None;
    }

    let mut rows: Vec<CoverageTableRow> = counts
        .into_values()
        .map(|(label, items)| CoverageTableRow {
            label,
            values: columns
                .iter()
                .map(|kind| items.get(kind).copied().unwrap_or(0))
                .collect(),
        })
        .collect();
    rows.sort_by(|left, right| {
        right
            .values
            .iter()
            .sum::<i64>()
            .cmp(&left.values.iter().sum::<i64>())
            .then_with(|| left.label.cmp(&right.label))
    });

    let mut header = vec!["Polres".to_string()];
    header.extend(columns.iter().map(|kind| (*kind).to_string()));
    Some(CoverageTable {
        key: "kerawanan-per-polres".to_string(),
        label: "Rekap jenis kerawanan per polres".to_string(),
        columns: header,
        rows,
        filled_rows: filled,
    })
}

fn area_kind_counts(matching: &[&RecordView]) -> (Vec<LabeledCount>, i64) {
    let mut counts: BTreeMap<&'static str, i64> = BTreeMap::new();
    let mut filled = 0_i64;
    for row in matching {
        if let Some(kind) = area_kind(&green_text(row)) {
            *counts.entry(kind).or_default() += 1;
            filled += 1;
        }
    }
    let mut values: Vec<LabeledCount> = counts
        .into_iter()
        .map(|(label, count)| LabeledCount::new(label, count))
        .collect();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    (values, filled)
}

fn is_community_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if lower.contains("saran") || lower.contains("problem") || lower.contains("keluhan") {
        return false;
    }
    lower.contains("komunitas") || lower.contains("paguyuban")
}

fn is_community_noise(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    is_card_noise(value)
        || lower.starts_with("tidak melaksanakan")
        || lower.starts_with("tidak dilaksanakan")
        || lower.starts_with("masih proses")
        || lower == "nihl"
}

const COMMUNITY_RULES: &[(&str, &[&str])] = &[
    (
        "Nelayan",
        &["nelayan", "abk", "kapal", "perahu", "perikanan", "tpi"],
    ),
    (
        "Ojek online/driver",
        &["ojek", "ojol", "driver", "gojek", "grab", "taksi", "angkot"],
    ),
    ("Petani", &["tani", "subak", "sawah", "ladang"]),
    ("Peternak", &["ternak", "peternak", "ayam", "sapi", "babi"]),
    (
        "Pedagang",
        &[
            "pedagang", "pasar", "warung", "toko", "usaha", "pkl", "tiket",
        ],
    ),
    (
        "Pelajar/mahasiswa",
        &[
            "siswa",
            "siswi",
            "pelajar",
            "guru",
            "sekolah",
            "smp",
            "sma",
            "smk",
            "mahasiswa",
            "kampus",
            "universitas",
        ],
    ),
    (
        "Pariwisata",
        &["wisata", "pariwisata", "obyek", "hotel", "pemandu", "tour"],
    ),
    (
        "Buruh/pekerja",
        &[
            "buruh",
            "karyawan",
            "pekerja",
            "porter",
            "petugas",
            "pengelola",
            "kebersihan",
        ],
    ),
    ("Lansia", &["lansia", "jompo"]),
    (
        "Tokoh masyarakat/adat",
        &[
            "tokoh", "adat", "prajuru", "pecalang", "bendesa", "pemangku", "krama", "toga",
            "tomas", "todat",
        ],
    ),
    (
        "Perangkat desa/kelurahan",
        &[
            "perangkat",
            "aparatur",
            "kepala desa",
            "lurah",
            "perbekel",
            "kantor desa",
            "kadus",
            "dinas",
            "instansi",
        ],
    ),
    (
        "Komunitas/paguyuban",
        &[
            "komunitas",
            "paguyuban",
            "forum",
            "ormas",
            "organisasi",
            "sekehe",
            "perkumpulan",
            "klub",
            "himpunan",
            "pesemetonan",
            "banjar",
        ],
    ),
    (
        "Warga/masyarakat umum",
        &[
            "warga",
            "masyarakat",
            "penduduk",
            "umum",
            "setempat",
            "keluarga",
            "kampung",
            "desa",
            "kelurahan",
        ],
    ),
];

fn community_kinds(value: &str) -> BTreeSet<&'static str> {
    let mut kinds = BTreeSet::new();
    if is_community_noise(value) {
        return kinds;
    }
    let lower = value.to_ascii_lowercase();
    for (label, needles) in COMMUNITY_RULES {
        if needles.iter().any(|needle| lower.contains(needle)) {
            kinds.insert(*label);
        }
    }
    if kinds.is_empty() {
        kinds.insert("Lainnya");
    }
    kinds
}

/// Kategori masyarakat yang terlibat pada satu laporan.
pub(crate) fn row_community_kinds(row: &RecordView) -> BTreeSet<&'static str> {
    let mut kinds = BTreeSet::new();
    for (key, value) in &row.fields {
        if is_community_key(key) {
            kinds.extend(community_kinds(value));
        }
    }
    kinds
}

/// Jumlah laporan per kategori masyarakat yang terlibat.
pub(crate) fn community_counts(matching: &[&RecordView]) -> Vec<LabeledCount> {
    let mut counts: BTreeMap<&'static str, i64> = BTreeMap::new();
    for row in matching {
        for kind in row_community_kinds(row) {
            *counts.entry(kind).or_default() += 1;
        }
    }
    let mut values: Vec<LabeledCount> = counts
        .into_iter()
        .map(|(label, count)| LabeledCount::new(label, count))
        .collect();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    values
}

/// Total personel Polri pada sekumpulan laporan.
pub(crate) fn polri_total(matching: &[&RecordView]) -> i64 {
    matching
        .iter()
        .filter_map(|row| row_attendance(row).polri)
        .sum()
}

fn is_public_feedback_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if lower.contains("personel") || lower.contains("polri") {
        return false;
    }
    (lower.contains("saran") || lower.contains("masukan") || lower.contains("keluhan"))
        && (lower.contains("masyarakat") || lower.contains("warga"))
}

fn public_feedback(row: &RecordView) -> Option<&str> {
    row.fields.iter().find_map(|(key, value)| {
        (is_public_feedback_key(key) && !is_card_noise(value)).then_some(value.as_str())
    })
}

#[derive(Default)]
struct FeedbackSummary {
    apresiasi: i64,
    keluhan: i64,
    tindak_lanjut: i64,
    filled: i64,
}

fn feedback_summary(matching: &[&RecordView]) -> FeedbackSummary {
    let mut summary = FeedbackSummary::default();
    for row in matching {
        let Some(text) = public_feedback(row) else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        summary.filled += 1;
        if lower.contains("apresiasi")
            || lower.contains("terima kasih")
            || lower.contains("trimakasih")
            || lower.contains("ucapan terima")
        {
            summary.apresiasi += 1;
        }
        summary.keluhan += sum_number_before(text, "saran atau keluhan");
        summary.tindak_lanjut += sum_number_before(text, "ditindaklanjuti");
    }
    summary
}

fn strip_list_prefix(key: &str) -> &str {
    let trimmed = key.trim_start();
    let digits = trimmed
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits > 0 {
        let rest = &trimmed[digits..];
        if let Some(rest) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')) {
            return rest.trim_start();
        }
    }
    trimmed
}

/// Jenis barang bukti dari teks rincian.
fn barang_jenis(value: &str) -> &'static str {
    let lower = value.to_ascii_lowercase();
    if lower.contains("sabu") || lower.contains("shabu") || lower.contains("metamfetamin") {
        return "Sabu";
    }
    if lower.contains("ganja") || lower.contains("biji kering") {
        return "Ganja";
    }
    if lower.contains("ekstasi") || lower.contains("tablet") {
        return "Ekstasi";
    }
    if lower.contains("tembakau") || lower.contains("gorila") {
        return "Tembakau";
    }
    if lower.contains("bong") {
        return "Alat/bong";
    }
    "Lainnya"
}

/// Mengambil angka setelah label pertama, mis. "1) Golongan 1: 2, ..." -> 2.
fn labeled_number(value: &str) -> Option<i64> {
    let (_, rest) = value.split_once(':')?;
    let token = rest.split(',').next()?.trim();
    parse_count(token)
}

/// Membangun kartu angka dan distribusi dari isian laporan satu sub-kode.
fn build_code_cards(
    matching: &[&RecordView],
    code: &str,
) -> (Vec<CoverageMetric>, Vec<CoverageBreakdown>) {
    let total = matching.len();
    if total == 0 {
        return (Vec::new(), Vec::new());
    }

    let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in matching {
        for (key, value) in &row.fields {
            if is_card_noise(value) {
                continue;
            }
            fields
                .entry(key.clone())
                .or_default()
                .push(value.trim().to_string());
        }
    }

    let min_rows = std::cmp::max(3, total / 6);
    let mut entries: Vec<(String, Vec<String>)> = fields.into_iter().collect();
    entries.sort_by(|left, right| {
        right
            .1
            .len()
            .cmp(&left.1.len())
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut metrics: Vec<CoverageMetric> = Vec::new();
    let mut breakdowns: Vec<CoverageBreakdown> = Vec::new();
    let mut jenjang_done = false;

    let has_community = matching
        .iter()
        .any(|row| row.fields.keys().any(|key| is_community_key(key)));
    let has_ibadah_place = matching
        .iter()
        .any(|row| row.fields.keys().any(|key| is_ibadah_place_key(key)));

    // Personel Polri dan masyarakat yang hadir dari field angka maupun teks laporan.
    // Laporan masjid tidak dihitung pada QW1.8 (khusus non-Islam).
    let attendance: Vec<Attendance> = matching
        .iter()
        .map(|row| {
            if has_ibadah_place && row_is_masjid(row) {
                Attendance::default()
            } else {
                row_attendance(row)
            }
        })
        .collect();
    let polri_total: i64 = attendance.iter().filter_map(|item| item.polri).sum();
    let polri_filled = attendance
        .iter()
        .filter(|item| item.polri.is_some())
        .count();
    // QW1.1 selalu menampilkan kartu personel agar bisa dipakai di tab Desain,
    // meski laporan periode ini belum mengisi jumlah personel.
    let is_gatur = code == "QW1.1";
    if polri_filled > 0 || is_gatur {
        metrics.push(card_metric(
            if is_gatur {
                "jumlah-personel-bertugas"
            } else {
                "jumlah-personel"
            },
            if is_gatur {
                "Jumlah personel yang bertugas"
            } else {
                "Jumlah personel"
            },
            "orang",
            polri_total as f64,
            polri_filled,
            total,
        ));
    }
    if is_gatur {
        if let Some(breakdown) = personnel_breakdown(matching) {
            breakdowns.push(breakdown);
        }
    }

    let jamaah_total: i64 = attendance.iter().filter_map(|item| item.jamaah).sum();
    let jamaah_filled = attendance
        .iter()
        .filter(|item| item.jamaah.is_some())
        .count();
    if jamaah_filled > 0 {
        metrics.push(card_metric(
            "jumlah-jamaah",
            "Jumlah jamaah",
            "orang",
            jamaah_total as f64,
            jamaah_filled,
            total,
        ));
    }

    let jemaat_total: i64 = attendance.iter().filter_map(|item| item.jemaat).sum();
    let jemaat_filled = attendance
        .iter()
        .filter(|item| item.jemaat.is_some())
        .count();
    if jemaat_filled > 0 {
        metrics.push(card_metric(
            "jumlah-jemaat",
            "Jumlah jemaat",
            "orang",
            jemaat_total as f64,
            jemaat_filled,
            total,
        ));
    }

    if has_ibadah_place {
        let (kinds, filled) = ibadah_kind_counts(matching);
        if !kinds.is_empty() {
            breakdowns.push(CoverageBreakdown {
                key: "jenis-ibadah".to_string(),
                label: "Jenis tempat ibadah".to_string(),
                filled_rows: filled,
                values: kinds,
            });
        }
    }

    let has_green = matching.iter().any(|row| {
        row.fields
            .keys()
            .any(|key| is_area_key(key) || is_bibit_key(key))
    });
    if has_green {
        let mut area_total = 0_i64;
        let mut area_filled = 0_usize;
        let mut bibit_total = 0_i64;
        let mut bibit_filled = 0_usize;
        for row in matching {
            let mut area: Option<i64> = None;
            let mut bibit: Option<i64> = None;
            for (key, value) in &row.fields {
                if is_area_key(key) {
                    if let Some(count) = first_count(value) {
                        merge_best(&mut area, count);
                    }
                }
                if is_bibit_key(key) {
                    if let Some(count) = first_count(value) {
                        merge_best(&mut bibit, count);
                    }
                }
            }
            if let Some(count) = area {
                area_total += count;
                area_filled += 1;
            }
            if let Some(count) = bibit {
                bibit_total += count;
                bibit_filled += 1;
            }
        }
        if area_filled > 0 {
            metrics.push(card_metric(
                "luas-area",
                "Luas area kegiatan",
                "m²",
                area_total as f64,
                area_filled,
                total,
            ));
        }
        if bibit_filled > 0 {
            metrics.push(card_metric(
                "jumlah-bibit",
                "Jumlah bibit",
                "bibit",
                bibit_total as f64,
                bibit_filled,
                total,
            ));
        }
        let (kinds, filled) = area_kind_counts(matching);
        if !kinds.is_empty() {
            breakdowns.push(CoverageBreakdown {
                key: "jenis-area".to_string(),
                label: "Jenis area kegiatan".to_string(),
                filled_rows: filled,
                values: kinds,
            });
        }
    }

    if has_community {
        let masyarakat_total: i64 = attendance.iter().filter_map(|item| item.masyarakat).sum();
        let masyarakat_filled = attendance
            .iter()
            .filter(|item| item.masyarakat.is_some())
            .count();
        if masyarakat_filled > 0 {
            metrics.push(card_metric(
                "jumlah-masyarakat",
                "Jumlah masyarakat",
                "orang",
                masyarakat_total as f64,
                masyarakat_filled,
                total,
            ));
        }

        let feedback = feedback_summary(matching);
        if feedback.filled > 0 {
            metrics.push(card_metric(
                "apresiasi",
                "Apresiasi",
                "",
                feedback.apresiasi as f64,
                feedback.filled as usize,
                total,
            ));
            metrics.push(card_metric(
                "keluhan",
                "Keluhan",
                "",
                feedback.keluhan as f64,
                feedback.filled as usize,
                total,
            ));
            metrics.push(card_metric(
                "tindak-lanjut",
                "Tindak lanjut",
                "",
                feedback.tindak_lanjut as f64,
                feedback.filled as usize,
                total,
            ));
        }

        let polda_rows = matching.iter().filter(|row| record_is_polda(row)).count();
        breakdowns.push(CoverageBreakdown {
            key: "giat-polda-polres".to_string(),
            label: "Giat Polda dan polres jajaran".to_string(),
            filled_rows: total as i64,
            values: vec![
                LabeledCount::new("Polda", polda_rows as i64),
                LabeledCount::new("Polres jajaran", (total - polda_rows) as i64),
            ],
        });

        let communities = community_counts(matching);
        if !communities.is_empty() {
            let filled = matching
                .iter()
                .filter(|row| !row_community_kinds(row).is_empty())
                .count() as i64;
            breakdowns.push(CoverageBreakdown {
                key: "masyarakat-terlibat".to_string(),
                label: "Masyarakat yang terlibat".to_string(),
                filled_rows: filled,
                values: communities,
            });
        }
    }

    // Personel/peserta yang ditulis per baris: "Polri: 5 pers", "BNN: 3 pers".
    let mut personnel: BTreeMap<&'static str, i64> = BTreeMap::new();
    let mut personnel_rows = 0_i64;
    for (key, values) in &entries {
        let Some(label) = personnel_label(key) else {
            continue;
        };
        let numbers: Vec<i64> = values
            .iter()
            .filter_map(|value| parse_count(value))
            .collect();
        if numbers.is_empty() || numbers.len() * 10 < values.len() * 7 {
            continue;
        }
        *personnel.entry(label).or_default() += numbers.iter().sum::<i64>();
        personnel_rows += numbers.len() as i64;
    }
    if !personnel.is_empty() {
        let mut items: Vec<LabeledCount> = personnel
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|(label, count)| LabeledCount::new(label, count))
            .collect();
        if !items.is_empty() {
            items.sort_by(|left, right| right.count.cmp(&left.count));
            breakdowns.push(CoverageBreakdown {
                key: "personel-terlibat".to_string(),
                label: "Personel/peserta terlibat".to_string(),
                filled_rows: personnel_rows,
                values: items,
            });
        }
    }

    for (key, values) in entries {
        let lower = key.to_ascii_lowercase();

        // Field personel sudah diringkas pada kartu "Personel/peserta terlibat".
        if personnel_label(&key).is_some() {
            continue;
        }

        // Personel Polri sudah dijumlahkan pada kartu "Jumlah personel".
        if is_polri_personnel_key(&key) {
            continue;
        }

        // Rincian masyarakat dan masukan sudah menjadi kartu tersendiri.
        if (has_community && is_community_key(&key)) || is_public_feedback_key(&key) {
            continue;
        }

        // Luas area dan bibit sudah dijumlahkan pada kartu tersendiri.
        if is_area_key(&key) || is_bibit_key(&key) {
            continue;
        }

        // Jumlah jamaah sudah dijumlahkan pada kartu tersendiri.
        if is_jamaah_key(&key) {
            continue;
        }

        // Jumlah jemaat sudah dijumlahkan pada kartu tersendiri.
        if is_jemaat_key(&key) {
            continue;
        }

        // Barang bukti berformat "Golongan 1: <jumlah>, <rincian>".
        if lower.contains("barang bukti")
            && !lower.contains("rincian")
            && !lower.contains("uraian")
            && !lower.contains("jumlah")
        {
            if values.len() >= 3 && metrics.len() < 8 {
                let mut total_qty = 0_i64;
                let mut per_jenis: BTreeMap<&'static str, i64> = BTreeMap::new();
                for value in &values {
                    let qty = labeled_number(value).unwrap_or(0);
                    total_qty += qty;
                    *per_jenis.entry(barang_jenis(value)).or_default() += qty;
                }
                metrics.push(card_metric(
                    "barang-bukti",
                    "Barang bukti diamankan",
                    "",
                    total_qty as f64,
                    values.len(),
                    total,
                ));
                let mut items: Vec<LabeledCount> = per_jenis
                    .into_iter()
                    .filter(|(_, count)| *count > 0)
                    .map(|(jenis, count)| LabeledCount::new(jenis, count))
                    .collect();
                if !items.is_empty() && breakdowns.len() < 6 {
                    items.sort_by(|left, right| right.count.cmp(&left.count));
                    breakdowns.push(CoverageBreakdown {
                        key: "barang-bukti-jenis".to_string(),
                        label: "Barang bukti per jenis".to_string(),
                        filled_rows: values.len() as i64,
                        values: items,
                    });
                }
            }
            continue;
        }

        if values.len() < min_rows {
            continue;
        }

        // Nama sekolah/masjid/tempat/lokasi: hitung jumlah tempat unik.
        if let Some((card_key, label, unit)) = place_card(&key) {
            if metrics.iter().any(|metric| metric.key == card_key) {
                continue;
            }
            let distinct: BTreeSet<String> = values
                .iter()
                .filter(|value| card_key != "ibadah" || ibadah_kind(value).is_some())
                .map(|value| place_name(value))
                .collect();
            if distinct.is_empty() {
                continue;
            }
            if metrics.len() < 8 {
                metrics.push(card_metric(
                    card_key,
                    label,
                    unit,
                    distinct.len() as f64,
                    values.len(),
                    total,
                ));
            }
            if card_key == "sekolah" && !jenjang_done {
                let mut per_jenjang: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
                for value in &values {
                    if let Some(jenjang) = jenjang_of(value) {
                        per_jenjang
                            .entry(jenjang)
                            .or_default()
                            .insert(value.to_ascii_lowercase());
                    }
                }
                if per_jenjang.len() > 1 {
                    let mut items: Vec<LabeledCount> = per_jenjang
                        .into_iter()
                        .map(|(jenjang, set)| LabeledCount::new(jenjang, set.len() as i64))
                        .collect();
                    items.sort_by(|left, right| right.count.cmp(&left.count));
                    breakdowns.push(CoverageBreakdown {
                        key: "jenjang".to_string(),
                        label: "Jenjang pendidikan".to_string(),
                        filled_rows: values.len() as i64,
                        values: items,
                    });
                    jenjang_done = true;
                }
            }
            continue;
        }

        if is_card_skip(&key) {
            continue;
        }

        let numbers: Vec<i64> = values
            .iter()
            .filter_map(|value| parse_count(value))
            .collect();
        if numbers.len() * 10 >= values.len() * 7 && metrics.len() < 8 {
            let sum: f64 = numbers.iter().map(|value| *value as f64).sum();
            let label = strip_list_prefix(&key).to_string();
            metrics.push(card_metric(
                &format!("field-{}", card_slug(&key)),
                &label,
                card_unit(&key),
                sum,
                numbers.len(),
                total,
            ));
            continue;
        }

        if breakdowns.len() >= 5 {
            continue;
        }
        let mut counts: BTreeMap<String, (String, i64)> = BTreeMap::new();
        for value in &values {
            let label = card_value_label(value);
            let entry = counts
                .entry(label.to_ascii_lowercase())
                .or_insert((label, 0));
            entry.1 += 1;
        }
        if counts.len() < 2 || counts.len() > 10 {
            continue;
        }
        let mut items: Vec<LabeledCount> = counts
            .into_values()
            .map(|(label, count)| LabeledCount::new(label, count))
            .collect();
        items.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.label.cmp(&right.label))
        });
        items.truncate(8);
        breakdowns.push(CoverageBreakdown {
            key: format!("field-{}", card_slug(&key)),
            label: strip_list_prefix(&key).to_string(),
            filled_rows: values.len() as i64,
            values: items,
        });
    }

    if let Some(breakdown) = gatur_location_breakdown(matching) {
        breakdowns.push(breakdown);
    }

    (metrics, breakdowns)
}

/// Kartu dan tabel unit untuk seluruh sub-kode Quick Wins.
fn build_code_programs(
    rows: &[RecordView],
    known: &KnownUnits,
    mode: UnitMode,
) -> Vec<CoverageProgram> {
    qw::CODES
        .iter()
        .map(|code| {
            let matching: Vec<&RecordView> = rows
                .iter()
                .filter(|row| qw::normalize(&row.kode) == code.code)
                .collect();
            let (metrics, breakdowns) = build_code_cards(&matching, code.code);
            let tables: Vec<CoverageTable> = if code.code == "QW1.1" {
                kerawanan_table(&matching).into_iter().collect()
            } else {
                Vec::new()
            };
            let performed_polres = units_of(&matching, mode, Scope::Polres);
            let performed_satker = units_of(&matching, mode, Scope::Satker);
            CoverageProgram {
                key: code.code.to_string(),
                title: code.title.to_string(),
                code: code.code.to_string(),
                rows: matching.len() as i64,
                polres: performed_polres.len() as i64,
                satker: performed_satker.len() as i64,
                polres_total: responsible_units(
                    known,
                    Scope::Polres,
                    mode,
                    code.code,
                    &performed_polres,
                )
                .len() as i64,
                satker_total: responsible_units(
                    known,
                    Scope::Satker,
                    mode,
                    code.code,
                    &performed_satker,
                )
                .len() as i64,
                polres_missing: missing_units(
                    known,
                    Scope::Polres,
                    mode,
                    code.code,
                    &performed_polres,
                ),
                satker_missing: missing_units(
                    known,
                    Scope::Satker,
                    mode,
                    code.code,
                    &performed_satker,
                ),
                metrics,
                by_polres: unit_row_counts(&matching, mode, Scope::Polres),
                by_satker: unit_row_counts(&matching, mode, Scope::Satker),
                breakdowns,
                tables,
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
struct MetricSpec {
    key: &'static str,
    label: &'static str,
    unit: &'static str,
    kind: MetricKind,
}

#[derive(Clone, Copy)]
struct ProgramSpec {
    key: &'static str,
    title: &'static str,
    code: &'static str,
    metrics: &'static [MetricSpec],
}

#[derive(Clone, Copy)]
enum MetricKind {
    Sum(SumSource),
    Unique(UniqueSource),
    Ibadah,
    Flag(FlagSource),
}

#[derive(Clone, Copy)]
enum SumSource {
    Personel,
    Bibit,
    Area,
    Jamaah,
    NarkobaOrang,
    NarkobaKawasan,
    NarkobaRehab,
    NarkobaPenyuluhan,
    Peserta,
}

#[derive(Clone, Copy)]
enum UniqueSource {
    Location,
}

#[derive(Clone, Copy)]
enum FlagSource {
    KerjaBakti,
    MediaPublication,
    MediaDocumentation,
    MediaAny,
}

const NARKOBA_ORANG_KEYS: &[&str] = &[
    "Orang yang ditangkap (Jumlah)",
    "Jumlah orang yang ditangkap (Jumlah)",
    "Jumlah orang yang diamankan (Jumlah)",
    "Jumlah tersangka",
];
const NARKOBA_KAWASAN_KEYS: &[&str] = &[
    "Jumlah kawasan narkoba (Jumlah)",
    "Jumlah kawasan yang sudah dibina (Jumlah)",
];
const NARKOBA_REHAB_KEYS: &[&str] = &[
    "Orang yang direhabilitasi (Jumlah)",
    "Jumlah pengguna yang direhabilitasi",
];
const NARKOBA_PENYULUHAN_KEYS: &[&str] = &[
    "Jumlah peserta penyuluhan narkoba",
    "Orang yang diberikan penyuluhan (Jumlah)",
];
const LOCATION_KEYS: &[&str] = &[
    "Lokasi kegiatan",
    "Nama lokasi dan titik koordinat",
    "Nama masjid dan titik koordinat",
    "Nama tempat ibadah dan titik koordinat",
    "Nama sekolah dan titik koordinat",
    "Rute patroli",
];

const GREEN_METRICS: &[MetricSpec] = &[
    MetricSpec {
        key: "area",
        label: "Luas area",
        unit: "m²",
        kind: MetricKind::Sum(SumSource::Area),
    },
    MetricSpec {
        key: "bibit",
        label: "Bibit ditanam",
        unit: "pohon",
        kind: MetricKind::Sum(SumSource::Bibit),
    },
    MetricSpec {
        key: "personel",
        label: "Personel",
        unit: "orang",
        kind: MetricKind::Sum(SumSource::Personel),
    },
    MetricSpec {
        key: "kerja_bakti",
        label: "Kerja bakti",
        unit: "kegiatan",
        kind: MetricKind::Flag(FlagSource::KerjaBakti),
    },
    MetricSpec {
        key: "peserta",
        label: "Peserta",
        unit: "orang",
        kind: MetricKind::Sum(SumSource::Peserta),
    },
];

const RELIGIOUS_METRICS: &[MetricSpec] = &[
    MetricSpec {
        key: "jemaah",
        label: "Jemaah",
        unit: "orang",
        kind: MetricKind::Sum(SumSource::Jamaah),
    },
    MetricSpec {
        key: "lokasi",
        label: "Lokasi kegiatan",
        unit: "lokasi",
        kind: MetricKind::Unique(UniqueSource::Location),
    },
    MetricSpec {
        key: "ibadah",
        label: "Masjid/tempat ibadah",
        unit: "lokasi",
        kind: MetricKind::Ibadah,
    },
    MetricSpec {
        key: "personel",
        label: "Personel",
        unit: "orang",
        kind: MetricKind::Sum(SumSource::Personel),
    },
];

const NARKOBA_METRICS: &[MetricSpec] = &[
    MetricSpec {
        key: "orang",
        label: "Orang diamankan",
        unit: "orang",
        kind: MetricKind::Sum(SumSource::NarkobaOrang),
    },
    MetricSpec {
        key: "kawasan",
        label: "Kawasan",
        unit: "kawasan",
        kind: MetricKind::Sum(SumSource::NarkobaKawasan),
    },
    MetricSpec {
        key: "rehabilitasi",
        label: "Rehabilitasi",
        unit: "orang",
        kind: MetricKind::Sum(SumSource::NarkobaRehab),
    },
    MetricSpec {
        key: "penyuluhan",
        label: "Peserta penyuluhan",
        unit: "peserta",
        kind: MetricKind::Sum(SumSource::NarkobaPenyuluhan),
    },
    MetricSpec {
        key: "personel",
        label: "Personel",
        unit: "orang",
        kind: MetricKind::Sum(SumSource::Personel),
    },
];

const MEDIA_METRICS: &[MetricSpec] = &[
    MetricSpec {
        key: "publikasi",
        label: "Publikasi",
        unit: "baris",
        kind: MetricKind::Flag(FlagSource::MediaPublication),
    },
    MetricSpec {
        key: "dokumentasi",
        label: "Dokumentasi",
        unit: "baris",
        kind: MetricKind::Flag(FlagSource::MediaDocumentation),
    },
    MetricSpec {
        key: "media",
        label: "Ada media",
        unit: "baris",
        kind: MetricKind::Flag(FlagSource::MediaAny),
    },
];

const PROGRAM_SPECS: &[ProgramSpec] = &[
    ProgramSpec {
        key: "green_policing",
        title: "Green Policing",
        code: "QW1.5",
        metrics: GREEN_METRICS,
    },
    ProgramSpec {
        key: "jumat_keliling",
        title: "Jumat Keliling",
        code: "QW1.6",
        metrics: RELIGIOUS_METRICS,
    },
    ProgramSpec {
        key: "subuh_keliling",
        title: "Subuh Keliling",
        code: "QW1.7",
        metrics: RELIGIOUS_METRICS,
    },
    ProgramSpec {
        key: "minggu_kasih",
        title: "Minggu Kasih",
        code: "QW1.8",
        metrics: RELIGIOUS_METRICS,
    },
    ProgramSpec {
        key: "narkoba",
        title: "Berantas Narkoba",
        code: "QW2",
        metrics: NARKOBA_METRICS,
    },
    ProgramSpec {
        key: "media",
        title: "Amplifikasi Media",
        code: "",
        metrics: MEDIA_METRICS,
    },
];

type UnitCountMap = BTreeMap<&'static str, BTreeMap<String, (String, i64)>>;

fn unit_count_map(keys: impl Iterator<Item = &'static str>) -> UnitCountMap {
    keys.map(|key| (key, BTreeMap::new())).collect()
}

fn bump_unit_count(map: &mut UnitCountMap, code: &'static str, key: &str, label: &str) {
    if let Some(counts) = map.get_mut(code) {
        let entry = counts
            .entry(key.to_string())
            .or_insert((label.to_string(), 0));
        entry.1 += 1;
    }
}

fn unit_counts(counts: Option<&BTreeMap<String, (String, i64)>>) -> Vec<CoverageUnitCount> {
    let mut items: Vec<CoverageUnitCount> = counts
        .map(|counts| {
            counts
                .values()
                .map(|(label, rows)| CoverageUnitCount {
                    label: label.clone(),
                    rows: *rows,
                })
                .collect()
        })
        .unwrap_or_default();
    items.sort_by(|left, right| {
        right
            .rows
            .cmp(&left.rows)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    items
}

pub fn build_coverage(start: &str, end: &str, rows: &[RecordView]) -> CoverageResponse {
    build_coverage_scoped(start, end, rows, None)
}

/// Cakupan yang bisa dipersempit ke satu polres. Saat difilter, dimensi
/// `polres` berisi polsek dan dimensi `satker` berisi satker polres.
pub fn build_coverage_scoped(
    start: &str,
    end: &str,
    rows: &[RecordView],
    polres: Option<&str>,
) -> CoverageResponse {
    let polres_options: Vec<String> = KnownUnits::from_rows(rows, UnitMode::Polda)
        .set(Scope::Polres)
        .values()
        .cloned()
        .collect();
    let filter = polres.map(str::trim).filter(|value| !value.is_empty());
    let scoped_rows: Vec<RecordView> = match filter {
        Some(label) => rows
            .iter()
            .filter(|row| row.polres.trim().eq_ignore_ascii_case(label))
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    let rows: &[RecordView] = if filter.is_some() { &scoped_rows } else { rows };
    let mode = if filter.is_some() {
        UnitMode::Polres
    } else {
        UnitMode::Polda
    };
    let known = KnownUnits::from_rows(rows, mode);

    let mut group_rows: BTreeMap<&'static str, i64> =
        qw::GROUPS.iter().map(|group| (group.code, 0)).collect();
    let mut group_polres: BTreeMap<&'static str, BTreeSet<String>> = qw::GROUPS
        .iter()
        .map(|group| (group.code, BTreeSet::new()))
        .collect();
    let mut group_satker: BTreeMap<&'static str, BTreeSet<String>> = qw::GROUPS
        .iter()
        .map(|group| (group.code, BTreeSet::new()))
        .collect();
    let mut code_rows: BTreeMap<&'static str, i64> =
        qw::CODES.iter().map(|code| (code.code, 0)).collect();
    let mut code_polres: BTreeMap<&'static str, BTreeSet<String>> = qw::CODES
        .iter()
        .map(|code| (code.code, BTreeSet::new()))
        .collect();
    let mut code_satker: BTreeMap<&'static str, BTreeSet<String>> = qw::CODES
        .iter()
        .map(|code| (code.code, BTreeSet::new()))
        .collect();
    let mut group_polres_counts = unit_count_map(qw::GROUPS.iter().map(|group| group.code));
    let mut group_satker_counts = unit_count_map(qw::GROUPS.iter().map(|group| group.code));
    let mut code_polres_counts = unit_count_map(qw::CODES.iter().map(|code| code.code));
    let mut code_satker_counts = unit_count_map(qw::CODES.iter().map(|code| code.code));
    let mut active_polres = BTreeSet::new();
    let mut active_satker = BTreeSet::new();

    for row in rows {
        let normalized = qw::normalize(&row.kode);
        let Some(group) = qw::group_of(&normalized) else {
            continue;
        };
        if let Some(count) = group_rows.get_mut(group.code) {
            *count += 1;
        }
        if let Some((scope, label)) = row_unit(row, mode) {
            let key = label.to_lowercase();
            match scope {
                Scope::Polres => {
                    active_polres.insert(key.clone());
                    if let Some(set) = group_polres.get_mut(group.code) {
                        set.insert(key.clone());
                    }
                    bump_unit_count(&mut group_polres_counts, group.code, &key, &label);
                }
                Scope::Satker => {
                    active_satker.insert(key.clone());
                    if let Some(set) = group_satker.get_mut(group.code) {
                        set.insert(key.clone());
                    }
                    bump_unit_count(&mut group_satker_counts, group.code, &key, &label);
                }
            }
        }

        let Some(item) = qw::find(&normalized) else {
            continue;
        };
        if let Some(count) = code_rows.get_mut(item.code) {
            *count += 1;
        }
        if let Some((scope, label)) = row_unit(row, mode) {
            let key = label.to_lowercase();
            match scope {
                Scope::Polres => {
                    if let Some(set) = code_polres.get_mut(item.code) {
                        set.insert(key.clone());
                    }
                    bump_unit_count(&mut code_polres_counts, item.code, &key, &label);
                }
                Scope::Satker => {
                    if let Some(set) = code_satker.get_mut(item.code) {
                        set.insert(key.clone());
                    }
                    bump_unit_count(&mut code_satker_counts, item.code, &key, &label);
                }
            }
        }
    }

    let groups = qw::GROUPS
        .iter()
        .map(|group| {
            let performed_polres = group_polres.get(group.code).cloned().unwrap_or_default();
            let performed_satker = group_satker.get(group.code).cloned().unwrap_or_default();
            let codes = qw::CODES
                .iter()
                .filter(|code| code.group == group.code)
                .map(|code| {
                    let performed_polres = code_polres.get(code.code).cloned().unwrap_or_default();
                    let performed_satker = code_satker.get(code.code).cloned().unwrap_or_default();
                    CoverageCode {
                        code: code.code.to_string(),
                        title: code.title.to_string(),
                        rows: *code_rows.get(code.code).unwrap_or(&0),
                        polres: performed_polres.len() as i64,
                        satker: performed_satker.len() as i64,
                        polres_missing: missing_units(
                            &known,
                            Scope::Polres,
                            mode,
                            code.code,
                            &performed_polres,
                        )
                        .len() as i64,
                        satker_missing: missing_units(
                            &known,
                            Scope::Satker,
                            mode,
                            code.code,
                            &performed_satker,
                        )
                        .len() as i64,
                        by_polres: unit_counts(code_polres_counts.get(code.code)),
                        by_satker: unit_counts(code_satker_counts.get(code.code)),
                    }
                })
                .collect();
            CoverageGroup {
                code: group.code.to_string(),
                title: group.title.to_string(),
                rows: *group_rows.get(group.code).unwrap_or(&0),
                polres: performed_polres.len() as i64,
                satker: performed_satker.len() as i64,
                polres_total: responsible_units(
                    &known,
                    Scope::Polres,
                    mode,
                    group.code,
                    &performed_polres,
                )
                .len() as i64,
                satker_total: responsible_units(
                    &known,
                    Scope::Satker,
                    mode,
                    group.code,
                    &performed_satker,
                )
                .len() as i64,
                polres_missing: missing_units(
                    &known,
                    Scope::Polres,
                    mode,
                    group.code,
                    &performed_polres,
                ),
                satker_missing: missing_units(
                    &known,
                    Scope::Satker,
                    mode,
                    group.code,
                    &performed_satker,
                ),
                by_polres: unit_counts(group_polres_counts.get(group.code)),
                by_satker: unit_counts(group_satker_counts.get(group.code)),
                codes,
            }
        })
        .collect();

    let programs = PROGRAM_SPECS
        .iter()
        .map(|spec| build_program(spec, rows, &known, mode))
        .collect();
    let codes = build_code_programs(rows, &known, mode);

    CoverageResponse {
        date_start: start.to_string(),
        date_end: end.to_string(),
        total_rows: rows.len() as i64,
        active_polres: active_polres.len() as i64,
        active_satker: active_satker.len() as i64,
        known_polres: known
            .set(Scope::Polres)
            .values()
            .cloned()
            .collect::<Vec<_>>(),
        known_satker: known
            .set(Scope::Satker)
            .values()
            .cloned()
            .collect::<Vec<_>>(),
        polres_options,
        filter_polres: filter.unwrap_or_default().to_string(),
        groups,
        programs,
        codes,
    }
}

fn build_program(
    spec: &ProgramSpec,
    rows: &[RecordView],
    known: &KnownUnits,
    mode: UnitMode,
) -> CoverageProgram {
    let program_rows: Vec<&RecordView> = rows
        .iter()
        .filter(|row| {
            if spec.code.is_empty() {
                return true;
            }
            qw::normalize(&row.kode).starts_with(spec.code)
        })
        .collect();

    let mut performed_polres = BTreeSet::new();
    let mut performed_satker = BTreeSet::new();
    let mut polres_acc: BTreeMap<String, UnitAcc> = BTreeMap::new();
    let mut satker_acc: BTreeMap<String, UnitAcc> = BTreeMap::new();
    let mut metric_acc: Vec<MetricAcc> =
        spec.metrics.iter().map(|_| MetricAcc::default()).collect();

    for row in &program_rows {
        let unit = row_unit(row, mode);
        if let Some((scope, label)) = &unit {
            let key = label.to_lowercase();
            let acc = match scope {
                Scope::Polres => {
                    performed_polres.insert(key.clone());
                    polres_acc.entry(key).or_insert_with(|| UnitAcc::new(label))
                }
                Scope::Satker => {
                    performed_satker.insert(key.clone());
                    satker_acc.entry(key).or_insert_with(|| UnitAcc::new(label))
                }
            };
            acc.rows += 1;
            for metric in spec.metrics {
                acc.add(metric, metric_sample(row, metric.kind));
            }
        }
        for (index, metric) in spec.metrics.iter().enumerate() {
            metric_acc[index].add(metric_sample(row, metric.kind), unit.as_ref());
        }
    }

    let metrics = spec
        .metrics
        .iter()
        .enumerate()
        .map(|(index, metric)| {
            let acc = &metric_acc[index];
            CoverageMetric {
                key: metric.key.to_string(),
                label: metric.label.to_string(),
                unit: metric.unit.to_string(),
                total: acc.total(metric.kind),
                filled_rows: acc.filled_rows,
                missing_rows: acc.rows - acc.filled_rows,
                filled_polres: acc.filled_polres.len() as i64,
                filled_satker: acc.filled_satker.len() as i64,
            }
        })
        .collect();

    CoverageProgram {
        key: spec.key.to_string(),
        title: spec.title.to_string(),
        code: spec.code.to_string(),
        rows: program_rows.len() as i64,
        polres: performed_polres.len() as i64,
        satker: performed_satker.len() as i64,
        polres_total: responsible_units(known, Scope::Polres, mode, spec.code, &performed_polres)
            .len() as i64,
        satker_total: responsible_units(known, Scope::Satker, mode, spec.code, &performed_satker)
            .len() as i64,
        polres_missing: missing_units(known, Scope::Polres, mode, spec.code, &performed_polres),
        satker_missing: missing_units(known, Scope::Satker, mode, spec.code, &performed_satker),
        metrics,
        by_polres: build_units(polres_acc, spec),
        by_satker: build_units(satker_acc, spec),
        breakdowns: Vec::new(),
        tables: Vec::new(),
    }
}

/// Mengelompokkan baris ke dimensi unit sesuai mode cakupan.
pub(crate) fn row_unit(row: &RecordView, mode: UnitMode) -> Option<(Scope, String)> {
    if mode == UnitMode::Polres {
        let polsek = row.polsek.trim();
        if !is_noise(polsek) {
            return Some((Scope::Polres, polsek.to_string()));
        }
        let label = record_satker_label(row);
        if !is_noise(&label) {
            return Some((Scope::Satker, label));
        }
        return None;
    }
    if record_is_polda(row) {
        let label = record_satker_label(row);
        if !is_noise(&label) {
            return Some((Scope::Satker, label));
        }
        return None;
    }
    if !is_noise(&row.polres) {
        return Some((Scope::Polres, row.polres.trim().to_string()));
    }
    None
}

#[derive(Default)]
struct MetricAcc {
    rows: i64,
    filled_rows: i64,
    sum: f64,
    texts: BTreeSet<String>,
    filled_polres: BTreeSet<String>,
    filled_satker: BTreeSet<String>,
}

impl MetricAcc {
    fn add(&mut self, sample: MetricSample, unit: Option<&(Scope, String)>) {
        self.rows += 1;
        let filled = match &sample {
            MetricSample::Number { filled, .. } => *filled,
            MetricSample::Text { value } => value.is_some(),
            MetricSample::Flag { value } => *value,
        };
        if !filled {
            return;
        }
        self.filled_rows += 1;
        match sample {
            MetricSample::Number { value, .. } => self.sum += value,
            MetricSample::Text { value: Some(text) } => {
                self.texts.insert(text.to_lowercase());
            }
            MetricSample::Flag { value: true } => self.sum += 1.0,
            _ => {}
        }
        if let Some((scope, label)) = unit {
            let key = label.to_lowercase();
            match scope {
                Scope::Polres => {
                    self.filled_polres.insert(key);
                }
                Scope::Satker => {
                    self.filled_satker.insert(key);
                }
            }
        }
    }

    fn total(&self, kind: MetricKind) -> f64 {
        match kind {
            MetricKind::Unique(_) => self.texts.len() as f64,
            MetricKind::Ibadah => self.sum + self.texts.len() as f64,
            _ => self.sum,
        }
    }
}

struct UnitAcc {
    label: String,
    rows: i64,
    sums: BTreeMap<&'static str, f64>,
    texts: BTreeMap<&'static str, BTreeSet<String>>,
    filled: BTreeMap<&'static str, bool>,
}

impl UnitAcc {
    fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            rows: 0,
            sums: BTreeMap::new(),
            texts: BTreeMap::new(),
            filled: BTreeMap::new(),
        }
    }

    fn add(&mut self, metric: &MetricSpec, sample: MetricSample) {
        match sample {
            MetricSample::Number { value, filled } => {
                if filled {
                    *self.sums.entry(metric.key).or_default() += value;
                    self.filled.insert(metric.key, true);
                }
            }
            MetricSample::Text { value: Some(text) } => {
                self.texts
                    .entry(metric.key)
                    .or_default()
                    .insert(text.to_lowercase());
                self.filled.insert(metric.key, true);
            }
            MetricSample::Text { value: None } => {}
            MetricSample::Flag { value: true } => {
                *self.sums.entry(metric.key).or_default() += 1.0;
                self.filled.insert(metric.key, true);
            }
            MetricSample::Flag { value: false } => {}
        }
    }
}

fn build_units(acc: BTreeMap<String, UnitAcc>, spec: &ProgramSpec) -> Vec<CoverageUnit> {
    let mut units: Vec<CoverageUnit> = acc
        .into_values()
        .map(|acc| {
            let values = spec
                .metrics
                .iter()
                .map(|metric| {
                    let filled = acc.filled.get(metric.key).copied().unwrap_or(false);
                    let value = match metric.kind {
                        MetricKind::Unique(_) => acc
                            .texts
                            .get(metric.key)
                            .map(|set| set.len() as f64)
                            .unwrap_or(0.0),
                        MetricKind::Ibadah => {
                            acc.sums.get(metric.key).copied().unwrap_or(0.0)
                                + acc
                                    .texts
                                    .get(metric.key)
                                    .map(|set| set.len() as f64)
                                    .unwrap_or(0.0)
                        }
                        _ => acc.sums.get(metric.key).copied().unwrap_or(0.0),
                    };
                    CoverageUnitValue {
                        key: metric.key.to_string(),
                        value,
                        filled,
                    }
                })
                .collect();
            let missing = spec
                .metrics
                .iter()
                .filter(|metric| !acc.filled.get(metric.key).copied().unwrap_or(false))
                .map(|metric| metric.key.to_string())
                .collect();
            CoverageUnit {
                label: acc.label,
                rows: acc.rows,
                values,
                missing,
            }
        })
        .collect();
    units.sort_by(|left, right| left.label.to_lowercase().cmp(&right.label.to_lowercase()));
    units
}

enum MetricSample {
    Number { value: f64, filled: bool },
    Text { value: Option<String> },
    Flag { value: bool },
}

fn metric_sample(row: &RecordView, kind: MetricKind) -> MetricSample {
    match kind {
        MetricKind::Sum(source) => {
            let (value, filled) = sum_sample(row, source);
            MetricSample::Number { value, filled }
        }
        MetricKind::Unique(source) => MetricSample::Text {
            value: unique_text(row, source),
        },
        MetricKind::Ibadah => ibadah_sample(row),
        MetricKind::Flag(source) => MetricSample::Flag {
            value: flag_sample(row, source),
        },
    }
}

fn sum_sample(row: &RecordView, source: SumSource) -> (f64, bool) {
    match source {
        SumSource::Personel => (
            row.personel_count as f64,
            personel_answered(row) || row.personel_count > 0,
        ),
        SumSource::Bibit => (
            row.bibit_count as f64,
            field_contains_answered(row, &["bibit"], &[]) || row.bibit_count > 0,
        ),
        SumSource::Area => (
            row.area_m2,
            field_contains_answered(row, &["luas"], &[])
                || area_field_answered(row)
                || row.area_m2 > 0.0,
        ),
        SumSource::Jamaah => (
            row.jamaah_count as f64,
            field_contains_answered(row, &["jemaah", "jemaat", "jamaah", "umat"], &[])
                || row.jamaah_count > 0,
        ),
        SumSource::NarkobaOrang => (
            number_for(row, NARKOBA_ORANG_KEYS),
            any_answered(row, NARKOBA_ORANG_KEYS),
        ),
        SumSource::NarkobaKawasan => (
            number_for(row, NARKOBA_KAWASAN_KEYS),
            any_answered(row, NARKOBA_KAWASAN_KEYS),
        ),
        SumSource::NarkobaRehab => (
            number_for(row, NARKOBA_REHAB_KEYS),
            any_answered(row, NARKOBA_REHAB_KEYS),
        ),
        SumSource::NarkobaPenyuluhan => (
            number_for(row, NARKOBA_PENYULUHAN_KEYS),
            any_answered(row, NARKOBA_PENYULUHAN_KEYS),
        ),
        SumSource::Peserta => (
            number_for_contains(row, "peserta"),
            field_contains_answered(row, &["peserta"], &[]),
        ),
    }
}

fn unique_text(row: &RecordView, source: UniqueSource) -> Option<String> {
    match source {
        UniqueSource::Location => location_text(row),
    }
}

fn ibadah_sample(row: &RecordView) -> MetricSample {
    for (key, value) in &row.fields {
        let key = key.to_ascii_lowercase();
        if (key.contains("jumlah masjid")
            || key.contains("jumlah tempat ibadah")
            || key.contains("jumlah lokasi ibadah"))
            && answered(value)
        {
            if let Some(count) = parse_count(value) {
                return MetricSample::Number {
                    value: count as f64,
                    filled: true,
                };
            }
        }
    }
    MetricSample::Text {
        value: ibadah_text(row),
    }
}

fn flag_sample(row: &RecordView, source: FlagSource) -> bool {
    match source {
        FlagSource::KerjaBakti => {
            row.kegiatan.to_ascii_lowercase().contains("kerja bakti")
                || row.fields.iter().any(|(key, value)| {
                    key.to_ascii_lowercase().contains("kegiatan")
                        && value.to_ascii_lowercase().contains("kerja bakti")
                })
        }
        FlagSource::MediaPublication => !publication_links(row).is_empty(),
        FlagSource::MediaDocumentation => !documentation_links(row).is_empty(),
        FlagSource::MediaAny => {
            !publication_links(row).is_empty() || !documentation_links(row).is_empty()
        }
    }
}

fn location_text(row: &RecordView) -> Option<String> {
    for key in LOCATION_KEYS {
        if let Some(value) = row.fields.get(*key) {
            let value = value.trim();
            if !is_noise(value) && value.chars().any(|ch| ch.is_alphabetic()) {
                return Some(value.to_string());
            }
        }
    }
    row.fields
        .iter()
        .find(|(key, value)| {
            let key = key.to_ascii_lowercase();
            (key.contains("lokasi") || key.contains("alamat"))
                && !is_noise(value)
                && value.chars().any(|ch| ch.is_alphabetic())
        })
        .map(|(_, value)| value.trim().to_string())
}

fn ibadah_text(row: &RecordView) -> Option<String> {
    for key in [
        "Nama masjid dan titik koordinat",
        "Nama tempat ibadah dan titik koordinat",
        "Nama masjid",
        "Nama tempat ibadah",
    ] {
        if let Some(value) = row.fields.get(key) {
            let value = value.trim();
            if !is_noise(value) && value.chars().any(|ch| ch.is_alphabetic()) {
                return Some(value.to_string());
            }
        }
    }
    row.fields
        .iter()
        .find(|(key, value)| {
            let key = key.to_ascii_lowercase();
            (key.contains("masjid")
                || key.contains("tempat ibadah")
                || key.contains("gereja")
                || key.contains("pura")
                || key.contains("vihara")
                || key.contains("klenteng"))
                && !is_noise(value)
                && value.chars().any(|ch| ch.is_alphabetic())
        })
        .map(|(_, value)| value.trim().to_string())
}

fn personel_answered(row: &RecordView) -> bool {
    field_contains_answered(
        row,
        &["personel", "personil", "jumlah anggota", "anggota hadir"],
        &["keluhan"],
    )
}

fn area_field_answered(row: &RecordView) -> bool {
    row.fields.iter().any(|(key, value)| {
        let key = key.to_ascii_lowercase();
        let value = value.to_ascii_lowercase();
        key.contains("area") && (value.contains("m2") || value.contains("m²")) && answered(&value)
    })
}

fn any_answered(row: &RecordView, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        row.fields
            .get(*key)
            .map(|value| answered(value))
            .unwrap_or(false)
    })
}

fn field_contains_answered(row: &RecordView, needles: &[&str], excludes: &[&str]) -> bool {
    row.fields.iter().any(|(key, value)| {
        let key = key.to_ascii_lowercase();
        needles.iter().any(|needle| key.contains(needle))
            && !excludes.iter().any(|exclude| key.contains(exclude))
            && answered(value)
    })
}

fn number_for(row: &RecordView, keys: &[&str]) -> f64 {
    for key in keys {
        if let Some(value) = row.fields.get(*key) {
            if let Some(count) = parse_count(value) {
                return count as f64;
            }
        }
    }
    0.0
}

fn number_for_contains(row: &RecordView, needle: &str) -> f64 {
    for (key, value) in &row.fields {
        if key.to_ascii_lowercase().contains(needle) {
            if let Some(count) = parse_count(value) {
                return count as f64;
            }
        }
    }
    0.0
}

fn answered(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !matches!(value.to_ascii_lowercase().as_str(), "-" | "—" | "null")
}

fn publication_links(row: &RecordView) -> Vec<String> {
    let mut links = Vec::new();
    for key in [
        "Link posting media sosial atau berita online",
        "Link posting media sosial",
        "Link berita online",
    ] {
        if let Some(value) = row.fields.get(key) {
            for link in extract_links(value) {
                if !is_documentation_link(&link) {
                    links.push(link);
                }
            }
        }
    }
    links
}

fn documentation_links(row: &RecordView) -> Vec<String> {
    let mut links = Vec::new();
    if let Some(value) = row.fields.get("Dokumentasi kegiatan") {
        links.extend(extract_links(value));
    }
    for link in &row.links {
        if is_documentation_link(link) {
            links.push(link.clone());
        }
    }
    links
}

fn is_documentation_link(url: &str) -> bool {
    let url = url.to_ascii_lowercase();
    url.starts_with("/uploads/")
        || url.contains("drive.google.")
        || url.contains("docs.google.")
        || url.contains("1drv.ms")
        || url.contains("dropbox.")
        || url.contains("mega.nz")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::imports::models::EvidenceView;

    fn row(
        kode: &str,
        level: &str,
        polres: &str,
        fields: &[(&str, &str)],
        personel: i64,
        bibit: i64,
        area: f64,
        jamaah: i64,
    ) -> RecordView {
        RecordView {
            id: 1,
            source_row: 1,
            import_id: "import-1".into(),
            import_filename: "contoh.xlsx".into(),
            tanggal: "2026-10-20".into(),
            waktu_kirim_wib: "08:00".into(),
            satuan: if level == "Polda" {
                "Polda Bali".into()
            } else {
                polres.into()
            },
            tingkat: level.into(),
            polda: "Polda Bali".into(),
            polres: polres.into(),
            polsek: String::new(),
            level: level.into(),
            unit: String::new(),
            kode: kode.into(),
            qw_title: qw::title(kode).into(),
            kegiatan: String::new(),
            quick_wins: qw::group_of(kode)
                .map(|group| group.code.to_string())
                .unwrap_or_default(),
            pejabat_pelaksana: if level == "Polda" {
                "Kabid Propam".into()
            } else {
                String::new()
            },
            jabatan: String::new(),
            pelaksana: String::new(),
            pelapor: String::new(),
            status_penerimaan: String::new(),
            bukti_dukung: String::new(),
            keluhan_personel_polri: String::new(),
            isian_laporan: String::new(),
            personel_count: personel,
            bibit_count: bibit,
            area_m2: area,
            jamaah_count: jamaah,
            latitude: None,
            longitude: None,
            map_url: String::new(),
            fields: fields
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            links: Vec::new(),
            evidence: Vec::<EvidenceView>::new(),
        }
    }

    #[test]
    fn builds_group_and_code_coverage() {
        let rows = vec![
            row(
                "QW1.5",
                "Polda",
                "",
                &[
                    ("Luas area yang ditanami", "100 m²"),
                    ("Jumlah bibit pohon yang ditanam", "10"),
                    ("Jumlah personel", "5"),
                    ("Kegiatan", "Kerja bakti"),
                ],
                5,
                10,
                100.0,
                0,
            ),
            row(
                "QW1.5",
                "Polres",
                "Polres Tabanan",
                &[("Jumlah personel", "3")],
                3,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.7",
                "Polres",
                "Polres Badung",
                &[
                    ("Jumlah jemaah", "50"),
                    ("Nama masjid dan titik koordinat", "Masjid Agung"),
                    ("Jumlah personel", "4"),
                ],
                4,
                0,
                0.0,
                50,
            ),
            row(
                "QW1.7",
                "Polda",
                "",
                &[("Nama masjid dan titik koordinat", "Masjid Polda")],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        assert_eq!(coverage.total_rows, 4);
        assert_eq!(coverage.known_polres.len(), 2);
        assert_eq!(coverage.known_satker.len(), 1);
        assert_eq!(coverage.active_polres, 2);
        assert_eq!(coverage.active_satker, 1);

        let qw1 = coverage
            .groups
            .iter()
            .find(|group| group.code == "QW1")
            .unwrap();
        assert_eq!(qw1.rows, 4);
        assert_eq!(qw1.polres, 2);
        assert_eq!(qw1.satker, 1);
        assert_eq!(qw1.by_polres.len(), 2);
        assert_eq!(qw1.by_polres[0].label, "Polres Badung");
        assert_eq!(qw1.by_polres[0].rows, 1);
        assert_eq!(qw1.by_satker.len(), 1);
        assert_eq!(qw1.by_satker[0].label, "Propam");
        assert_eq!(qw1.by_satker[0].rows, 2);
        let qw15 = qw1.codes.iter().find(|code| code.code == "QW1.5").unwrap();
        assert_eq!(qw15.rows, 2);
        assert_eq!(qw15.polres, 1);
        assert_eq!(qw15.satker, 1);
        assert_eq!(qw15.by_polres.len(), 1);
        assert_eq!(qw15.by_polres[0].label, "Polres Tabanan");
        assert_eq!(qw15.by_polres[0].rows, 1);
        let qw17 = qw1.codes.iter().find(|code| code.code == "QW1.7").unwrap();
        assert_eq!(qw17.rows, 2);
        assert_eq!(qw17.polres, 1);
        assert_eq!(qw17.satker, 1);
        let qw11 = qw1.codes.iter().find(|code| code.code == "QW1.1").unwrap();
        assert_eq!(qw11.rows, 0);
        assert_eq!(qw11.polres_missing, 2);
        // Propam bukan pelaksana pengungkit QW1.1, jadi tidak dihitung belum melaksanakan.
        assert_eq!(qw11.satker_missing, 0);
    }

    #[test]
    fn missing_lists_follow_assigned_duties() {
        let mut lantas = row("QW1.5", "Polda", "", &[], 1, 0, 0.0, 0);
        lantas.pejabat_pelaksana = "Kabid Lantas".into();
        let mut propam = row("QW1.5", "Polda", "", &[], 1, 0, 0.0, 0);
        propam.pejabat_pelaksana = "Kabid Propam".into();
        let rows = vec![lantas, propam];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        assert_eq!(coverage.known_satker.len(), 2);

        let qw1 = coverage
            .groups
            .iter()
            .find(|group| group.code == "QW1")
            .unwrap();
        let qw11 = qw1.codes.iter().find(|code| code.code == "QW1.1").unwrap();
        assert_eq!(qw11.rows, 0);
        assert_eq!(qw11.satker_missing, 1);

        let lantas_code = qw1.codes.iter().find(|code| code.code == "QW1.5").unwrap();
        assert_eq!(lantas_code.satker, 2, "Lantas dan Propam sama-sama melapor");

        let qw7 = coverage
            .groups
            .iter()
            .find(|group| group.code == "QW7")
            .unwrap();
        let qw71 = qw7.codes.iter().find(|code| code.code == "QW7.1").unwrap();
        // QW7.1 hanya dibebankan ke polres/polsek, bukan satker Polda.
        assert_eq!(qw71.satker_missing, 0);
    }

    #[test]
    fn pejabat_report_covers_its_satker_tier() {
        // QW1.2: pejabat pelaksana KAPOLDA, pengungkit BIRO SDM.
        let mut kapolda = row("QW1.2", "Polda", "", &[], 1, 0, 0.0, 0);
        kapolda.pejabat_pelaksana = String::new();
        kapolda.pelaksana = "Kapolda".into();
        let mut sdm = row("QW1.5", "Polda", "", &[], 1, 0, 0.0, 0);
        sdm.pejabat_pelaksana = String::new();
        sdm.pelaksana = "Biro SDM Polda".into();

        let coverage = build_coverage("2026-10-15", "2026-10-23", &[kapolda, sdm]);
        let qw1 = coverage
            .groups
            .iter()
            .find(|group| group.code == "QW1")
            .unwrap();
        let qw12 = qw1.codes.iter().find(|code| code.code == "QW1.2").unwrap();
        assert_eq!(qw12.rows, 1);
        assert_eq!(qw12.satker, 1);
        // Kapolda sudah melapor, jadi Biro SDM tidak dicatat belum melaksanakan.
        assert_eq!(qw12.satker_missing, 0);

        let mut sdm_only = row("QW1.5", "Polda", "", &[], 1, 0, 0.0, 0);
        sdm_only.pejabat_pelaksana = String::new();
        sdm_only.pelaksana = "Biro SDM Polda".into();
        let coverage = build_coverage("2026-10-15", "2026-10-23", &[sdm_only]);
        let qw1 = coverage
            .groups
            .iter()
            .find(|group| group.code == "QW1")
            .unwrap();
        let qw12 = qw1.codes.iter().find(|code| code.code == "QW1.2").unwrap();
        assert_eq!(qw12.rows, 0);
        // Tanpa laporan Kapolda, Biro SDM muncul sebagai belum melaksanakan.
        assert_eq!(qw12.satker_missing, 1);
    }

    #[test]
    fn reports_missing_program_metrics_per_unit() {
        let rows = vec![
            row(
                "QW1.5",
                "Polda",
                "",
                &[
                    ("Luas area yang ditanami", "100 m²"),
                    ("Jumlah bibit pohon yang ditanam", "10"),
                    ("Jumlah personel", "5"),
                    ("Kegiatan", "Kerja bakti"),
                ],
                5,
                10,
                100.0,
                0,
            ),
            row(
                "QW1.5",
                "Polres",
                "Polres Tabanan",
                &[("Jumlah personel", "3")],
                3,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.7",
                "Polres",
                "Polres Badung",
                &[
                    ("Jumlah jemaah", "50"),
                    ("Nama masjid dan titik koordinat", "Masjid Agung"),
                    ("Jumlah personel", "4"),
                ],
                4,
                0,
                0.0,
                50,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let green = coverage
            .programs
            .iter()
            .find(|program| program.key == "green_policing")
            .unwrap();
        assert_eq!(green.rows, 2);
        assert_eq!(green.polres, 1);
        assert_eq!(green.satker, 1);
        let area = green
            .metrics
            .iter()
            .find(|metric| metric.key == "area")
            .unwrap();
        assert_eq!(area.total, 100.0);
        assert_eq!(area.filled_rows, 1);
        assert_eq!(area.missing_rows, 1);
        assert_eq!(area.filled_polres, 0);
        assert_eq!(area.filled_satker, 1);
        let polres = green.by_polres.first().unwrap();
        assert!(polres.missing.contains(&"area".to_string()));
        assert!(polres.missing.contains(&"bibit".to_string()));
        assert!(!polres.missing.contains(&"personel".to_string()));

        let subuh = coverage
            .programs
            .iter()
            .find(|program| program.key == "subuh_keliling")
            .unwrap();
        assert_eq!(subuh.rows, 1);
        assert_eq!(subuh.polres, 1);
        let jemaah = subuh
            .metrics
            .iter()
            .find(|metric| metric.key == "jemaah")
            .unwrap();
        assert_eq!(jemaah.total, 50.0);
        assert_eq!(jemaah.filled_rows, 1);
        let ibadah = subuh
            .metrics
            .iter()
            .find(|metric| metric.key == "ibadah")
            .unwrap();
        assert_eq!(ibadah.total, 1.0);
        assert_eq!(ibadah.filled_rows, 1);
    }

    #[test]
    fn sums_numeric_ibadah_counts() {
        let rows = vec![row(
            "QW1.7",
            "Polres",
            "Polres X",
            &[("Jumlah masjid", "3"), ("Jumlah jemaah", "50")],
            0,
            0,
            0.0,
            50,
        )];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let subuh = coverage
            .programs
            .iter()
            .find(|program| program.key == "subuh_keliling")
            .unwrap();
        let ibadah = subuh
            .metrics
            .iter()
            .find(|metric| metric.key == "ibadah")
            .unwrap();
        assert_eq!(ibadah.total, 3.0);
        assert_eq!(ibadah.filled_rows, 1);
    }

    #[test]
    fn detects_narkoba_and_media_fields() {
        let mut sample = row(
            "QW2.1",
            "Polres",
            "Polres X",
            &[
                ("Jumlah orang yang ditangkap (Jumlah)", "2"),
                ("Jumlah kawasan narkoba (Jumlah)", "1"),
                ("Jumlah personel", "4"),
            ],
            4,
            0,
            0.0,
            0,
        );
        sample.fields.insert(
            "Link posting media sosial atau berita online".into(),
            "https://instagram.com/p/abc".into(),
        );
        sample.fields.insert(
            "Dokumentasi kegiatan".into(),
            "https://drive.google.com/file/d/abc/view".into(),
        );
        let coverage = build_coverage("2026-10-15", "2026-10-23", &[sample]);

        let narkoba = coverage
            .programs
            .iter()
            .find(|program| program.key == "narkoba")
            .unwrap();
        assert_eq!(narkoba.rows, 1);
        let orang = narkoba
            .metrics
            .iter()
            .find(|metric| metric.key == "orang")
            .unwrap();
        assert_eq!(orang.total, 2.0);
        assert_eq!(orang.filled_rows, 1);

        let media = coverage
            .programs
            .iter()
            .find(|program| program.key == "media")
            .unwrap();
        for key in ["publikasi", "dokumentasi", "media"] {
            let metric = media
                .metrics
                .iter()
                .find(|metric| metric.key == key)
                .unwrap();
            assert_eq!(metric.total, 1.0);
            assert_eq!(metric.filled_rows, 1);
        }
    }

    #[test]
    fn labels_personel_field() {
        let mut rows = Vec::new();
        for value in ["2", "3", "4"] {
            rows.push(row(
                "QW1.1",
                "Polres",
                "Polres Tabanan",
                &[("Cara bertindak (Jumlah personel)", value)],
                0,
                0,
                0.0,
                0,
            ));
        }

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.1")
            .unwrap();
        let personel = code
            .metrics
            .iter()
            .find(|metric| metric.key == "jumlah-personel-bertugas")
            .unwrap();
        assert_eq!(personel.label, "Jumlah personel yang bertugas");
        assert_eq!(personel.total, 9.0);
        assert_eq!(personel.unit, "orang");
    }

    #[test]
    fn reads_personnel_lines() {
        let mut rows = Vec::new();
        for (polri, bnn, warga) in [
            ("5 pers", "3 pers", "40 orang"),
            ("2 pers", "1 pers", "20 orang"),
            ("1 pers", "1 pers", "10 orang"),
        ] {
            rows.push(row(
                "QW2.2",
                "Polres",
                "Polres Tabanan",
                &[("Polri", polri), ("BNN", bnn), ("Warga", warga)],
                0,
                0,
                0.0,
                0,
            ));
        }

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW2.2")
            .unwrap();
        let card = code
            .breakdowns
            .iter()
            .find(|item| item.key == "personel-terlibat")
            .unwrap();
        assert_eq!(card.values[0].label, "Warga/masyarakat");
        assert_eq!(card.values[0].count, 70);
        assert!(
            card.values
                .iter()
                .any(|value| value.label == "Polri" && value.count == 8)
        );
        assert!(
            card.values
                .iter()
                .any(|value| value.label == "BNN" && value.count == 5)
        );
    }

    #[test]
    fn folds_polri_personnel_into_total() {
        let rows = vec![
            row(
                "QW1.2",
                "Polda",
                "",
                &[
                    ("Jumlah peserta", "1105"),
                    ("2. Jumlah Siswa", "1035 orang"),
                    (
                        "Cara bertindak",
                        "1. Polri : 10 Personel. 2. Guru dan Staf : 70 Orang. 3. Siswa : 1035 Orang.",
                    ),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.2",
                "Polres",
                "Polres Klungkung",
                &[
                    ("Jumlah peserta", "590"),
                    ("2. Jumlah Siswa", "590 orang"),
                    ("Jumlah Anggota Polri", "8 Orang"),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.2",
                "Polres",
                "Polres Bangli",
                &[
                    ("Jumlah peserta", "147"),
                    ("2. Jumlah Siswa", "147 orang"),
                    ("Cara bertindak", "Pembina upacara (irup)"),
                ],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.2")
            .unwrap();
        let personel = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah personel")
            .unwrap();
        assert_eq!(personel.total, 18.0);
        assert_eq!(personel.filled_rows, 2);
        assert_eq!(personel.unit, "orang");

        let peserta = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah peserta")
            .unwrap();
        assert_eq!(peserta.total, 1842.0);

        let siswa = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah Siswa")
            .unwrap();
        assert_eq!(siswa.total, 1772.0);
    }

    #[test]
    fn counts_polri_once_per_report() {
        let rows = vec![row(
            "QW1.2",
            "Polres",
            "Polres Klungkung",
            &[
                ("Jumlah Anggota Polri", "8 Orang"),
                (
                    "Jumlah peserta (Uraian)",
                    "- Polri sebanyak 8 Personil\n- Guru sebanyak 10 Orang\n- Siswa sebanyak 35 Siswa",
                ),
            ],
            0,
            0,
            0.0,
            0,
        )];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.2")
            .unwrap();
        let personel = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah personel")
            .unwrap();
        assert_eq!(personel.total, 8.0);
        assert_eq!(personel.filled_rows, 1);
    }

    #[test]
    fn ignores_numbered_list_items_as_personnel() {
        let rows = vec![
            row(
                "QW1.2",
                "Polres",
                "Polres Karangasem",
                &[("Cara bertindak", "4. Personel Polsek Bebandem.")],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.2",
                "Polres",
                "Polres Karangasem",
                &[(
                    "Cara bertindak",
                    "2. Kegiatan tersebut melibatkan Polri di sekolah.",
                )],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.2",
                "Polres",
                "Polres Karangasem",
                &[("Cara bertindak", "Pembina upacara")],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.2")
            .unwrap();
        assert!(
            code.metrics
                .iter()
                .all(|metric| metric.label != "Jumlah personel")
        );
    }

    #[test]
    fn builds_qw14_cards() {
        let community = "Komunitas atau paguyuban yang ditemui";
        let feedback = "Saran masukan dan problem, masalah, keluhan, kegelisahan, atau komplain dari masyarakat";
        let rows = vec![
            row(
                "QW1.4",
                "Polda",
                "",
                &[
                    (community, "Komunitas Ojek Online Kota Denpasar"),
                    (
                        feedback,
                        "1) APRESIASI: 0 saran atau keluhan, 0 ditindaklanjuti. Terima kasih kepada Polri",
                    ),
                    (
                        "Cara bertindak",
                        "Jumlah personel yang hadir:\n1.Polri = 9 personel;\n2.Dinas Terkait=0\n3.Warga ( anggota organisasi )= 30 orang\n4.Komunitas= 0\n5.Tokoh Masyarakat = 4 orang",
                    ),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.4",
                "Polres",
                "Polres Tabanan",
                &[
                    (community, "KELOMPOK NELAYAN MERTA SEGARA,DESA CANGGU"),
                    (
                        feedback,
                        "1) Harkamtibmas: 1 saran atau keluhan, 1 ditindaklanjuti. Warga minta patroli",
                    ),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.4",
                "Polres",
                "Polres Badung",
                &[
                    (community, "warga masyarakat"),
                    (
                        feedback,
                        "1) Harkamtibmas: 0 saran atau keluhan, 0 ditindaklanjuti. Aman",
                    ),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.4",
                "Polsek",
                "Polres Bangli",
                &[(community, "NIHIL")],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.4")
            .unwrap();

        let personel = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah personel")
            .unwrap();
        assert_eq!(personel.total, 9.0);

        let masyarakat = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah masyarakat")
            .unwrap();
        assert_eq!(masyarakat.total, 34.0);

        let apresiasi = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Apresiasi")
            .unwrap();
        assert_eq!(apresiasi.total, 1.0);
        let keluhan = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Keluhan")
            .unwrap();
        assert_eq!(keluhan.total, 1.0);
        let tindak = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Tindak lanjut")
            .unwrap();
        assert_eq!(tindak.total, 1.0);

        let giat = code
            .breakdowns
            .iter()
            .find(|item| item.key == "giat-polda-polres")
            .unwrap();
        assert!(
            giat.values
                .iter()
                .any(|value| value.label == "Polda" && value.count == 1)
        );
        assert!(
            giat.values
                .iter()
                .any(|value| value.label == "Polres jajaran" && value.count == 3)
        );

        let communities = code
            .breakdowns
            .iter()
            .find(|item| item.key == "masyarakat-terlibat")
            .unwrap();
        for (label, count) in [
            ("Warga/masyarakat umum", 2),
            ("Nelayan", 1),
            ("Ojek online/driver", 1),
            ("Komunitas/paguyuban", 1),
        ] {
            assert!(
                communities
                    .values
                    .iter()
                    .any(|value| value.label == label && value.count == count),
                "kategori {label} tidak ditemukan"
            );
        }
    }

    #[test]
    fn builds_qw15_area_and_bibit_cards() {
        let rows = vec![
            row(
                "QW1.5",
                "Polres",
                "Polres Tabanan",
                &[
                    (
                        "Pertimbangan pilihan lokasi (Luas area yang ditangani (m²))",
                        "500",
                    ),
                    (
                        "Jumlah bibit pohon yang ditanam",
                        "50 Pohon, terdiri dari: a. Peneduh 50",
                    ),
                    ("Nama lokasi dan titik koordinat", "Hutan Mangrove Tabanan"),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.5",
                "Polres",
                "Polres Badung",
                &[
                    ("Luas area yang ditanami", "1.500 m²"),
                    ("Jumlah pohon yang ditanam (Jumlah)", "25"),
                    ("Nama lokasi dan titik koordinat", "Pantai Kuta"),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.5",
                "Polsek",
                "Polres Bangli",
                &[
                    ("Luas area yang ditangani", "0"),
                    ("Jumlah pohon yang ditanam (Rincian)", "Pohon Kopi 5 batang"),
                    ("Nama lokasi dan titik koordinat", "Taman Lestari"),
                ],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.5")
            .unwrap();

        let area = code
            .metrics
            .iter()
            .find(|metric| metric.key == "luas-area")
            .unwrap();
        assert_eq!(area.label, "Luas area kegiatan");
        assert_eq!(area.total, 2000.0);
        assert_eq!(area.unit, "m²");
        assert_eq!(area.filled_rows, 3);

        let bibit = code
            .metrics
            .iter()
            .find(|metric| metric.key == "jumlah-bibit")
            .unwrap();
        assert_eq!(bibit.label, "Jumlah bibit");
        assert_eq!(bibit.total, 75.0);
        assert_eq!(bibit.filled_rows, 2);

        let kinds = code
            .breakdowns
            .iter()
            .find(|item| item.key == "jenis-area")
            .unwrap();
        for (label, count) in [
            ("Hutan mangrove", 1),
            ("Pantai/pesisir", 1),
            ("Ruang publik", 1),
        ] {
            assert!(
                kinds
                    .values
                    .iter()
                    .any(|value| value.label == label && value.count == count),
                "jenis area {label} tidak ditemukan"
            );
        }

        assert!(
            !code
                .metrics
                .iter()
                .any(|metric| metric.key.starts_with("field-luas"))
        );
    }

    #[test]
    fn builds_qw16_jamaah_and_personel_cards() {
        let rows = vec![
            row(
                "QW1.6",
                "Polda",
                "",
                &[
                    (
                        "Nama masjid dan titik koordinat",
                        "Masjid At Taufiq Denpasar (-8.63, 115.25)",
                    ),
                    (
                        "Peran",
                        "Adapun Jumlah personel yang hadir: Polri = 9 Dinas Terkait=0 Warga/Jemaah= 450 Komunitas= 0 Tokoh Agama = 4",
                    ),
                    (
                        "Pertimbangan pilihan lokasi",
                        "Permintaan pengurus masjid, dan jamaah sekitar 450 orang",
                    ),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.6",
                "Polres",
                "Polres Tabanan",
                &[
                    (
                        "Nama masjid dan titik koordinat",
                        "Masjid Al-Amin Tabanan (-8.53, 115.12)",
                    ),
                    (
                        "Pertimbangan pilihan lokasi (Perkiraan jumlah jemaah)",
                        "150",
                    ),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.6",
                "Polsek",
                "Polres Bangli",
                &[
                    (
                        "Nama masjid dan titik koordinat",
                        "Masjid Baiturrahman Bangli (-8.47, 115.35)",
                    ),
                    ("Peran", "Lainnya: MENGHADIRI UNDANGAN"),
                ],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.6")
            .unwrap();

        let personel = code
            .metrics
            .iter()
            .find(|metric| metric.key == "jumlah-personel")
            .unwrap();
        assert_eq!(personel.total, 9.0);
        assert_eq!(personel.filled_rows, 1);

        let jamaah = code
            .metrics
            .iter()
            .find(|metric| metric.key == "jumlah-jamaah")
            .unwrap();
        assert_eq!(jamaah.label, "Jumlah jamaah");
        assert_eq!(jamaah.total, 600.0);
        assert_eq!(jamaah.filled_rows, 2);
        assert_eq!(jamaah.unit, "orang");

        let masjid = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah masjid")
            .unwrap();
        assert_eq!(masjid.total, 3.0);

        assert!(
            !code
                .metrics
                .iter()
                .any(|metric| metric.label.contains("Perkiraan jumlah jemaah"))
        );
    }

    #[test]
    fn builds_qw18_jemaat_and_ibadah_cards() {
        let place = "Nama tempat ibadah dan titik koordinat";
        let rows = vec![
            row(
                "QW1.8",
                "Polda",
                "",
                &[
                    (
                        place,
                        "Gereja Katolik Paroki Yesus Gembala Yang Baik, Denpasar (-8.62962, 115.19049)",
                    ),
                    (
                        "Peran",
                        "Adapun jumlah yang hadir: 1. Polri : 10 2. Dinas terkait : 0 3. Tokoh : 4 4. Jemaat : 450",
                    ),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.8",
                "Polres",
                "Polres Badung",
                &[
                    (place, "Pura Jagat Tirtha, Tuban (-8.74, 115.17)"),
                    (
                        "* *jumlah masyarakat yang ibadah di Pura Jagat Tirtha",
                        "± 200 orang*",
                    ),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.8",
                "Polsek",
                "Polres Denpasar",
                &[
                    (place, "Vihara Dharma Bhakti (-8.65, 115.21)"),
                    ("Peran", "Melakukan tatap muka"),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.8",
                "Polsek",
                "Polres Denpasar",
                &[
                    (place, "Klenteng Caow Eng Bio (-8.72, 115.17)"),
                    ("Peran", "Melakukan tatap muka"),
                ],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.8")
            .unwrap();

        let personel = code
            .metrics
            .iter()
            .find(|metric| metric.key == "jumlah-personel")
            .unwrap();
        assert_eq!(personel.total, 10.0);

        let jemaat = code
            .metrics
            .iter()
            .find(|metric| metric.key == "jumlah-jemaat")
            .unwrap();
        assert_eq!(jemaat.label, "Jumlah jemaat");
        assert_eq!(jemaat.total, 650.0);
        assert_eq!(jemaat.filled_rows, 2);
        assert_eq!(jemaat.unit, "orang");

        let lokasi = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah tempat ibadah")
            .unwrap();
        assert_eq!(lokasi.total, 4.0);

        let kinds = code
            .breakdowns
            .iter()
            .find(|item| item.key == "jenis-ibadah")
            .unwrap();
        for label in ["Gereja", "Pura", "Wihara", "Klenteng"] {
            assert!(
                kinds
                    .values
                    .iter()
                    .any(|value| value.label == label && value.count == 1),
                "jenis ibadah {label} tidak ditemukan"
            );
        }
    }

    #[test]
    fn excludes_masjid_from_qw18_cards() {
        let place = "Nama tempat ibadah dan titik koordinat";
        let rows = vec![
            row(
                "QW1.8",
                "Polres",
                "Polres Gianyar",
                &[(place, "Gereja A (-8.1, 115.1)"), ("Peran", "Jemaat : 100")],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.8",
                "Polres",
                "Polres Tabanan",
                &[(place, "Gereja B (-8.2, 115.2)"), ("Peran", "Jemaat : 100")],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.8",
                "Polsek",
                "Polres Buleleng",
                &[
                    (place, "Mesjid Nurul Mubin (-8.3, 115.3)"),
                    ("Peran", "Polri : 50 Jemaat : 300"),
                ],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.8")
            .unwrap();

        assert!(
            !code
                .metrics
                .iter()
                .any(|metric| metric.key == "jumlah-personel")
        );

        let jemaat = code
            .metrics
            .iter()
            .find(|metric| metric.key == "jumlah-jemaat")
            .unwrap();
        assert_eq!(jemaat.total, 200.0);
        assert_eq!(jemaat.filled_rows, 2);

        let lokasi = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah tempat ibadah")
            .unwrap();
        assert_eq!(lokasi.total, 2.0);

        let kinds = code
            .breakdowns
            .iter()
            .find(|item| item.key == "jenis-ibadah")
            .unwrap();
        assert!(
            kinds
                .values
                .iter()
                .any(|value| value.label == "Gereja" && value.count == 2)
        );
        assert!(!kinds.values.iter().any(|value| value.label == "Masjid"));
    }

    #[test]
    fn builds_gatur_locations_per_polres() {
        let field = "Jumlah lokasi gatur pagi";
        let personel = "Cara bertindak (Jumlah personel)";
        let rows = vec![
            row(
                "QW1.1",
                "Polres",
                "Polres Gianyar",
                &[(field, "5"), (personel, "4")],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.1",
                "Polsek",
                "Polres Gianyar",
                &[(field, "3"), (personel, "2")],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.1",
                "Polres",
                "Polres Bangli",
                &[(field, "2"), (personel, "3")],
                0,
                0,
                0.0,
                0,
            ),
            row("QW1.1", "Polda", "", &[(field, "1")], 0, 0, 0.0, 0),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.1")
            .unwrap();
        let breakdown = code
            .breakdowns
            .iter()
            .find(|item| item.key == "lokasi-gatur-per-polres")
            .unwrap();
        assert_eq!(breakdown.label, "Jumlah lokasi titik gatur per polres");
        assert_eq!(breakdown.filled_rows, 4);
        assert!(
            breakdown
                .values
                .iter()
                .any(|value| value.label == "Polres Gianyar" && value.count == 8)
        );
        assert!(
            breakdown
                .values
                .iter()
                .any(|value| value.label == "Polres Bangli" && value.count == 2)
        );
        assert!(
            breakdown
                .values
                .iter()
                .any(|value| value.label == "Polda Bali" && value.count == 1)
        );

        let personel_card = code
            .metrics
            .iter()
            .find(|item| item.key == "jumlah-personel-bertugas")
            .unwrap();
        assert_eq!(personel_card.label, "Jumlah personel yang bertugas");
        assert_eq!(personel_card.unit, "orang");
        assert_eq!(personel_card.total, 9.0);
        assert_eq!(personel_card.filled_rows, 3);

        let personel_per_polres = code
            .breakdowns
            .iter()
            .find(|item| item.key == "personel-bertugas-per-polres")
            .unwrap();
        assert_eq!(
            personel_per_polres.label,
            "Jumlah personel bertugas per polres"
        );
        assert_eq!(personel_per_polres.filled_rows, 3);
        assert!(
            personel_per_polres
                .values
                .iter()
                .any(|value| value.label == "Polres Gianyar" && value.count == 6)
        );
        assert!(
            personel_per_polres
                .values
                .iter()
                .any(|value| value.label == "Polres Bangli" && value.count == 3)
        );
    }

    #[test]
    fn exposes_gatur_personnel_card_without_data() {
        let rows = vec![row(
            "QW1.1",
            "Polres",
            "Polres Gianyar",
            &[("Jumlah lokasi gatur pagi", "5")],
            0,
            0,
            0.0,
            0,
        )];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.1")
            .unwrap();
        let personel = code
            .metrics
            .iter()
            .find(|item| item.key == "jumlah-personel-bertugas")
            .unwrap();
        assert_eq!(personel.total, 0.0);
        assert_eq!(personel.filled_rows, 0);
        assert_eq!(personel.missing_rows, 1);
        assert!(
            !code
                .breakdowns
                .iter()
                .any(|item| item.key == "personel-bertugas-per-polres")
        );
    }

    #[test]
    fn builds_kerawanan_table_per_polres() {
        let field = "Pertimbangan pilihan lokasi";
        let rows = vec![
            row(
                "QW1.1",
                "Polres",
                "Polres Gianyar",
                &[(field, "Rawan kemacetan")],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.1",
                "Polsek",
                "Polres Gianyar",
                &[(field, "Rawan keramaian")],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.1",
                "Polres",
                "Polres Bangli",
                &[(field, "Lainnya: RAWAN KEMACETAN DAN PELANGGARAN")],
                0,
                0,
                0.0,
                0,
            ),
            row("QW1.1", "Polda", "", &[(field, "Rawan laka")], 0, 0, 0.0, 0),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.1")
            .unwrap();
        let table = code
            .tables
            .iter()
            .find(|item| item.key == "kerawanan-per-polres")
            .unwrap();
        assert_eq!(table.label, "Rekap jenis kerawanan per polres");
        assert_eq!(table.filled_rows, 4);
        assert_eq!(
            table.columns,
            vec![
                "Polres",
                "Rawan kemacetan",
                "Rawan keramaian",
                "Rawan pelanggaran",
                "Rawan kecelakaan",
            ]
        );
        let bangli = table
            .rows
            .iter()
            .find(|row| row.label == "Polres Bangli")
            .unwrap();
        assert_eq!(bangli.values, vec![1, 0, 1, 0]);
        let gianyar = table
            .rows
            .iter()
            .find(|row| row.label == "Polres Gianyar")
            .unwrap();
        assert_eq!(gianyar.values, vec![1, 1, 0, 0]);
        let polda = table
            .rows
            .iter()
            .find(|row| row.label == "Polda Bali")
            .unwrap();
        assert_eq!(polda.values, vec![0, 0, 0, 1]);
    }

    #[test]
    fn counts_evidence_from_labeled_values() {
        let mut rows = Vec::new();
        for (value, aset) in [
            ("1) Golongan 1: 2, dua paket sabu", "1"),
            ("1) Golongan 1: 3, ganja kering", "0"),
            ("1) Nihil: 0, nihil", "4"),
        ] {
            rows.push(row(
                "QW2.1",
                "Polda",
                "",
                &[
                    ("Barang bukti yang diamankan", value),
                    ("Aset yang disita (Jumlah)", aset),
                ],
                0,
                0,
                0.0,
                0,
            ));
        }

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW2.1")
            .unwrap();
        let bukti = code
            .metrics
            .iter()
            .find(|metric| metric.key == "barang-bukti")
            .unwrap();
        assert_eq!(bukti.total, 5.0);
        assert_eq!(bukti.filled_rows, 3);

        let jenis = code
            .breakdowns
            .iter()
            .find(|item| item.key == "barang-bukti-jenis")
            .unwrap();
        assert_eq!(jenis.values.len(), 2);
        assert_eq!(jenis.values[0].label, "Ganja");
        assert_eq!(jenis.values[0].count, 3);
        assert_eq!(jenis.values[1].label, "Sabu");
        assert_eq!(jenis.values[1].count, 2);

        let aset = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Aset yang disita (Jumlah)")
            .unwrap();
        assert_eq!(aset.total, 5.0);
    }

    #[test]
    fn builds_cards_per_sub_code() {
        let rows = vec![
            row(
                "QW1.2",
                "Polres",
                "Polres Tabanan",
                &[
                    ("Jumlah peserta", "100"),
                    ("Cara bertindak", "Ceramah"),
                    ("Nama sekolah dan titik koordinat", "SMAN 1 Tabanan"),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.2",
                "Polres",
                "Polres Tabanan",
                &[
                    ("Jumlah peserta", "50"),
                    ("Cara bertindak", "Ceramah"),
                    ("Nama sekolah dan titik koordinat", "SMKN 1 Tabanan"),
                ],
                0,
                0,
                0.0,
                0,
            ),
            row(
                "QW1.2",
                "Polda",
                "",
                &[
                    ("Jumlah peserta", "25"),
                    ("Cara bertindak", "Penyuluhan"),
                    ("Nama sekolah dan titik koordinat", "SDN 1 Denpasar"),
                ],
                0,
                0,
                0.0,
                0,
            ),
        ];

        let coverage = build_coverage("2026-10-15", "2026-10-23", &rows);
        let code = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW1.2")
            .unwrap();
        assert_eq!(code.rows, 3);
        let peserta = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah peserta")
            .unwrap();
        assert_eq!(peserta.total, 175.0);
        let sekolah = code
            .metrics
            .iter()
            .find(|metric| metric.label == "Jumlah sekolah")
            .unwrap();
        assert_eq!(sekolah.total, 3.0);

        let tindakan = code
            .breakdowns
            .iter()
            .find(|item| item.label == "Cara bertindak")
            .unwrap();
        assert_eq!(tindakan.values[0].label, "Ceramah");
        assert_eq!(tindakan.values[0].count, 2);

        let jenjang = code
            .breakdowns
            .iter()
            .find(|item| item.key == "jenjang")
            .unwrap();
        assert_eq!(jenjang.values.len(), 3);

        let empty = coverage
            .codes
            .iter()
            .find(|item| item.code == "QW8.4")
            .unwrap();
        assert_eq!(empty.rows, 0);
        assert!(empty.metrics.is_empty());
    }

    #[test]
    fn scopes_coverage_to_one_polres() {
        let mut polsek_row = row("QW1.5", "Polsek", "Polres Buleleng", &[], 3, 0, 0.0, 0);
        polsek_row.polsek = "Polsek Kuta".into();
        polsek_row.unit = "Polsek Kuta".into();

        let mut polres_row = row("QW1.5", "Polres", "Polres Buleleng", &[], 4, 0, 0.0, 0);
        polres_row.pelaksana = "Lantas Polres".into();
        polres_row.unit = "Polres Buleleng".into();

        let other_row = row("QW1.5", "Polres", "Polres Bangli", &[], 1, 0, 0.0, 0);
        let rows = vec![polsek_row, polres_row, other_row];

        let plain = build_coverage("2026-10-15", "2026-10-23", &rows);
        assert_eq!(plain.filter_polres, "");
        assert_eq!(plain.total_rows, 3);
        assert_eq!(plain.polres_options.len(), 2);
        assert_eq!(plain.known_polres.len(), 2);

        let scoped =
            build_coverage_scoped("2026-10-15", "2026-10-23", &rows, Some("polres buleleng"));
        assert_eq!(scoped.filter_polres, "polres buleleng");
        assert_eq!(scoped.total_rows, 2);
        assert_eq!(scoped.polres_options.len(), 2);
        assert_eq!(scoped.known_polres, vec!["Polsek Kuta".to_string()]);
        assert_eq!(scoped.known_satker, vec!["Lantas Polres".to_string()]);
        assert_eq!(scoped.active_polres, 1);
        assert_eq!(scoped.active_satker, 1);

        let green = scoped
            .programs
            .iter()
            .find(|program| program.key == "green_policing")
            .unwrap();
        assert_eq!(green.rows, 2);
        assert_eq!(green.polres, 1);
        assert_eq!(green.satker, 1);
        assert_eq!(green.by_polres.len(), 1);
        assert_eq!(green.by_polres[0].label, "Polsek Kuta");
        assert_eq!(green.by_polres[0].rows, 1);
        assert_eq!(green.by_satker.len(), 1);
        assert_eq!(green.by_satker[0].label, "Lantas Polres");
        assert_eq!(green.by_satker[0].rows, 1);

        let narkoba = scoped
            .programs
            .iter()
            .find(|program| program.key != "green_policing")
            .unwrap();
        assert_eq!(narkoba.rows, 0);
    }
}
