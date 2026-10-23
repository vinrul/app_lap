use std::{collections::HashMap, sync::OnceLock};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{Datelike, NaiveDate};

use crate::{
    config::Config,
    modules::reports::{
        layout::{DesignSource, LayoutElement, ReportLayout, image_id_from_src, valid_id},
        models::{GroupMetrics, ReportContent, ReportMetrics, RowSummary},
        sources::design_sources,
    },
    shared::{escape_html, format_count, is_noise, paragraphs},
};

struct DesignContext<'a> {
    layout: &'a ReportLayout,
    images: &'a HashMap<String, String>,
    sources: &'a [DesignSource],
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

const WEEKDAYS: [&str; 7] = [
    "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu", "Minggu",
];

#[allow(dead_code)]
pub fn render(metrics: &ReportMetrics, content: &ReportContent, config: &Config) -> String {
    render_with_layout(
        metrics,
        content,
        config,
        &ReportLayout::default(),
        &HashMap::new(),
    )
}

pub fn render_with_layout(
    metrics: &ReportMetrics,
    content: &ReportContent,
    config: &Config,
    layout: &ReportLayout,
    images: &HashMap<String, String>,
) -> String {
    let sources = design_sources(metrics);
    let ctx = DesignContext {
        layout,
        images,
        sources: &sources,
    };
    let cover = render_cover(metrics, content, &ctx);

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
        &render_narkoba(metrics, content, &ctx),
        "",
        &ctx,
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
        &render_green(metrics, content, &ctx),
        "",
        &ctx,
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
            4,
            &ctx,
        ),
        "",
        &ctx,
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
            5,
            &ctx,
        ),
        "",
        &ctx,
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
            6,
            &ctx,
        ),
        "",
        &ctx,
    );

    let aspirations = page(
        Some(7),
        metrics,
        "ASPIRASI WARGA · SUBUH KELILING & MINGGU KASIH",
        "Aspirasi Masyarakat",
        "Tema aspirasi yang muncul dari kegiatan Jumat Keliling, Subuh Keliling, dan Minggu Kasih.",
        &render_aspirations(metrics, content, &ctx),
        "",
        &ctx,
    );

    let follow_up = page(
        Some(8),
        metrics,
        "TINDAK LANJUT ASPIRASI · KEGIATAN GABUNGAN / KRYD",
        "Menjawab Aspirasi Masyarakat",
        "Tindak lanjut lintas fungsi atas keluhan dan masukan yang disampaikan masyarakat.",
        &render_follow_up(metrics, content, &ctx),
        "",
        &ctx,
    );

    let media = page(
        None,
        metrics,
        "AMPLIFIKASI MEDIA PER KEGIATAN",
        "Amplifikasi Media",
        "Rekap publikasi dan dokumentasi yang dilampirkan pada laporan satuan wilayah.",
        &render_media(metrics, content, config, &ctx),
        "last-page",
        &ctx,
    );

    let css = format!(
        "{}{}",
        REPORT_CSS.replace("__LOGO_DATA_URI__", logo_data_uri()),
        block_css(layout)
    );
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

fn render_cover(metrics: &ReportMetrics, content: &ReportContent, ctx: &DesignContext) -> String {
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

    let summary_fallback = format!(
        "Rekap pelaksanaan Program Prioritas dan Quick Wins Kapolri periode {} di lingkungan {}. \
Data disusun dari {} baris laporan yang diimpor dari {}.",
        metrics.period_label,
        metrics.polda_name,
        format_count(metrics.total_rows),
        metrics.source_filename
    );
    let summary_text =
        override_text(ctx, 1, "cover-summary", "body").unwrap_or_else(|| content.summary.clone());
    let summary = editorial(&summary_text, &summary_fallback);

    let polda_text = text_or(ctx, 1, "cover-top", "polda", &metrics.polda_name);
    let doc_text = text_or(
        ctx,
        1,
        "cover-top",
        "doc",
        "PROGRAM PRIORITAS & QUICK WINS KAPOLRI",
    );
    let title_text = text_or(
        ctx,
        1,
        "cover-top",
        "title",
        "Laporan Mingguan\nPelaksanaan",
    );
    let period_text = text_or(ctx, 1, "cover-top", "period", &metrics.period_label);
    let source_default = format!(
        "Sumber data: {} · {} baris laporan",
        metrics.source_filename,
        format_count(metrics.total_rows)
    );
    let source_text = text_or(ctx, 1, "cover-top", "source", &source_default);
    let footer_left_default = format!("LAPORAN MINGGUAN PBM · {}", metrics.polda_name);
    let footer_left = text_or(ctx, 1, "cover-footer", "left", &footer_left_default);
    let footer_right = text_or(ctx, 1, "cover-footer", "right", &metrics.period_label);

    format!(
        "<section class=\"page cover\">\
  <div class=\"cover-top\" data-block=\"cover-top\">\
    <div class=\"cover-brand\"><span class=\"crest-logo cover-crest\" role=\"img\" aria-label=\"Logo Polda Bali\"></span><span><strong data-text-id=\"polda\">{polda}</strong><small>LAPORAN MINGGUAN PBM</small></span></div>\
    <span class=\"cover-doc\" data-text-id=\"doc\">{doc}</span>\
    <h1 data-text-id=\"title\">{title}</h1>\
    <p class=\"cover-period\" data-text-id=\"period\">{label}</p>\
    <p class=\"cover-source\" data-text-id=\"source\">{source}</p>\
  </div>\
  <div class=\"cover-kpis\" data-block=\"cover-kpis\">{cards}</div>\
  <div class=\"cover-summary\" data-block=\"cover-summary\"><span class=\"eyebrow gold\">RINGKASAN PELAKSANAAN</span><div data-text-id=\"body\">{summary}</div></div>\
  <div class=\"cover-footer\" data-block=\"cover-footer\"><span data-text-id=\"left\">{footer_left}</span><span data-text-id=\"right\">{footer_right}</span></div>\
  {layer}\
</section>",
        polda = escape_html(&polda_text),
        doc = escape_html(&doc_text),
        title = text_with_breaks(&title_text),
        label = escape_html(&period_text),
        source = escape_html(&source_text),
        cards = cards,
        summary = summary,
        footer_left = escape_html(&footer_left),
        footer_right = escape_html(&footer_right),
        layer = layout_layer(1, ctx),
    )
}

