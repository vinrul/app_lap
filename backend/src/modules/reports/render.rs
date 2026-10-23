use std::sync::OnceLock;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{Datelike, NaiveDate};

use crate::{
    config::Config,
    modules::reports::models::{GroupMetrics, ReportContent, ReportMetrics, RowSummary},
    shared::{escape_html, format_count, is_noise, paragraphs},
};

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

const WEEKDAYS: [&str; 7] = [
    "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu", "Minggu",
];

pub fn render(metrics: &ReportMetrics, content: &ReportContent, config: &Config) -> String {
    let cover = render_cover(metrics, content);

    let narkoba = page(
        Some(2),
        metrics,
        "PROGRAM PRIORITAS 4 · BERANTAS NARKOBA",
        "Berantas Narkoba",
        &format!(
            "{} laporan kegiatan dari {} polres pada periode {}.",
            format_count(metrics.narkoba.rows),
            format_count(metrics.narkoba.by_polres.len() as i64),
            metrics.period_label
        ),
        &render_narkoba(metrics, content),
        "",
    );

    let green = page(
        Some(3),
        metrics,
        "QUICK WINS · GREEN POLICING",
        "Green Policing",
        &format!(
            "{} kegiatan penghijauan dengan {} bibit dan area terkelola {} m².",
            format_count(metrics.green_policing.rows),
            format_count(metrics.green_policing.bibit),
            format_count(metrics.green_policing.area_m2)
        ),
        &render_green(metrics, content),
        "",
    );

    let jumat = page(
        Some(4),
        metrics,
        "QUICK WINS · JUMAT KELILING",
        "Jumat Keliling",
        &format!(
            "{} jemaah dijangkau di {} masjid oleh {} polres dan {} polsek.",
            format_count(metrics.jumat_keliling.people),
            format_count(metrics.jumat_keliling.locations),
            format_count(metrics.jumat_keliling.polres),
            format_count(metrics.jumat_keliling.polsek)
        ),
        &render_religious(
            metrics,
            &metrics.jumat_keliling,
            &content.jumat_keliling,
            "Masjid",
            false,
        ),
        "",
    );

    let subuh = page(
        Some(5),
        metrics,
        "QUICK WINS · SUBUH KELILING",
        "Subuh Keliling",
        &format!(
            "{} jemaah dijangkau di {} masjid oleh {} polres dan {} polsek.",
            format_count(metrics.subuh_keliling.people),
            format_count(metrics.subuh_keliling.locations),
            format_count(metrics.subuh_keliling.polres),
            format_count(metrics.subuh_keliling.polsek)
        ),
        &render_religious(
            metrics,
            &metrics.subuh_keliling,
            &content.subuh_keliling,
            "Masjid",
            false,
        ),
        "",
    );

    let minggu = page(
        Some(6),
        metrics,
        "QUICK WINS · MINGGU KASIH",
        "Minggu Kasih",
        &format!(
            "{} jemaat dijangkau di {} lokasi ibadah oleh {} polres dan {} polsek.",
            format_count(metrics.minggu_kasih.people),
            format_count(metrics.minggu_kasih.locations),
            format_count(metrics.minggu_kasih.polres),
            format_count(metrics.minggu_kasih.polsek)
        ),
        &render_religious(
            metrics,
            &metrics.minggu_kasih,
            &content.minggu_kasih,
            "Lokasi ibadah",
            true,
        ),
        "",
    );

    let aspirations = page(
        Some(7),
        metrics,
        "ASPIRASI WARGA · SUBUH KELILING & MINGGU KASIH",
        "Aspirasi Masyarakat",
        "Tema aspirasi yang muncul dari kegiatan Jumat Keliling, Subuh Keliling, dan Minggu Kasih.",
        &render_aspirations(metrics, content),
        "",
    );

    let follow_up = page(
        Some(8),
        metrics,
        "TINDAK LANJUT ASPIRASI · KEGIATAN GABUNGAN / KRYD",
        "Menjawab Aspirasi Masyarakat",
        "Tindak lanjut lintas fungsi atas keluhan dan masukan yang disampaikan masyarakat.",
        &render_follow_up(metrics, content),
        "",
    );

    let media = page(
        None,
        metrics,
        "AMPLIFIKASI MEDIA PER KEGIATAN",
        "Amplifikasi Media",
        "Rekap publikasi dan dokumentasi yang dilampirkan pada laporan satuan wilayah.",
        &render_media(metrics, content, config),
        "last-page",
    );

    let css = REPORT_CSS.replace("__LOGO_DATA_URI__", logo_data_uri());
    format!(
        "<!doctype html><html lang=\"id\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{} · {}</title><style>{}</style></head><body>{}{}{}{}{}{}{}{}{}</body></html>",
        escape_html(&config.app_name),
        escape_html(&metrics.period_label),
        css,
        cover,
        narkoba,
        green,
        jumat,
        subuh,
        minggu,
        aspirations,
        follow_up,
        media,
    )
}

