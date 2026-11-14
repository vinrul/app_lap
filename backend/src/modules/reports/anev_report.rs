//! PDF ringkasan hasil analisis dan evaluasi: duplikasi isian per satuan dan
//! tautan Google Drive yang dipakai berulang.

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::{
    branding,
    config::Config,
    error::ApiResult,
    modules::{
        auth::DataScope,
        imports::anev::{self, AnevIndicators, DriveGroup, DuplicateGroup},
        periods,
        reports::{
            bot_report::{escape, generated_label},
            fonts, service,
        },
    },
};

pub struct AnevReportMeta<'a> {
    pub polda_name: &'a str,
    pub period_label: &'a str,
    pub period_start: &'a str,
    pub period_end: &'a str,
    pub generated_at: &'a str,
    pub filter_polres: &'a str,
}

pub async fn generate_duplicate_pdf(
    db: &SqlitePool,
    config: &Config,
    pdf_lock: &Mutex<()>,
    period_id: &str,
    polres: Option<&str>,
    scope: &DataScope,
) -> ApiResult<(Vec<u8>, String)> {
    let (start, end, label) = periods::service::dates(db, period_id).await?;
    let indicators = anev::indicators_for_period(db, period_id, polres, scope).await?;
    let generated_at = chrono::Utc::now().to_rfc3339();
    let meta = AnevReportMeta {
        polda_name: &config.polda_name,
        period_label: &label,
        period_start: &start,
        period_end: &end,
        generated_at: &generated_at,
        filter_polres: polres.unwrap_or_default(),
    };
    let html = render_duplicate_report(config, &meta, &indicators);
    let bytes =
        service::html_to_pdf_bytes(config, pdf_lock, period_id, "anev-duplicate", html).await?;
    Ok((
        bytes,
        report_filename("pemeriksaan-duplikasi-isian", &label),
    ))
}

pub async fn generate_drive_pdf(
    db: &SqlitePool,
    config: &Config,
    pdf_lock: &Mutex<()>,
    period_id: &str,
    polres: Option<&str>,
    scope: &DataScope,
) -> ApiResult<(Vec<u8>, String)> {
    let (start, end, label) = periods::service::dates(db, period_id).await?;
    let indicators = anev::indicators_for_period(db, period_id, polres, scope).await?;
    let generated_at = chrono::Utc::now().to_rfc3339();
    let meta = AnevReportMeta {
        polda_name: &config.polda_name,
        period_label: &label,
        period_start: &start,
        period_end: &end,
        generated_at: &generated_at,
        filter_polres: polres.unwrap_or_default(),
    };
    let html = render_drive_report(config, &meta, &indicators);
    let bytes = service::html_to_pdf_bytes(config, pdf_lock, period_id, "anev-drive", html).await?;
    Ok((bytes, report_filename("pemeriksaan-tautan-drive", &label)))
}

