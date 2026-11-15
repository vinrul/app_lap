//! Laporan PDF temuan batas waktu laporan QW1.1–QW1.8 dan peringkat polres.
//! Memakai A4 landscape karena tabel peringkat lebih nyaman dibaca melebar.

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::{
    branding,
    config::Config,
    error::ApiResult,
    modules::{
        auth::DataScope,
        imports::models::CoverageDeadlineReport,
        reports::{
            bot_report::{escape, generated_label},
            fonts, service,
        },
    },
};

pub struct DeadlineReportData<'a> {
    pub polda_name: &'a str,
    pub period_label: &'a str,
    pub period_start: &'a str,
    pub period_end: &'a str,
    pub generated_at: &'a str,
    pub source_filename: &'a str,
    pub filter_polres: &'a str,
    pub deadline: &'a CoverageDeadlineReport,
}

pub async fn generate_deadline_pdf(
    db: &SqlitePool,
    config: &Config,
    pdf_lock: &Mutex<()>,
    period_id: &str,
    polres: Option<&str>,
    scope: &DataScope,
) -> ApiResult<(Vec<u8>, String)> {
    let (metrics, coverage) =
        service::coverage_for_report(db, config, period_id, polres, scope).await?;
    let data = DeadlineReportData {
        polda_name: &metrics.polda_name,
        period_label: &metrics.period_label,
        period_start: &metrics.period_start,
        period_end: &metrics.period_end,
        generated_at: &metrics.generated_at,
        source_filename: &metrics.source_filename,
        filter_polres: &coverage.filter_polres,
        deadline: &coverage.deadline,
    };
    let html = render_deadline_report(config, &data);
    let bytes = service::html_to_pdf_bytes(config, pdf_lock, period_id, "deadline", html).await?;
    Ok((bytes, report_filename(&metrics.period_label)))
}

