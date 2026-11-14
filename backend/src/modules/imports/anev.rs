//! Analisis dan evaluasi khusus pemeriksaan kualitas laporan:
//! - duplikasi isian di dalam satuan (persis dan mirip ≥75%),
//! - tautan Google Drive yang dipakai berulang, terutama pada foto kegiatan.
//!
//! Baris unik diambil dari `import_rows`; baris yang tersaring deduplikasi
//! operasional diambil dari `import_duplicate_rows` agar analisis tetap melihat
//! data mentah tanpa mengubah hitungan laporan utama.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use reqwest::Url;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    modules::{
        auth::DataScope,
        imports::{
            models::{DuplicateRow, RecordView},
            service,
        },
        periods,
    },
    shared::extract_links,
};

const MAX_GROUPS: usize = 100;
const MAX_ROWS_PER_GROUP: usize = 30;
const SAMPLE_CHARS: usize = 220;
const ROW_TEXT_CHARS: usize = 700;
const MIN_FUZZY_CHARS: usize = 80;
const FUZZY_THRESHOLD: f64 = 0.75;
const FUZZY_BANDS: usize = 8;
const MAX_SHINGLES: usize = 64;
const MAX_BUCKET: usize = 300;

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct AnevIndicators {
    pub total_rows: usize,
    pub duplicate_ready: bool,
    pub pending_imports: Vec<String>,
    pub duplicate: DuplicateReport,
    pub drive: DriveReport,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct DuplicateReport {
    pub units: Vec<DuplicateUnit>,
    pub exact_groups: Vec<DuplicateGroup>,
    pub fuzzy_groups: Vec<DuplicateGroup>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DuplicateUnit {
    pub satuan: String,
    pub rows: usize,
    pub exact_rows: usize,
    pub fuzzy_rows: usize,
    pub exact_percent: f64,
    pub fuzzy_percent: f64,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DuplicateGroup {
    pub satuan: String,
    pub kode: String,
    pub kegiatan: String,
    pub count: usize,
    pub similarity: f64,
    pub sample: String,
    pub source_rows: Vec<i64>,
    pub rows: Vec<AnevRowView>,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct DriveReport {
    pub total_drive_rows: usize,
    pub repeated_drive_rows: usize,
    pub foto_drive_rows: usize,
    pub repeated_foto_rows: usize,
    pub distinct_ids: usize,
    pub repeated_ids: usize,
    pub max_reuse: usize,
    pub units: Vec<DriveUnit>,
    pub groups: Vec<DriveGroup>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DriveUnit {
    pub satuan: String,
    pub rows: usize,
    pub drive_rows: usize,
    pub repeated_rows: usize,
    pub foto_rows: usize,
    pub repeated_foto_rows: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DriveGroup {
    pub drive_id: String,
    pub link: String,
    pub count: usize,
    pub units: Vec<String>,
    pub sample: String,
    pub source_rows: Vec<i64>,
    pub rows: Vec<AnevRowView>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct AnevRowView {
    pub source_row: i64,
    pub tanggal: String,
    pub satuan: String,
    pub kode: String,
    pub kegiatan: String,
    pub isian_laporan: String,
    pub bukti_dukung: String,
}

struct AnevRow {
    source_row: i64,
    tanggal: String,
    satuan: String,
    kode: String,
    kegiatan: String,
    isian: String,
    bukti: String,
}

struct PreparedText {
    exact: Vec<u8>,
    shingles: Vec<u64>,
    signature: u64,
}

#[derive(Default)]
struct FuzzyUnion {
    parent: Vec<usize>,
    size: Vec<usize>,
    max_similarity: Vec<f64>,
}

impl FuzzyUnion {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            size: vec![1; len],
            max_similarity: vec![0.0; len],
        }
    }

    fn find(&mut self, index: usize) -> usize {
        let mut root = index;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut current = index;
        while self.parent[current] != current {
            let next = self.parent[current];
            self.parent[current] = root;
            current = next;
        }
        root
    }

    fn union(&mut self, left: usize, right: usize, similarity: f64) {
        let mut left_root = self.find(left);
        let mut right_root = self.find(right);
        if left_root == right_root {
            self.max_similarity[left_root] = self.max_similarity[left_root].max(similarity);
            return;
        }
        if self.size[left_root] < self.size[right_root] {
            std::mem::swap(&mut left_root, &mut right_root);
        }
        self.parent[right_root] = left_root;
        self.size[left_root] += self.size[right_root];
        self.max_similarity[left_root] = self.max_similarity[left_root]
            .max(self.max_similarity[right_root])
            .max(similarity);
    }
}

pub async fn indicators_for_period(
    db: &SqlitePool,
    period_id: &str,
    polres: Option<&str>,
    scope: &DataScope,
) -> ApiResult<AnevIndicators> {
    let (start, end, _) = periods::service::dates(db, period_id).await?;
    let (unique, _, _) = service::load_period_rows(db, &start, &end).await?;
    let duplicates = load_duplicate_rows(db, &start, &end).await?;
    let pending_imports = pending_imports(db, &start, &end).await?;

    let mut rows = Vec::with_capacity(unique.len() + duplicates.len());
    for row in unique {
        if scope.matches(&row) && matches_polres(&row.polres, polres) {
            rows.push(row_from_record(row));
        }
    }
    for row in duplicates {
        if scope.matches_fields(&row.level, &row.polres, &row.satker)
            && matches_polres(&row.polres, polres)
        {
            rows.push(row_from_duplicate(row));
        }
    }

    let duplicate = analyze_duplicates(&rows);
    let drive = analyze_drive(&rows);
    Ok(AnevIndicators {
        total_rows: rows.len(),
        duplicate_ready: pending_imports.is_empty(),
        pending_imports,
        duplicate,
        drive,
    })
}

pub async fn rebuild_period_duplicates(
    db: &SqlitePool,
    config: &Config,
    actor_id: &str,
    period_id: &str,
) -> ApiResult<usize> {
    let (start, end, _) = periods::service::dates(db, period_id).await?;
    let imports = sqlx::query(
        "SELECT id, filename FROM imports
         WHERE date_start <= ? AND date_end >= ? AND duplicates_synced = 0
         ORDER BY created_at ASC, id ASC",
    )
    .bind(&end)
    .bind(&start)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;
    let mut total = 0usize;
    for row in imports {
        let import_id: String = row.get("id");
        total += service::rebuild_duplicates(db, config, actor_id, &import_id).await?;
    }
    Ok(total)
}

async fn load_duplicate_rows(
    db: &SqlitePool,
    start: &str,
    end: &str,
) -> ApiResult<Vec<DuplicateRow>> {
    let rows = sqlx::query(
        "SELECT import_id, source_row, tanggal, waktu_kirim_wib, satuan, tingkat, polres,
                polsek, level, unit, satker, kode, kegiatan, isian_laporan, bukti_dukung,
                dedupe_key
         FROM import_duplicate_rows
         WHERE tanggal BETWEEN ? AND ?
         ORDER BY tanggal ASC, satuan ASC, source_row ASC",
    )
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;
    Ok(rows
        .into_iter()
        .map(|row| DuplicateRow {
            source_row: row.get("source_row"),
            tanggal: row.get("tanggal"),
            waktu_kirim_wib: row.get("waktu_kirim_wib"),
            satuan: row.get("satuan"),
            tingkat: row.get("tingkat"),
            polres: row.get("polres"),
            polsek: row.get("polsek"),
            level: row.get("level"),
            unit: row.get("unit"),
            satker: row.get("satker"),
            kode: row.get("kode"),
            kegiatan: row.get("kegiatan"),
            isian_laporan: row.get("isian_laporan"),
            bukti_dukung: row.get("bukti_dukung"),
            dedupe_key: row.get("dedupe_key"),
        })
        .collect())
}

async fn pending_imports(db: &SqlitePool, start: &str, end: &str) -> ApiResult<Vec<String>> {
    sqlx::query_scalar(
        "SELECT filename FROM imports
         WHERE date_start <= ? AND date_end >= ? AND duplicates_synced = 0
         ORDER BY created_at ASC, id ASC",
    )
    .bind(end)
    .bind(start)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)
}

fn matches_polres(value: &str, polres: Option<&str>) -> bool {
    polres.is_none_or(|target| value.trim().eq_ignore_ascii_case(target.trim()))
}

fn row_from_record(row: RecordView) -> AnevRow {
    let satuan = if row.satuan.trim().is_empty() {
        row.unit.clone()
    } else {
        row.satuan
    };
    AnevRow {
        source_row: row.source_row,
        tanggal: row.tanggal,
        satuan,
        kode: row.kode,
        kegiatan: row.kegiatan,
        isian: row.isian_laporan,
        bukti: row.bukti_dukung,
    }
}

fn row_from_duplicate(row: DuplicateRow) -> AnevRow {
    let satuan = if row.satuan.trim().is_empty() {
        row.unit.clone()
    } else {
        row.satuan
    };
    AnevRow {
        source_row: row.source_row,
        tanggal: row.tanggal,
        satuan,
        kode: row.kode,
        kegiatan: row.kegiatan,
        isian: row.isian_laporan,
        bukti: row.bukti_dukung,
    }
}

fn analyze_duplicates(rows: &[AnevRow]) -> DuplicateReport {
    let mut by_unit: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        by_unit
            .entry(unit_key(&row.satuan))
            .or_default()
            .push(index);
    }

    let mut units = Vec::new();
    let mut exact_groups = Vec::new();
    let mut fuzzy_groups = Vec::new();

    for indices in by_unit.values() {
        let display = rows[*indices.first().unwrap()].satuan.trim().to_string();
        let display = if display.is_empty() {
            "(tanpa satuan)".to_string()
        } else {
            display
        };

        let mut by_hash: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();
        for index in indices {
            let normalized = normalize_text(&rows[*index].isian);
            if normalized.is_empty() {
                continue;
            }
            by_hash
                .entry(hash_bytes(normalized.as_bytes()))
                .or_default()
                .push(*index);
        }
        let mut exact_rows = 0usize;
        for group in by_hash.values().filter(|group| group.len() >= 2) {
            exact_rows += group.len();
            exact_groups.push(group_view(rows, group, 1.0));
        }

        // Unit yang sudah punya duplikat persis dan ukurannya masih wajar
        // dihitung dengan kandidat shingle penuh agar hasilnya mendekati
        // pemeriksaan manual. Unit besar memakai SimHash/LSH sebagai perkiraan.
        let (fuzzy_count, mut groups) = if indices.len() <= 3000 && exact_rows > 0 {
            fuzzy_rows_exact_candidates(indices, rows)
        } else {
            let prepared: Vec<Option<PreparedText>> = indices
                .iter()
                .map(|index| prepare_text(&rows[*index].isian))
                .collect();
            fuzzy_rows_from_prepared(&prepared, indices, rows).unwrap_or_default()
        };
        fuzzy_groups.append(&mut groups);
        units.push(DuplicateUnit {
            satuan: display,
            rows: indices.len(),
            exact_rows,
            fuzzy_rows: fuzzy_count.max(exact_rows),
            exact_percent: percent(exact_rows, indices.len()),
            fuzzy_percent: percent(fuzzy_count.max(exact_rows), indices.len()),
        });
    }

    exact_groups.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.satuan.cmp(&right.satuan))
    });
    fuzzy_groups.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.satuan.cmp(&right.satuan))
    });
    exact_groups.truncate(MAX_GROUPS);
    fuzzy_groups.truncate(MAX_GROUPS);
    units.sort_by(|left, right| {
        right
            .fuzzy_percent
            .partial_cmp(&left.fuzzy_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.rows.cmp(&left.rows))
    });

    DuplicateReport {
        units,
        exact_groups,
        fuzzy_groups,
    }
}

