//! Laporan PDF kepatuhan jadwal pelaporan QW1.1–QW1.8. Memakai A4 landscape
//! karena tabel unit dan tanggal lebih nyaman dibaca melebar.

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::{
    branding,
    config::Config,
    error::ApiResult,
    modules::{
        auth::DataScope,
        imports::models::{CoverageSchedule, CoverageScheduleUnit},
        reports::{
            bot_report::{escape, generated_label},
            fonts, service,
        },
    },
};

pub struct ScheduleReportData<'a> {
    pub polda_name: &'a str,
    pub period_label: &'a str,
    pub period_start: &'a str,
    pub period_end: &'a str,
    pub generated_at: &'a str,
    pub source_filename: &'a str,
    pub filter_polres: &'a str,
    pub schedules: &'a [CoverageSchedule],
}

pub async fn generate_schedule_pdf(
    db: &SqlitePool,
    config: &Config,
    pdf_lock: &Mutex<()>,
    period_id: &str,
    polres: Option<&str>,
    scope: &DataScope,
) -> ApiResult<(Vec<u8>, String)> {
    let (metrics, coverage) =
        service::coverage_for_report(db, config, period_id, polres, scope).await?;
    let data = ScheduleReportData {
        polda_name: &metrics.polda_name,
        period_label: &metrics.period_label,
        period_start: &metrics.period_start,
        period_end: &metrics.period_end,
        generated_at: &metrics.generated_at,
        source_filename: &metrics.source_filename,
        filter_polres: &coverage.filter_polres,
        schedules: &coverage.schedule,
    };
    let html = render_schedule_report(config, &data);
    let bytes = service::html_to_pdf_bytes(config, pdf_lock, period_id, "schedule", html).await?;
    Ok((bytes, report_filename(&metrics.period_label)))
}