pub fn render_duplicate_report(
    config: &Config,
    meta: &AnevReportMeta<'_>,
    indicators: &AnevIndicators,
) -> String {
    let units = &indicators.duplicate.units;
    let exact_rows: usize = units.iter().map(|unit| unit.exact_rows).sum();
    let fuzzy_rows: usize = units.iter().map(|unit| unit.fuzzy_rows).sum();
    let mut unit_rows = String::new();
    for unit in units.iter().filter(|unit| unit.rows > 0).take(80) {
        unit_rows.push_str(&format!(
            "<tr><td><strong>{satuan}</strong></td><td class=\"num\">{rows}</td>\
<td class=\"num\">{exact}</td><td class=\"num\">{exact_pct}%</td>\
<td class=\"num\">{fuzzy}</td><td class=\"num\">{fuzzy_pct}%</td></tr>",
            satuan = escape(&unit.satuan),
            rows = unit.rows,
            exact = unit.exact_rows,
            exact_pct = unit.exact_percent,
            fuzzy = unit.fuzzy_rows,
            fuzzy_pct = unit.fuzzy_percent,
        ));
    }
    let exact_groups = group_table(&indicators.duplicate.exact_groups, false);
    let fuzzy_groups = group_table(&indicators.duplicate.fuzzy_groups, true);
    let pending = pending_notice(&indicators.pending_imports);
    let filter = filter_chip(meta.filter_polres);

    let css = css(config);
    format!(
        "<!doctype html><html lang=\"id\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>Duplikasi Isian · {period}</title><style>{font_css}{css}</style></head><body>\
{header}\
<section class=\"cards\">\
<div class=\"card\"><span class=\"label\">Baris diperiksa</span><strong>{rows}</strong></div>\
<div class=\"card\"><span class=\"label\">Satuan diperiksa</span><strong>{units}</strong></div>\
<div class=\"card\"><span class=\"label\">Duplikat persis</span><strong>{exact}</strong></div>\
<div class=\"card\"><span class=\"label\">Mirip ≥75%</span><strong>{fuzzy}</strong></div>\
</section>\
<div class=\"note\">Duplikat persis memakai isian yang sama setelah normalisasi huruf, spasi, dan tautan. Kemiripan ≥75% memakai Jaccard shingle lima kata; angka mirip sudah mencakup duplikat persis. Klik data mentah di aplikasi untuk melihat baris lengkap.{filter}</div>\
{pending}\
<h2 class=\"section-title\">Ringkasan per satuan</h2>\
<table class=\"summary-table\">\
<colgroup><col style=\"width:40%\"><col style=\"width:12%\"><col style=\"width:12%\"><col style=\"width:12%\"><col style=\"width:12%\"><col style=\"width:12%\"></colgroup>\
<thead><tr><th>Satuan</th><th class=\"num\">Baris</th><th class=\"num\">Persis</th><th class=\"num\">%</th><th class=\"num\">Mirip</th><th class=\"num\">%</th></tr></thead>\
<tbody>{unit_rows}</tbody></table>\
<h2 class=\"section-title\">Grup duplikat persis</h2>\
{exact_groups}\
<h2 class=\"section-title\">Grup mirip ≥75% (di luar yang persis)</h2>\
{fuzzy_groups}\
<p class=\"doc-note\">Dihasilkan otomatis oleh {app_name}. Analisis memakai baris unik dan baris yang tersaring deduplikasi operasional pada impor.</p>\
</body></html>",
        period = escape(meta.period_label),
        font_css = fonts::face_css(),
        css = css,
        header = header(config, meta, "Laporan Duplikasi Isian"),
        rows = indicators.total_rows,
        units = units.iter().filter(|unit| unit.rows > 0).count(),
        exact = exact_rows,
        fuzzy = fuzzy_rows,
        filter = filter,
        pending = pending,
        unit_rows = unit_rows,
        exact_groups = exact_groups,
        fuzzy_groups = fuzzy_groups,
        app_name = escape(&config.app_name),
    )
}

pub fn render_drive_report(
    config: &Config,
    meta: &AnevReportMeta<'_>,
    indicators: &AnevIndicators,
) -> String {
    let drive = &indicators.drive;
    let mut unit_rows = String::new();
    for unit in drive.units.iter().filter(|unit| unit.rows > 0).take(80) {
        unit_rows.push_str(&format!(
            "<tr><td><strong>{satuan}</strong></td><td class=\"num\">{rows}</td>\
<td class=\"num\">{drive_rows}</td><td class=\"num\">{repeated}</td>\
<td class=\"num\">{foto}</td><td class=\"num\">{foto_repeated}</td></tr>",
            satuan = escape(&unit.satuan),
            rows = unit.rows,
            drive_rows = unit.drive_rows,
            repeated = unit.repeated_rows,
            foto = unit.foto_rows,
            foto_repeated = unit.repeated_foto_rows,
        ));
    }
    let groups = drive_table(&drive.groups);
    let pending = pending_notice(&indicators.pending_imports);
    let filter = filter_chip(meta.filter_polres);
    let css = css(config);
    format!(
        "<!doctype html><html lang=\"id\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>Tautan Drive Berulang · {period}</title><style>{font_css}{css}</style></head><body>\
{header}\
<section class=\"cards\">\
<div class=\"card\"><span class=\"label\">Baris diperiksa</span><strong>{total_rows}</strong></div>\
<div class=\"card\"><span class=\"label\">Baris dengan Drive</span><strong>{drive_rows}</strong></div>\
<div class=\"card\"><span class=\"label\">Drive berulang</span><strong>{repeated}</strong></div>\
<div class=\"card\"><span class=\"label\">Foto Drive</span><strong>{foto}</strong></div>\
<div class=\"card\"><span class=\"label\">Foto berulang</span><strong>{foto_repeated}</strong></div>\
<div class=\"card\"><span class=\"label\">ID berulang</span><strong>{repeated_ids}</strong></div>\
</section>\
<div class=\"note\">Tautan diambil dari isian laporan (termasuk baris Foto kegiatan) dan kolom Bukti dukung. Sebuah baris dihitung berulang jika ID Google Drive-nya dipakai minimal dua kali pada data periode ini. ID unik: {distinct_ids}; pemakaian terbanyak: {max_reuse} kali.{filter}</div>\
{pending}\
<h2 class=\"section-title\">Ringkasan per satuan</h2>\
<table class=\"summary-table\">\
<colgroup><col style=\"width:36%\"><col style=\"width:11%\"><col style=\"width:13%\"><col style=\"width:14%\"><col style=\"width:13%\"><col style=\"width:13%\"></colgroup>\
<thead><tr><th>Satuan</th><th class=\"num\">Baris</th><th class=\"num\">Drive</th><th class=\"num\">Berulang</th><th class=\"num\">Foto</th><th class=\"num\">Foto berulang</th></tr></thead>\
<tbody>{unit_rows}</tbody></table>\
<h2 class=\"section-title\">Tautan Drive yang berulang</h2>\
{groups}\
<p class=\"doc-note\">Dihasilkan otomatis oleh {app_name}. Tautan lengkap dapat dibuka dari data periode di aplikasi.</p>\
</body></html>",
        period = escape(meta.period_label),
        font_css = fonts::face_css(),
        css = css,
        header = header(config, meta, "Laporan Tautan Drive Berulang"),
        total_rows = indicators.total_rows,
        drive_rows = drive.total_drive_rows,
        repeated = drive.repeated_drive_rows,
        foto = drive.foto_drive_rows,
        foto_repeated = drive.repeated_foto_rows,
        repeated_ids = drive.repeated_ids,
        distinct_ids = drive.distinct_ids,
        max_reuse = drive.max_reuse,
        filter = filter,
        pending = pending,
        unit_rows = unit_rows,
        groups = groups,
        app_name = escape(&config.app_name),
    )
}