pub fn render_deadline_report(config: &Config, data: &DeadlineReportData<'_>) -> String {
    let deadline = data.deadline;
    let units_with_findings = deadline
        .units
        .iter()
        .filter(|unit| unit.findings > 0)
        .count();

    let mut rules_rows = String::new();
    for rule in &deadline.rules {
        rules_rows.push_str(&format!(
            "<tr>\
<td><strong>{code}</strong></td>\
<td>{title}</td>\
<td>{day}</td>\
<td class=\"num\">{time}</td>\
<td>{note}</td>\
</tr>",
            code = escape(&rule.code),
            title = escape(&rule.title),
            day = escape(&rule.day),
            time = escape(&rule.time),
            note = escape(&rule.note),
        ));
    }

    let mut ranking_rows = String::new();
    for (index, unit) in deadline.units.iter().enumerate() {
        let codes = if unit.codes.is_empty() {
            "—".to_string()
        } else {
            unit.codes
                .iter()
                .map(|code| format!("{} ({})", code.code, code.findings))
                .collect::<Vec<_>>()
                .join(" · ")
        };
        ranking_rows.push_str(&format!(
            "<tr class=\"{class}\">\
<td class=\"num\">{number}</td>\
<td><strong>{label}</strong></td>\
<td class=\"num\">{findings}</td>\
<td class=\"num\">{rows}</td>\
<td>{codes}</td>\
<td class=\"num\">{percent}</td>\
</tr>",
            class = if unit.findings == 0 { "muted" } else { "" },
            number = index + 1,
            label = escape(&unit.label),
            findings = unit.findings,
            rows = unit.rows,
            codes = escape(&codes),
            percent = format!("{:.2}%", unit.percent),
        ));
    }

    let empty_ranking = if deadline.units.is_empty() {
        "<div class=\"empty\">Tidak ada data polres pada periode ini.</div>"
    } else {
        ""
    };
    let empty_rules = if deadline.rules.is_empty() {
        "<div class=\"empty\">Aturan batas waktu belum tersedia.</div>"
    } else {
        ""
    };
    let filter = if data.filter_polres.trim().is_empty() {
        String::new()
    } else {
        format!(
            "<span class=\"chip\">Filter polres: {}</span>",
            escape(data.filter_polres)
        )
    };

    let logo = branding::logo_data_uri(config);
    let css = DEADLINE_REPORT_CSS
        .replace("__LOGO__", &logo)
        .replace("__SANS__", fonts::stack("inter"))
        .replace("__DISPLAY__", fonts::stack("montserrat"));

    format!(
        "<!doctype html><html lang=\"id\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>Temuan Batas Waktu · {period}</title>\
<style>{font_css}{css}</style></head><body>\
<header class=\"report-head\">\
<div class=\"brand\">\
<div class=\"logo\"></div>\
<div>\
<p class=\"eyebrow\">PEMERIKSAAN BATAS WAKTU LAPORAN</p>\
<h1>Temuan Batas Waktu dan Peringkat Polres</h1>\
<p class=\"subtitle\">{polda} · {period}</p>\
</div>\
</div>\
<div class=\"head-meta\">\
<p><strong>Periode</strong><br>{start} – {end}</p>\
<p><strong>Dibuat</strong><br>{generated}</p>\
<p><strong>Sumber data</strong><br>{source}</p>\
</div>\
</header>\
<section class=\"cards\">\
<div class=\"card\"><span class=\"label\">Total temuan</span><strong>{findings}</strong></div>\
<div class=\"card\"><span class=\"label\">Polres dengan temuan</span><strong>{units_with_findings}</strong></div>\
<div class=\"card\"><span class=\"label\">Baris diperiksa</span><strong>{rows}</strong></div>\
<div class=\"card\"><span class=\"label\">Tepat waktu</span><strong>{percent}</strong></div>\
</section>\
<div class=\"note\">\
<strong>Ruang lingkup:</strong> hanya QW1.1–QW1.8 yang dihitung sebagai temuan; QW4.1–QW4.3 tidak diikutkan. Pemeriksaan memakai <em>Tanggal lapor (WIB)</em>. QW1.7 boleh dilaporkan hari Sabtu atau hari Minggu sampai pukul 08:00. Peringkat diurutkan dari temuan terbanyak sampai polres tanpa temuan.\
{filter}\
</div>\
<h2 class=\"section-title\">Aturan batas waktu</h2>\
<table class=\"rules-table\">\
<colgroup>\
<col style=\"width:7%\">\
<col style=\"width:30%\">\
<col style=\"width:13%\">\
<col style=\"width:8%\">\
<col style=\"width:42%\">\
</colgroup>\
<thead><tr>\
<th>Kode</th>\
<th>Kegiatan</th>\
<th>Hari batas</th>\
<th class=\"num\">Jam</th>\
<th>Catatan</th>\
</tr></thead>\
<tbody>{rules_rows}</tbody>\
</table>\
{empty_rules}\
<section class=\"ranking\">\
<h2 class=\"section-title\">Peringkat polres</h2>\
<p class=\"ranking-note\">Temuan = baris yang melewati batas; baris = jumlah baris yang diperiksa pada satuan; kode temuan = sebaran temuan per sub-kode; tepat waktu = persentase baris yang tidak melewati batas. Satuan tanpa laporan tetap ditampilkan.</p>\
<table class=\"ranking-table\">\
<colgroup>\
<col style=\"width:4%\">\
<col style=\"width:30%\">\
<col style=\"width:10%\">\
<col style=\"width:9%\">\
<col style=\"width:37%\">\
<col style=\"width:10%\">\
</colgroup>\
<thead><tr>\
<th class=\"num\">No</th>\
<th>Polres</th>\
<th class=\"num\">Temuan</th>\
<th class=\"num\">Baris</th>\
<th>Kode temuan</th>\
<th class=\"num\">Tepat waktu</th>\
</tr></thead>\
<tbody>{ranking_rows}</tbody>\
</table>\
{empty_ranking}\
</section>\
<p class=\"doc-note\">Laporan ini dihasilkan otomatis oleh {app_name}. Rincian baris temuan dapat dibuka pada section Batas waktu laporan di tab Anev aplikasi.</p>\
</body></html>",
        period = escape(data.period_label),
        font_css = fonts::face_css(),
        css = css,
        polda = escape(data.polda_name),
        start = escape(data.period_start),
        end = escape(data.period_end),
        generated = escape(&generated_label(data.generated_at)),
        source = escape(data.source_filename),
        findings = deadline.findings,
        units_with_findings = units_with_findings,
        rows = deadline.total_rows,
        percent = format!("{:.2}%", deadline.percent),
        filter = filter,
        rules_rows = rules_rows,
        empty_rules = empty_rules,
        ranking_rows = ranking_rows,
        empty_ranking = empty_ranking,
        app_name = escape(&config.app_name),
    )
}

fn report_filename(label: &str) -> String {
    let sanitized: String = label
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        "temuan-batas-waktu.pdf".to_string()
    } else {
        format!("temuan-batas-waktu-{sanitized}.pdf")
    }
}