fn render_cover(metrics: &ReportMetrics, content: &ReportContent) -> String {
    let mut cards = String::new();
    for (index, card) in metrics.cards.iter().enumerate() {
        cards.push_str(&format!(
            "<div class=\"cover-kpi tone-{}\"><span class=\"cover-kpi-index\">{:02}</span>\
             <span class=\"cover-kpi-label\">{}</span>\
             <strong>{}</strong><span class=\"cover-kpi-unit\">{}</span>\
             <span class=\"cover-kpi-detail\">{}</span></div>",
            index % 3,
            index + 1,
            escape_html(&card.label),
            format_count(card.value),
            escape_html(&card.unit),
            escape_html(&card.detail),
        ));
    }

    let summary = editorial(
        &content.summary,
        &format!(
            "Rekap pelaksanaan Program Prioritas dan Quick Wins Kapolri periode {} di lingkungan {}. \
Data disusun dari {} baris laporan yang diimpor dari {}.",
            metrics.period_label,
            metrics.polda_name,
            format_count(metrics.total_rows),
            metrics.source_filename
        ),
    );

    format!(
        "<section class=\"page cover\">\
  <div class=\"cover-top\">\
    <div class=\"cover-brand\"><span class=\"crest-logo cover-crest\" role=\"img\" aria-label=\"Logo Polda Bali\"></span><span><strong>{polda}</strong><small>LAPORAN MINGGUAN PBM</small></span></div>\
    <span class=\"cover-doc\">PROGRAM PRIORITAS &amp; QUICK WINS KAPOLRI</span>\
    <h1>Laporan Mingguan<br>Pelaksanaan</h1>\
    <p class=\"cover-period\">{label}</p>\
    <p class=\"cover-source\">Sumber data: {source} · {rows} baris laporan</p>\
  </div>\
  <div class=\"cover-kpis\">{cards}</div>\
  <div class=\"cover-summary\"><span class=\"eyebrow gold\">RINGKASAN PELAKSANAAN</span>{summary}</div>\
  <div class=\"cover-footer\"><span>LAPORAN MINGGUAN PBM · {polda}</span><span>{label}</span></div>\
</section>",
        polda = escape_html(&metrics.polda_name),
        label = escape_html(&metrics.period_label),
        source = escape_html(&metrics.source_filename),
        rows = format_count(metrics.total_rows),
        cards = cards,
        summary = summary,
    )
}

fn render_narkoba(metrics: &ReportMetrics, content: &ReportContent) -> String {
    let mut body = String::new();
    body.push_str(&kpi_row(&[
        ("Orang ditangkap", metrics.narkoba.orang_diamankan, "orang"),
        ("Kawasan narkoba", metrics.narkoba.kawasan, "kawasan"),
        ("Rehabilitasi", metrics.narkoba.rehabilitasi, "orang"),
        (
            "Peserta penyuluhan",
            metrics.narkoba.peserta_penyuluhan,
            "peserta",
        ),
    ]));
    body.push_str(&editorial_block(
        "Catatan pelaksanaan",
        &content.narkoba,
        "Bagian ini dapat diisi dengan konteks penindakan, hasil assault, dan rencana pemulihan kawasan.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel\"><h3>Sebaran per polres</h3>{}</div>",
        group_table(&metrics.narkoba.by_polres, false)
    ));
    body.push_str(&format!(
        "<div class=\"panel\"><h3>Jenis kegiatan</h3>{}</div>",
        bar_list(&metrics.narkoba.by_code, 8)
    ));
    body.push_str("</div>");
    body.push_str(&activity_panel(
        "Kegiatan menonjol",
        &metrics.narkoba.activities,
    ));
    body
}