fn fuzzy_rows_exact_candidates(
    indices: &[usize],
    rows: &[AnevRow],
) -> (usize, Vec<DuplicateGroup>) {
    let mut prepared: Vec<Option<(Vec<u8>, Vec<u64>)>> = Vec::with_capacity(indices.len());
    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
    for index_row in indices {
        let normalized = normalize_text(&rows[*index_row].isian);
        if normalized.chars().count() < MIN_FUZZY_CHARS {
            prepared.push(None);
            continue;
        }
        let tokens = tokenize(&normalized);
        let mut shingles = shingle_hashes(&tokens);
        shingles.sort_unstable();
        shingles.dedup();
        if shingles.is_empty() {
            prepared.push(None);
            continue;
        }
        let local = prepared.len();
        for shingle in &shingles {
            index.entry(*shingle).or_default().push(local);
        }
        prepared.push(Some((hash_bytes(normalized.as_bytes()), shingles)));
    }

    let mut union = FuzzyUnion::new(prepared.len());
    for local in 0..prepared.len() {
        let Some((_, shingles)) = prepared[local].as_ref() else {
            continue;
        };
        let mut candidates: HashMap<usize, usize> = HashMap::new();
        for shingle in shingles {
            let Some(row_ids) = index.get(shingle) else {
                continue;
            };
            if row_ids.len() > 250 {
                continue;
            }
            for other in row_ids {
                if *other != local {
                    *candidates.entry(*other).or_default() += 1;
                }
            }
        }
        let mut ranked: Vec<(usize, usize)> = candidates.into_iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(&left.1));
        ranked.truncate(80);
        for (other, _) in ranked {
            if local >= other {
                continue;
            }
            let (Some((_, left_shingles)), Some((_, right_shingles))) =
                (prepared[local].as_ref(), prepared[other].as_ref())
            else {
                continue;
            };
            let similarity = jaccard(left_shingles, right_shingles);
            if similarity >= FUZZY_THRESHOLD {
                union.union(local, other, similarity);
            }
        }
    }

    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for local in 0..prepared.len() {
        if prepared[local].is_some() {
            let root = union.find(local);
            components.entry(root).or_default().push(local);
        }
    }
    let mut fuzzy_count = 0usize;
    let mut groups = Vec::new();
    for (root, component) in components {
        if component.len() < 2 {
            continue;
        }
        fuzzy_count += component.len();
        let exact = prepared[component[0]]
            .as_ref()
            .map(|(hash, _)| hash.clone())
            .unwrap_or_default();
        let all_exact = component.iter().all(|local| {
            prepared[*local]
                .as_ref()
                .is_some_and(|(hash, _)| *hash == exact)
        });
        if all_exact {
            continue;
        }
        let global: Vec<usize> = component.iter().map(|local| indices[*local]).collect();
        let similarity = union.max_similarity[root].max(FUZZY_THRESHOLD);
        groups.push(group_view(rows, &global, similarity));
    }
    (fuzzy_count, groups)
}

