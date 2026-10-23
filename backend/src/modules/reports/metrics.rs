use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use sqlx::{Row, SqlitePool};

use crate::{
    error::{ApiError, ApiResult},
    modules::{
        imports::models::ParsedRow,
        reports::models::{
            AspirationMetrics, DailyCount, FollowUpMetrics, GreenMetrics, GroupMetrics,
            MediaMetrics, MetricCard, NarkobaMetrics, ReligiousMetrics, ReportMetrics, RowSummary,
        },
    },
    shared::{
        LabeledCount, extract_links, format_count, is_noise, parse_count, parse_fields, truncate,
    },
};

struct MetricRow {
    source_row: i64,
    tanggal: String,
    satuan: String,
    tingkat: String,
    polda: String,
    polres: String,
    polsek: String,
    kode: String,
    kegiatan: String,
    quick_wins: String,
    status_penerimaan: String,
    fields: BTreeMap<String, String>,
    links: Vec<String>,
}

pub async fn load(
    db: &SqlitePool,
    import_id: &str,
    source_filename: &str,
    period_label: &str,
    start: &str,
    end: &str,
    polda_name: &str,
) -> ApiResult<ReportMetrics> {
    let rows = sqlx::query(
        "SELECT source_row, tanggal, satuan, tingkat, polda, polres, polsek, kode, kegiatan,
                quick_wins, status_penerimaan, isian_laporan, parsed_json
         FROM import_rows
         WHERE import_id = ? AND tanggal BETWEEN ? AND ?
         ORDER BY tanggal ASC, source_row ASC",
    )
    .bind(import_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await
    .map_err(ApiError::internal)?;

    let outside_period: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM import_rows WHERE import_id = ? AND tanggal NOT BETWEEN ? AND ?",
    )
    .bind(import_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await
    .map_err(ApiError::internal)?;

    let mut metric_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let isian: String = row.get("isian_laporan");
        let parsed: ParsedRow =
            serde_json::from_str(&row.get::<String, _>("parsed_json")).unwrap_or_default();
        let fields = if parsed.fields.is_empty() {
            parse_fields(&isian)
        } else {
            parsed.fields
        };
        let mut links = if parsed.links.is_empty() {
            extract_links(&isian)
        } else {
            parsed.links
        };
        links.extend(extract_links(&isian));
        links.sort();
        links.dedup();

        metric_rows.push(MetricRow {
            source_row: row.get("source_row"),
            tanggal: row.get("tanggal"),
            satuan: row.get("satuan"),
            tingkat: row.get("tingkat"),
            polda: row.get("polda"),
            polres: row.get("polres"),
            polsek: row.get("polsek"),
            kode: row.get("kode"),
            kegiatan: row.get("kegiatan"),
            quick_wins: row.get("quick_wins"),
            status_penerimaan: row.get("status_penerimaan"),
            fields,
            links,
        });
    }

    Ok(build(
        metric_rows,
        outside_period,
        import_id,
        source_filename,
        period_label,
        start,
        end,
        polda_name,
    ))
}

