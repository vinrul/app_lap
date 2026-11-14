//! Laporan PDF khusus indikasi input cepat berulang. Dipisahkan dari template
//! desain 15 halaman karena sifatnya pemeriksaan tabel: A4 landscape dengan
//! ringkasan temuan dan rincian baris per satuan.

use std::collections::BTreeSet;

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::{
    branding,
    config::Config,
    error::ApiResult,
    modules::{
        auth::DataScope,
        imports::models::{BotSuspectRow, RapidInputSuspect},
        reports::{fonts, service},
    },
};

pub struct RapidReportData<'a> {
    pub polda_name: &'a str,
    pub period_label: &'a str,
    pub period_start: &'a str,
    pub period_end: &'a str,
    pub generated_at: &'a str,
    pub source_filename: &'a str,
    pub filter_polres: &'a str,
    pub suspects: &'a [RapidInputSuspect],
}

const MONTHS: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

pub async fn generate_rapid_pdf(
    db: &SqlitePool,
    config: &Config,
    pdf_lock: &Mutex<()>,
    period_id: &str,
    polres: Option<&str>,
    scope: &DataScope,
) -> ApiResult<(Vec<u8>, String)> {
    let (metrics, coverage) =
        service::coverage_for_report(db, config, period_id, polres, scope).await?;
    let data = RapidReportData {
        polda_name: &metrics.polda_name,
        period_label: &metrics.period_label,
        period_start: &metrics.period_start,
        period_end: &metrics.period_end,
        generated_at: &metrics.generated_at,
        source_filename: &metrics.source_filename,
        filter_polres: &coverage.filter_polres,
        suspects: &coverage.bot_detection.rapid_input,
    };
    let html = render_rapid_report(config, &data);
    let bytes = service::html_to_pdf_bytes(config, pdf_lock, period_id, "rapid", html).await?;
    Ok((bytes, report_filename(&metrics.period_label)))
}