fn render_narkoba(metrics: &ReportMetrics, content: &ReportContent, ctx: &DesignContext) -> String {
    let mut body = String::new();
    body.push_str(&kpi_row(
        &[
            ("Orang ditangkap", metrics.narkoba.orang_diamankan, "orang"),
            ("Kawasan narkoba", metrics.narkoba.kawasan, "kawasan"),
            ("Rehabilitasi", metrics.narkoba.rehabilitasi, "orang"),
            (
                "Peserta penyuluhan",
                metrics.narkoba.peserta_penyuluhan,
                "peserta",
            ),
        ],
        "page-2-kpi",
    ));
    body.push_str(&editorial_block(
        ctx,
        2,
        "page-2-editorial",
        "Catatan pelaksanaan",
        &content.narkoba,
        "Bagian ini dapat diisi dengan konteks penindakan, hasil assault, dan rencana pemulihan kawasan.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-2-panel-polres\"><h3>Sebaran per polres</h3>{}</div>",
        group_table(&metrics.narkoba.by_polres, false)
    ));
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-2-panel-kegiatan\"><h3>Jenis kegiatan</h3>{}</div>",
        bar_list(&metrics.narkoba.by_code, 8)
    ));
    body.push_str("</div>");
    body.push_str(&activity_panel(
        "Kegiatan menonjol",
        &metrics.narkoba.activities,
        "page-2-panel-activity",
    ));
    body
}

fn render_green(metrics: &ReportMetrics, content: &ReportContent, ctx: &DesignContext) -> String {
    let green = &metrics.green_policing;
    let mut body = String::new();
    body.push_str(&kpi_row(
        &[
            ("Bibit ditanam", green.bibit, "bibit"),
            ("Kegiatan hijau", green.rows, "laporan"),
            ("Luas area", green.area_m2, "m²"),
            ("Kerja bakti", green.kerja_bakti, "kegiatan"),
        ],
        "page-3-kpi",
    ));
    body.push_str(&editorial_block(
        ctx,
        3,
        "page-3-editorial",
        "Catatan pelaksanaan",
        &content.green_policing,
        "Uraikan lokasi prioritas, jenis tanaman, kolaborasi dengan pemda, serta pemeliharaan pasca-tanam.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-3-panel-polres\"><h3>Bibit per polres</h3>{}</div>",
        group_table(&green.by_polres, true)
    ));
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-3-panel-kegiatan\"><h3>Jenis kegiatan</h3>{}</div>",
        bar_list(&green.by_kegiatan, 8)
    ));
    body.push_str("</div>");
    body.push_str(&note_strip(
        ctx,
        3,
        "page-3-note",
        &format_count(green.penanaman),
        &format!(
            " kegiatan penanaman · {} peserta terlibat · {} m² area ditangani",
            format_count(green.peserta),
            format_count(green.area_m2)
        ),
    ));
    body
}

fn render_religious(
    metrics: &ReportMetrics,
    section: &crate::modules::reports::models::ReligiousMetrics,
    editorial_text: &str,
    location_label: &str,
    show_religion: bool,
    page_no: usize,
    ctx: &DesignContext,
) -> String {
    let mut body = String::new();
    body.push_str(&kpi_row(
        &[
            ("Jemaah / jemaat", section.people, "orang"),
            (location_label, section.locations, "lokasi"),
            ("Polres", section.polres, "satuan"),
            ("Polsek", section.polsek, "satuan"),
        ],
        &format!("page-{page_no}-kpi"),
    ));
    body.push_str(&editorial_block(
        ctx,
        page_no,
        &format!("page-{page_no}-editorial"),
        "Catatan pelaksanaan",
        editorial_text,
        "Lengkapi dengan pesan kamtibmas yang disampaikan, tokoh yang ditemui, dan aspirasi utama warga.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel wide\" data-block=\"page-{page_no}-panel-polres\"><h3>Sebaran per polres</h3>{}</div>",
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
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-{page_no}-panel-lokasi\">{aside}</div>"
    ));
    body.push_str("</div>");
    body.push_str(&note_strip(
        ctx,
        page_no,
        &format!("page-{page_no}-note"),
        &format_count(section.rows),
        &format!(
            " laporan kegiatan keagamaan dalam periode {} · total {} orang dijangkau",
            metrics.period_label,
            format_count(section.people)
        ),
    ));
    body
}

fn render_aspirations(
    metrics: &ReportMetrics,
    content: &ReportContent,
    ctx: &DesignContext,
) -> String {
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
    body.push_str(&kpi_row(
        &[
            ("Aspirasi tercatat", aspirations.total, "masukan"),
            ("Tema utama", aspirations.themes.len() as i64, "tema"),
            ("Sudah ditindaklanjuti", completed, "masukan"),
            (
                "Sumber kegiatan",
                aspirations.by_source.len() as i64,
                "kegiatan",
            ),
        ],
        "page-7-kpi",
    ));
    body.push_str(&editorial_block(
        ctx,
        7,
        "page-7-editorial",
        "Catatan pelaksanaan",
        &content.aspirations,
        "Rangkum pola aspirasi lintas kegiatan dan tentukan prioritas tindak lanjut bersama satuan wilayah.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel wide\" data-block=\"page-7-panel-tema\"><h3>Tema aspirasi masyarakat</h3>{}</div>",
        aspiration_table(&aspirations.themes)
    ));
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-7-panel-status\"><h3>Status tindak lanjut</h3>{}</div>",
        bar_list(&aspirations.statuses, 6)
    ));
    body.push_str("</div>");
    if !aspirations.quotes.is_empty() {
        body.push_str("<div class=\"quote-grid\">");
        for (index, quote) in aspirations.quotes.iter().enumerate() {
            body.push_str(&format!(
                "<div class=\"quote\" data-block=\"page-7-quote-{index}\">“{}”</div>",
                escape_html(quote)
            ));
        }
        body.push_str("</div>");
    }
    body
}