fn render_green(metrics: &ReportMetrics, content: &ReportContent) -> String {
    let green = &metrics.green_policing;
    let mut body = String::new();
    body.push_str(&kpi_row(&[
        ("Bibit ditanam", green.bibit, "bibit"),
        ("Kegiatan hijau", green.rows, "laporan"),
        ("Luas area", green.area_m2, "m²"),
        ("Kerja bakti", green.kerja_bakti, "kegiatan"),
    ]));
    body.push_str(&editorial_block(
        "Catatan pelaksanaan",
        &content.green_policing,
        "Uraikan lokasi prioritas, jenis tanaman, kolaborasi dengan pemda, serta pemeliharaan pasca-tanam.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel\"><h3>Bibit per polres</h3>{}</div>",
        group_table(&green.by_polres, true)
    ));
    body.push_str(&format!(
        "<div class=\"panel\"><h3>Jenis kegiatan</h3>{}</div>",
        bar_list(&green.by_kegiatan, 8)
    ));
    body.push_str("</div>");
    body.push_str(&format!(
        "<div class=\"note-strip\"><strong>{}</strong><span> kegiatan penanaman · {} peserta terlibat · {} m² area ditangani</span></div>",
        format_count(green.penanaman),
        format_count(green.peserta),
        format_count(green.area_m2)
    ));
    body
}

fn render_religious(
    metrics: &ReportMetrics,
    section: &crate::modules::reports::models::ReligiousMetrics,
    editorial_text: &str,
    location_label: &str,
    show_religion: bool,
) -> String {
    let mut body = String::new();
    body.push_str(&kpi_row(&[
        ("Jemaah / jemaat", section.people, "orang"),
        (location_label, section.locations, "lokasi"),
        ("Polres", section.polres, "satuan"),
        ("Polsek", section.polsek, "satuan"),
    ]));
    body.push_str(&editorial_block(
        "Catatan pelaksanaan",
        editorial_text,
        "Lengkapi dengan pesan kamtibmas yang disampaikan, tokoh yang ditemui, dan aspirasi utama warga.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel wide\"><h3>Sebaran per polres</h3>{}</div>",
        group_table(&section.by_polres, false)
    ));
    let mut aside = String::new();
    if show_religion {
        aside.push_str(&format!(
            "<h3>Agama / lokasi ibadah</h3>{}",
            bar_list(&section.by_religion, 6)
        ));
    } else {
        aside.push_str(&format!(
            "<h3>{}</h3>{}",
            location_label,
            bar_list(&section.top_locations, 6)
        ));
    }
    body.push_str(&format!("<div class=\"panel\">{aside}</div>"));
    body.push_str("</div>");
    body.push_str(&format!(
        "<div class=\"note-strip\"><strong>{}</strong><span> laporan kegiatan keagamaan dalam periode {} · total {} orang dijangkau</span></div>",
        format_count(section.rows),
        escape_html(&metrics.period_label),
        format_count(section.people)
    ));
    body
}

fn render_aspirations(metrics: &ReportMetrics, content: &ReportContent) -> String {
    let aspirations = &metrics.aspirations;
    let completed: i64 = aspirations
        .statuses
        .iter()
        .filter(|item| {
            let label = item.label.to_ascii_lowercase();
            label.contains("selesai") || label.contains("ditangani") || label.contains("diterima")
        })
        .map(|item| item.count)
        .sum();

    let mut body = String::new();
    body.push_str(&kpi_row(&[
        ("Aspirasi tercatat", aspirations.total, "masukan"),
        ("Tema utama", aspirations.themes.len() as i64, "tema"),
        ("Sudah ditindaklanjuti", completed, "masukan"),
        (
            "Sumber kegiatan",
            aspirations.by_source.len() as i64,
            "kegiatan",
        ),
    ]));
    body.push_str(&editorial_block(
        "Catatan pelaksanaan",
        &content.aspirations,
        "Rangkum pola aspirasi lintas kegiatan dan tentukan prioritas tindak lanjut bersama satuan wilayah.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel wide\"><h3>Tema aspirasi masyarakat</h3>{}</div>",
        aspiration_table(&aspirations.themes)
    ));
    body.push_str(&format!(
        "<div class=\"panel\"><h3>Status tindak lanjut</h3>{}</div>",
        bar_list(&aspirations.statuses, 6)
    ));
    body.push_str("</div>");
    if !aspirations.quotes.is_empty() {
        body.push_str("<div class=\"quote-grid\">");
        for quote in &aspirations.quotes {
            body.push_str(&format!(
                "<div class=\"quote\">“{}”</div>",
                escape_html(quote)
            ));
        }
        body.push_str("</div>");
    }
    body
}