fn group_table(groups: &[DuplicateGroup], fuzzy: bool) -> String {
    if groups.is_empty() {
        return "<p class=\"empty\">Tidak ada grup pada kategori ini.</p>".to_string();
    }
    let mut rows = String::new();
    for group in groups.iter().take(100) {
        rows.push_str(&format!(
            "<tr><td><strong>{satuan}</strong><br><span class=\"small\">{kode} · {kegiatan}</span></td>\
<td class=\"num\">{count}</td>\
<td>{sample}</td>\
<td class=\"small\">{source_rows}</td></tr>",
            satuan = escape(&group.satuan),
            kode = escape(&group.kode),
            kegiatan = escape(&group.kegiatan),
            count = group.count,
            sample = escape(&group.sample),
            source_rows = group
                .source_rows
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }
    let heading = if fuzzy {
        "Satuan / kegiatan, jumlah baris, contoh isian, baris sumber. Kemiripan ≥75%."
    } else {
        "Satuan / kegiatan, jumlah baris, contoh isian, baris sumber."
    };
    format!(
        "<p class=\"small\">{heading}</p>\
<table class=\"group-table\"><colgroup><col style=\"width:30%\"><col style=\"width:7%\"><col style=\"width:48%\"><col style=\"width:15%\"></colgroup>\
<thead><tr><th>Satuan / kegiatan</th><th class=\"num\">Baris</th><th>Contoh isian</th><th>Baris sumber</th></tr></thead>\
<tbody>{rows}</tbody></table>"
    )
}

fn drive_table(groups: &[DriveGroup]) -> String {
    if groups.is_empty() {
        return "<p class=\"empty\">Tidak ada tautan Drive yang berulang.</p>".to_string();
    }
    let mut rows = String::new();
    for group in groups.iter().take(100) {
        let units = if group.units.len() > 8 {
            format!(
                "{} +{} satuan",
                group
                    .units
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
                group.units.len() - 8
            )
        } else {
            group.units.join(", ")
        };
        rows.push_str(&format!(
            "<tr><td><strong>{id}</strong><br><span class=\"small\">{link}</span></td>\
<td class=\"num\">{count}</td>\
<td>{units}</td>\
<td>{sample}</td></tr>",
            id = escape(&group.drive_id),
            link = escape(&group.link),
            count = group.count,
            units = escape(&units),
            sample = escape(&group.sample),
        ));
    }
    format!(
        "<p class=\"small\">ID Drive, jumlah pemakaian, satuan pemakai, dan contoh isian.</p>\
<table class=\"group-table\"><colgroup><col style=\"width:28%\"><col style=\"width:7%\"><col style=\"width:28%\"><col style=\"width:37%\"></colgroup>\
<thead><tr><th>ID / tautan</th><th class=\"num\">Jumlah</th><th>Satuan</th><th>Contoh isian</th></tr></thead>\
<tbody>{rows}</tbody></table>"
    )
}

fn header(config: &Config, meta: &AnevReportMeta<'_>, title: &str) -> String {
    format!(
        "<header class=\"report-head\">\
<div class=\"brand\"><div class=\"logo\"></div><div>\
<p class=\"eyebrow\">ANALISIS DAN EVALUASI MINGGUAN</p>\
<h1>{title}</h1><p class=\"subtitle\">{polda} · {period}</p>\
</div></div>\
<div class=\"head-meta\">\
<p><strong>Periode</strong><br>{start} – {end}</p>\
<p><strong>Dibuat</strong><br>{generated}</p>\
<p><strong>Aplikasi</strong><br>{app}</p>\
</div></header>",
        title = escape(title),
        polda = escape(meta.polda_name),
        period = escape(meta.period_label),
        start = escape(meta.period_start),
        end = escape(meta.period_end),
        generated = escape(&generated_label(meta.generated_at)),
        app = escape(&config.app_name),
    )
}

fn pending_notice(pending: &[String]) -> String {
    if pending.is_empty() {
        return String::new();
    }
    let names = pending
        .iter()
        .take(5)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "<div class=\"warning\">Analisis duplikat belum lengkap. Impor yang belum disiapkan: {names}. Buka tab Anev lalu klik \"Siapkan analisis duplikat\".</div>",
        names = escape(&names)
    )
}

fn filter_chip(filter: &str) -> String {
    if filter.trim().is_empty() {
        String::new()
    } else {
        format!(
            " <span class=\"chip\">Filter polres: {}</span>",
            escape(filter)
        )
    }
}

fn css(config: &Config) -> String {
    ANEV_REPORT_CSS
        .replace("__LOGO__", &branding::logo_data_uri(config))
        .replace("__SANS__", fonts::stack("inter"))
        .replace("__DISPLAY__", fonts::stack("montserrat"))
}

fn report_filename(prefix: &str, label: &str) -> String {
    let sanitized: String = label
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        format!("{prefix}.pdf")
    } else {
        format!("{prefix}-{sanitized}.pdf")
    }
}