fn render_follow_up(
    metrics: &ReportMetrics,
    content: &ReportContent,
    ctx: &DesignContext,
) -> String {
    let follow_up = &metrics.follow_up;
    let mut body = String::new();
    body.push_str(&kpi_row(
        &[
            ("Tindak lanjut", follow_up.total, "masukan"),
            ("Bentuk tindakan", follow_up.forms.len() as i64, "jenis"),
            (
                "Kegiatan gabungan",
                follow_up.activities.len() as i64,
                "kegiatan",
            ),
            ("Total laporan", metrics.total_rows, "baris"),
        ],
        "page-8-kpi",
    ));
    body.push_str(&editorial_block(
        ctx,
        8,
        "page-8-editorial",
        "Catatan pelaksanaan",
        &content.follow_up,
        "Jelaskan kegiatan gabungan, KRYD, dan koordinasi lintas instansi yang dilakukan untuk menjawab aspirasi.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-8-panel-bentuk\"><h3>Bentuk tindak lanjut</h3>{}</div>",
        bar_list(&follow_up.forms, 8)
    ));
    body.push_str(&format!(
        "<div class=\"panel wide\" data-block=\"page-8-panel-kegiatan\"><h3>Kegiatan tindak lanjut</h3>{}</div>",
        activity_table(&follow_up.activities)
    ));
    body.push_str("</div>");
    body
}