fn fuzzy_rows_from_prepared(
    prepared: &[Option<PreparedText>],
    indices: &[usize],
    rows: &[AnevRow],
) -> Option<(usize, Vec<DuplicateGroup>)> {
    let active: Vec<usize> = prepared
        .iter()
        .enumerate()
        .filter_map(|(index, item)| item.as_ref().map(|_| index))
        .collect();
    if active.len() < 2 {
        return Some((0, Vec::new()));
    }

    let mut bands: Vec<HashMap<u8, Vec<usize>>> =
        (0..FUZZY_BANDS).map(|_| HashMap::new()).collect();
    for &local in &active {
        let signature = prepared[local].as_ref()?.signature;
        for (band, buckets) in bands.iter_mut().enumerate() {
            let key = ((signature >> (band * 8)) & 0xFF) as u8;
            buckets.entry(key).or_default().push(local);
        }
    }

    let mut candidates: HashSet<(usize, usize)> = HashSet::new();
    for buckets in &bands {
        for bucket in buckets.values() {
            if bucket.len() < 2 || bucket.len() > MAX_BUCKET {
                continue;
            }
            for left in 0..bucket.len() {
                for right in (left + 1)..bucket.len() {
                    let (a, b) = (bucket[left], bucket[right]);
                    candidates.insert(if a < b { (a, b) } else { (b, a) });
                }
            }
        }
    }

    let mut union = FuzzyUnion::new(prepared.len());
    for (left, right) in candidates {
        let (Some(a), Some(b)) = (prepared[left].as_ref(), prepared[right].as_ref()) else {
            continue;
        };
        let similarity = jaccard(&a.shingles, &b.shingles);
        if similarity >= FUZZY_THRESHOLD {
            union.union(left, right, similarity);
        }
    }

    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for &local in &active {
        let root = union.find(local);
        components.entry(root).or_default().push(local);
    }

    let mut fuzzy_count = 0usize;
    let mut groups = Vec::new();
    for (root, component) in components {
        if component.len() < 2 {
            continue;
        }
        fuzzy_count += component.len();
        let exact = prepared[component[0]]
            .as_ref()
            .map(|item| item.exact.clone())
            .unwrap_or_default();
        let all_exact = component.iter().all(|local| {
            prepared[*local]
                .as_ref()
                .is_some_and(|item| item.exact == exact)
        });
        if all_exact {
            continue;
        }
        let global: Vec<usize> = component.iter().map(|local| indices[*local]).collect();
        let similarity = union.max_similarity[root].max(FUZZY_THRESHOLD);
        groups.push(group_view(rows, &global, similarity));
    }
    Some((fuzzy_count, groups))
}

