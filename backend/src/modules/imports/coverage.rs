use std::collections::{BTreeMap, BTreeSet};

use crate::{
    modules::imports::{
        models::{
            CoverageBreakdown, CoverageCode, CoverageGroup, CoverageMetric, CoverageProgram,
            CoverageResponse, CoverageUnit, CoverageUnitCount, CoverageUnitValue, RecordView,
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
fn build_code_cards(matching: &[&RecordView]) -> (Vec<CoverageMetric>, Vec<CoverageBreakdown>) {
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
                .map(|value| value.to_ascii_lowercase())
                .collect();
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
        let numbers: Vec<i64> = values
            .iter()
            .filter_map(|value| parse_count(value))
            .collect();
        if numbers.len() * 10 >= values.len() * 7 && metrics.len() < 8 {
            let sum: f64 = numbers.iter().map(|value| *value as f64).sum();
            let label = if lower.contains("personel") || lower.contains("personil") {
                "Jumlah personel".to_string()
            } else {
                key.clone()
            };
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
            label: key.clone(),
            filled_rows: values.len() as i64,
            values: items,
        });
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
            let (metrics, breakdowns) = build_code_cards(&matching);
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
            .find(|metric| metric.label == "Jumlah personel")
            .unwrap();
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