fn render_media(
    metrics: &ReportMetrics,
    content: &ReportContent,
    config: &Config,
    ctx: &DesignContext,
) -> String {
    let media = &metrics.media;
    let mut body = String::new();
    body.push_str(&kpi_row(
        &[
            ("Publikasi", media.publications, "tautan"),
            ("Dokumentasi", media.documentation, "berkas"),
            ("Kegiatan teramplifikasi", media.rows_with_media, "kegiatan"),
            ("Kanal publikasi", media.by_channel.len() as i64, "kanal"),
        ],
        "page-9-kpi",
    ));
    body.push_str(&editorial_block(
        ctx,
        9,
        "page-9-editorial",
        "Catatan pelaksanaan",
        &content.media,
        "Rekap kanal pemberitaan, akun media sosial, dan tautan dokumentasi yang paling banyak digunakan.",
    ));
    body.push_str("<div class=\"two-col\">");
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-9-panel-kanal\"><h3>Publikasi per kanal</h3>{}</div>",
        bar_list(&media.by_channel, 6)
    ));
    body.push_str(&format!(
        "<div class=\"panel\" data-block=\"page-9-panel-kegiatan\"><h3>Publikasi per kegiatan</h3>{}</div>",
        bar_list(&media.by_activity, 6)
    ));
    body.push_str("</div>");
    body.push_str(&note_strip(
        ctx,
        9,
        "page-9-note",
        &format_count(media.total),
        &format!(
            " total tautan terverifikasi · {} dokumentasi kegiatan · {} kegiatan memiliki publikasi",
            format_count(media.documentation),
            format_count(media.rows_with_media)
        ),
    ));

    let closing = editorial(
        &content.closing,
        "Demikian laporan mingguan pelaksanaan Program Prioritas dan Quick Wins Kapolri ini disusun sebagai bahan evaluasi pimpinan dan tindak lanjut satuan wilayah.",
    );
    body.push_str(&format!(
        "<div class=\"closing\" data-block=\"page-9-closing\"><h3>Penutup</h3>{}</div>",
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

    let place_default = format!("{}, {}", city, date_long(&report_date));
    let place_text = text_or(ctx, 9, "page-9-signature", "place", &place_default);
    let title_default = if rank.is_empty() { "Pimpinan" } else { rank };
    let title_text = text_or(ctx, 9, "page-9-signature", "title", title_default);
    let name_text = text_or(ctx, 9, "page-9-signature", "name", name);
    let nip_default = if nip.is_empty() {
        String::new()
    } else {
        format!("NIP. {nip}")
    };
    let nip_text = text_or(ctx, 9, "page-9-signature", "nip", &nip_default);
    let foot_text = text_or(ctx, 9, "page-9-signature", "foot", &config.polda_name);
    let nip_html = if nip_text.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"signature-nip\" data-text-id=\"nip\">{}</div>",
            escape_html(&nip_text)
        )
    };

    body.push_str(&format!(
        "<div class=\"signature\" data-block=\"page-9-signature\">\
           <div class=\"signature-place\" data-text-id=\"place\">{}</div>\
           <div class=\"signature-title\" data-text-id=\"title\">{}</div>\
           <div class=\"signature-space\"></div>\
           <div class=\"signature-name\" data-text-id=\"name\">{}</div>\
           {}\
           <div class=\"signature-foot\" data-text-id=\"foot\">{}</div>\
         </div>",
        escape_html(&place_text),
        escape_html(&title_text),
        escape_html(&name_text),
        nip_html,
        escape_html(&foot_text)
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
    ctx: &DesignContext,
) -> String {
    let page_number = number
        .map(|value| format!("<span>Halaman {value}</span>"))
        .unwrap_or_default();
    let page_no = number.unwrap_or(9);
    let head_id = format!("page-{page_no}-head");
    let foot_id = format!("page-{page_no}-foot");
    let eyebrow_text = text_or(ctx, page_no, &head_id, "eyebrow", eyebrow);
    let title_text = text_or(ctx, page_no, &head_id, "title", title);
    let subtitle_text = text_or(ctx, page_no, &head_id, "subtitle", subtitle);
    let foot_default = format!(
        "LAPORAN MINGGUAN PBM · {} · {}",
        metrics.polda_name, metrics.period_label
    );
    let foot_text = text_or(ctx, page_no, &foot_id, "left", &foot_default);
    format!(
        "<section class=\"page {extra_class}\">\
  <header class=\"page-head\" data-block=\"page-{page_no}-head\"><div class=\"page-head-text\"><span class=\"eyebrow gold\" data-text-id=\"eyebrow\">{eyebrow}</span><h1 data-text-id=\"title\">{title}</h1><p data-text-id=\"subtitle\">{subtitle}</p></div><span class=\"crest-logo page-crest\" role=\"img\" aria-label=\"Logo Polda Bali\"></span></header>\
  <div class=\"page-body\">{body}</div>\
  <footer class=\"page-foot\" data-block=\"page-{page_no}-foot\"><span data-text-id=\"left\">{foot}</span>{page_number}</footer>\
  {layer}\
</section>",
        extra_class = extra_class,
        eyebrow = escape_html(&eyebrow_text),
        title = escape_html(&title_text),
        subtitle = escape_html(&subtitle_text),
        body = body,
        foot = escape_html(&foot_text),
        page_number = page_number,
        page_no = page_no,
        layer = layout_layer(page_no, ctx),
    )
}

fn override_text(
    ctx: &DesignContext,
    page_no: usize,
    block_id: &str,
    text_id: &str,
) -> Option<String> {
    let page = ctx.layout.pages.iter().find(|page| page.page == page_no)?;
    page.blocks.get(block_id)?.texts.get(text_id).cloned()
}

fn text_or(
    ctx: &DesignContext,
    page_no: usize,
    block_id: &str,
    text_id: &str,
    fallback: &str,
) -> String {
    override_text(ctx, page_no, block_id, text_id).unwrap_or_else(|| fallback.to_string())
}

fn text_with_breaks(value: &str) -> String {
    escape_html(value).replace('\n', "<br>")
}

fn layout_layer(page_no: usize, ctx: &DesignContext) -> String {
    let mut html = String::new();
    if let Some(page) = ctx.layout.pages.iter().find(|page| page.page == page_no) {
        for element in &page.elements {
            if element.hidden {
                continue;
            }
            html.push_str(&render_layout_element(element, ctx));
        }
    }
    format!("<div class=\"layout-layer\" data-layout-page=\"{page_no}\">{html}</div>")
}

fn render_layout_element(element: &LayoutElement, ctx: &DesignContext) -> String {
    let mut style = format!(
        "left:{:.3}mm;top:{:.3}mm;width:{:.3}mm;height:{:.3}mm;opacity:{:.3};z-index:{};",
        element.x, element.y, element.width, element.height, element.opacity, element.z
    );
    if element.rotation.abs() > 0.005 {
        style.push_str(&format!("transform:rotate({:.3}deg);", element.rotation));
    }

    match element.kind.as_str() {
        "image" => {
            let src = match image_id_from_src(&element.src) {
                Some(id) => ctx.images.get(id).cloned().unwrap_or_default(),
                None => element.src.clone(),
            };
            if src.is_empty() {
                return String::new();
            }
            style.push_str(&format!("object-fit:{};", element.object_fit));
            format!(
                "<img class=\"layout-element layout-image\" data-element-id=\"{}\" src=\"{}\" alt=\"{}\" style=\"{}\">",
                escape_html(&element.id),
                escape_html(&src),
                escape_html(&element.alt),
                style
            )
        }
        "rect" => {
            style.push_str(&format!(
                "background:{};border:{}pt solid {};border-radius:{}mm;",
                element.fill, element.border_width, element.border_color, element.radius
            ));
            format!(
                "<div class=\"layout-element layout-rect\" data-element-id=\"{}\" style=\"{}\"></div>",
                escape_html(&element.id),
                style
            )
        }
        "text" => {
            let font_family = if element.font_family == "serif" {
                "Georgia, 'Times New Roman', serif"
            } else {
                "'Segoe UI', Arial, sans-serif"
            };
            style.push_str(&format!(
                "font-family:{};font-size:{}pt;color:{};font-weight:{};font-style:{};text-align:{};background:{};",
                font_family,
                element.font_size,
                element.color,
                if element.bold { 700 } else { 400 },
                if element.italic { "italic" } else { "normal" },
                element.align,
                element.background
            ));
            format!(
                "<div class=\"layout-element layout-text\" data-element-id=\"{}\" style=\"{}\">{}</div>",
                escape_html(&element.id),
                style,
                escape_html(&element.text)
            )
        }
        "table" => render_table_element(element, ctx, &style),
        "chart" => render_chart_element(element, ctx, &style),
        _ => String::new(),
    }
}

fn bound_source<'a>(element: &LayoutElement, ctx: &'a DesignContext) -> Option<&'a DesignSource> {
    if element.binding.is_empty() {
        return None;
    }
    ctx.sources
        .iter()
        .find(|source| source.key == element.binding)
}

fn render_missing_source(element: &LayoutElement, style: &str, message: &str) -> String {
    format!(
        "<div class=\"layout-element layout-table\" data-element-id=\"{}\" style=\"{}display:flex;align-items:center;justify-content:center;box-sizing:border-box;padding:2mm;background:#f6f8f5;border:.5pt dashed #d7ddd6;color:#6d7872;font-size:8pt;text-align:center;\">{}</div>",
        escape_html(&element.id),
        style,
        escape_html(message)
    )
}