pub fn render_rapid_report(config: &Config, data: &RapidReportData<'_>) -> String {
    let unique_units: BTreeSet<&str> = data
        .suspects
        .iter()
        .map(|item| item.satuan.as_str())
        .collect();
    let unique_dates: BTreeSet<&str> = data
        .suspects
        .iter()
        .map(|item| item.tanggal.as_str())
        .collect();
    let total_reports: usize = data.suspects.iter().map(|item| item.report_count).sum();
    let total_detail_rows: usize = data.suspects.iter().map(|item| item.rows.len()).sum();

    let mut summary_rows = String::new();
    for (index, item) in data.suspects.iter().enumerate() {
        summary_rows.push_str(&format!(
            "<tr>\
<td class=\"num\">{number}</td>\
<td><strong>{satuan}</strong></td>\
<td>{tanggal}</td>\
<td class=\"num\">{laporan}</td>\
<td>{jendela}</td>\
<td>{waktu}</td>\
<td class=\"num\">{menit}</td>\
<td>{kegiatan}</td>\
</tr>",
            number = index + 1,
            satuan = escape(&item.satuan),
            tanggal = escape(&long_date(&item.tanggal)),
            laporan = item.report_count,
            jendela = escape(&window_label(item.window_seconds)),
            waktu = format!("{}–{}", escape(&item.first_time), escape(&item.last_time)),
            menit = item.same_minute_count,
            kegiatan = escape(&item.activities.join(" · ")),
        ));
    }

    let mut details = String::new();
    for (index, item) in data.suspects.iter().enumerate() {
        let mut rows = String::new();
        for row in &item.rows {
            rows.push_str(&detail_row(row));
        }
        if item.rows.is_empty() {
            rows.push_str(
                "<tr><td colspan=\"6\" class=\"empty-cell\">Tidak ada rincian baris untuk temuan ini.</td></tr>",
            );
        }
        details.push_str(&format!(
            "<section class=\"detail\">\
<div class=\"detail-head\">\
<div>\
<p class=\"eyebrow\">TEMUAN {number} DARI {total}</p>\
<h2>{satuan}</h2>\
<p class=\"meta\">{tanggal} · {waktu} · {laporan} laporan · {jendela} · {kecepatan}</p>\
</div>\
<span class=\"detail-badge\">{laporan} laporan</span>\
</div>\
<p class=\"activities\"><strong>Kegiatan:</strong> {kegiatan}</p>\
<table class=\"detail-table\">\
<colgroup>\
<col style=\"width:5%\">\
<col style=\"width:9%\">\
<col style=\"width:6%\">\
<col style=\"width:17%\">\
<col style=\"width:8%\">\
<col style=\"width:55%\">\
</colgroup>\
<thead><tr>\
<th class=\"num\">Baris</th>\
<th>Waktu kirim</th>\
<th>Kode</th>\
<th>Kegiatan</th>\
<th>Status</th>\
<th>Isian laporan</th>\
</tr></thead>\
<tbody>{rows}</tbody>\
</table>\
{truncated}\
</section>",
            number = index + 1,
            total = data.suspects.len(),
            satuan = escape(&item.satuan),
            tanggal = escape(&long_date(&item.tanggal)),
            waktu = format!(
                "{}–{}",
                escape(&item.first_time),
                escape(&item.last_time)
            ),
            laporan = item.report_count,
            jendela = escape(&window_label(item.window_seconds)),
            kecepatan = escape(&pace_label(item.report_count, item.window_seconds)),
            kegiatan = escape(&item.activities.join(" · ")),
            truncated = if item.report_count > item.rows.len() {
                format!(
                    "<p class=\"truncated\">Menampilkan {} dari {} baris dalam jendela ini. Baris lainnya dapat diperiksa di aplikasi.</p>",
                    item.rows.len(),
                    item.report_count
                )
            } else {
                String::new()
            },
        ));
    }

    let empty_state = if data.suspects.is_empty() {
        "<div class=\"empty\">Tidak ada indikasi input cepat berulang pada periode ini.</div>"
            .to_string()
    } else {
        String::new()
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
    let css = BOT_REPORT_CSS
        .replace("__LOGO__", &logo)
        .replace("__SANS__", fonts::stack("inter"))
        .replace("__DISPLAY__", fonts::stack("montserrat"));

    format!(
        "<!doctype html><html lang=\"id\"><head><meta charset=\"utf-8\">\
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
    <title>Laporan Input Cepat · {period}</title>\
    <style>{font_css}{css}</style></head><body>\
    <header class=\"report-head\">\
    <div class=\"brand\">\
    <div class=\"logo\"></div>\
    <div>\
    <p class=\"eyebrow\">PEMERIKSAAN INDIKASI LAPORAN OTOMATIS</p>\
    <h1>Laporan Input Cepat Berulang</h1>\
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
    <div class=\"card\"><span class=\"label\">Satuan terindikasi</span><strong>{units}</strong></div>\
    <div class=\"card\"><span class=\"label\">Temuan satuan + tanggal</span><strong>{findings}</strong></div>\
    <div class=\"card\"><span class=\"label\">Laporan dalam jendela</span><strong>{reports}</strong></div>\
    <div class=\"card\"><span class=\"label\">Tanggal terindikasi</span><strong>{dates}</strong></div>\
    <div class=\"card\"><span class=\"label\">Baris dirinci</span><strong>{rows}</strong></div>\
    </section>\
    <div class=\"note\">\
    <strong>Kriteria pemeriksaan:</strong> minimal 8 laporan dalam 5 menit atau 10 laporan dalam 10 menit dari satuan yang sama pada tanggal yang sama. Indikasi ini bukan penilaian otomatis dan perlu diverifikasi pada data sumber.\
    {filter}\
    </div>\
    <h2 class=\"section-title\">Ringkasan temuan</h2>\
    <table class=\"summary-table\">\
    <colgroup>\
    <col style=\"width:4%\">\
    <col style=\"width:20%\">\
    <col style=\"width:9%\">\
    <col style=\"width:7%\">\
    <col style=\"width:8%\">\
    <col style=\"width:10%\">\
    <col style=\"width:7%\">\
    <col style=\"width:35%\">\
    </colgroup>\
    <thead><tr>\
    <th class=\"num\">No</th>\
    <th>Satuan</th>\
    <th>Tanggal</th>\
    <th class=\"num\">Laporan</th>\
    <th>Jendela</th>\
    <th>Waktu</th>\
    <th class=\"num\">Menit sama</th>\
    <th>Kegiatan</th>\
    </tr></thead>\
    <tbody>{summary_rows}</tbody>\
    </table>\
    {empty_state}\
    {details}\
    <p class=\"doc-note\">Laporan ini dihasilkan otomatis oleh {app_name}. Jumlah baris rincian mengikuti data pemeriksaan pada aplikasi; isian laporan ditampilkan ringkas dan teks lengkapnya dapat dibuka pada data periode.</p>\
    </body></html>",
        period = escape(data.period_label),
        font_css = fonts::face_css(),
        css = css,
        polda = escape(data.polda_name),
        start = escape(data.period_start),
        end = escape(data.period_end),
        generated = escape(&generated_label(data.generated_at)),
        source = escape(data.source_filename),
        units = unique_units.len(),
        findings = data.suspects.len(),
        reports = total_reports,
        dates = unique_dates.len(),
        rows = total_detail_rows,
        filter = filter,
        summary_rows = summary_rows,
        empty_state = empty_state,
        details = details,
        app_name = escape(&config.app_name),
    )
}

fn detail_row(row: &BotSuspectRow) -> String {
    format!(
        "<tr>\
<td class=\"num\">{baris}</td>\
<td>{waktu}</td>\
<td>{kode}</td>\
<td>{kegiatan}</td>\
<td>{status}</td>\
<td class=\"isian\">{isian}</td>\
</tr>",
        baris = row.source_row,
        waktu = escape(value_or(&row.waktu_kirim_wib, &row.tanggal)),
        kode = escape(value_or(&row.kode, "—")),
        kegiatan = escape(value_or(&row.kegiatan, "—")),
        status = escape(value_or(&row.status_penerimaan, "Belum diisi")),
        isian = escape(value_or(&row.isian_laporan, "—")),
    )
}

fn report_filename(label: &str) -> String {
    let sanitized: String = label
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        "pemeriksaan-input-cepat.pdf".to_string()
    } else {
        format!("pemeriksaan-input-cepat-{sanitized}.pdf")
    }
}