const ANEV_REPORT_CSS: &str = r#"
:root { --logo: url("__LOGO__"); --ink: #1f2a24; --forest: #143d30; --gold: #b8894b; --soft: #5c6a63; --muted: #7a857f; --line: #cfd8cf; --panel: #f7faf6; }
@page { size: A4 landscape; margin: 9mm 10mm 11mm; }
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; }
body { font-family: __SANS__; font-size: 8.2pt; line-height: 1.38; color: var(--ink); background: #fff; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
h1, h2, h3, p { margin: 0; }
h1, h2 { font-family: __DISPLAY__; color: var(--forest); }
.report-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 8mm; border-bottom: 1.2pt solid var(--forest); padding-bottom: 3.5mm; }
.brand { display: flex; align-items: center; gap: 4mm; min-width: 0; }
.logo { width: 14mm; height: 18mm; flex: none; background-image: var(--logo); background-position: center; background-repeat: no-repeat; background-size: contain; }
.eyebrow { font-size: 6.8pt; font-weight: 700; letter-spacing: .14em; text-transform: uppercase; color: var(--gold); }
h1 { font-size: 19pt; line-height: 1.08; margin: 1.5mm 0 1mm; }
.subtitle { font-size: 9pt; color: var(--soft); }
.head-meta { flex: none; width: 68mm; font-size: 7.1pt; color: var(--soft); text-align: right; overflow-wrap: anywhere; }
.head-meta p { margin-bottom: 1.4mm; }
.head-meta strong { color: var(--forest); font-size: 6.6pt; text-transform: uppercase; letter-spacing: .06em; }
.cards { display: flex; gap: 2.4mm; margin: 4mm 0 3mm; }
.card { flex: 1; border: .6pt solid var(--line); border-radius: 2mm; background: var(--panel); padding: 2.4mm 2.8mm; }
.card .label { display: block; font-size: 6.3pt; font-weight: 700; letter-spacing: .05em; text-transform: uppercase; color: var(--muted); }
.card strong { display: block; margin-top: .8mm; font-family: __DISPLAY__; font-size: 15pt; line-height: 1; color: var(--forest); }
.note { border-left: 1.4mm solid var(--gold); background: #fbf5e6; padding: 2.4mm 3mm; font-size: 7.3pt; color: #5b4a35; }
.note strong { color: #7b5a2f; }
.warning { margin-top: 2.5mm; border: .6pt solid #e3c99b; background: #fff7e8; padding: 2.2mm 3mm; font-size: 7.2pt; color: #7b5a2f; }
.chip { display: inline-block; margin-left: 2mm; border: .5pt solid #d9c49e; border-radius: 1.5mm; padding: .3mm 1.4mm; font-weight: 700; }
.section-title { font-size: 11.5pt; margin: 4mm 0 1.8mm; }
table { width: 100%; border-collapse: collapse; table-layout: fixed; }
th, td { border: .5pt solid var(--line); padding: 1.1mm 1.4mm; vertical-align: top; word-wrap: break-word; overflow-wrap: anywhere; }
th { background: var(--forest); color: #fff; font-size: 6.8pt; font-weight: 700; text-align: left; letter-spacing: .03em; text-transform: uppercase; }
tbody tr:nth-child(even) td { background: var(--panel); }
.num { text-align: right; white-space: nowrap; }
.summary-table { font-size: 7.1pt; }
.summary-table th, .summary-table td { padding: 1mm 1.3mm; }
.summary-table tbody tr { page-break-inside: avoid; }
.group-table { font-size: 6.9pt; }
.group-table thead { display: table-header-group; }
.group-table tr { page-break-inside: avoid; }
.group-table th, .group-table td { padding: .9mm 1.2mm; }
.small { font-size: 6.7pt; color: var(--soft); }
.empty { padding: 4mm; text-align: center; color: var(--muted); font-style: italic; }
.doc-note { margin-top: 4mm; border-top: .6pt solid var(--line); padding-top: 1.8mm; font-size: 6.7pt; color: var(--muted); }
"#;

#[cfg(test)]
mod tests {
    use super::{AnevReportMeta, render_drive_report, render_duplicate_report};
    use crate::{
        config::Config,
        modules::imports::anev::{
            AnevIndicators, DriveGroup, DriveReport, DriveUnit, DuplicateGroup, DuplicateReport,
            DuplicateUnit,
        },
    };

    fn indicators() -> AnevIndicators {
        AnevIndicators {
            total_rows: 10,
            duplicate_ready: true,
            pending_imports: vec![],
            duplicate: DuplicateReport {
                units: vec![DuplicateUnit {
                    satuan: "Polsek Uji".into(),
                    rows: 10,
                    exact_rows: 2,
                    fuzzy_rows: 4,
                    exact_percent: 20.0,
                    fuzzy_percent: 40.0,
                }],
                exact_groups: vec![DuplicateGroup {
                    satuan: "Polsek Uji".into(),
                    kode: "QW5.1".into(),
                    kegiatan: "SP2HP".into(),
                    count: 2,
                    similarity: 1.0,
                    sample: "contoh <isian>".into(),
                    source_rows: vec![1, 2],
                    rows: vec![],
                }],
                fuzzy_groups: vec![],
            },
            drive: DriveReport {
                total_drive_rows: 5,
                repeated_drive_rows: 3,
                foto_drive_rows: 4,
                repeated_foto_rows: 2,
                distinct_ids: 3,
                repeated_ids: 1,
                max_reuse: 3,
                units: vec![DriveUnit {
                    satuan: "Polsek Uji".into(),
                    rows: 10,
                    drive_rows: 5,
                    repeated_rows: 3,
                    foto_rows: 4,
                    repeated_foto_rows: 2,
                }],
                groups: vec![DriveGroup {
                    drive_id: "abc123456789".into(),
                    link: "https://drive.google.com/file/d/abc123456789/view".into(),
                    count: 3,
                    units: vec!["Polsek Uji".into()],
                    sample: "contoh".into(),
                    source_rows: vec![1, 2, 3],
                    rows: vec![],
                }],
            },
        }
    }

    #[test]
    fn renders_both_reports_in_landscape() {
        let config = Config::test_config();
        let meta = AnevReportMeta {
            polda_name: "POLDA JATENG",
            period_label: "9–15 November 2026",
            period_start: "2026-11-09",
            period_end: "2026-11-15",
            generated_at: "2026-11-13T02:00:00+00:00",
            filter_polres: "",
        };
        let data = indicators();
        let duplicate = render_duplicate_report(&config, &meta, &data);
        let drive = render_drive_report(&config, &meta, &data);
        assert!(duplicate.contains("size: A4 landscape"));
        assert!(duplicate.contains("Polsek Uji"));
        assert!(duplicate.contains("contoh &lt;isian&gt;"));
        assert!(drive.contains("size: A4 landscape"));
        assert!(drive.contains("abc123456789"));
        assert!(drive.contains("pemakaian terbanyak: 3 kali"));
    }
}
