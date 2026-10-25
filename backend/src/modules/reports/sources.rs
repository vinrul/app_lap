use crate::{
    modules::reports::{
        layout::DesignSource,
        models::{GroupMetrics, ReportMetrics},
    },
    shared::{LabeledCount, format_count},
};

const MAX_TABLE_ROWS: usize = 30;
const MAX_CHART_POINTS: usize = 20;

/// Katalog sumber data yang dapat diikatkan ke elemen tabel, grafik, atau kartu
/// pada desain. Setiap sumber sudah siap dipakai sebagai tabel (baris pertama =
/// header), grafik (label + nilai), atau kartu (label/nilai/satuan/keterangan).
pub fn design_sources(metrics: &ReportMetrics) -> Vec<DesignSource> {
    let mut sources = vec![
        cards_source(metrics),
        labeled_source(
            "quick_wins",
            "Quick Wins",
            "Ringkasan",
            "Jumlah Quick Wins",
            &metrics.quick_wins,
        ),
        labeled_source(
            "codes",
            "Kode kegiatan",
            "Ringkasan",
            "Kode kegiatan",
            &metrics.codes,
        ),
        labeled_source(
            "statuses",
            "Status kegiatan",
            "Ringkasan",
            "Status kegiatan",
            &metrics.statuses,
        ),
        daily_source(metrics),
        labeled_source(
            "by_polres",
            "Kegiatan per polres",
            "Ringkasan",
            "Kegiatan per polres",
            &metrics.by_polres,
        ),
        labeled_source(
            "by_level",
            "Tingkat pelaksanaan",
            "Ringkasan",
            "Tingkat pelaksanaan",
            &metrics.by_level,
        ),
        highlights_source(metrics),
        labeled_source(
            "narkoba.by_code",
            "Narkoba per kode",
            "Narkoba",
            "Narkoba per kode kegiatan",
            &metrics.narkoba.by_code,
        ),
        group_source(
            "narkoba.by_polres",
            "Narkoba per polres",
            "Narkoba",
            "Polres",
            "Narkoba per polres",
            &metrics.narkoba.by_polres,
            &[
                ("Laporan", GroupField::Rows),
                ("Personel", GroupField::People),
                ("Lokasi", GroupField::Locations),
            ],
        ),
        labeled_source(
            "narkoba.top_locations",
            "Lokasi narkoba terbanyak",
            "Narkoba",
            "Lokasi narkoba terbanyak",
            &metrics.narkoba.top_locations,
        ),
        group_source(
            "green.by_polres",
            "Green Policing per polres",
            "Green Policing",
            "Polres",
            "Green Policing per polres",
            &metrics.green_policing.by_polres,
            &[
                ("Laporan", GroupField::Rows),
                ("Bibit", GroupField::Bibit),
                ("Luas (m²)", GroupField::Area),
            ],
        ),
        labeled_source(
            "green.by_kegiatan",
            "Green Policing per kegiatan",
            "Green Policing",
            "Green Policing per kegiatan",
            &metrics.green_policing.by_kegiatan,
        ),
        group_source(
            "jumat.by_polres",
            "Jumat Keliling per polres",
            "Jumat Keliling",
            "Polres",
            "Jumat Keliling per polres",
            &metrics.jumat_keliling.by_polres,
            &[
                ("Laporan", GroupField::Rows),
                ("Personel", GroupField::People),
                ("Lokasi", GroupField::Locations),
            ],
        ),
        labeled_source(
            "jumat.by_religion",
            "Jumat Keliling per agama",
            "Jumat Keliling",
            "Jumat Keliling per agama",
            &metrics.jumat_keliling.by_religion,
        ),
        group_source(
            "subuh.by_polres",
            "Subuh Keliling per polres",
            "Subuh Keliling",
            "Polres",
            "Subuh Keliling per polres",
            &metrics.subuh_keliling.by_polres,
            &[
                ("Laporan", GroupField::Rows),
                ("Personel", GroupField::People),
                ("Lokasi", GroupField::Locations),
            ],
        ),
        labeled_source(
            "subuh.by_religion",
            "Subuh Keliling per agama",
            "Subuh Keliling",
            "Subuh Keliling per agama",
            &metrics.subuh_keliling.by_religion,
        ),
        group_source(
            "minggu.by_polres",
            "Minggu Kasih per polres",
            "Minggu Kasih",
            "Polres",
            "Minggu Kasih per polres",
            &metrics.minggu_kasih.by_polres,
            &[
                ("Laporan", GroupField::Rows),
                ("Personel", GroupField::People),
                ("Lokasi", GroupField::Locations),
            ],
        ),
        labeled_source(
            "minggu.by_religion",
            "Minggu Kasih per agama",
            "Minggu Kasih",
            "Minggu Kasih per agama",
            &metrics.minggu_kasih.by_religion,
        ),
        labeled_source(
            "aspirations.themes",
            "Aspirasi per tema",
            "Aspirasi",
            "Aspirasi per tema",
            &metrics.aspirations.themes,
        ),
        labeled_source(
            "aspirations.statuses",
            "Aspirasi per status",
            "Aspirasi",
            "Aspirasi per status",
            &metrics.aspirations.statuses,
        ),
        labeled_source(
            "aspirations.by_source",
            "Aspirasi per sumber",
            "Aspirasi",
            "Aspirasi per sumber",
            &metrics.aspirations.by_source,
        ),
        labeled_source(
            "follow_up.forms",
            "Tindak lanjut per bentuk",
            "Tindak Lanjut",
            "Tindak lanjut per bentuk",
            &metrics.follow_up.forms,
        ),
        labeled_source(
            "media.by_channel",
            "Media per kanal",
            "Media",
            "Publikasi media per kanal",
            &metrics.media.by_channel,
        ),
        labeled_source(
            "media.by_activity",
            "Media per aktivitas",
            "Media",
            "Publikasi media per aktivitas",
            &metrics.media.by_activity,
        ),
        labeled_source(
            "media.top_domains",
            "Domain media terbanyak",
            "Media",
            "Domain media terbanyak",
            &metrics.media.top_domains,
        ),
    ];
    sources.splice(1..1, summary_card_sources(metrics));
    sources.extend(analysis_card_sources(metrics));
    sources
}