fn render_table_element(element: &LayoutElement, ctx: &DesignContext, style: &str) -> String {
    let mut rows = if element.binding.is_empty() {
        element.table_rows.clone()
    } else {
        match bound_source(element, ctx) {
            Some(source) => source.table_rows.clone(),
            None => {
                return render_missing_source(element, style, "Sumber data tidak tersedia.");
            }
        }
    };
    if rows.is_empty() {
        return render_missing_source(element, style, "Belum ada data pada sumber ini.");
    }
    let header = if element.table_header {
        Some(rows.remove(0))
    } else {
        None
    };
    let border = format!("{}pt solid {}", element.border_width, element.border_color);
    let mut html = format!(
        "<div class=\"layout-element layout-table\" data-element-id=\"{}\" style=\"{}background:{};overflow:hidden;\"><table class=\"design-table\" style=\"font-size:{}pt;color:{};--table-border:{};\">",
        escape_html(&element.id),
        style,
        element.fill,
        element.font_size,
        element.color,
        border
    );

    if let Some(header_row) = header {
        html.push_str("<thead><tr>");
        for cell in header_row {
            html.push_str(&format!(
                "<th style=\"background:{};color:{};\">{}</th>",
                element.table_header_bg,
                element.table_header_color,
                escape_html(&cell)
            ));
        }
        html.push_str("</tr></thead>");
    }

    html.push_str("<tbody>");
    for (row_index, row) in rows.iter().enumerate() {
        html.push_str("<tr>");
        for cell in row {
            let align = if is_numeric_cell(cell) {
                "text-align:right;"
            } else {
                ""
            };
            let stripe = if row_index % 2 == 1 && !element.table_stripe.is_empty() {
                format!("background:{};", element.table_stripe)
            } else {
                String::new()
            };
            html.push_str(&format!(
                "<td style=\"{}{}\">{}</td>",
                stripe,
                align,
                escape_html(cell)
            ));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table></div>");
    html
}

fn render_chart_element(element: &LayoutElement, ctx: &DesignContext, style: &str) -> String {
    let bound;
    let element = match bound_source(element, ctx) {
        Some(source) => {
            if !source.can_chart || source.chart_labels.is_empty() {
                return render_missing_source(
                    element,
                    style,
                    "Sumber data ini tidak memiliki seri grafik.",
                );
            }
            let mut copy = element.clone();
            copy.chart_labels = source.chart_labels.clone();
            copy.chart_values = source.chart_values.clone();
            if copy.chart_title.trim().is_empty() {
                copy.chart_title = source.chart_title.clone();
            }
            bound = copy;
            &bound
        }
        None if !element.binding.is_empty() => {
            return render_missing_source(element, style, "Sumber data tidak tersedia.");
        }
        None => element,
    };
    let base = (element.font_size * 1.6).clamp(10.0, 22.0);
    let is_hbar = element.chart_type == "hbar";
    let top = if element.chart_title.trim().is_empty() {
        if is_hbar { 20.0 } else { 18.0 }
    } else if is_hbar {
        58.0
    } else {
        50.0
    };
    let mut body = String::new();
    if !element.fill.is_empty() {
        body.push_str(&format!(
            "<rect x=\"0\" y=\"0\" width=\"600\" height=\"360\" fill=\"{}\"/>",
            element.fill
        ));
    }
    if !is_hbar && !element.chart_title.trim().is_empty() {
        body.push_str(&format!(
            "<text x=\"300\" y=\"30\" text-anchor=\"middle\" font-size=\"{:.1}\" font-weight=\"700\" fill=\"{}\">{}</text>",
            base + 4.0,
            element.color,
            escape_html(&element.chart_title)
        ));
    }

    let chart_svg = match element.chart_type.as_str() {
        "line" => render_line_chart(element, top, base),
        "hbar" => render_hbar_chart(element, top, base),
        "pie" | "donut" => render_pie_chart(element, top, base, element.chart_type == "donut"),
        _ => render_bar_chart(element, top, base),
    };
    body.push_str(&chart_svg);

    format!(
        "<svg class=\"layout-element layout-chart\" data-element-id=\"{}\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 600 360\" preserveAspectRatio=\"xMidYMid meet\" font-family=\"Segoe UI, Arial, sans-serif\" style=\"{}display:block;overflow:hidden;\">{}</svg>",
        escape_html(&element.id),
        style,
        body
    )
}

fn chart_color(element: &LayoutElement, index: usize) -> String {
    element
        .chart_colors
        .get(index)
        .cloned()
        .unwrap_or_else(|| chart_fallback_color(index).to_string())
}

fn chart_fallback_color(index: usize) -> &'static str {
    const COLORS: [&str; 8] = [
        "#143d30", "#b8894b", "#3f765d", "#5b8fa8", "#a4743d", "#d9b87e", "#7a857f", "#2f5d7c",
    ];
    COLORS[index % COLORS.len()]
}

fn chart_value_text(value: f64) -> String {
    if (value.fract()).abs() < 0.05 {
        format_count(value.round() as i64)
    } else {
        format!("{value:.1}")
    }
}

fn render_bar_chart(element: &LayoutElement, top: f64, base: f64) -> String {
    let left = 55.0;
    let right = 575.0;
    let bottom = 316.0;
    let plot_height = (bottom - top).max(40.0);
    let count = element.chart_values.len().max(1);
    let step = (right - left) / count as f64;
    let bar_width = (step * 0.64).max(3.0);
    let max = element
        .chart_values
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let mut svg = format!(
        "<line x1=\"{left}\" y1=\"{bottom}\" x2=\"{right}\" y2=\"{bottom}\" stroke=\"{}\" stroke-width=\"1.2\"/>",
        element.chart_axis_color
    );
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
        left - 8.0,
        top + base,
        base - 2.0,
        element.chart_axis_color,
        chart_value_text(max)
    ));

    for (index, value) in element.chart_values.iter().enumerate() {
        let height = (value / max) * plot_height;
        let x = left + step * index as f64 + (step - bar_width) / 2.0;
        let y = bottom - height;
        let color = chart_color(element, index);
        svg.push_str(&format!(
            "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{bar_width:.2}\" height=\"{height:.2}\" rx=\"2\" fill=\"{color}\"/>"
        ));
        if element.chart_show_values {
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
                x + bar_width / 2.0,
                (y - 4.0).max(top + base),
                base - 2.0,
                element.color,
                chart_value_text(*value)
            ));
        }
        if let Some(label) = element.chart_labels.get(index) {
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
                x + bar_width / 2.0,
                bottom + base + 8.0,
                base - 2.0,
                element.chart_axis_color,
                escape_html(&shorten(label, 14))
            ));
        }
    }
    svg
}