fn build(
    rows: Vec<MetricRow>,
    outside_period: i64,
    import_id: &str,
    source_filename: &str,
    period_label: &str,
    start: &str,
    end: &str,
    polda_name: &str,
) -> ReportMetrics {
    let all: Vec<&MetricRow> = rows.iter().collect();
    let narkoba = build_narkoba(&all);
    let green_policing = build_green(&all);
    let jumat_keliling = build_religious(
        &all,
        "QW1.6",
        &[
            "Pertimbangan pilihan lokasi (Perkiraan jumlah jemaah)",
            "Jumlah jemaah",
            "Jumlah jemaah dijangkau",
        ],
    );
    let subuh_keliling = build_religious(
        &all,
        "QW1.7",
        &[
            "Pertimbangan pilihan lokasi (Perkiraan jumlah jemaah)",
            "Jumlah jemaah",
            "Jumlah jemaah dijangkau",
        ],
    );
    let minggu_kasih = build_religious(
        &all,
        "QW1.8",
        &[
            "Pertimbangan pilihan lokasi (Perkiraan jumlah jemaat atau umat)",
            "Jumlah jemaat",
            "Jumlah jemaat dijangkau",
        ],
    );
    let aspirations = build_aspirations(&all);
    let follow_up = build_follow_up(&all);
    let media = build_media(&all);

    let cards = build_cards(
        &narkoba,
        &green_policing,
        &jumat_keliling,
        &subuh_keliling,
        &minggu_kasih,
        &media,
    );

    let mut highlights = Vec::new();
    for row in &all {
        if highlights.len() >= 8 {
            break;
        }
        let location = row_location(row);
        if !is_noise(&row.kegiatan) && !is_noise(&location) {
            highlights.push(row_summary(row));
        }
    }

    ReportMetrics {
        import_id: import_id.to_string(),
        source_filename: source_filename.to_string(),
        polda_name: polda_name.to_string(),
        period_label: period_label.to_string(),
        period_start: start.to_string(),
        period_end: end.to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        total_rows: rows.len() as i64,
        outside_period,
        cards,
        quick_wins: count_by(&all, |row| quick_win_of(row), 0),
        codes: count_by(&all, |row| row.kode.clone(), 20),
        statuses: count_by(
            &all,
            |row| {
                if row.status_penerimaan.trim().is_empty() {
                    "Belum diisi".to_string()
                } else {
                    row.status_penerimaan.clone()
                }
            },
            20,
        ),
        daily: daily_series(start, end, &all),
        by_polres: count_by(&all, |row| row.polres.clone(), 30),
        by_level: count_by(&all, |row| row.tingkat.clone(), 10),
        narkoba,
        green_policing,
        jumat_keliling,
        subuh_keliling,
        minggu_kasih,
        aspirations,
        follow_up,
        media,
        highlights,
    }
}

fn build_cards(
    narkoba: &NarkobaMetrics,
    green: &GreenMetrics,
    jumat: &ReligiousMetrics,
    subuh: &ReligiousMetrics,
    minggu: &ReligiousMetrics,
    media: &MediaMetrics,
) -> Vec<MetricCard> {
    let narkoba_value = if narkoba.orang_diamankan > 0 {
        (narkoba.orang_diamankan, "orang ditangkap")
    } else if narkoba.kawasan > 0 {
        (narkoba.kawasan, "kawasan narkoba")
    } else {
        (narkoba.rows, "kegiatan narkoba")
    };

    let green_value = if green.bibit > 0 {
        (green.bibit, "bibit ditanam")
    } else {
        (green.rows, "kegiatan hijau")
    };

    vec![
        MetricCard {
            key: "narkoba".into(),
            label: "Berantas Narkoba".into(),
            value: narkoba_value.0,
            unit: narkoba_value.1.into(),
            detail: format!(
                "{} kegiatan · {} kawasan · {} rehabilitasi",
                format_count(narkoba.rows),
                format_count(narkoba.kawasan),
                format_count(narkoba.rehabilitasi)
            ),
        },
        MetricCard {
            key: "green_policing".into(),
            label: "Green Policing".into(),
            value: green_value.0,
            unit: green_value.1.into(),
            detail: format!(
                "{} kegiatan · {} m² area · {} kerja bakti",
                format_count(green.rows),
                format_count(green.area_m2),
                format_count(green.kerja_bakti)
            ),
        },
        MetricCard {
            key: "jumat_keliling".into(),
            label: "Jumat Keliling".into(),
            value: jumat.people,
            unit: "jemaah dijangkau".into(),
            detail: format!(
                "{} masjid · {} polres · {} polsek",
                format_count(jumat.locations),
                format_count(jumat.polres),
                format_count(jumat.polsek)
            ),
        },
        MetricCard {
            key: "subuh_keliling".into(),
            label: "Subuh Keliling".into(),
            value: subuh.people,
            unit: "jemaah dijangkau".into(),
            detail: format!(
                "{} masjid · {} polres · {} polsek",
                format_count(subuh.locations),
                format_count(subuh.polres),
                format_count(subuh.polsek)
            ),
        },
        MetricCard {
            key: "minggu_kasih".into(),
            label: "Minggu Kasih".into(),
            value: minggu.people,
            unit: "jemaat dijangkau".into(),
            detail: format!(
                "{} lokasi ibadah · {} polres · {} polsek",
                format_count(minggu.locations),
                format_count(minggu.polres),
                format_count(minggu.polsek)
            ),
        },
        MetricCard {
            key: "media".into(),
            label: "Amplifikasi Media".into(),
            value: media.publications,
            unit: "berita / tautan".into(),
            detail: format!(
                "{} kegiatan teramplifikasi · {} dokumentasi",
                format_count(media.rows_with_media),
                format_count(media.documentation)
            ),
        },
    ]
}