fn cards_source(metrics: &ReportMetrics) -> DesignSource {
    let columns = vec![
        "Indikator".to_string(),
        "Nilai".to_string(),
        "Satuan".to_string(),
        "Keterangan".to_string(),
    ];
    let mut rows = vec![columns.clone()];
    for card in metrics.cards.iter().take(MAX_TABLE_ROWS - 1) {
        rows.push(vec![
            card.label.clone(),
            card.value.to_string(),
            card.unit.clone(),
            card.detail.clone(),
        ]);
    }
    DesignSource {
        key: "cards".to_string(),
        label: "Indikator utama".to_string(),
        category: "Ringkasan".to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: Vec::new(),
        chart_values: Vec::new(),
        chart_title: String::new(),
        can_table: true,
        can_chart: false,
        can_card: false,
        card_label: String::new(),
        card_value: String::new(),
        card_unit: String::new(),
        card_detail: String::new(),
        card_icon: String::new(),
    }
}

fn summary_card_sources(metrics: &ReportMetrics) -> Vec<DesignSource> {
    metrics
        .cards
        .iter()
        .map(|card| {
            card_source(
                &format!("card.{}", card.key),
                &format!("Kartu · {}", card.label),
                "Kartu Ringkasan",
                card.label.clone(),
                format_count(card.value),
                card.unit.clone(),
                card.detail.clone(),
                summary_card_icon(&card.key),
            )
        })
        .collect()
}

fn summary_card_icon(key: &str) -> &'static str {
    match key {
        "narkoba" => "shield-alert",
        "green_policing" => "sprout",
        "jumat_keliling" | "subuh_keliling" => "users",
        "minggu_kasih" => "heart-handshake",
        "media" => "megaphone",
        _ => "chart-column",
    }
}