fn prepare_text(value: &str) -> Option<PreparedText> {
    let normalized = normalize_text(value);
    if normalized.chars().count() < MIN_FUZZY_CHARS {
        return None;
    }
    let tokens = tokenize(&normalized);
    if tokens.is_empty() {
        return None;
    }
    let mut shingles = shingle_hashes(&tokens);
    if shingles.is_empty() {
        return None;
    }
    shingles.sort_unstable();
    shingles.dedup();
    if shingles.len() > MAX_SHINGLES {
        let step = shingles.len() as f64 / MAX_SHINGLES as f64;
        shingles = (0..MAX_SHINGLES)
            .map(|index| shingles[(index as f64 * step) as usize])
            .collect();
        shingles.sort_unstable();
        shingles.dedup();
    }
    let signature = simhash(&shingles);
    Some(PreparedText {
        exact: hash_bytes(normalized.as_bytes()),
        shingles,
        signature,
    })
}

fn group_view(rows: &[AnevRow], indices: &[usize], similarity: f64) -> DuplicateGroup {
    let first = &rows[indices[0]];
    let view_rows: Vec<AnevRowView> = indices
        .iter()
        .take(MAX_ROWS_PER_GROUP)
        .map(|index| row_view(&rows[*index]))
        .collect();
    DuplicateGroup {
        satuan: first.satuan.trim().to_string(),
        kode: first.kode.clone(),
        kegiatan: first.kegiatan.clone(),
        count: indices.len(),
        similarity,
        sample: truncate_text(&first.isian, SAMPLE_CHARS),
        source_rows: indices
            .iter()
            .take(MAX_ROWS_PER_GROUP)
            .map(|index| rows[*index].source_row)
            .collect(),
        rows: view_rows,
    }
}