fn render_hbar_chart(element: &LayoutElement, top: f64, base: f64) -> String {
    let left = 24.0;
    let right = 576.0;
    let bottom = 336.0;
    let gap = 12.0;
    let value_width: f64 = if element.chart_show_values { 46.0 } else { 0.0 };
    let label_width = (right - left) * 0.38;
    let track_left = left + label_width + gap;
    let track_width = (right - value_width - gap - track_left).max(40.0);
    let count = element.chart_values.len().max(1);
    let row_height = ((bottom - top) / count as f64).clamp(12.0, 44.0);
    let bar_height = (row_height * 0.36).clamp(6.0, 16.0);
    let bar_radius = bar_height / 2.0;
    let label_size = if count > 12 {
        (base - 4.0).max(7.0)
    } else {
        (base - 3.0).max(8.0)
    };
    let value_size = (base - 2.0).max(8.0);
    let max = element
        .chart_values
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let bar_color = chart_color(element, 0);

    let mut svg = String::from(
        "<rect x=\"1\" y=\"1\" width=\"598\" height=\"358\" rx=\"5\" fill=\"none\" stroke=\"#d7ddd6\" stroke-width=\"1.4\"/>",
    );
    if !element.chart_title.trim().is_empty() {
        svg.push_str(&format!(
            "<text x=\"{left}\" y=\"32\" font-size=\"{:.1}\" font-weight=\"700\" fill=\"{}\">{}</text>",
            (base + 4.0).max(13.0),
            element.color,
            escape_html(&element.chart_title)
        ));
    }

    for (index, value) in element.chart_values.iter().enumerate() {
        let center = top + row_height * (index as f64 + 0.5);
        let baseline = center + label_size * 0.34;
        if let Some(label) = element.chart_labels.get(index) {
            svg.push_str(&format!(
                "<text x=\"{left}\" y=\"{baseline:.2}\" font-size=\"{label_size:.1}\" fill=\"#46534b\">{}</text>",
                escape_html(&shorten(label, 26))
            ));
        }
        let y = center - bar_height / 2.0;
        svg.push_str(&format!(
            "<rect x=\"{track_left:.2}\" y=\"{y:.2}\" width=\"{track_width:.2}\" height=\"{bar_height:.2}\" rx=\"{bar_radius:.2}\" fill=\"#eef1ec\"/>"
        ));
        let width = track_width * (value / max).clamp(0.0, 1.0);
        if width > 0.5 {
            svg.push_str(&format!(
                "<rect x=\"{track_left:.2}\" y=\"{y:.2}\" width=\"{:.2}\" height=\"{bar_height:.2}\" rx=\"{bar_radius:.2}\" fill=\"{bar_color}\"/>",
                width.max(bar_height)
            ));
        }
        if element.chart_show_values {
            svg.push_str(&format!(
                "<text x=\"{right:.2}\" y=\"{baseline:.2}\" text-anchor=\"end\" font-size=\"{value_size:.1}\" font-weight=\"700\" fill=\"{}\">{}</text>",
                element.color,
                chart_value_text(*value)
            ));
        }
    }
    svg
}

fn render_line_chart(element: &LayoutElement, top: f64, base: f64) -> String {
    let left = 55.0;
    let right = 575.0;
    let bottom = 316.0;
    let plot_height = (bottom - top).max(40.0);
    let count = element.chart_values.len().max(1);
    let step = if count > 1 {
        (right - left) / (count - 1) as f64
    } else {
        0.0
    };
    let max = element
        .chart_values
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let color = chart_color(element, 0);
    let mut points = String::new();
    let mut svg = format!(
        "<line x1=\"{left}\" y1=\"{bottom}\" x2=\"{right}\" y2=\"{bottom}\" stroke=\"{}\" stroke-width=\"1.2\"/>",
        element.chart_axis_color
    );

    let mut coordinates = Vec::new();
    for (index, value) in element.chart_values.iter().enumerate() {
        let x = if count > 1 {
            left + step * index as f64
        } else {
            (left + right) / 2.0
        };
        let y = bottom - (value / max) * plot_height;
        coordinates.push((x, y, *value));
        if index > 0 {
            points.push(' ');
        }
        points.push_str(&format!("{x:.2},{y:.2}"));
    }

    svg.push_str(&format!(
        "<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"3\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>"
    ));
    for (index, (x, y, value)) in coordinates.iter().enumerate() {
        svg.push_str(&format!(
            "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"4.5\" fill=\"{color}\"/>"
        ));
        if element.chart_show_values {
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
                x,
                (y - 8.0).max(top + base),
                base - 2.0,
                element.color,
                chart_value_text(*value)
            ));
        }
        if let Some(label) = element.chart_labels.get(index) {
            svg.push_str(&format!(
                "<text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
                bottom + base + 8.0,
                base - 2.0,
                element.chart_axis_color,
                escape_html(&shorten(label, 14))
            ));
        }
    }
    svg
}