fn render_follow_up(metrics: &ReportMetrics, content: &ReportContent) -> String {
    let follow_up = &metrics.follow_up;
    let mut body = String::new();
    body.push_str(&kpi_row(&[
        ("Tindak lanjut", follow_up.total, "masukan"),
        ("Bentuk tindakan", follow_up.forms.len() as i64, "jenis"),
        (
            "Kegiatan gabungan",
            follow_up.activities.len() as i64,
            "kegiatan",
        ),
        ("Total laporan", metrics.total_rows, "baris"),
    ]));
    body.push_str(&editorial_block(
        "Catatan pelaksanaan",
        &content.follow_up,
        "Jelaskan kegiatan gabungan, KRYD, dan koordinasi lintas instansi yang dilakukan untuk menjawab aspirasi.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel\"><h3>Bentuk tindak lanjut</h3>{}</div>",
        bar_list(&follow_up.forms, 8)
    ));
    body.push_str(&format!(
        "<div class=\"panel wide\"><h3>Kegiatan tindak lanjut</h3>{}</div>",
        activity_table(&follow_up.activities)
    ));
    body.push_str("</div>");
    body
}

fn render_media(metrics: &ReportMetrics, content: &ReportContent, config: &Config) -> String {
    let media = &metrics.media;
    let mut body = String::new();
    body.push_str(&kpi_row(&[
        ("Publikasi", media.publications, "tautan"),
        ("Dokumentasi", media.documentation, "berkas"),
        ("Kegiatan teramplifikasi", media.rows_with_media, "kegiatan"),
        ("Kanal publikasi", media.by_channel.len() as i64, "kanal"),
    ]));
    body.push_str(&editorial_block(
        "Catatan pelaksanaan",
        &content.media,
        "Rekap kanal pemberitaan, akun media sosial, dan tautan dokumentasi yang paling banyak digunakan.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel\"><h3>Publikasi per kanal</h3>{}</div>",
        bar_list(&media.by_channel, 6)
    ));
    body.push_str(&format!(
        "<div class=\"panel\"><h3>Publikasi per kegiatan</h3>{}</div>",
        bar_list(&media.by_activity, 6)
    ));
    body.push_str("</div>");
    body.push_str(&format!(
        "<div class=\"note-strip\"><strong>{}</strong><span> total tautan terverifikasi · {} dokumentasi kegiatan · {} kegiatan memiliki publikasi</span></div>",
        format_count(media.total),
        format_count(media.documentation),
        format_count(media.rows_with_media)
    ));

    let closing = editorial(
        &content.closing,
        "Demikian laporan mingguan pelaksanaan Program Prioritas dan Quick Wins Kapolri ini disusun sebagai bahan evaluasi pimpinan dan tindak lanjut satuan wilayah.",
    );
    body.push_str(&format!(
        "<div class=\"closing\"><h3>Penutup</h3>{}</div>",
        closing
    ));

    let city = if content.report_city.trim().is_empty() {
        "Denpasar"
    } else {
        content.report_city.trim()
    };
    let report_date = if content.report_date.trim().is_empty() {
        metrics.period_end.clone()
    } else {
        content.report_date.trim().to_string()
    };
    let name = if content.signatory_name.trim().is_empty() {
        "……………………………………"
    } else {
        content.signatory_name.trim()
    };
    let rank = content.signatory_rank.trim();
    let nip = content.signatory_nip.trim();

    body.push_str(&format!(
        "<div class=\"signature\">\
           <div class=\"signature-place\">{}, {}</div>\
           <div class=\"signature-title\">{}</div>\
           <div class=\"signature-space\"></div>\
           <div class=\"signature-name\">{}</div>\
           {}{}\
         </div>",
        escape_html(city),
        escape_html(&date_long(&report_date)),
        escape_html(if rank.is_empty() { "Pimpinan" } else { rank }),
        escape_html(name),
        if nip.is_empty() {
            String::new()
        } else {
            format!(
                "<div class=\"signature-nip\">NIP. {}</div>",
                escape_html(nip)
            )
        },
        format!(
            "<div class=\"signature-foot\">{}</div>",
            escape_html(&config.polda_name)
        )
    ));

    body
}