fn analyze_drive(rows: &[AnevRow]) -> DriveReport {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut first_link: HashMap<String, String> = HashMap::new();
    let mut per_row: Vec<(HashSet<String>, HashSet<String>)> = Vec::with_capacity(rows.len());

    for row in rows {
        let mut keys = HashSet::new();
        let mut links = extract_links(&row.isian);
        links.extend(extract_links(&row.bukti));
        for link in links {
            if let Some(id) = drive_id(&link) {
                counts.entry(id.clone()).or_insert(0);
                *counts.get_mut(&id).unwrap() += 1;
                first_link.entry(id.clone()).or_insert(link);
                keys.insert(id);
            }
        }
        let foto = row
            .isian
            .lines()
            .find(|line| line.to_ascii_lowercase().contains("foto kegiatan"))
            .map(|line| {
                extract_links(line)
                    .into_iter()
                    .filter_map(|link| drive_id(&link))
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        per_row.push((keys, foto));
    }

    let mut units: BTreeMap<String, DriveUnit> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let key = unit_key(&row.satuan);
        let display = if row.satuan.trim().is_empty() {
            "(tanpa satuan)".to_string()
        } else {
            row.satuan.trim().to_string()
        };
        let entry = units.entry(key).or_insert_with(|| DriveUnit {
            satuan: display,
            rows: 0,
            drive_rows: 0,
            repeated_rows: 0,
            foto_rows: 0,
            repeated_foto_rows: 0,
        });
        entry.rows += 1;
        let (keys, foto) = &per_row[index];
        if !keys.is_empty() {
            entry.drive_rows += 1;
            if keys
                .iter()
                .any(|key| counts.get(key).copied().unwrap_or(0) >= 2)
            {
                entry.repeated_rows += 1;
            }
        }
        if !foto.is_empty() {
            entry.foto_rows += 1;
            if foto
                .iter()
                .any(|key| counts.get(key).copied().unwrap_or(0) >= 2)
            {
                entry.repeated_foto_rows += 1;
            }
        }
    }

    let total_drive_rows = per_row.iter().filter(|(keys, _)| !keys.is_empty()).count();
    let repeated_drive_rows = per_row
        .iter()
        .filter(|(keys, _)| {
            keys.iter()
                .any(|key| counts.get(key).copied().unwrap_or(0) >= 2)
        })
        .count();
    let foto_drive_rows = per_row.iter().filter(|(_, foto)| !foto.is_empty()).count();
    let repeated_foto_rows = per_row
        .iter()
        .filter(|(_, foto)| {
            foto.iter()
                .any(|key| counts.get(key).copied().unwrap_or(0) >= 2)
        })
        .count();

    let mut groups = Vec::new();
    for (id, count) in &counts {
        if *count < 2 {
            continue;
        }
        let mut group_rows = Vec::new();
        let mut unit_set = BTreeSet::new();
        for (index, row) in rows.iter().enumerate() {
            if per_row[index].0.contains(id) {
                unit_set.insert(row.satuan.trim().to_string());
                if group_rows.len() < MAX_ROWS_PER_GROUP {
                    group_rows.push(row_view(row));
                }
            }
        }
        let first = group_rows.first();
        groups.push(DriveGroup {
            drive_id: id.clone(),
            link: first_link.get(id).cloned().unwrap_or_default(),
            count: *count,
            units: unit_set.into_iter().collect(),
            sample: first
                .map(|row| truncate_text(&row.isian_laporan, SAMPLE_CHARS))
                .unwrap_or_default(),
            source_rows: group_rows.iter().map(|row| row.source_row).collect(),
            rows: group_rows,
        });
    }
    groups.sort_by(|left, right| right.count.cmp(&left.count));
    groups.truncate(MAX_GROUPS);

    let mut unit_list: Vec<DriveUnit> = units.into_values().collect();
    unit_list.sort_by(|left, right| {
        right
            .repeated_rows
            .cmp(&left.repeated_rows)
            .then_with(|| right.rows.cmp(&left.rows))
    });

    DriveReport {
        total_drive_rows,
        repeated_drive_rows,
        foto_drive_rows,
        repeated_foto_rows,
        distinct_ids: counts.len(),
        repeated_ids: counts.values().filter(|count| **count >= 2).count(),
        max_reuse: counts.values().copied().max().unwrap_or(0),
        units: unit_list,
        groups,
    }
}

fn row_view(row: &AnevRow) -> AnevRowView {
    AnevRowView {
        source_row: row.source_row,
        tanggal: row.tanggal.clone(),
        satuan: row.satuan.clone(),
        kode: row.kode.clone(),
        kegiatan: row.kegiatan.clone(),
        isian_laporan: truncate_text(&row.isian, ROW_TEXT_CHARS),
        bukti_dukung: truncate_text(&row.bukti, 300),
    }
}

fn unit_key(value: &str) -> String {
    let value = value.trim().to_lowercase();
    if value.is_empty() {
        "(tanpa satuan)".to_string()
    } else {
        value
    }
}

fn normalize_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for token in value.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        if lower.contains("http://") || lower.contains("https://") {
            continue;
        }
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(&lower);
    }
    output
}