fn render_pie_chart(element: &LayoutElement, top: f64, base: f64, donut: bool) -> String {
    let values = &element.chart_values;
    let total: f64 = values.iter().sum();
    if total <= 0.0 {
        return format!(
            "<text x=\"300\" y=\"180\" text-anchor=\"middle\" font-size=\"{:.1}\" fill=\"{}\">Belum ada data</text>",
            base + 2.0,
            element.chart_axis_color
        );
    }

    let show_legend = element.chart_show_legend;
    let cx = if show_legend { 205.0 } else { 300.0 };
    let cy = top + (316.0 - top) / 2.0;
    let outer = ((316.0 - top) / 2.0 - 12.0).clamp(40.0, 125.0);
    let inner = if donut { outer * 0.55 } else { 0.0 };
    let mut svg = String::new();
    let mut angle = -std::f64::consts::FRAC_PI_2;

    for (index, value) in values.iter().enumerate() {
        if *value <= 0.0 {
            continue;
        }
        let sweep = (*value / total) * std::f64::consts::TAU;
        let end = angle + sweep;
        let color = chart_color(element, index);
        let x1 = cx + outer * angle.cos();
        let y1 = cy + outer * angle.sin();
        let x2 = cx + outer * end.cos();
        let y2 = cy + outer * end.sin();
        let large = if sweep > std::f64::consts::PI { 1 } else { 0 };

        if values.iter().filter(|item| **item > 0.0).count() == 1 {
            svg.push_str(&format!(
                "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{outer}\" fill=\"{color}\"/>"
            ));
        } else if donut {
            let ix1 = cx + inner * angle.cos();
            let iy1 = cy + inner * angle.sin();
            let ix2 = cx + inner * end.cos();
            let iy2 = cy + inner * end.sin();
            svg.push_str(&format!(
                "<path d=\"M {x1:.2} {y1:.2} A {outer:.2} {outer:.2} 0 {large} 1 {x2:.2} {y2:.2} L {ix2:.2} {iy2:.2} A {inner:.2} {inner:.2} 0 {large} 0 {ix1:.2} {iy1:.2} Z\" fill=\"{color}\"/>"
            ));
        } else {
            svg.push_str(&format!(
                "<path d=\"M {cx} {cy} L {x1:.2} {y1:.2} A {outer:.2} {outer:.2} 0 {large} 1 {x2:.2} {y2:.2} Z\" fill=\"{color}\"/>"
            ));
        }

        if element.chart_show_values && sweep > 0.35 {
            let mid = angle + sweep / 2.0;
            let radius = if donut {
                (outer + inner) / 2.0
            } else {
                outer * 0.62
            };
            let tx = cx + radius * mid.cos();
            let ty = cy + radius * mid.sin() + base * 0.35;
            svg.push_str(&format!(
                "<text x=\"{tx:.2}\" y=\"{ty:.2}\" text-anchor=\"middle\" font-size=\"{:.1}\" font-weight=\"700\" fill=\"#ffffff\">{:.0}%</text>",
                base - 2.0,
                sweep / std::f64::consts::TAU * 100.0
            ));
        }
        angle = end;
    }

    if donut {
        svg.push_str(&format!(
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{inner}\" fill=\"{}\"/>",
            element.fill
        ));
    }

    if show_legend {
        let mut y = top + base;
        for (index, label) in element.chart_labels.iter().take(12).enumerate() {
            let color = chart_color(element, index);
            svg.push_str(&format!(
                "<rect x=\"355\" y=\"{:.2}\" width=\"12\" height=\"12\" rx=\"2\" fill=\"{color}\"/>",
                y - 10.0
            ));
            svg.push_str(&format!(
                "<text x=\"374\" y=\"{y:.2}\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
                base - 3.0,
                element.color,
                escape_html(&shorten(label, 20))
            ));
            y += base + 7.0;
        }
    }
    svg
}

fn is_numeric_cell(value: &str) -> bool {
    let normalized = value.trim().replace('.', "").replace(',', ".");
    !normalized.is_empty() && normalized.parse::<f64>().is_ok()
}

fn block_css(layout: &ReportLayout) -> String {
    let mut css = String::new();
    for page in &layout.pages {
        for (id, block) in &page.blocks {
            if !valid_id(id) {
                continue;
            }
            let mut declarations = String::new();
            if block.hidden {
                if !block.preserve_space {
                    css.push_str(&format!("[data-block=\"{id}\"]{{display:none!important;}}"));
                    continue;
                }
                declarations.push_str("visibility:hidden;");
            }
            if block.dx.abs() > 0.005 || block.dy.abs() > 0.005 {
                declarations.push_str(&format!(
                    "transform:translate({:.3}mm,{:.3}mm);",
                    block.dx, block.dy
                ));
            }
            if block.width > 0.0 {
                declarations.push_str(&format!("width:{:.3}mm;", block.width));
            }
            if block.height > 0.0 {
                declarations.push_str(&format!("height:{:.3}mm;", block.height));
            }
            if block.width > 0.0 || block.height > 0.0 {
                declarations.push_str("overflow:hidden;box-sizing:border-box;");
            }
            if !block.background.is_empty() {
                declarations.push_str(&format!("background:{};", block.background));
            }
            if block.border_width > 0.0 {
                declarations.push_str(&format!(
                    "border:{}pt solid {};",
                    block.border_width, block.border_color
                ));
            }
            if block.radius > 0.0 {
                declarations.push_str(&format!("border-radius:{:.3}mm;", block.radius));
            }
            if !declarations.is_empty() {
                css.push_str(&format!("[data-block=\"{id}\"]{{{declarations}}}"));
            }
        }
    }
    css
}