fn page(
    number: Option<usize>,
    metrics: &ReportMetrics,
    eyebrow: &str,
    title: &str,
    subtitle: &str,
    body: &str,
    extra_class: &str,
) -> String {
    let page_number = number
        .map(|value| format!("<span>Halaman {value}</span>"))
        .unwrap_or_default();
    format!(
        "<section class=\"page {extra_class}\">\
  <header class=\"page-head\"><div class=\"page-head-text\"><span class=\"eyebrow gold\">{eyebrow}</span><h1>{title}</h1><p>{subtitle}</p></div><span class=\"crest-logo page-crest\" role=\"img\" aria-label=\"Logo Polda Bali\"></span></header>\
  <div class=\"page-body\">{body}</div>\
  <footer class=\"page-foot\"><span>LAPORAN MINGGUAN PBM · {polda} · {period}</span>{page_number}</footer>\
</section>",
        extra_class = extra_class,
        eyebrow = escape_html(eyebrow),
        title = escape_html(title),
        subtitle = escape_html(subtitle),
        body = body,
        polda = escape_html(&metrics.polda_name),
        period = escape_html(&metrics.period_label),
        page_number = page_number,
    )
}

fn kpi_row(items: &[(&str, i64, &str)]) -> String {
    let mut output = String::from("<div class=\"kpi-row\">");
    for (label, value, unit) in items {
        output.push_str(&format!(
            "<div class=\"kpi\"><span class=\"kpi-label\">{}</span><strong>{}</strong><span class=\"kpi-unit\">{}</span></div>",
            escape_html(label),
            format_count(*value),
            escape_html(unit)
        ));
    }
    output.push_str("</div>");
    output
}

fn editorial_block(title: &str, text: &str, fallback: &str) -> String {
    format!(
        "<div class=\"editorial-block\"><h3>{}</h3>{}</div>",
        escape_html(title),
        editorial(text, fallback)
    )
}

fn editorial(text: &str, fallback: &str) -> String {
    if is_noise(text) {
        format!("<p class=\"muted\">{}</p>", escape_html(fallback))
    } else {
        paragraphs(text)
    }
}

fn bar_list(items: &[crate::shared::LabeledCount], limit: usize) -> String {
    if items.is_empty() {
        return "<p class=\"muted\">Belum ada data yang dapat ditampilkan.</p>".to_string();
    }
    let max = items
        .iter()
        .map(|item| item.count)
        .max()
        .unwrap_or(1)
        .max(1);
    let mut output = String::from("<div class=\"bars\">");
    for item in items.iter().take(limit) {
        let width = (item.count as f64 / max as f64 * 100.0).clamp(2.0, 100.0);
        output.push_str(&format!(
            "<div class=\"bar-row\"><span class=\"bar-label\">{}</span><span class=\"bar-track\"><span class=\"bar-fill\" style=\"width:{width:.1}%\"></span></span><span class=\"bar-value\">{}</span></div>",
            escape_html(&item.label),
            format_count(item.count)
        ));
    }
    output.push_str("</div>");
    output
}

fn group_table(items: &[GroupMetrics], show_bibit: bool) -> String {
    if items.is_empty() {
        return "<p class=\"muted\">Belum ada data sebaran wilayah.</p>".to_string();
    }
    let show_people = items.iter().any(|item| item.people != 0);
    let show_locations = items.iter().any(|item| item.locations != 0);
    let show_bibit = show_bibit && items.iter().any(|item| item.bibit != 0);

    let mut output = String::from(
        "<table class=\"data\"><thead><tr><th>Polres</th><th class=\"num\">Laporan</th>",
    );
    if show_people {
        output.push_str("<th class=\"num\">Orang</th>");
    }
    if show_locations {
        output.push_str("<th class=\"num\">Lokasi</th>");
    }
    if show_bibit {
        output.push_str("<th class=\"num\">Bibit</th>");
    }
    output.push_str("</tr></thead><tbody>");
    for item in items.iter().take(12) {
        output.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{}</td>",
            escape_html(&shorten(&item.label, 34)),
            format_count(item.rows)
        ));
        if show_people {
            output.push_str(&format!(
                "<td class=\"num\">{}</td>",
                format_count(item.people)
            ));
        }
        if show_locations {
            output.push_str(&format!(
                "<td class=\"num\">{}</td>",
                format_count(item.locations)
            ));
        }
        if show_bibit {
            output.push_str(&format!(
                "<td class=\"num\">{}</td>",
                format_count(item.bibit)
            ));
        }
        output.push_str("</tr>");
    }
    output.push_str("</tbody></table>");
    output
}