pub fn render_schedule_report(config: &Config, data: &ScheduleReportData<'_>) -> String {
    let primary_label = if data.filter_polres.trim().is_empty() {
        "Polres"
    } else {
        "Polsek"
    };
    let secondary_label = if data.filter_polres.trim().is_empty() {
        "Satker Polda"
    } else {
        "Satker Polres"
    };

    let mut total = ScheduleCounts::default();
    let mut scheduled_count = 0usize;
    for schedule in data.schedules {
        if !schedule.expected_dates.is_empty() {
            scheduled_count += 1;
        }
        total.add_schedule(schedule);
    }

    let mut summary_rows = String::new();
    for schedule in data.schedules {
        let counts = ScheduleCounts::from_schedule(schedule);
        let evaluated = counts.on_time + counts.off_schedule + counts.missing;
        let percentage = if evaluated == 0 {
            "—".to_string()
        } else {
            format!(
                "{}%",
                (counts.on_time as f64 / evaluated as f64 * 100.0).round() as i64
            )
        };
        summary_rows.push_str(&format!(
            "<tr>\
<td><strong>{code}</strong></td>\
<td>{title}</td>\
<td>{day}</td>\
<td>{dates}</td>\
<td class=\"num\">{units}</td>\
<td class=\"num\">{on_time}</td>\
<td class=\"num\">{off_schedule}</td>\
<td class=\"num\">{missing}</td>\
<td class=\"num\">{percentage}</td>\
</tr>",
            code = escape(&schedule.code),
            title = escape(&schedule.title),
            day = escape(&schedule.day),
            dates = date_list(&schedule.expected_dates),
            units = schedule.polres.len() + schedule.satker.len(),
            on_time = counts.on_time,
            off_schedule = counts.off_schedule,
            missing = counts.missing,
            percentage = percentage,
        ));
    }

    let mut details = String::new();
    for (index, schedule) in data.schedules.iter().enumerate() {
        if schedule.expected_dates.is_empty() {
            continue;
        }
        let mut units: Vec<(&CoverageScheduleUnit, &str)> = schedule
            .polres
            .iter()
            .map(|unit| (unit, primary_label))
            .chain(schedule.satker.iter().map(|unit| (unit, secondary_label)))
            .collect();
        units.sort_by(|left, right| {
            status_rank(&left.0.status)
                .cmp(&status_rank(&right.0.status))
                .then_with(|| left.1.cmp(right.1))
                .then_with(|| left.0.label.cmp(&right.0.label))
        });
        let mut rows = String::new();
        for (unit, level) in units {
            rows.push_str(&format!(
                "<tr>\
<td><strong>{label}</strong></td>\
<td>{level}</td>\
<td><span class=\"status {class}\">{status}</span></td>\
<td>{on_time}</td>\
<td>{off_schedule}</td>\
<td>{children}</td>\
</tr>",
                label = escape(&unit.label),
                level = level,
                class = status_class(&unit.status),
                status = status_label(&unit.status),
                on_time = date_list(&unit.on_time_dates),
                off_schedule = date_list(&unit.off_schedule_dates),
                children = child_notes(unit),
            ));
        }
        if rows.is_empty() {
            rows.push_str(
                "<tr><td colspan=\"6\" class=\"empty-cell\">Belum ada unit yang dievaluasi pada kode ini.</td></tr>",
            );
        }
        let counts = ScheduleCounts::from_schedule(schedule);
        details.push_str(&format!(
            "<section class=\"detail\">\
<div class=\"detail-head\">\
<div>\
<p class=\"eyebrow\">JADWAL {number} DARI {total}</p>\
<h2>{code} · {title}</h2>\
<p class=\"meta\">Hari {day} · Tanggal jadwal: {dates}</p>\
</div>\
<div class=\"count-badges\">\
<span class=\"status on_time\">{on_time} sesuai</span>\
<span class=\"status off_schedule\">{off_schedule} di luar</span>\
<span class=\"status missing\">{missing} belum</span>\
</div>\
</div>\
<table class=\"detail-table\">\
<colgroup>\
<col style=\"width:20%\">\
<col style=\"width:9%\">\
<col style=\"width:9%\">\
<col style=\"width:19%\">\
<col style=\"width:19%\">\
<col style=\"width:24%\">\
</colgroup>\
<thead><tr>\
<th>Unit</th>\
<th>Tingkat</th>\
<th>Status</th>\
<th>Tanggal tepat</th>\
<th>Tanggal di luar</th>\
<th>Sub-unit belum sesuai</th>\
</tr></thead>\
<tbody>{rows}</tbody>\
</table>\
</section>",
            number = index + 1,
            total = scheduled_count,
            code = escape(&schedule.code),
            title = escape(&schedule.title),
            day = escape(&schedule.day),
            dates = date_list(&schedule.expected_dates),
            on_time = counts.on_time,
            off_schedule = counts.off_schedule,
            missing = counts.missing,
            rows = rows,
        ));
    }

    let summary_table = if data.schedules.is_empty() {
        String::new()
    } else {
        format!(
            "<table class=\"summary-table\">\
<colgroup>\
<col style=\"width:7%\">\
<col style=\"width:24%\">\
<col style=\"width:7%\">\
<col style=\"width:23%\">\
<col style=\"width:6%\">\
<col style=\"width:8%\">\
<col style=\"width:8%\">\
<col style=\"width:8%\">\
<col style=\"width:9%\">\
</colgroup>\
<thead><tr>\
<th>Kode</th>\
<th>Kegiatan</th>\
<th>Hari</th>\
<th>Tanggal jadwal</th>\
<th class=\"num\">Unit</th>\
<th class=\"num\">Sesuai</th>\
<th class=\"num\">Di luar</th>\
<th class=\"num\">Belum</th>\
<th class=\"num\">Kepatuhan</th>\
</tr></thead>\
<tbody>{summary_rows}</tbody>\
</table>"
        )
    };

    let empty_state = if data.schedules.is_empty() {
        "<div class=\"empty\">Belum ada data jadwal pelaporan QW1.1–QW1.8 pada periode ini.</div>"
            .to_string()
    } else {
        String::new()
    };
    let no_schedule_note = if scheduled_count == 0 {
        "<p class=\"note-line\">Tidak ada tanggal yang jatuh pada hari jadwal QW1.1–QW1.8 dalam periode ini.</p>"
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
    let css = SCHEDULE_REPORT_CSS
        .replace("__LOGO__", &logo)
        .replace("__SANS__", fonts::stack("inter"))
        .replace("__DISPLAY__", fonts::stack("montserrat"));
    let evaluated = total.on_time + total.off_schedule + total.missing;
    let compliance = if evaluated == 0 {
        "—".to_string()
    } else {
        format!(
            "{}%",
            (total.on_time as f64 / evaluated as f64 * 100.0).round() as i64
        )
    };

    format!(
        "<!doctype html><html lang=\"id\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>Jadwal Pelaporan QW1.1–QW1.8 · {period}</title>\
<style>{font_css}{css}</style></head><body>\
<header class=\"report-head\">\
<div class=\"brand\">\
<div class=\"logo\"></div>\
<div>\
<p class=\"eyebrow\">ANALISIS DAN EVALUASI MINGGUAN</p>\
<h1>Jadwal Pelaporan QW1.1–QW1.8</h1>\
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
<div class=\"card\"><span class=\"label\">Kode terjadwal</span><strong>{scheduled_count}</strong></div>\
<div class=\"card\"><span class=\"label\">Unit dievaluasi</span><strong>{evaluated}</strong></div>\
<div class=\"card\"><span class=\"label\">Sesuai jadwal</span><strong>{on_time}</strong></div>\
<div class=\"card\"><span class=\"label\">Di luar jadwal</span><strong>{off_schedule}</strong></div>\
<div class=\"card\"><span class=\"label\">Belum melapor</span><strong>{missing}</strong></div>\
<div class=\"card\"><span class=\"label\">Kepatuhan</span><strong>{compliance}</strong></div>\
</section>\
<div class=\"note\">\
<strong>Aturan jadwal:</strong> Senin QW1.1 & QW1.2 · Rabu QW1.4 · Kamis QW1.3 · Jumat QW1.5 & QW1.6 · Sabtu QW1.7 · Minggu QW1.8. Laporan pada hari lain dihitung di luar jadwal; sub-kode lain dapat dilaporkan setiap hari. Kepatuhan dihitung dari unit yang dievaluasi (sesuai, di luar jadwal, dan belum melapor).\
{filter}\
</div>\
<h2 class=\"section-title\">Ringkasan per kode</h2>\
{summary_table}\
{empty_state}\
{no_schedule_note}\
{details}\
<p class=\"doc-note\">Laporan ini dihasilkan otomatis oleh {app_name}. Rincian sub-unit hanya menampilkan yang belum sesuai jadwal; data lengkap dapat dibuka pada tab Anev di aplikasi.</p>\
</body></html>",
        period = escape(data.period_label),
        font_css = fonts::face_css(),
        css = css,
        polda = escape(data.polda_name),
        start = escape(data.period_start),
        end = escape(data.period_end),
        generated = escape(&generated_label(data.generated_at)),
        source = escape(data.source_filename),
        scheduled_count = scheduled_count,
        evaluated = evaluated,
        on_time = total.on_time,
        off_schedule = total.off_schedule,
        missing = total.missing,
        compliance = compliance,
        filter = filter,
        summary_table = summary_table,
        empty_state = empty_state,
        no_schedule_note = no_schedule_note,
        details = details,
        app_name = escape(&config.app_name),
    )
}

#[derive(Default)]
struct ScheduleCounts {
    on_time: usize,
    off_schedule: usize,
    missing: usize,
}

impl ScheduleCounts {
    fn from_schedule(schedule: &CoverageSchedule) -> Self {
        let mut counts = Self::default();
        counts.add_schedule(schedule);
        counts
    }

    fn add_schedule(&mut self, schedule: &CoverageSchedule) {
        for unit in schedule.polres.iter().chain(schedule.satker.iter()) {
            match unit.status.as_str() {
                "on_time" => self.on_time += 1,
                "off_schedule" => self.off_schedule += 1,
                "missing" => self.missing += 1,
                _ => {}
            }
        }
    }
}

fn status_rank(status: &str) -> u8 {
    match status {
        "missing" => 0,
        "off_schedule" => 1,
        "on_time" => 2,
        _ => 3,
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "on_time" => "Sesuai jadwal",
        "off_schedule" => "Di luar jadwal",
        "missing" => "Belum melapor",
        _ => "Tidak terjadwal",
    }
}

fn status_class(status: &str) -> &'static str {
    match status {
        "on_time" => "on_time",
        "off_schedule" => "off_schedule",
        "missing" => "missing",
        _ => "not_scheduled",
    }
}