fn kpi_row(items: &[(&str, i64, &str)], block_id: &str) -> String {
    let mut output = format!("<div class=\"kpi-row\" data-block=\"{block_id}\">");
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

fn editorial_block(
    ctx: &DesignContext,
    page_no: usize,
    block_id: &str,
    title: &str,
    text: &str,
    fallback: &str,
) -> String {
    let effective =
        override_text(ctx, page_no, block_id, "body").unwrap_or_else(|| text.to_string());
    format!(
        "<div class=\"editorial-block\" data-block=\"{block_id}\"><h3>{}</h3><div class=\"editorial-body\" data-text-id=\"body\">{}</div></div>",
        escape_html(title),
        editorial(&effective, fallback)
    )
}

fn note_strip(
    ctx: &DesignContext,
    page_no: usize,
    block_id: &str,
    strong: &str,
    text: &str,
) -> String {
    let effective = text_or(ctx, page_no, block_id, "text", text);
    format!(
        "<div class=\"note-strip\" data-block=\"{block_id}\"><strong>{}</strong><span data-text-id=\"text\">{}</span></div>",
        escape_html(strong),
        escape_html(&effective)
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

fn activity_panel(title: &str, items: &[RowSummary], block_id: &str) -> String {
    if items.is_empty() {
        return String::new();
    }
    format!(
        "<div class=\"panel\" data-block=\"{block_id}\"><h3>{}</h3>{}</div>",
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
.layout-layer { position: absolute; inset: 0; z-index: 20; pointer-events: none; }
.layout-layer > * { pointer-events: auto; }
.layout-element { position: absolute; box-sizing: border-box; }
.layout-image { display: block; width: 100%; height: 100%; }
.layout-text { white-space: pre-wrap; overflow: hidden; padding: .8mm; }
.layout-table table { width: 100%; border-collapse: collapse; table-layout: fixed; }
.layout-table th, .layout-table td { border: var(--table-border, .5pt solid #d7ddd6); padding: 1.2mm 1.5mm; vertical-align: top; word-break: break-word; }
.layout-table th { font-weight: 700; }
.layout-chart { background: transparent; }
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
    fn renders_horizontal_bar_chart() {
        let metrics = ReportMetrics::default();
        let layout = ReportLayout {
            version: 1,
            pages: vec![crate::modules::reports::layout::PageLayout {
                page: 2,
                blocks: std::collections::BTreeMap::new(),
                elements: vec![LayoutElement {
                    id: "chart-hbar".into(),
                    kind: "chart".into(),
                    x: 10.0,
                    y: 10.0,
                    width: 120.0,
                    height: 80.0,
                    chart_type: "hbar".into(),
                    chart_title: "Jenis kegiatan".into(),
                    chart_labels: vec![
                        "QW2.1".into(),
                        "QW2.3".into(),
                        "QW2.4".into(),
                        "QW2.2".into(),
                    ],
                    chart_values: vec![17.0, 2.0, 2.0, 1.0],
                    chart_colors: (0..4).map(|_| "#3f765d".to_string()).collect(),
                    chart_show_values: true,
                    fill: "#ffffff".into(),
                    color: "#143d30".into(),
                    font_size: 9.0,
                    ..Default::default()
                }],
            }],
        };

        let html = render_with_layout(
            &metrics,
            &ReportContent::default(),
            &Config::test_config(),
            &layout,
            &HashMap::new(),
        );
        assert!(html.contains("Jenis kegiatan"));
        assert!(html.contains("#eef1ec"));
        assert!(html.contains("#3f765d"));
        assert!(html.contains(">QW2.1<"));
        assert!(html.contains(">17<"));
    }

    #[test]
    fn renders_data_bound_table_and_chart() {
        let mut metrics = ReportMetrics::default();
        metrics.period_label = "16–18 Oktober 2026".into();
        metrics.polda_name = "POLDA BALI".into();
        metrics.source_filename = "report.xlsx".into();
        metrics.quick_wins = vec![
            crate::shared::LabeledCount::new("Gelar perkara", 12),
            crate::shared::LabeledCount::new("Patroli", 8),
        ];

        let layout = ReportLayout {
            version: 1,
            pages: vec![crate::modules::reports::layout::PageLayout {
                page: 2,
                blocks: std::collections::BTreeMap::new(),
                elements: vec![
                    LayoutElement {
                        id: "table-bound".into(),
                        kind: "table".into(),
                        x: 10.0,
                        y: 10.0,
                        width: 100.0,
                        height: 50.0,
                        binding: "quick_wins".into(),
                        table_rows: vec![vec!["Manual".into(), "1".into()]],
                        ..Default::default()
                    },
                    LayoutElement {
                        id: "chart-bound".into(),
                        kind: "chart".into(),
                        x: 10.0,
                        y: 70.0,
                        width: 100.0,
                        height: 60.0,
                        binding: "quick_wins".into(),
                        chart_labels: vec!["Manual".into()],
                        chart_values: vec![99.0],
                        ..Default::default()
                    },
                    LayoutElement {
                        id: "table-stale".into(),
                        kind: "table".into(),
                        x: 10.0,
                        y: 140.0,
                        width: 100.0,
                        height: 40.0,
                        binding: "tidak.ada".into(),
                        ..Default::default()
                    },
                ],
            }],
        };

        let html = render_with_layout(
            &metrics,
            &ReportContent::default(),
            &Config::test_config(),
            &layout,
            &HashMap::new(),
        );
        assert!(html.contains("Gelar perkara"));
        assert!(html.contains("Patroli"));
        assert!(!html.contains("Manual"));
        assert!(html.contains("Sumber data tidak tersedia."));
    }

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