fn analysis_card_sources(metrics: &ReportMetrics) -> Vec<DesignSource> {
    vec![
        card_source(
            "analysis.rows",
            "Kartu · Baris unik",
            "Kartu Analisis",
            "Baris unik",
            format_count(metrics.total_rows),
            "baris".to_string(),
            format!(
                "{} berkas · {} duplikat",
                format_count(metrics.source_count),
                format_count(metrics.duplicate_rows)
            ),
            "clipboard-list",
        ),
        card_source(
            "analysis.personel",
            "Kartu · Personel",
            "Kartu Analisis",
            "Personel",
            format_count(metrics.personel_total),
            "personel".to_string(),
            "total personel tercatat".to_string(),
            "users",
        ),
        card_source(
            "analysis.bibit",
            "Kartu · Bibit ditanam",
            "Kartu Analisis",
            "Bibit ditanam",
            format_count(metrics.bibit_total),
            "bibit".to_string(),
            "dari kolom isian".to_string(),
            "sprout",
        ),
        card_source(
            "analysis.area",
            "Kartu · Luas area",
            "Kartu Analisis",
            "Luas area",
            format!("{:.2}", metrics.area_total),
            "m²".to_string(),
            "area yang ditangani".to_string(),
            "map",
        ),
        card_source(
            "analysis.jamaah",
            "Kartu · Jemaah",
            "Kartu Analisis",
            "Jemaah",
            format_count(metrics.jamaah_total),
            "jemaah".to_string(),
            "jemaah / jemaat / umat".to_string(),
            "users-round",
        ),
        card_source(
            "analysis.coordinates",
            "Kartu · Titik koordinat",
            "Kartu Analisis",
            "Titik koordinat",
            format_count(metrics.rows_with_coordinates),
            "baris".to_string(),
            "baris dengan lat/long".to_string(),
            "map-pin",
        ),
        card_source(
            "analysis.evidence",
            "Kartu · Tautan bukti",
            "Kartu Analisis",
            "Tautan bukti",
            format_count(metrics.evidence_links),
            "tautan".to_string(),
            "tautan pada baris sumber".to_string(),
            "link",
        ),
        card_source(
            "analysis.polres",
            "Kartu · Polres melapor",
            "Kartu Analisis",
            "Polres melapor",
            format_count(metrics.by_polres_breakdown.len() as i64),
            "polres".to_string(),
            "polres dengan kegiatan".to_string(),
            "building-2",
        ),
        card_source(
            "analysis.satker",
            "Kartu · Satker Polda",
            "Kartu Analisis",
            "Satker Polda",
            format_count(metrics.by_satker.len() as i64),
            "satker".to_string(),
            "satker/pejabat Polda pelaksana".to_string(),
            "landmark",
        ),
        card_source(
            "analysis.period",
            "Kartu · Periode",
            "Kartu Analisis",
            "Periode",
            short_date(&metrics.period_start),
            String::new(),
            format!("s.d. {}", short_date(&metrics.period_end)),
            "calendar",
        ),
    ]
}

fn card_source(
    key: &str,
    label: &str,
    category: &str,
    card_label: impl Into<String>,
    card_value: impl Into<String>,
    card_unit: impl Into<String>,
    card_detail: impl Into<String>,
    card_icon: impl Into<String>,
) -> DesignSource {
    DesignSource {
        key: key.to_string(),
        label: label.to_string(),
        category: category.to_string(),
        table_columns: Vec::new(),
        table_rows: Vec::new(),
        chart_labels: Vec::new(),
        chart_values: Vec::new(),
        chart_title: String::new(),
        can_table: false,
        can_chart: false,
        can_card: true,
        card_label: card_label.into(),
        card_value: card_value.into(),
        card_unit: card_unit.into(),
        card_detail: card_detail.into(),
        card_icon: card_icon.into(),
    }
}

fn labeled_source(
    key: &str,
    label: &str,
    category: &str,
    chart_title: &str,
    items: &[LabeledCount],
) -> DesignSource {
    let items = items.iter().take(MAX_TABLE_ROWS - 1).collect::<Vec<_>>();
    let columns = vec!["Kategori".to_string(), "Jumlah".to_string()];
    let mut rows = vec![columns.clone()];
    for item in &items {
        rows.push(vec![item.label.clone(), item.count.to_string()]);
    }
    let chart_items = items
        .iter()
        .take(MAX_CHART_POINTS)
        .copied()
        .collect::<Vec<_>>();
    DesignSource {
        key: key.to_string(),
        label: label.to_string(),
        category: category.to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: chart_items.iter().map(|item| item.label.clone()).collect(),
        chart_values: chart_items.iter().map(|item| item.count as f64).collect(),
        chart_title: chart_title.to_string(),
        can_table: true,
        can_chart: !chart_items.is_empty(),
        can_card: false,
        card_label: String::new(),
        card_value: String::new(),
        card_unit: String::new(),
        card_detail: String::new(),
        card_icon: String::new(),
    }
}