fn child_notes(unit: &CoverageScheduleUnit) -> String {
    let mut notes = Vec::new();
    for child in unit.polsek.iter().chain(unit.satker.iter()) {
        if child.status != "on_time" {
            notes.push(format!(
                "{} ({})",
                escape(&child.label),
                status_label(&child.status)
            ));
        }
    }
    if notes.is_empty() {
        "—".to_string()
    } else {
        notes.join("; ")
    }
}

fn date_list(values: &[String]) -> String {
    if values.is_empty() {
        return "—".to_string();
    }
    values
        .iter()
        .map(|value| short_date(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn short_date(value: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des",
    ];
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

fn report_filename(label: &str) -> String {
    let sanitized: String = label
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        "jadwal-pelaporan-qw1.pdf".to_string()
    } else {
        format!("jadwal-pelaporan-qw1-{sanitized}.pdf")
    }
}

const SCHEDULE_REPORT_CSS: &str = r#"
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
.head-meta { flex: none; width: 68mm; font-size: 7.1pt; color: var(--soft); text-align: right; overflow-wrap: anywhere; }
.head-meta p { margin-bottom: 1.4mm; }
.head-meta strong { color: var(--forest); font-size: 6.6pt; text-transform: uppercase; letter-spacing: .06em; }
.cards { display: flex; gap: 2.4mm; margin: 4mm 0 3mm; }
.card { flex: 1; border: .6pt solid var(--line); border-radius: 2mm; background: var(--panel); padding: 2.4mm 2.8mm; }
.card .label { display: block; font-size: 6.3pt; font-weight: 700; letter-spacing: .05em; text-transform: uppercase; color: var(--muted); }
.card strong { display: block; margin-top: .8mm; font-family: __DISPLAY__; font-size: 15pt; line-height: 1; color: var(--forest); }
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
.note-line { margin-top: 2mm; font-size: 7pt; color: var(--muted); font-style: italic; }
.detail { page-break-before: always; }
.detail-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 5mm; border-bottom: .8pt solid var(--line); padding-bottom: 2.2mm; margin-bottom: 2.2mm; }
.detail h2 { font-size: 14pt; margin-top: .5mm; }
.detail .meta { margin-top: .8mm; font-size: 7.4pt; color: var(--soft); }
.count-badges { display: flex; flex-wrap: wrap; gap: 1.5mm; justify-content: flex-end; }
.status { display: inline-block; border-radius: 1.5mm; padding: .35mm 1.5mm; font-weight: 700; font-size: 6.5pt; white-space: nowrap; }
.status.on_time { background: #e6f4ea; color: #1d6b3a; }
.status.off_schedule { background: #fbf1e0; color: #8a5a1f; }
.status.missing { background: #fbe9e7; color: #9c2f26; }
.status.not_scheduled { background: #eef1ef; color: #5c6a63; }
.detail-table { font-size: 6.9pt; }
.detail-table thead { display: table-header-group; }
.detail-table tr { page-break-inside: avoid; }
.detail-table th, .detail-table td { padding: .9mm 1.2mm; }
.doc-note { margin-top: 4mm; border-top: .6pt solid var(--line); padding-top: 1.8mm; font-size: 6.7pt; color: var(--muted); }
"#;

#[cfg(test)]
mod tests {
    use super::{ScheduleReportData, render_schedule_report};
    use crate::{
        config::Config,
        modules::imports::models::{CoverageSchedule, CoverageScheduleChild, CoverageScheduleUnit},
    };

    fn unit(label: &str, status: &str) -> CoverageScheduleUnit {
        CoverageScheduleUnit {
            label: label.to_string(),
            status: status.to_string(),
            on_time_rows: 1,
            off_schedule_rows: 0,
            on_time_dates: vec!["2026-11-09".to_string()],
            off_schedule_dates: vec![],
            polsek: vec![CoverageScheduleChild {
                label: "Polsek Uji".to_string(),
                status: "missing".to_string(),
                on_time_rows: 0,
                off_schedule_rows: 0,
                on_time_dates: vec![],
                off_schedule_dates: vec![],
            }],
            satker: vec![],
        }
    }

    #[test]
    fn renders_schedule_summary_and_details() {
        let config = Config::test_config();
        let schedules = vec![CoverageSchedule {
            code: "QW1.1".to_string(),
            title: "Gatur Pagi <Polda>".to_string(),
            day: "Senin".to_string(),
            weekday: 1,
            expected_dates: vec!["2026-11-09".to_string()],
            polres: vec![
                unit("Polres Uji", "on_time"),
                unit("Polres Belum", "missing"),
            ],
            satker: vec![unit("Satker Uji", "off_schedule")],
        }];
        let data = ScheduleReportData {
            polda_name: "POLDA JATENG",
            period_label: "9–15 November 2026",
            period_start: "2026-11-09",
            period_end: "2026-11-15",
            generated_at: "2026-11-13T02:00:00+00:00",
            source_filename: "contoh.xlsx",
            filter_polres: "",
            schedules: &schedules,
        };

        let html = render_schedule_report(&config, &data);
        assert!(html.contains("size: A4 landscape"));
        assert!(html.contains("Jadwal Pelaporan QW1.1–QW1.8"));
        assert!(html.contains("Gatur Pagi &lt;Polda&gt;"));
        assert!(!html.contains("Gatur Pagi <Polda>"));
        assert!(html.contains("Polres Belum"));
        assert!(html.contains("<span class=\"status missing\">1 belum</span>"));
        assert!(html.contains("Polsek Uji (Belum melapor)"));
    }
}