fn aspiration_table(items: &[crate::shared::LabeledCount]) -> String {
    if items.is_empty() {
        return "<p class=\"muted\">Belum ada aspirasi yang tercatat.</p>".to_string();
    }
    let mut output = String::from(
        "<table class=\"data compact\"><thead><tr><th>Tema aspirasi</th><th class=\"num\">Jumlah</th></tr></thead><tbody>",
    );
    for item in items.iter().take(10) {
        output.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{}</td></tr>",
            escape_html(&item.label),
            format_count(item.count)
        ));
    }
    output.push_str("</tbody></table>");
    output
}

fn activity_panel(title: &str, items: &[RowSummary]) -> String {
    if items.is_empty() {
        return String::new();
    }
    format!(
        "<div class=\"panel\"><h3>{}</h3>{}</div>",
        escape_html(title),
        activity_table(items)
    )
}

fn activity_table(items: &[RowSummary]) -> String {
    if items.is_empty() {
        return "<p class=\"muted\">Belum ada kegiatan menonjol yang dapat ditampilkan.</p>"
            .to_string();
    }
    let mut output = String::from(
        "<table class=\"data compact\"><thead><tr><th>Tanggal</th><th>Satuan</th><th>Kegiatan</th><th>Lokasi</th></tr></thead><tbody>",
    );
    for item in items.iter().take(6) {
        let activity = if item.detail.trim().is_empty() {
            item.kegiatan.clone()
        } else {
            format!("{} · {}", item.kegiatan, item.detail)
        };
        output.push_str(&format!(
            "<tr><td class=\"nowrap\">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&date_short(&item.tanggal)),
            escape_html(&shorten(&item.satuan, 26)),
            escape_html(&shorten(&activity, 58)),
            escape_html(&shorten(&item.location, 36))
        ));
    }
    output.push_str("</tbody></table>");
    output
}

fn shorten(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        let mut output: String = value.chars().take(max_chars.saturating_sub(1)).collect();
        output.push('…');
        output
    }
}

fn date_long(value: &str) -> String {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|date| {
            format!(
                "{}, {} {} {}",
                WEEKDAYS[date.weekday().num_days_from_monday() as usize],
                date.day(),
                MONTHS[(date.month0()) as usize],
                date.year()
            )
        })
        .unwrap_or_else(|_| value.to_string())
}

fn logo_data_uri() -> &'static str {
    static LOGO: OnceLock<String> = OnceLock::new();
    LOGO.get_or_init(|| {
        format!(
            "data:image/png;base64,{}",
            STANDARD.encode(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../logo_polda_bali.png"
            )))
        )
    })
}

fn date_short(value: &str) -> String {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|date| {
            format!(
                "{} {} {}",
                date.day(),
                &MONTHS[(date.month0()) as usize][..3],
                date.year()
            )
        })
        .unwrap_or_else(|_| value.to_string())
}