fn build_narkoba(rows: &[&MetricRow]) -> NarkobaMetrics {
    let subset: Vec<&MetricRow> = rows
        .iter()
        .copied()
        .filter(|row| row.kode.starts_with("QW2"))
        .collect();

    let orang_diamankan = sum_numbers(
        &subset,
        &[
            "Orang yang ditangkap (Jumlah)",
            "Jumlah orang yang ditangkap (Jumlah)",
            "Jumlah orang yang diamankan (Jumlah)",
            "Jumlah tersangka",
        ],
    );
    let kawasan = sum_numbers(
        &subset,
        &[
            "Jumlah kawasan narkoba (Jumlah)",
            "Jumlah kawasan yang sudah dibina (Jumlah)",
        ],
    );
    let rehabilitasi = sum_numbers(
        &subset,
        &[
            "Orang yang direhabilitasi (Jumlah)",
            "Jumlah pengguna yang direhabilitasi",
        ],
    );
    let peserta_penyuluhan = sum_numbers(
        &subset,
        &[
            "Jumlah peserta penyuluhan narkoba",
            "Orang yang diberikan penyuluhan (Jumlah)",
        ],
    );

    let mut top_locations = Counter::default();
    for row in &subset {
        top_locations.add(&row_location(row));
    }

    let activities = subset
        .iter()
        .filter(|row| !is_noise(&row.kegiatan))
        .take(6)
        .map(|row| row_summary(row))
        .collect();

    NarkobaMetrics {
        rows: subset.len() as i64,
        orang_diamankan,
        kawasan,
        rehabilitasi,
        peserta_penyuluhan,
        by_code: count_by(&subset, |row| row.kode.clone(), 10),
        by_polres: group_metrics(
            &subset,
            &[
                "Orang yang ditangkap (Jumlah)",
                "Jumlah orang yang ditangkap (Jumlah)",
                "Jumlah orang yang diamankan (Jumlah)",
            ],
            &[],
        ),
        top_locations: top_locations.into_vec(8),
        activities,
    }
}

fn build_green(rows: &[&MetricRow]) -> GreenMetrics {
    let subset: Vec<&MetricRow> = rows
        .iter()
        .copied()
        .filter(|row| row.kode.starts_with("QW1.5"))
        .collect();

    let bibit = sum_numbers(&subset, &["Jumlah bibit pohon yang ditanam"]);
    let area_m2 = sum_numbers(
        &subset,
        &["Pertimbangan pilihan lokasi (Luas area yang ditangani (m²))"],
    );
    let peserta = sum_numbers(&subset, &["Jumlah peserta"]);

    let kerja_bakti = subset
        .iter()
        .filter(|row| {
            field(row, &["Kegiatan"])
                .map(|value| value.to_ascii_lowercase().contains("kerja bakti"))
                .unwrap_or(false)
        })
        .count() as i64;
    let penanaman = subset
        .iter()
        .filter(|row| {
            number_for(row, &["Jumlah bibit pohon yang ditanam"]) > 0
                || field(row, &["Kegiatan"])
                    .map(|value| value.to_ascii_lowercase().contains("tanam"))
                    .unwrap_or(false)
        })
        .count() as i64;

    GreenMetrics {
        rows: subset.len() as i64,
        bibit,
        area_m2,
        kerja_bakti,
        penanaman,
        peserta,
        by_polres: group_metrics(
            &subset,
            &["Jumlah peserta"],
            &["Jumlah bibit pohon yang ditanam"],
        ),
        by_kegiatan: count_by(
            &subset,
            |row| field(row, &["Kegiatan"]).unwrap_or_else(|| "Kegiatan hijau".to_string()),
            10,
        ),
    }
}