fn daily_source(metrics: &ReportMetrics) -> DesignSource {
    let items = metrics
        .daily
        .iter()
        .take(MAX_TABLE_ROWS - 1)
        .collect::<Vec<_>>();
    let columns = vec!["Tanggal".to_string(), "Jumlah".to_string()];
    let mut rows = vec![columns.clone()];
    for item in &items {
        rows.push(vec![item.date.clone(), item.count.to_string()]);
    }
    let chart_items = items
        .iter()
        .take(MAX_CHART_POINTS)
        .copied()
        .collect::<Vec<_>>();
    DesignSource {
        key: "daily".to_string(),
        label: "Kegiatan harian".to_string(),
        category: "Ringkasan".to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: chart_items
            .iter()
            .map(|item| short_date(&item.date))
            .collect(),
        chart_values: chart_items.iter().map(|item| item.count as f64).collect(),
        chart_title: "Kegiatan harian".to_string(),
        can_table: true,
        can_chart: !chart_items.is_empty(),
        can_card: false,
        card_label: String::new(),
        card_value: String::new(),
        card_unit: String::new(),
        card_detail: String::new(),
        card_icon: String::new(),
    }
}

fn highlights_source(metrics: &ReportMetrics) -> DesignSource {
    let columns = vec![
        "Tanggal".to_string(),
        "Polres".to_string(),
        "Kode".to_string(),
        "Kegiatan".to_string(),
        "Lokasi".to_string(),
    ];
    let mut rows = vec![columns.clone()];
    for item in metrics.highlights.iter().take(MAX_TABLE_ROWS - 1) {
        rows.push(vec![
            item.tanggal.clone(),
            item.polres.clone(),
            item.kode.clone(),
            item.kegiatan.clone(),
            item.location.clone(),
        ]);
    }
    DesignSource {
        key: "highlights".to_string(),
        label: "Kegiatan menonjol".to_string(),
        category: "Ringkasan".to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: Vec::new(),
        chart_values: Vec::new(),
        chart_title: String::new(),
        can_table: true,
        can_chart: false,
        can_card: false,
        card_label: String::new(),
        card_value: String::new(),
        card_unit: String::new(),
        card_detail: String::new(),
        card_icon: String::new(),
    }
}

#[derive(Clone, Copy)]
enum GroupField {
    Rows,
    People,
    Locations,
    Bibit,
    Area,
}

impl GroupField {
    fn value(self, item: &GroupMetrics) -> i64 {
        match self {
            Self::Rows => item.rows,
            Self::People => item.people,
            Self::Locations => item.locations,
            Self::Bibit => item.bibit,
            Self::Area => item.area_m2,
        }
    }
}

fn group_source(
    key: &str,
    label: &str,
    category: &str,
    name_header: &str,
    chart_title: &str,
    items: &[GroupMetrics],
    fields: &[(&str, GroupField)],
) -> DesignSource {
    let mut columns = vec![name_header.to_string()];
    columns.extend(fields.iter().map(|(title, _)| (*title).to_string()));

    let items = items.iter().take(MAX_TABLE_ROWS - 1).collect::<Vec<_>>();
    let mut rows = vec![columns.clone()];
    for item in &items {
        let mut row = vec![item.label.clone()];
        row.extend(
            fields
                .iter()
                .map(|(_, field)| field.value(item).to_string()),
        );
        rows.push(row);
    }

    let chart_items = items
        .iter()
        .take(MAX_CHART_POINTS)
        .copied()
        .collect::<Vec<_>>();
    DesignSource {
        key: key.to_string(),
        label: label.to_string(),
        category: category.to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: chart_items.iter().map(|item| item.label.clone()).collect(),
        chart_values: chart_items.iter().map(|item| item.rows as f64).collect(),
        chart_title: chart_title.to_string(),
        can_table: true,
        can_chart: !chart_items.is_empty(),
        can_card: false,
        card_label: String::new(),
        card_value: String::new(),
        card_unit: String::new(),
        card_detail: String::new(),
        card_icon: String::new(),
    }
}