const DEADLINE_REPORT_CSS: &str = r#"
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
.head-meta { flex: none; width: 68mm; font-size: 7.1pt; color: var(--soft); text-align: right; }
.head-meta p { margin-bottom: 1.4mm; }
.head-meta strong { color: var(--forest); font-size: 6.6pt; text-transform: uppercase; letter-spacing: .06em; }
.cards { display: flex; gap: 3mm; margin: 4mm 0 3mm; }
.card { flex: 1; border: .6pt solid var(--line); border-radius: 2mm; background: var(--panel); padding: 2.6mm 3mm; }
.card .label { display: block; font-size: 6.6pt; font-weight: 700; letter-spacing: .06em; text-transform: uppercase; color: var(--muted); }
.card strong { display: block; margin-top: .8mm; font-family: __DISPLAY__; font-size: 16pt; line-height: 1; color: var(--forest); }
.note { border-left: 1.4mm solid var(--gold); background: #fbf5e6; padding: 2.4mm 3mm; font-size: 7.3pt; color: #5b4a35; }
.note strong { color: #7b5a2f; }
.chip { display: inline-block; margin-left: 2mm; border: .5pt solid #d9c49e; border-radius: 1.5mm; padding: .3mm 1.4mm; font-weight: 700; }
.section-title { font-size: 11.5pt; margin: 4mm 0 1.8mm; }
table { width: 100%; border-collapse: collapse; table-layout: fixed; }
th, td { border: .5pt solid var(--line); padding: 1.1mm 1.4mm; vertical-align: top; word-wrap: break-word; overflow-wrap: anywhere; }
th { background: var(--forest); color: #fff; font-size: 6.8pt; font-weight: 700; text-align: left; letter-spacing: .03em; text-transform: uppercase; }
tbody tr:nth-child(even) td { background: var(--panel); }
.num { text-align: right; white-space: nowrap; }
.rules-table { font-size: 7.2pt; }
.rules-table tbody tr { page-break-inside: avoid; }
.ranking { page-break-before: always; }
.ranking-note { margin-bottom: 2mm; font-size: 7.2pt; color: var(--soft); }
.ranking-table { font-size: 7.1pt; }
.ranking-table thead { display: table-header-group; }
.ranking-table tbody tr { page-break-inside: avoid; }
.ranking-table tr.muted td { color: var(--muted); }
.empty { padding: 4mm; text-align: center; color: var(--muted); font-style: italic; }
.doc-note { margin-top: 4mm; border-top: .6pt solid var(--line); padding-top: 1.8mm; font-size: 6.7pt; color: var(--muted); }
"#;

#[cfg(test)]
mod tests {
    use super::{DeadlineReportData, render_deadline_report};
    use crate::{
        config::Config,
        modules::imports::models::{
            CoverageDeadlineCode, CoverageDeadlineReport, CoverageDeadlineRule,
            CoverageDeadlineUnit,
        },
    };

    fn sample() -> CoverageDeadlineReport {
        CoverageDeadlineReport {
            rules: vec![CoverageDeadlineRule {
                code: "QW1.1".to_string(),
                title: "Pamapta Hadir".to_string(),
                day: "Senin".to_string(),
                time: "08:00".to_string(),
                note: "Laporan maksimal Senin pukul 08:00.".to_string(),
            }],
            total_rows: 4,
            on_time: 1,
            findings: 3,
            percent: 25.0,
            units: vec![CoverageDeadlineUnit {
                label: "Polres Contoh".to_string(),
                rows: 4,
                on_time: 1,
                findings: 3,
                percent: 25.0,
                codes: vec![CoverageDeadlineCode {
                    code: "QW1.1".to_string(),
                    findings: 3,
                }],
                details: Vec::new(),
            }],
        }
    }

    #[test]
    fn renders_rules_and_ranking() {
        let config = Config::test_config();
        let deadline = sample();
        let data = DeadlineReportData {
            polda_name: "POLDA JATENG",
            period_label: "9–15 November 2026",
            period_start: "2026-11-09",
            period_end: "2026-11-15",
            generated_at: "2026-11-13T02:00:00+00:00",
            source_filename: "contoh.xlsx",
            filter_polres: "",
            deadline: &deadline,
        };
        let html = render_deadline_report(&config, &data);
        assert!(html.contains("size: A4 landscape"));
        assert!(html.contains("Temuan Batas Waktu dan Peringkat Polres"));
        assert!(html.contains("Polres Contoh"));
        assert!(html.contains("QW1.1 (3)"));
        assert!(html.contains("hanya QW1.1–QW1.8"));
        assert!(html.contains("QW4.1–QW4.3 tidak diikutkan"));
    }
}
