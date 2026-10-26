use crate::{
    modules::{
        imports::{
            models::{CoverageProgram, CoverageResponse},
            qw,
        },
        reports::{
            layout::DesignSource,
            models::{GroupMetrics, ReportMetrics},
        },
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
        described_labeled_source(
            "quick_wins",
            "Quick Wins",
            "Ringkasan",
            "Jumlah Quick Wins",
            &metrics.quick_wins,
            qw::group_title,
        ),
        described_labeled_source(
            "codes",
            "Kode kegiatan",
            "Ringkasan",
            "Kode kegiatan",
            &metrics.codes,
            qw::title,
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
    sources.extend(coverage_sources(metrics));
    sources.extend(coverage_card_sources(metrics));
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

fn described_labeled_source(
    key: &str,
    label: &str,
    category: &str,
    chart_title: &str,
    items: &[LabeledCount],
    describe: fn(&str) -> &'static str,
) -> DesignSource {
    let items = items.iter().take(MAX_TABLE_ROWS - 1).collect::<Vec<_>>();
    let columns = vec![
        "Kode".to_string(),
        "Deskripsi".to_string(),
        "Jumlah".to_string(),
    ];
    let mut rows = vec![columns.clone()];
    for item in &items {
        rows.push(vec![
            item.label.clone(),
            describe(&item.label).to_string(),
            item.count.to_string(),
        ]);
    }
    let chart_items = items
        .iter()
        .take(MAX_CHART_POINTS)
        .copied()
        .collect::<Vec<_>>();
    let chart_labels = chart_items
        .iter()
        .map(|item| {
            let description = describe(&item.label);
            if description.is_empty() {
                item.label.clone()
            } else {
                format!("{} · {}", item.label, description)
            }
        })
        .collect();
    DesignSource {
        key: key.to_string(),
        label: label.to_string(),
        category: category.to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels,
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

fn coverage_sources(metrics: &ReportMetrics) -> Vec<DesignSource> {
    let Some(coverage) = metrics.coverage.as_ref() else {
        return Vec::new();
    };

    let mut sources = vec![
        coverage_group_source(coverage),
        coverage_code_source(coverage),
    ];
    for program in &coverage.programs {
        sources.push(program_metric_source(program));
        sources.push(program_unit_source(program, true));
        sources.push(program_unit_source(program, false));
        sources.push(program_missing_source(program));
    }
    sources
}

fn coverage_group_source(coverage: &CoverageResponse) -> DesignSource {
    let columns = vec![
        "Kode".to_string(),
        "Deskripsi".to_string(),
        "Kegiatan".to_string(),
        "Polres".to_string(),
        "Satker".to_string(),
    ];
    let mut rows = vec![columns.clone()];
    for group in coverage.groups.iter().take(MAX_TABLE_ROWS - 1) {
        rows.push(vec![
            group.code.clone(),
            group.title.clone(),
            group.rows.to_string(),
            group.polres.to_string(),
            group.satker.to_string(),
        ]);
    }
    DesignSource {
        key: "coverage.groups".to_string(),
        label: "Cakupan kelompok Quick Wins".to_string(),
        category: "Cakupan".to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: coverage
            .groups
            .iter()
            .map(|group| format!("{} · {}", group.code, group.title))
            .collect(),
        chart_values: coverage
            .groups
            .iter()
            .map(|group| group.rows as f64)
            .collect(),
        chart_title: "Kegiatan per kelompok Quick Wins".to_string(),
        can_table: true,
        can_chart: !coverage.groups.is_empty(),
        can_card: false,
        card_label: String::new(),
        card_value: String::new(),
        card_unit: String::new(),
        card_detail: String::new(),
        card_icon: String::new(),
    }
}

fn coverage_code_source(coverage: &CoverageResponse) -> DesignSource {
    let columns = vec![
        "Kode".to_string(),
        "Deskripsi".to_string(),
        "Kegiatan".to_string(),
        "Polres".to_string(),
        "Satker".to_string(),
    ];
    let mut codes = coverage
        .groups
        .iter()
        .flat_map(|group| group.codes.iter())
        .collect::<Vec<_>>();
    codes.sort_by(|left, right| {
        right
            .rows
            .cmp(&left.rows)
            .then(right.polres.cmp(&left.polres))
            .then(left.code.cmp(&right.code))
    });

    let mut rows = vec![columns.clone()];
    for code in codes.iter().take(MAX_TABLE_ROWS - 1) {
        rows.push(vec![
            code.code.clone(),
            code.title.clone(),
            code.rows.to_string(),
            code.polres.to_string(),
            code.satker.to_string(),
        ]);
    }

    let chart_items = codes
        .iter()
        .filter(|code| code.rows > 0)
        .take(MAX_CHART_POINTS)
        .collect::<Vec<_>>();

    DesignSource {
        key: "coverage.codes".to_string(),
        label: "Cakupan sub-kode".to_string(),
        category: "Cakupan".to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: chart_items
            .iter()
            .map(|code| format!("{} · {}", code.code, code.title))
            .collect(),
        chart_values: chart_items.iter().map(|code| code.rows as f64).collect(),
        chart_title: "Kegiatan per sub-kode".to_string(),
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

fn program_metric_source(program: &CoverageProgram) -> DesignSource {
    let columns = vec![
        "Indikator".to_string(),
        "Total".to_string(),
        "Satuan".to_string(),
        "Baris terisi".to_string(),
        "Baris kosong".to_string(),
        "Polres terisi".to_string(),
        "Satker terisi".to_string(),
    ];
    let mut rows = vec![columns.clone()];
    for metric in &program.metrics {
        rows.push(vec![
            metric.label.clone(),
            coverage_value(metric.total, &metric.unit),
            metric.unit.clone(),
            metric.filled_rows.to_string(),
            metric.missing_rows.to_string(),
            metric.filled_polres.to_string(),
            metric.filled_satker.to_string(),
        ]);
    }
    let chart_items = program.metrics.iter().collect::<Vec<_>>();
    DesignSource {
        key: format!("coverage.{}.metrics", program.key),
        label: format!("{} · indikator isian", program.title),
        category: "Cakupan".to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: chart_items
            .iter()
            .map(|metric| metric.label.clone())
            .collect(),
        chart_values: chart_items.iter().map(|metric| metric.total).collect(),
        chart_title: format!("{} · indikator isian", program.title),
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

fn program_unit_source(program: &CoverageProgram, by_polres: bool) -> DesignSource {
    let units = if by_polres {
        &program.by_polres
    } else {
        &program.by_satker
    };
    let scope = if by_polres { "Polres" } else { "Satker" };
    let mut columns = vec![scope.to_string(), "Kegiatan".to_string()];
    columns.extend(program.metrics.iter().map(|metric| {
        if metric.unit.is_empty() {
            metric.label.clone()
        } else {
            format!("{} ({})", metric.label, metric.unit)
        }
    }));

    let mut rows = vec![columns.clone()];
    for unit in units.iter().take(MAX_TABLE_ROWS - 1) {
        let mut row = vec![unit.label.clone(), unit.rows.to_string()];
        row.extend(unit.values.iter().enumerate().map(|(index, value)| {
            let label = program
                .metrics
                .get(index)
                .map(|metric| metric.unit.as_str())
                .unwrap_or("");
            coverage_value(value.value, label)
        }));
        rows.push(row);
    }

    let chart_items = units.iter().take(MAX_CHART_POINTS).collect::<Vec<_>>();
    let scope_label = if by_polres { "polres" } else { "satker" };
    DesignSource {
        key: format!("coverage.{}.by_{}", program.key, scope_label),
        label: format!("{} · per {}", program.title, scope_label),
        category: "Cakupan".to_string(),
        table_columns: columns,
        table_rows: rows,
        chart_labels: chart_items.iter().map(|unit| unit.label.clone()).collect(),
        chart_values: chart_items.iter().map(|unit| unit.rows as f64).collect(),
        chart_title: format!("{} · per {}", program.title, scope_label),
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

fn program_missing_source(program: &CoverageProgram) -> DesignSource {
    let columns = vec![
        "Cakupan".to_string(),
        "Jumlah".to_string(),
        "Daftar".to_string(),
    ];
    let mut rows = vec![columns.clone()];
    for (label, missing) in [
        ("Polres", &program.polres_missing),
        ("Satker Polda", &program.satker_missing),
    ] {
        rows.push(vec![
            label.to_string(),
            missing.len().to_string(),
            if missing.is_empty() {
                "—".to_string()
            } else {
                missing.join(", ")
            },
        ]);
    }
    DesignSource {
        key: format!("coverage.{}.missing", program.key),
        label: format!("{} · unit belum mengisi", program.title),
        category: "Cakupan".to_string(),
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

fn coverage_card_sources(metrics: &ReportMetrics) -> Vec<DesignSource> {
    let Some(coverage) = metrics.coverage.as_ref() else {
        return Vec::new();
    };
    let groups_done = coverage
        .groups
        .iter()
        .filter(|group| group.rows > 0)
        .count();
    let codes_done = coverage
        .groups
        .iter()
        .flat_map(|group| group.codes.iter())
        .filter(|code| code.rows > 0)
        .count();
    let codes_total: usize = coverage.groups.iter().map(|group| group.codes.len()).sum();

    let mut cards = vec![
        card_source(
            "card.coverage.polres",
            "Kartu Cakupan · Polres aktif",
            "Kartu Cakupan",
            "Polres aktif",
            format_count(coverage.active_polres),
            "polres",
            format!("dari {} polres", coverage.known_polres.len()),
            "building-2",
        ),
        card_source(
            "card.coverage.satker",
            "Kartu Cakupan · Satker Polda aktif",
            "Kartu Cakupan",
            "Satker Polda aktif",
            format_count(coverage.active_satker),
            "satker",
            format!("dari {} satker", coverage.known_satker.len()),
            "landmark",
        ),
        card_source(
            "card.coverage.groups",
            "Kartu Cakupan · Kelompok terlaksana",
            "Kartu Cakupan",
            "Kelompok terlaksana",
            format_count(groups_done as i64),
            "kelompok",
            format!("dari {} kelompok", coverage.groups.len()),
            "target",
        ),
        card_source(
            "card.coverage.codes",
            "Kartu Cakupan · Sub-kode terlaksana",
            "Kartu Cakupan",
            "Sub-kode terlaksana",
            format_count(codes_done as i64),
            "sub-kode",
            format!("dari {} sub-kode", codes_total),
            "check",
        ),
        card_source(
            "card.coverage.rows",
            "Kartu Cakupan · Kegiatan tercakup",
            "Kartu Cakupan",
            "Kegiatan tercakup",
            format_count(coverage.total_rows),
            "kegiatan",
            format!(
                "{} polres · {} satker",
                coverage.active_polres, coverage.active_satker
            ),
            "clipboard-list",
        ),
    ];

    for group in &coverage.groups {
        cards.push(card_source(
            &format!("card.coverage.group.{}", group.code),
            &format!("Kartu Cakupan · {} {}", group.code, group.title),
            "Kartu Cakupan",
            group.title.clone(),
            format_count(group.rows),
            "kegiatan",
            format!("{} polres · {} satker", group.polres, group.satker),
            coverage_group_icon(&group.code),
        ));
    }

    for program in &coverage.programs {
        cards.push(card_source(
            &format!("card.coverage.{}", program.key),
            &format!("Kartu Cakupan · {}", program.title),
            "Kartu Cakupan",
            program.title.clone(),
            format_count(program.rows),
            "kegiatan",
            format!(
                "{}/{} polres · {}/{} satker",
                program.polres, program.polres_total, program.satker, program.satker_total
            ),
            coverage_program_icon(&program.key),
        ));
        for metric in &program.metrics {
            cards.push(card_source(
                &format!("card.coverage.{}.{}", program.key, metric.key),
                &format!("Kartu Cakupan · {} · {}", program.title, metric.label),
                "Kartu Cakupan",
                format!("{} · {}", program.title, metric.label),
                coverage_value(metric.total, &metric.unit),
                metric.unit.clone(),
                format!(
                    "{}/{} baris terisi · {} polres · {} satker",
                    metric.filled_rows,
                    metric.filled_rows + metric.missing_rows,
                    metric.filled_polres,
                    metric.filled_satker
                ),
                coverage_program_icon(&program.key),
            ));
        }
    }

    cards
}

fn coverage_value(value: f64, unit: &str) -> String {
    if unit == "m²" {
        format!("{:.2}", value)
    } else if (value - value.round()).abs() < 0.005 {
        format_count(value.round() as i64)
    } else {
        format!("{:.2}", value)
    }
}

fn coverage_group_icon(code: &str) -> &'static str {
    match code {
        "QW1" => "shield",
        "QW2" => "shield-alert",
        "QW3" => "smartphone",
        "QW4" => "siren",
        "QW5" => "file-text",
        "QW6" => "lock",
        "QW7" => "building-2",
        "QW8" => "microscope",
        _ => "clipboard-list",
    }
}

fn coverage_program_icon(key: &str) -> &'static str {
    match key {
        "green_policing" => "sprout",
        "jumat_keliling" => "users",
        "subuh_keliling" => "sunrise",
        "minggu_kasih" => "heart-handshake",
        "narkoba" => "shield-alert",
        "media" => "megaphone",
        _ => "chart-column",
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
    use crate::modules::{
        imports::{coverage::build_coverage, models::RecordView},
        reports::models::{DailyCount, MetricCard, NarkobaMetrics, ReportMetrics},
    };
    use crate::shared::LabeledCount;

    fn sample_metrics() -> ReportMetrics {
        let mut metrics = ReportMetrics::default();
        metrics.cards = vec![MetricCard {
            key: "narkoba".into(),
            label: "Berantas Narkoba".into(),
            value: 37,
            unit: "orang".into(),
            detail: "89 kegiatan".into(),
        }];
        metrics.quick_wins = vec![LabeledCount::new("QW1", 12), LabeledCount::new("QW2", 8)];
        metrics.codes = vec![
            LabeledCount::new("QW1.10", 9),
            LabeledCount::new("QW2.1", 2),
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
        assert_eq!(
            quick_wins.table_rows[0],
            vec!["Kode", "Deskripsi", "Jumlah"]
        );
        assert_eq!(
            quick_wins.table_rows[1],
            vec!["QW1", "Ruang Publik Aman", "12"]
        );
        assert_eq!(
            quick_wins.chart_labels,
            vec!["QW1 · Ruang Publik Aman", "QW2 · Pemulihan Kawasan Narkoba"]
        );
        assert_eq!(quick_wins.chart_values, vec![12.0, 8.0]);
        assert_eq!(quick_wins.chart_title, "Jumlah Quick Wins");
        assert!(quick_wins.can_chart);

        let codes = sources.iter().find(|source| source.key == "codes").unwrap();
        assert_eq!(codes.table_rows[0], vec!["Kode", "Deskripsi", "Jumlah"]);
        assert_eq!(codes.table_rows[1], vec!["QW1.10", "Patroli Sabhara", "9"]);
        assert_eq!(codes.chart_labels[0], "QW1.10 · Patroli Sabhara");
        assert_eq!(
            codes.table_rows[2],
            vec!["QW2.1", "Pemberantasan dan pemulihan kawasan narkoba", "2"]
        );

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

    fn sample_coverage() -> CoverageResponse {
        build_coverage(
            "2026-10-15",
            "2026-10-23",
            &[
                sample_row("QW1.5", "Polres Tabanan", 5, 10, 100.0, 0),
                sample_row("QW1.7", "Polres Badung", 4, 0, 0.0, 50),
            ],
        )
    }

    fn sample_row(
        kode: &str,
        polres: &str,
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
            satuan: polres.into(),
            tingkat: "Polres".into(),
            polda: "Polda Bali".into(),
            polres: polres.into(),
            polsek: String::new(),
            level: "Polres".into(),
            unit: String::new(),
            kode: kode.into(),
            qw_title: qw::title(kode).into(),
            kegiatan: String::new(),
            quick_wins: qw::group_of(kode)
                .map(|group| group.code.to_string())
                .unwrap_or_default(),
            pejabat_pelaksana: String::new(),
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
            fields: Default::default(),
            links: Vec::new(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn coverage_sources_bind_tables_charts_and_cards() {
        let mut metrics = sample_metrics();
        metrics.coverage = Some(sample_coverage());
        let sources = design_sources(&metrics);

        let groups = sources
            .iter()
            .find(|source| source.key == "coverage.groups")
            .unwrap();
        assert!(groups.can_table);
        assert_eq!(
            groups.table_rows[0],
            vec!["Kode", "Deskripsi", "Kegiatan", "Polres", "Satker"]
        );
        let qw1 = groups
            .table_rows
            .iter()
            .find(|row| row[0] == "QW1")
            .unwrap();
        assert_eq!(qw1, &vec!["QW1", "Ruang Publik Aman", "2", "2", "0"]);
        assert_eq!(groups.chart_labels[0], "QW1 · Ruang Publik Aman");
        assert_eq!(groups.chart_values[0], 2.0);

        let green = sources
            .iter()
            .find(|source| source.key == "coverage.green_policing.by_polres")
            .unwrap();
        assert_eq!(
            green.table_rows[0],
            vec![
                "Polres",
                "Kegiatan",
                "Luas area (m²)",
                "Bibit ditanam (pohon)",
                "Personel (orang)",
                "Kerja bakti (kegiatan)",
                "Peserta (orang)"
            ]
        );
        assert_eq!(
            green.table_rows[1],
            vec!["Polres Tabanan", "1", "100.00", "10", "5", "0", "0"]
        );

        let indicators = sources
            .iter()
            .find(|source| source.key == "coverage.green_policing.metrics")
            .unwrap();
        assert_eq!(
            indicators.table_rows[0],
            vec![
                "Indikator",
                "Total",
                "Satuan",
                "Baris terisi",
                "Baris kosong",
                "Polres terisi",
                "Satker terisi"
            ]
        );
        assert_eq!(
            indicators.table_rows[1],
            vec!["Luas area", "100.00", "m²", "1", "0", "1", "0"]
        );

        let card = sources
            .iter()
            .find(|source| source.key == "card.coverage.polres")
            .unwrap();
        assert!(card.can_card);
        assert_eq!(card.card_label, "Polres aktif");
        assert_eq!(card.card_value, "2");
        assert_eq!(card.card_unit, "polres");
        assert_eq!(card.card_detail, "dari 2 polres");

        let group_card = sources
            .iter()
            .find(|source| source.key == "card.coverage.group.QW1")
            .unwrap();
        assert_eq!(group_card.card_value, "2");
        assert_eq!(group_card.card_detail, "2 polres · 0 satker");

        let area_card = sources
            .iter()
            .find(|source| source.key == "card.coverage.green_policing.area")
            .unwrap();
        assert_eq!(area_card.card_label, "Green Policing · Luas area");
        assert_eq!(area_card.card_value, "100.00");
        assert_eq!(area_card.card_unit, "m²");
        assert_eq!(
            area_card.card_detail,
            "1/1 baris terisi · 1 polres · 0 satker"
        );
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