fn build_religious(rows: &[&MetricRow], code: &str, people_keys: &[&str]) -> ReligiousMetrics {
    let subset: Vec<&MetricRow> = rows
        .iter()
        .copied()
        .filter(|row| row.kode.starts_with(code))
        .collect();

    let people = sum_numbers(&subset, people_keys);
    let locations = unique_count(&subset, |row| row_location(row));
    let polres = unique_count(&subset, |row| row.polres.clone());
    let polsek = unique_count(&subset, |row| row.polsek.clone());

    let mut top_locations = Counter::default();
    for row in &subset {
        top_locations.add(&row_location(row));
    }

    ReligiousMetrics {
        rows: subset.len() as i64,
        people,
        locations,
        polres,
        polsek,
        by_polres: group_metrics(&subset, people_keys, &[]),
        by_religion: count_by(
            &subset,
            |row| field(row, &["Kegiatan agama"]).unwrap_or_default(),
            10,
        ),
        top_locations: top_locations.into_vec(8),
    }
}

fn build_aspirations(rows: &[&MetricRow]) -> AspirationMetrics {
    let subset: Vec<&MetricRow> = rows
        .iter()
        .copied()
        .filter(|row| {
            row.kode.starts_with("QW1.6")
                || row.kode.starts_with("QW1.7")
                || row.kode.starts_with("QW1.8")
        })
        .collect();

    let theme_key = "Pendapat/masukan masyarakat (Tema masukan)";
    let status_key = "Tindak lanjut pendapat/masukan masyarakat (Status tindak lanjut)";
    let narrative_key = "Pendapat/masukan masyarakat (Uraian)";

    let total = subset
        .iter()
        .filter(|row| !is_noise(&field(row, &[theme_key, narrative_key]).unwrap_or_default()))
        .count() as i64;

    let mut quotes = Vec::new();
    let mut seen = BTreeSet::new();
    for row in &subset {
        if quotes.len() >= 4 {
            break;
        }
        if let Some(value) = field(row, &[narrative_key]) {
            if !is_noise(&value) && value.chars().count() > 20 {
                let normalized = value.to_ascii_lowercase();
                if seen.insert(normalized) {
                    quotes.push(truncate(&value, 240));
                }
            }
        }
    }

    AspirationMetrics {
        total,
        themes: count_by(
            &subset,
            |row| field(row, &[theme_key]).unwrap_or_default(),
            12,
        ),
        statuses: count_by(
            &subset,
            |row| field(row, &[status_key]).unwrap_or_default(),
            10,
        ),
        by_source: count_by(
            &subset
                .iter()
                .copied()
                .filter(|row| {
                    !is_noise(&field(row, &[theme_key, narrative_key]).unwrap_or_default())
                })
                .collect::<Vec<_>>(),
            |row| activity_label(row),
            10,
        ),
        quotes,
    }
}

fn build_follow_up(rows: &[&MetricRow]) -> FollowUpMetrics {
    let form_key = "Tindak lanjut pendapat/masukan masyarakat (Bentuk tindak lanjut)";
    let forms = count_by(rows, |row| field(row, &[form_key]).unwrap_or_default(), 10);
    let total = rows
        .iter()
        .filter(|row| !is_noise(&field(row, &[form_key]).unwrap_or_default()))
        .count() as i64;

    let mut activities = Vec::new();
    for row in rows {
        if activities.len() >= 6 {
            break;
        }
        let is_follow_up_activity = row.kode.starts_with("QW1.4")
            || row.kode.starts_with("QW1.9")
            || row.kode.starts_with("QW1.10")
            || row.kode.starts_with("QW1.11")
            || row.kode.starts_with("QW4.1")
            || row.kode.starts_with("QW4.3");
        if is_follow_up_activity && !is_noise(&row.kegiatan) {
            activities.push(row_summary(row));
        }
    }

    FollowUpMetrics {
        total,
        forms,
        activities,
    }
}