fn short_date(value: &str) -> String {
    let trimmed = value.trim();
    if let Some((_, rest)) = trimmed.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}-{}", day.trim_start_matches('0'), month);
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        modules::reports::models::{DailyCount, MetricCard, NarkobaMetrics, ReportMetrics},
        shared::LabeledCount,
    };

    fn sample_metrics() -> ReportMetrics {
        let mut metrics = ReportMetrics::default();
        metrics.cards = vec![MetricCard {
            key: "narkoba".into(),
            label: "Berantas Narkoba".into(),
            value: 37,
            unit: "orang".into(),
            detail: "89 kegiatan".into(),
        }];
        metrics.quick_wins = vec![
            LabeledCount::new("Gelar perkara", 12),
            LabeledCount::new("Patroli", 8),
        ];
        metrics.daily = vec![
            DailyCount {
                date: "2026-10-16".into(),
                count: 4,
            },
            DailyCount {
                date: "2026-10-17".into(),
                count: 6,
            },
        ];
        metrics.narkoba = NarkobaMetrics {
            by_code: vec![LabeledCount::new("QW5.3", 5)],
            ..NarkobaMetrics::default()
        };
        metrics
    }

    #[test]
    fn catalog_exposes_expected_keys() {
        let sources = design_sources(&sample_metrics());
        let keys = sources
            .iter()
            .map(|source| source.key.as_str())
            .collect::<Vec<_>>();
        for expected in [
            "cards",
            "card.narkoba",
            "analysis.rows",
            "analysis.personel",
            "quick_wins",
            "daily",
            "by_polres",
            "narkoba.by_code",
            "narkoba.by_polres",
            "media.by_channel",
        ] {
            assert!(keys.contains(&expected), "missing source {expected}");
        }
        assert!(sources.iter().all(|source| !source.label.is_empty()));
        assert!(sources.iter().all(|source| !source.category.is_empty()));
        assert!(
            sources
                .iter()
                .filter(|source| source.can_table)
                .all(|source| !source.table_rows.is_empty())
        );
        assert!(
            sources
                .iter()
                .filter(|source| source.can_table)
                .all(|source| source.table_rows[0] == source.table_columns)
        );
    }

    #[test]
    fn labeled_sources_build_tables_and_charts() {
        let sources = design_sources(&sample_metrics());
        let quick_wins = sources
            .iter()
            .find(|source| source.key == "quick_wins")
            .unwrap();
        assert_eq!(quick_wins.table_rows[0], vec!["Kategori", "Jumlah"]);
        assert_eq!(quick_wins.table_rows[1], vec!["Gelar perkara", "12"]);
        assert_eq!(quick_wins.chart_labels, vec!["Gelar perkara", "Patroli"]);
        assert_eq!(quick_wins.chart_values, vec![12.0, 8.0]);
        assert_eq!(quick_wins.chart_title, "Jumlah Quick Wins");
        assert!(quick_wins.can_chart);

        let cards = sources.iter().find(|source| source.key == "cards").unwrap();
        assert!(cards.can_table);
        assert!(!cards.can_chart);
        assert_eq!(cards.table_rows[1][1], "37");

        let card = sources
            .iter()
            .find(|source| source.key == "card.narkoba")
            .unwrap();
        assert!(card.can_card);
        assert!(!card.can_table);
        assert_eq!(card.card_label, "Berantas Narkoba");
        assert_eq!(card.card_value, "37");
        assert_eq!(card.card_unit, "orang");
        assert_eq!(card.card_icon, "shield-alert");

        let analysis = sources
            .iter()
            .find(|source| source.key == "analysis.rows")
            .unwrap();
        assert!(analysis.can_card);
        assert_eq!(analysis.card_label, "Baris unik");
    }

    #[test]
    fn daily_source_uses_short_chart_labels() {
        let sources = design_sources(&sample_metrics());
        let daily = sources.iter().find(|source| source.key == "daily").unwrap();
        assert_eq!(daily.table_rows[1], vec!["2026-10-16", "4"]);
        assert_eq!(daily.chart_labels, vec!["16-10", "17-10"]);
        assert_eq!(daily.chart_values, vec![4.0, 6.0]);
    }
}
