use std::collections::{BTreeMap, BTreeSet};

use crate::{
    modules::imports::{
        models::{
            CoverageCode, CoverageGroup, CoverageMetric, CoverageProgram, CoverageResponse,
            CoverageUnit, CoverageUnitValue, RecordView,
        },
        qw,
    },
    shared::{extract_links, is_noise, parse_count},
};

use super::service::{record_is_polda, record_satker_label};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Satker,
    Polres,
}

struct KnownUnits {
    polres: BTreeMap<String, String>,
    satker: BTreeMap<String, String>,
}

impl KnownUnits {
    fn from_rows(rows: &[RecordView]) -> Self {
        let mut polres = BTreeMap::new();
        let mut satker = BTreeMap::new();
        for row in rows {
            let Some((scope, label)) = row_unit(row) else {
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

    fn missing(&self, scope: Scope, performed: &BTreeSet<String>) -> Vec<String> {
        self.set(scope)
            .iter()
            .filter(|(key, _)| !performed.contains(*key))
            .map(|(_, label)| label.clone())
            .collect()
    }

    fn total(&self, scope: Scope) -> i64 {
        self.set(scope).len() as i64
    }
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

pub fn build_coverage(start: &str, end: &str, rows: &[RecordView]) -> CoverageResponse {
    let known = KnownUnits::from_rows(rows);

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
        if let Some((scope, label)) = row_unit(row) {
            let key = label.to_lowercase();
            match scope {
                Scope::Polres => {
                    active_polres.insert(key.clone());
                    if let Some(set) = group_polres.get_mut(group.code) {
                        set.insert(key);
                    }
                }
                Scope::Satker => {
                    active_satker.insert(key.clone());
                    if let Some(set) = group_satker.get_mut(group.code) {
                        set.insert(key);
                    }
                }
            }
        }

        let Some(item) = qw::find(&normalized) else {
            continue;
        };
        if let Some(count) = code_rows.get_mut(item.code) {
            *count += 1;
        }
        if let Some((scope, label)) = row_unit(row) {
            let key = label.to_lowercase();
            match scope {
                Scope::Polres => {
                    if let Some(set) = code_polres.get_mut(item.code) {
                        set.insert(key);
                    }
                }
                Scope::Satker => {
                    if let Some(set) = code_satker.get_mut(item.code) {
                        set.insert(key);
                    }
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
                        polres_missing: known.total(Scope::Polres) - performed_polres.len() as i64,
                        satker_missing: known.total(Scope::Satker) - performed_satker.len() as i64,
                    }
                })
                .collect();
            CoverageGroup {
                code: group.code.to_string(),
                title: group.title.to_string(),
                rows: *group_rows.get(group.code).unwrap_or(&0),
                polres: performed_polres.len() as i64,
                satker: performed_satker.len() as i64,
                polres_missing: known.missing(Scope::Polres, &performed_polres),
                satker_missing: known.missing(Scope::Satker, &performed_satker),
                codes,
            }
        })
        .collect();

    let programs = PROGRAM_SPECS
        .iter()
        .map(|spec| build_program(spec, rows, &known))
        .collect();

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
        groups,
        programs,
    }
}

fn build_program(spec: &ProgramSpec, rows: &[RecordView], known: &KnownUnits) -> CoverageProgram {
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
        let unit = row_unit(row);
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
        polres_total: known.total(Scope::Polres),
        satker_total: known.total(Scope::Satker),
        polres_missing: known.missing(Scope::Polres, &performed_polres),
        satker_missing: known.missing(Scope::Satker, &performed_satker),
        metrics,
        by_polres: build_units(polres_acc, spec),
        by_satker: build_units(satker_acc, spec),
    }
}

fn row_unit(row: &RecordView) -> Option<(Scope, String)> {
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
        self.rows += 1;
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
        let qw15 = qw1.codes.iter().find(|code| code.code == "QW1.5").unwrap();
        assert_eq!(qw15.rows, 2);
        assert_eq!(qw15.polres, 1);
        assert_eq!(qw15.satker, 1);
        let qw17 = qw1.codes.iter().find(|code| code.code == "QW1.7").unwrap();
        assert_eq!(qw17.rows, 2);
        assert_eq!(qw17.polres, 1);
        assert_eq!(qw17.satker, 1);
        let qw11 = qw1.codes.iter().find(|code| code.code == "QW1.1").unwrap();
        assert_eq!(qw11.rows, 0);
        assert_eq!(qw11.polres_missing, 2);
        assert_eq!(qw11.satker_missing, 1);
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
}