fn value_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn window_label(seconds: i64) -> String {
    if seconds <= 0 {
        return "kurang dari 1 menit".to_string();
    }
    if seconds < 60 {
        return format!("{seconds} detik");
    }
    let minutes = seconds / 60;
    let rest = seconds % 60;
    if rest == 0 {
        format!("{minutes} menit")
    } else {
        format!("{minutes} menit {rest} detik")
    }
}

fn pace_label(count: usize, seconds: i64) -> String {
    if count <= 1 {
        return "—".to_string();
    }
    let pace = (seconds / (count as i64 - 1)).max(1);
    format!("±{pace} detik/laporan")
}

pub(crate) fn generated_label(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|stamp| stamp.format("%d-%m-%Y %H:%M UTC").to_string())
        .unwrap_or_else(|_| value.to_string())
}

pub(crate) fn long_date(value: &str) -> String {
    let mut parts = value.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return value.to_string();
    };
    let Some(month) = month
        .parse::<usize>()
        .ok()
        .and_then(|month| MONTHS.get(month.wrapping_sub(1)))
    else {
        return value.to_string();
    };
    let Ok(day) = day.parse::<u32>() else {
        return value.to_string();
    };
    format!("{day} {month} {year}")
}

pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const BOT_REPORT_CSS: &str = r#"
:root { --logo: url("__LOGO__"); --ink: #1f2a24; --forest: #143d30; --gold: #b8894b; --soft: #5c6a63; --muted: #7a857f; --line: #cfd8cf; --panel: #f7faf6; }
@page { size: A4 landscape; margin: 9mm 10mm 11mm; }
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; }
body {
  font-family: __SANS__;
  font-size: 8.2pt;
  line-height: 1.38;
  color: var(--ink);
  background: #fff;
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}
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
.summary-table { font-size: 7.1pt; }
.summary-table th, .summary-table td { padding: 1mm 1.3mm; }
.summary-table tbody tr { page-break-inside: avoid; }
.empty, .empty-cell { padding: 4mm; text-align: center; color: var(--muted); font-style: italic; }
.detail { page-break-before: always; }
.detail-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 5mm; border-bottom: .8pt solid var(--line); padding-bottom: 2.2mm; margin-bottom: 2.2mm; }
.detail h2 { font-size: 14pt; margin-top: .5mm; }
.detail .meta { margin-top: .8mm; font-size: 7.4pt; color: var(--soft); }
.detail-badge { flex: none; border-radius: 2mm; background: #fbe9e7; color: #9c2f26; padding: 1.4mm 2.6mm; font-weight: 700; font-size: 8pt; }
.activities { margin-bottom: 2mm; color: var(--soft); font-size: 7.3pt; }
.detail-table { font-size: 6.8pt; }
.detail-table thead { display: table-header-group; }
.detail-table tr { page-break-inside: avoid; }
.detail-table th, .detail-table td { padding: .9mm 1.2mm; }
.detail-table .isian { white-space: pre-wrap; word-break: break-word; }
.truncated { margin-top: 1.4mm; font-size: 6.8pt; color: var(--muted); font-style: italic; }
.doc-note { margin-top: 4mm; border-top: .6pt solid var(--line); padding-top: 1.8mm; font-size: 6.7pt; color: var(--muted); }
"#;

#[cfg(test)]
mod tests {
    use super::{RapidReportData, render_rapid_report};
    use crate::{
        config::Config,
        modules::imports::models::{BotSuspectRow, RapidInputSuspect},
    };

    fn sample_suspect() -> RapidInputSuspect {
        RapidInputSuspect {
            satuan: "Polsek Cepat <script>".to_string(),
            tanggal: "2026-11-09".to_string(),
            report_count: 8,
            window_seconds: 60,
            first_time: "08:00".to_string(),
            last_time: "08:01".to_string(),
            same_minute_count: 4,
            activities: vec!["QW5.2 · Layanan Informasi Perkara".to_string()],
            source_rows: vec![1, 2, 3],
            rows: vec![BotSuspectRow {
                source_row: 1,
                tanggal: "2026-11-09".to_string(),
                waktu_kirim_wib: "2026-11-09 08:00:12".to_string(),
                satuan: "Polsek Cepat".to_string(),
                kode: "QW5.2".to_string(),
                kegiatan: "Layanan Informasi Perkara".to_string(),
                status_penerimaan: "Diterima".to_string(),
                isian_laporan: "Isian uji <b>tebal</b>".to_string(),
            }],
        }
    }

    #[test]
    fn renders_landscape_report_with_escaped_details() {
        let config = Config::test_config();
        let suspects = vec![sample_suspect()];
        let data = RapidReportData {
            polda_name: "POLDA JATENG",
            period_label: "9–15 November 2026",
            period_start: "2026-11-09",
            period_end: "2026-11-15",
            generated_at: "2026-11-13T02:00:00+00:00",
            source_filename: "contoh.xlsx",
            filter_polres: "",
            suspects: &suspects,
        };

        let html = render_rapid_report(&config, &data);
        assert!(html.contains("size: A4 landscape"));
        assert!(html.contains("Laporan Input Cepat Berulang"));
        assert!(html.contains("Polsek Cepat &lt;script&gt;"));
        assert!(!html.contains("Polsek Cepat <script>"));
        assert!(html.contains("08:00–08:01"));
        assert!(html.contains("Isian uji &lt;b&gt;tebal&lt;/b&gt;"));
        assert!(html.contains("minimal 8 laporan"));
    }

    #[test]
    fn renders_empty_state_when_no_suspects() {
        let config = Config::test_config();
        let data = RapidReportData {
            polda_name: "POLDA JATENG",
            period_label: "9–15 November 2026",
            period_start: "2026-11-09",
            period_end: "2026-11-15",
            generated_at: "",
            source_filename: "",
            filter_polres: "",
            suspects: &[],
        };

        let html = render_rapid_report(&config, &data);
        assert!(html.contains("Tidak ada indikasi input cepat berulang"));
    }
}