fn tokenize(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn shingle_hashes(tokens: &[String]) -> Vec<u64> {
    if tokens.len() < 5 {
        return tokens
            .iter()
            .map(|token| fnv1a64(token.as_bytes()))
            .collect();
    }
    (0..=tokens.len() - 5)
        .map(|start| {
            let mut hash = 1469598103934665603_u64;
            for token in &tokens[start..start + 5] {
                for byte in token.as_bytes() {
                    hash ^= *byte as u64;
                    hash = hash.wrapping_mul(1099511628211);
                }
                hash ^= 0xFF;
                hash = hash.wrapping_mul(1099511628211);
            }
            hash
        })
        .collect()
}

fn simhash(shingles: &[u64]) -> u64 {
    let mut weights = [0_i32; 64];
    for shingle in shingles {
        for (bit, weight) in weights.iter_mut().enumerate() {
            *weight += if (shingle >> bit) & 1 == 1 { 1 } else { -1 };
        }
    }
    let mut signature = 0_u64;
    for (bit, weight) in weights.iter().enumerate() {
        if *weight > 0 {
            signature |= 1_u64 << bit;
        }
    }
    signature
}

fn jaccard(left: &[u64], right: &[u64]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let mut i = 0usize;
    let mut j = 0usize;
    let mut intersection = 0usize;
    while i < left.len() && j < right.len() {
        match left[i].cmp(&right[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                intersection += 1;
                i += 1;
                j += 1;
            }
        }
    }
    let union = left.len() + right.len() - intersection;
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn hash_bytes(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 1469598103934665603_u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn drive_id(link: &str) -> Option<String> {
    let url = Url::parse(link).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if host != "drive.google.com" && host != "docs.google.com" {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.collect();
    for index in 0..segments.len().saturating_sub(1) {
        if matches!(segments[index], "d" | "folders") {
            let id = segments[index + 1];
            if id.len() >= 10 {
                return Some(id.to_string());
            }
        }
    }
    for (key, value) in url.query_pairs() {
        if key == "id" && value.len() >= 10 {
            return Some(value.into_owned());
        }
    }
    None
}

fn truncate_text(value: &str, limit: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let truncated: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn percent(value: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (value as f64 / total as f64 * 10000.0).round() / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AnevRow, analyze_drive, analyze_duplicates, drive_id, jaccard, normalize_text, simhash,
    };

    fn row(source_row: i64, satuan: &str, isian: &str, bukti: &str) -> AnevRow {
        AnevRow {
            source_row,
            tanggal: "2026-11-09".to_string(),
            satuan: satuan.to_string(),
            kode: "QW5.1".to_string(),
            kegiatan: "SP2HP Berkala kepada Pelapor".to_string(),
            isian: isian.to_string(),
            bukti: bukti.to_string(),
        }
    }

    #[test]
    fn normalizes_whitespace_and_drops_urls() {
        assert_eq!(
            normalize_text("Halo   DUNIA\nhttps://drive.google.com/x"),
            "halo dunia"
        );
    }

    #[test]
    fn extracts_drive_ids_from_common_links() {
        assert_eq!(
            drive_id("https://drive.google.com/file/d/abc123456789/view?usp=sharing"),
            Some("abc123456789".to_string())
        );
        assert_eq!(
            drive_id("https://drive.google.com/drive/folders/xyz987654321?usp=sharing"),
            Some("xyz987654321".to_string())
        );
        assert_eq!(drive_id("https://example.com/foto.jpg"), None);
    }

    #[test]
    fn jaccard_handles_overlap() {
        assert_eq!(jaccard(&[1, 2, 3], &[2, 3, 4]), 0.5);
        assert_eq!(jaccard(&[1, 2], &[1, 2]), 1.0);
    }

    #[test]
    fn simhash_is_stable() {
        let first = simhash(&[1, 2, 3, 4]);
        let second = simhash(&[1, 2, 3, 4]);
        assert_eq!(first, second);
    }

    #[test]
    fn detects_exact_duplicates_and_repeated_drive() {
        let isian = "a pelaksana kanit reskrim polsek b kegiatan yang dilaksanakan qw5.1 sp2hp berkala kepada pelapor c apakah satuan anda menangani penyidikan perkara tidak d lokasi kegiatan mapolsek uji e jam pelaksanaan 08.00 sampai 10.00 f jumlah laporan diterima 2";
        let link = "https://drive.google.com/file/d/abc123456789/view";
        let rows = vec![
            row(1, "Polsek Uji", isian, link),
            row(2, "Polsek Uji", isian, link),
            row(
                3,
                "Polsek Uji",
                "teks lain yang cukup panjang dan berbeda dari isian sebelumnya",
                "",
            ),
        ];
        let duplicate = analyze_duplicates(&rows);
        assert_eq!(duplicate.units.len(), 1);
        assert_eq!(duplicate.units[0].exact_rows, 2);
        assert_eq!(duplicate.exact_groups.len(), 1);
        assert_eq!(duplicate.exact_groups[0].count, 2);

        let drive = analyze_drive(&rows);
        assert_eq!(drive.total_drive_rows, 2);
        assert_eq!(drive.repeated_drive_rows, 2);
        assert_eq!(drive.groups.len(), 1);
        assert_eq!(drive.groups[0].count, 2);
    }

    #[test]
    fn detects_near_duplicate_text() {
        let base = "a pelaksana kanit reskrim polsek b kegiatan yang dilaksanakan qw5.1 sp2hp berkala kepada pelapor c apakah satuan anda menangani penyidikan perkara tidak d lokasi kegiatan mapolsek uji e jam pelaksanaan kegiatan 08.00 sampai 10.00 f jumlah laporan yang diterima 2 g tindak lanjut sudah dilakukan";
        let variant = base.replace("qw5.1", "qw5.2");
        let rows = vec![
            row(1, "Polsek Uji", base, ""),
            row(2, "Polsek Uji", &variant, ""),
        ];
        let duplicate = analyze_duplicates(&rows);
        assert_eq!(duplicate.exact_groups.len(), 0);
        assert_eq!(duplicate.fuzzy_groups.len(), 1);
        assert_eq!(duplicate.fuzzy_groups[0].count, 2);
        assert!(duplicate.units[0].fuzzy_rows >= 2);
    }
}