fn build_media(rows: &[&MetricRow]) -> MediaMetrics {
    let mut publications = BTreeSet::new();
    let mut documentation = BTreeSet::new();
    let mut channels = Counter::default();
    let mut activities = Counter::default();
    let mut domains = Counter::default();
    let mut rows_with_media = 0_i64;

    for row in rows {
        let mut row_has_publication = false;
        let mut candidates: Vec<String> = Vec::new();

        if let Some(value) = field(
            row,
            &[
                "Link posting media sosial atau berita online",
                "Link posting media sosial",
                "Link berita online",
            ],
        ) {
            candidates.extend(extract_links(&value));
        }
        candidates.extend(row.links.iter().cloned());
        if !is_noise(
            &row.fields
                .get("Dokumentasi kegiatan")
                .cloned()
                .unwrap_or_default(),
        ) {
            // Dokumentasi tetap dihitung terpisah, bukan sebagai publikasi.
        }

        for link in candidates {
            let lower = link.to_ascii_lowercase();
            if lower.contains("nihil") {
                continue;
            }
            if is_documentation_link(&lower) {
                documentation.insert(link);
                continue;
            }
            if publications.insert(link.clone()) {
                let channel = channel_of(&lower);
                channels.add(&channel);
                activities.add(&activity_label(row));
                domains.add(&host_of(&lower));
            }
            row_has_publication = true;
        }
        if row_has_publication {
            rows_with_media += 1;
        }
    }

    let publication_count = publications.len() as i64;
    let documentation_count = documentation.len() as i64;

    MediaMetrics {
        publications: publication_count,
        documentation: documentation_count,
        total: publication_count + documentation_count,
        rows_with_media,
        by_channel: channels.into_vec(8),
        by_activity: activities.into_vec(8),
        top_domains: domains.into_vec(10),
    }
}

fn count_by<F>(rows: &[&MetricRow], label: F, limit: usize) -> Vec<LabeledCount>
where
    F: Fn(&MetricRow) -> String,
{
    let mut counter = Counter::default();
    for row in rows {
        counter.add(&label(row));
    }
    counter.into_vec(limit)
}

fn group_metrics(
    rows: &[&MetricRow],
    people_keys: &[&str],
    bibit_keys: &[&str],
) -> Vec<GroupMetrics> {
    let mut groups: BTreeMap<String, (String, GroupMetrics, BTreeSet<String>)> = BTreeMap::new();

    for row in rows {
        let label = group_label(row);
        let key = label.to_ascii_lowercase();
        let entry = groups.entry(key).or_insert_with(|| {
            (
                label.clone(),
                GroupMetrics {
                    label,
                    ..GroupMetrics::default()
                },
                BTreeSet::new(),
            )
        });
        entry.1.rows += 1;
        entry.1.people += number_for(row, people_keys);
        entry.1.bibit += number_for(row, bibit_keys);
        let location = row_location(row);
        if !is_noise(&location) {
            entry.2.insert(location.to_ascii_lowercase());
        }
    }

    let mut result: Vec<GroupMetrics> = groups
        .into_values()
        .map(|(_, mut metrics, locations)| {
            metrics.locations = locations.len() as i64;
            metrics
        })
        .collect();
    result.sort_by(|left, right| {
        right
            .rows
            .cmp(&left.rows)
            .then_with(|| right.people.cmp(&left.people))
            .then_with(|| left.label.cmp(&right.label))
    });
    result
}

fn unique_count<F>(rows: &[&MetricRow], label: F) -> i64
where
    F: Fn(&MetricRow) -> String,
{
    let mut values = BTreeSet::new();
    for row in rows {
        let value = label(row);
        if !is_noise(&value) {
            values.insert(value.to_ascii_lowercase());
        }
    }
    values.len() as i64
}

fn daily_series(start: &str, end: &str, rows: &[&MetricRow]) -> Vec<DailyCount> {
    let (Ok(start_date), Ok(end_date)) = (
        NaiveDate::parse_from_str(start, "%Y-%m-%d"),
        NaiveDate::parse_from_str(end, "%Y-%m-%d"),
    ) else {
        return Vec::new();
    };

    let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
    for row in rows {
        *counts.entry(row.tanggal.as_str()).or_default() += 1;
    }

    let mut output = Vec::new();
    let mut current = start_date;
    while current <= end_date {
        let key = current.format("%Y-%m-%d").to_string();
        output.push(DailyCount {
            count: counts.get(key.as_str()).copied().unwrap_or(0),
            date: key,
        });
        current += Duration::days(1);
    }
    output
}

fn sum_numbers(rows: &[&MetricRow], keys: &[&str]) -> i64 {
    rows.iter().map(|row| number_for(row, keys)).sum()
}

fn number_for(row: &MetricRow, keys: &[&str]) -> i64 {
    for key in keys {
        if let Some(value) = row.fields.get(*key) {
            if let Some(number) = parse_count(value) {
                return number;
            }
        }
    }
    0
}