const REPORT_CSS: &str = r#"
:root { --logo: url("__LOGO_DATA_URI__"); }
@page { size: A4; margin: 0; }
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; }
body {
  background: #e7e9e5;
  color: #1f2a24;
  font-family: "Segoe UI", Arial, sans-serif;
  font-size: 9.2pt;
  line-height: 1.45;
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}
.page {
  width: 210mm;
  height: 297mm;
  margin: 0 auto 10px;
  padding: 15mm 17mm 14mm;
  background: #fff;
  position: relative;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  page-break-after: always;
  box-shadow: 0 2px 14px rgba(20, 40, 30, .12);
}
.page:last-child { page-break-after: auto; }
.page-body { flex: 1; min-height: 0; }
.eyebrow {
  display: block;
  font-size: 8pt;
  font-weight: 700;
  letter-spacing: .18em;
  text-transform: uppercase;
  color: #a4743d;
}
.eyebrow.gold { color: #a4743d; }
h1, h2, h3 { font-family: Georgia, "Times New Roman", serif; color: #143d30; margin: 0; }
.page-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 5mm; }
.page-head-text { flex: 1; min-width: 0; }
.page-head h1 { font-size: 25pt; font-weight: 500; line-height: 1.08; margin: 3mm 0 2mm; }
.page-head p { margin: 0 0 4mm; color: #5c6962; font-size: 9.5pt; }
.crest-logo {
  background-image: var(--logo);
  background-position: center;
  background-repeat: no-repeat;
  background-size: contain;
}
.page-crest { width: 10mm; height: 14mm; flex: none; margin-top: 1mm; }
h3 { font-size: 11pt; margin-bottom: 2.5mm; font-weight: 600; }
p { margin: 0 0 2.5mm; }
.muted { color: #7a857f; font-style: italic; }
.page-foot {
  display: flex;
  justify-content: space-between;
  border-top: .6pt solid #d7ddd6;
  padding-top: 3mm;
  color: #6d7872;
  font-size: 7.4pt;
  letter-spacing: .04em;
}
.cover {
  padding: 0;
  background: #fff;
}
.cover-top {
  background: #143d30;
  color: #fff;
  padding: 18mm 18mm 12mm;
  position: relative;
}
.cover-brand { display: flex; align-items: center; gap: 4mm; margin-bottom: 16mm; }
.cover-crest {
  width: 14mm; height: 19mm; flex: none;
  background-color: #fff; border-radius: 1.5mm; padding: 1mm;
  box-shadow: 0 0 0 .8pt rgba(217,184,126,.55);
}
.cover-brand strong { display: block; font-size: 11pt; letter-spacing: .12em; }
.cover-brand small { display: block; font-size: 7.6pt; letter-spacing: .22em; color: #c9d7ce; margin-top: 1mm; }
.cover-doc { font-size: 8pt; letter-spacing: .18em; color: #d9b87e; font-weight: 700; }
.cover h1 { color: #fff; font-size: 32pt; line-height: 1.05; font-weight: 400; margin: 5mm 0 5mm; max-width: 145mm; }
.cover-period { font-size: 14pt; color: #f3dfb8; font-weight: 600; margin: 0; }
.cover-source { margin: 2mm 0 0; color: #b9c9bf; font-size: 8pt; }
.cover-kpis {
  display: grid; grid-template-columns: repeat(3, 1fr);
  gap: 4mm; padding: 10mm 18mm 0;
}
.cover-kpi {
  border: .7pt solid #dfe5df; border-radius: 2mm; padding: 4mm 4mm 3.5mm;
  min-height: 33mm; display: flex; flex-direction: column;
  background: #fbfcfa;
}
.cover-kpi-index { font-size: 7pt; color: #a4743d; font-weight: 700; letter-spacing: .12em; }
.cover-kpi-label { font-size: 8.4pt; color: #46534b; font-weight: 600; margin-top: 1mm; }
.cover-kpi strong { font-family: Georgia, serif; font-size: 21pt; color: #143d30; font-weight: 500; line-height: 1.05; margin-top: auto; }
.cover-kpi-unit { font-size: 7.6pt; color: #5b6861; }
.cover-kpi-detail { font-size: 7.2pt; color: #7a857f; margin-top: 1.5mm; line-height: 1.3; }
.cover-summary { padding: 8mm 18mm 0; flex: 1; }
.cover-summary p { font-size: 9.4pt; color: #37443d; margin-top: 2mm; }
.cover-footer {
  display: flex; justify-content: space-between; padding: 0 18mm 12mm;
  color: #7a857f; font-size: 7.4pt; letter-spacing: .06em;
}
.kpi-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 3mm; margin-bottom: 5mm; }
.kpi {
  border-left: 2pt solid #b8894b; background: #f6f8f5; border-radius: 1.5mm;
  padding: 3mm 3.5mm; display: flex; flex-direction: column;
}
.kpi-label { font-size: 7.6pt; color: #5b6861; font-weight: 600; }
.kpi strong { font-family: Georgia, serif; font-size: 18pt; font-weight: 500; color: #143d30; line-height: 1.1; }
.kpi-unit { font-size: 7.2pt; color: #7a857f; }
.editorial-block {
  border: .7pt solid #e1e6e0; border-radius: 1.5mm; padding: 3.5mm 4mm;
  margin-bottom: 4mm; background: #fcfdfb;
}
.editorial-block h3 { font-size: 9.5pt; margin-bottom: 1.5mm; }
.editorial-block p { margin-bottom: 1.4mm; font-size: 9pt; }
.two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 4mm; margin-bottom: 4mm; }
.two-col .panel.wide { grid-column: span 1; }
.panel {
  border: .7pt solid #e1e6e0; border-radius: 1.5mm; padding: 3.5mm 4mm;
  background: #fff; margin-bottom: 4mm;
}
.two-col .panel { margin-bottom: 0; }
.panel h3 { font-size: 9.4pt; }
table.data { width: 100%; border-collapse: collapse; font-size: 7.8pt; }
table.data th {
  text-align: left; font-size: 6.9pt; letter-spacing: .06em; text-transform: uppercase;
  color: #6d7872; border-bottom: .7pt solid #cfd7cf; padding: 1.5mm 1.5mm;
}
table.data td { padding: 1.6mm 1.5mm; border-bottom: .5pt solid #e8ece7; vertical-align: top; }
table.data tr:last-child td { border-bottom: 0; }
table.data .num { text-align: right; white-space: nowrap; }
table.data.compact td { padding: 1.3mm 1.5mm; }
.nowrap { white-space: nowrap; }
.bars { display: flex; flex-direction: column; gap: 1.8mm; }
.bar-row { display: grid; grid-template-columns: 35% 1fr 12mm; gap: 2mm; align-items: center; }
.bar-label { font-size: 7.6pt; color: #46534b; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.bar-track { height: 2.5mm; background: #eef1ec; border-radius: 1mm; overflow: hidden; }
.bar-fill { display: block; height: 100%; background: #3f765d; border-radius: 1mm; }
.bar-value { text-align: right; font-size: 7.4pt; font-weight: 700; color: #143d30; }
.note-strip {
  border-left: 2pt solid #b8894b; background: #f8f5ee; padding: 2.6mm 3.5mm;
  font-size: 8pt; color: #46534b; margin-bottom: 4mm;
}
.quote-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 3mm; }
.quote {
  background: #f6f8f5; border-left: 2pt solid #b8894b; padding: 3mm 3.5mm;
  font-size: 8pt; color: #37443d; font-style: italic;
}
.closing { margin-top: 2mm; }
.closing p { font-size: 8.8pt; }
.signature { margin-top: 5mm; margin-left: auto; width: 78mm; text-align: center; font-size: 8.4pt; }
.signature-place { color: #5b6861; }
.signature-title { margin-top: 1mm; }
.signature-space { height: 15mm; }
.signature-name { font-weight: 700; text-decoration: underline; }
.signature-nip { color: #5b6861; }
.signature-foot { margin-top: 2mm; font-size: 7.4pt; letter-spacing: .08em; color: #7a857f; }
.last-page .page-foot span:last-child { visibility: hidden; }
@media print {
  body { background: #fff; }
  .page { margin: 0; box-shadow: none; }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::reports::models::{MetricCard, ReportMetrics};

    #[test]
    fn renders_nine_pages_and_escapes_editorial_content() {
        let mut metrics = ReportMetrics::default();
        metrics.period_label = "16–18 Oktober 2026".into();
        metrics.polda_name = "POLDA BALI".into();
        metrics.source_filename = "report.xlsx".into();
        metrics.cards = vec![MetricCard {
            key: "narkoba".into(),
            label: "Berantas Narkoba".into(),
            value: 37,
            unit: "orang ditangkap".into(),
            detail: "89 kegiatan".into(),
        }];
        let content = ReportContent {
            summary: "<script>alert(1)</script>".into(),
            ..ReportContent::default()
        };
        let html = render(&metrics, &content, &Config::test_config());
        assert_eq!(html.matches("<section class=\"page").count(), 9);
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("POLDA BALI"));
    }
}