fn field(row: &MetricRow, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = row.fields.get(*key) {
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn row_location(row: &MetricRow) -> String {
    field(
        row,
        &[
            "Lokasi kegiatan",
            "Nama lokasi dan titik koordinat",
            "Nama masjid dan titik koordinat",
            "Nama tempat ibadah dan titik koordinat",
            "Nama sekolah dan titik koordinat",
            "Rute patroli",
        ],
    )
    .unwrap_or_default()
}

fn row_summary(row: &MetricRow) -> RowSummary {
    let detail = if number_for(
        row,
        &[
            "Orang yang ditangkap (Jumlah)",
            "Jumlah orang yang ditangkap (Jumlah)",
        ],
    ) > 0
    {
        format!(
            "{} orang ditangkap",
            format_count(number_for(
                row,
                &[
                    "Orang yang ditangkap (Jumlah)",
                    "Jumlah orang yang ditangkap (Jumlah)",
                ],
            ))
        )
    } else if number_for(row, &["Jumlah orang yang diamankan (Jumlah)"]) > 0 {
        format!(
            "{} orang diamankan",
            format_count(number_for(row, &["Jumlah orang yang diamankan (Jumlah)"]))
        )
    } else if number_for(row, &["Jumlah kawasan narkoba (Jumlah)"]) > 0 {
        format!(
            "{} kawasan",
            format_count(number_for(row, &["Jumlah kawasan narkoba (Jumlah)"]))
        )
    } else if number_for(row, &["Jumlah peserta penyuluhan narkoba"]) > 0 {
        format!(
            "{} peserta penyuluhan",
            format_count(number_for(row, &["Jumlah peserta penyuluhan narkoba"]))
        )
    } else if number_for(
        row,
        &["Pertimbangan pilihan lokasi (Perkiraan jumlah jemaah)"],
    ) > 0
    {
        format!(
            "{} jemaah",
            format_count(number_for(
                row,
                &["Pertimbangan pilihan lokasi (Perkiraan jumlah jemaah)"]
            ))
        )
    } else if number_for(
        row,
        &["Pertimbangan pilihan lokasi (Perkiraan jumlah jemaat atau umat)"],
    ) > 0
    {
        format!(
            "{} jemaat",
            format_count(number_for(
                row,
                &["Pertimbangan pilihan lokasi (Perkiraan jumlah jemaat atau umat)"]
            ))
        )
    } else if number_for(row, &["Jumlah bibit pohon yang ditanam"]) > 0 {
        format!(
            "{} bibit",
            format_count(number_for(row, &["Jumlah bibit pohon yang ditanam"]))
        )
    } else {
        field(
            row,
            &[
                "Tindak lanjut pendapat/masukan masyarakat (Bentuk tindak lanjut)",
                "Inovasi kegiatan",
            ],
        )
        .unwrap_or_default()
    };

    RowSummary {
        source_row: row.source_row,
        tanggal: row.tanggal.clone(),
        satuan: row.satuan.clone(),
        polres: row.polres.clone(),
        kode: row.kode.clone(),
        kegiatan: row.kegiatan.clone(),
        location: row_location(row),
        detail,
    }
}

fn group_label(row: &MetricRow) -> String {
    if !is_noise(&row.polres) {
        row.polres.clone()
    } else if !is_noise(&row.satuan) {
        row.satuan.clone()
    } else if !is_noise(&row.polda) {
        row.polda.clone()
    } else {
        "Satuan lainnya".to_string()
    }
}

fn quick_win_of(row: &MetricRow) -> String {
    if !row.quick_wins.trim().is_empty() {
        return row.quick_wins.trim().to_uppercase();
    }
    row.kode
        .split_once('.')
        .map(|(prefix, _)| prefix.trim().to_uppercase())
        .unwrap_or_default()
}

fn activity_label(row: &MetricRow) -> String {
    if row.kode.starts_with("QW1.5") {
        "Green Policing".into()
    } else if row.kode.starts_with("QW1.6") {
        "Jumat Keliling".into()
    } else if row.kode.starts_with("QW1.7") {
        "Subuh Keliling".into()
    } else if row.kode.starts_with("QW1.8") {
        "Minggu Kasih".into()
    } else if row.kode.starts_with("QW2") {
        "Berantas Narkoba".into()
    } else if row.kode.starts_with("QW3") {
        "Perang Digital".into()
    } else if row.kode.starts_with("QW4") {
        "Pamapta Hadir".into()
    } else if row.kode.starts_with("QW5") {
        "Pelayanan Reskrim".into()
    } else if row.kode.starts_with("QW6") {
        "Propam Presisi".into()
    } else if row.kode.starts_with("QW7") {
        "Bhabinkamtibmas & Polsek".into()
    } else if row.kode.starts_with("QW8") {
        "Kejahatan Ekonomi".into()
    } else if row.kode.starts_with("QW1") {
        "Kegiatan rutin QW1".into()
    } else {
        "Kegiatan lainnya".into()
    }
}

fn host_of(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_documentation_link(lower_url: &str) -> bool {
    lower_url.starts_with("/uploads/")
        || lower_url.contains("drive.google.")
        || lower_url.contains("docs.google.")
        || lower_url.contains("1drv.ms")
        || lower_url.contains("dropbox.")
        || lower_url.contains("mega.nz")
}

fn channel_of(lower_url: &str) -> String {
    if lower_url.contains("instagram")
        || lower_url.contains("facebook")
        || lower_url.contains("tiktok")
        || lower_url.contains("x.com")
        || lower_url.contains("twitter")
        || lower_url.contains("youtube")
    {
        "Media sosial".into()
    } else if lower_url.contains("tribratanews") {
        "Kanal humas Polri".into()
    } else {
        "Media online".into()
    }
}

#[derive(Default)]
struct Counter {
    entries: BTreeMap<String, (String, i64)>,
}

impl Counter {
    fn add(&mut self, label: &str) {
        let display = label.trim();
        if is_noise(display) {
            return;
        }
        let key = display.to_ascii_lowercase();
        self.entries
            .entry(key)
            .and_modify(|(_, count)| *count += 1)
            .or_insert_with(|| (display.to_string(), 1));
    }

    fn into_vec(self, limit: usize) -> Vec<LabeledCount> {
        let mut items: Vec<LabeledCount> = self
            .entries
            .into_values()
            .map(|(label, count)| LabeledCount::new(label, count))
            .collect();
        items.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.label.cmp(&right.label))
        });
        if limit > 0 && items.len() > limit {
            items.truncate(limit);
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::{channel_of, host_of, is_documentation_link};

    #[test]
    fn classifies_links() {
        assert_eq!(channel_of("https://instagram.com/p/abc"), "Media sosial");
        assert_eq!(
            channel_of("https://tribratanews.polri.go.id/berita"),
            "Kanal humas Polri"
        );
        assert!(is_documentation_link("https://drive.google.com/file/d/abc"));
        assert!(!is_documentation_link("https://detik.com/berita"));
        assert_eq!(
            host_of("https://www.instagram.com/p/abc"),
            "www.instagram.com"
        );
    }

    #[tokio::test]
    async fn aggregates_sample_workbook_into_report_metrics() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = crate::config::Config::test_config();
        config.data_dir = dir.path().to_path_buf();
        let db = crate::db::connect(&config).await.unwrap();

        sqlx::query(
            "INSERT INTO users(id,username,password_hash,role) VALUES('u1','tester','x','admin')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO report_periods(id,start_date,end_date,label,created_by)
             VALUES('p1','2026-10-15','2026-10-19','15–19 Oktober 2026','u1')",
        )
        .execute(&db)
        .await
        .unwrap();

        let bytes = std::fs::read("../report-2026-10-19 (1).xlsx").unwrap();
        let import = crate::modules::imports::service::commit(
            &db,
            &config,
            "p1",
            "u1",
            "report-2026-10-19.xlsx",
            bytes,
        )
        .await
        .unwrap();

        let metrics = super::load(
            &db,
            &import.id,
            "report-2026-10-19.xlsx",
            "15–19 Oktober 2026",
            "2026-10-15",
            "2026-10-19",
            "POLDA BALI",
        )
        .await
        .unwrap();

        assert_eq!(metrics.total_rows, 4600);
        assert_eq!(metrics.narkoba.orang_diamankan, 37);
        assert_eq!(metrics.narkoba.kawasan, 7);
        assert_eq!(metrics.green_policing.bibit, 854);
        assert_eq!(metrics.jumat_keliling.people, 22_686);
        assert_eq!(metrics.subuh_keliling.people, 5_069);
        assert_eq!(metrics.minggu_kasih.people, 7_198);
        assert!(metrics.media.publications > 0);
        assert!(!metrics.cards.is_empty());
    }
}
