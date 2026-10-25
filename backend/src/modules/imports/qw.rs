#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuickWinGroup {
    pub code: &'static str,
    pub title: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuickWinCode {
    pub code: &'static str,
    pub group: &'static str,
    pub title: &'static str,
}

pub const GROUPS: &[QuickWinGroup] = &[
    QuickWinGroup {
        code: "QW1",
        title: "Ruang Publik Aman",
    },
    QuickWinGroup {
        code: "QW2",
        title: "Pemulihan Kawasan Narkoba",
    },
    QuickWinGroup {
        code: "QW3",
        title: "Bersih Judi Daring dan Penipuan Digital",
    },
    QuickWinGroup {
        code: "QW4",
        title: "110 Respons Cepat, SPKT Ramah Korban",
    },
    QuickWinGroup {
        code: "QW5",
        title: "Perkara Terlacak dan SP2HP Pasti",
    },
    QuickWinGroup {
        code: "QW6",
        title: "Bersih ke Dalam",
    },
    QuickWinGroup {
        code: "QW7",
        title: "Polres Kuat, Polsek Tangguh",
    },
    QuickWinGroup {
        code: "QW8",
        title: "KUHP/KUHAP Siap dan Scientific Crime Investigation",
    },
];

pub const CODES: &[QuickWinCode] = &[
    code("QW1.1", "QW1", "Gatur Pagi (Pengaturan Lalu Lintas Senin Pagi oleh Ka/Waka)"),
    code("QW1.2", "QW1", "Polisi Peduli Pendidikan (Polidik)"),
    code("QW1.3", "QW1", "Polisi Peduli Pengangguran (Poliran)"),
    code("QW1.4", "QW1", "Kumpul Bersama Masyarakat"),
    code("QW1.5", "QW1", "Green Policing"),
    code("QW1.6", "QW1", "Jumat Keliling (Jumling)"),
    code("QW1.7", "QW1", "Subuh Keliling Bersama Masyarakat (Suling)"),
    code("QW1.8", "QW1", "Minggu Kasih"),
    code("QW1.9", "QW1", "Patroli Siaga Reserse (Pagar)"),
    code("QW1.10", "QW1", "Patroli Sabhara"),
    code("QW1.11", "QW1", "Patroli Polisi Perairan"),
    code("QW1.12", "QW1", "Pencegahan ekstremisme pada anak"),
    code("QW1.13", "QW1", "Polisi Peduli Pangan"),
    code("QW1.14", "QW1", "DIMTAQ (Disiplin Iman dan Taqwa)"),
    code("QW2.1", "QW2", "Pemberantasan dan pemulihan kawasan narkoba"),
    code("QW2.2", "QW2", "Pembinaan kawasan bersama BNN dan Pemda"),
    code("QW2.3", "QW2", "Rujukan rehabilitasi pengguna"),
    code("QW2.4", "QW2", "Penyuluhan bahaya narkoba"),
    code(
        "QW3.1",
        "QW3",
        "Penegakan hukum judi dan penipuan di ruang digital",
    ),
    code("QW3.2", "QW3", "Layanan aduan korban penipuan daring"),
    code("QW3.3", "QW3", "Edukasi anti judi daring dan penipuan"),
    code("QW3.4", "QW3", "Patroli siber"),
    code("QW4.1", "QW4", "Pamapta Hadir dan Penanganan ke TKP"),
    code(
        "QW4.2",
        "QW4",
        "Petugas Yanmas Melayani Laporan dan Aduan di SPKT",
    ),
    code("QW4.3", "QW4", "Respons Cepat Panggilan 110"),
    code("QW4.4", "QW4", "Layanan Ramah Perempuan dan Anak"),
    code("QW5.1", "QW5", "SP2HP Berkala kepada Pelapor"),
    code("QW5.2", "QW5", "Layanan Informasi Perkara"),
    code("QW5.3", "QW5", "Gelar Perkara atas Pengaduan Warga"),
    code("QW5.4", "QW5", "Keadilan Restoratif Perkara Ringan"),
    code("QW6.1", "QW6", "Layanan SIM, SKCK, dan Tilang tanpa Pungli"),
    code("QW6.2", "QW6", "Copot Hari Itu Juga"),
    code("QW6.3", "QW6", "Kanal Pengaduan Propam/Dumas"),
    code("QW7.1", "QW7", "Satu Desa Satu Bhabin"),
    code("QW7.2", "QW7", "Penataan Personel"),
    code("QW7.3", "QW7", "Polsek Siaga 24 Jam"),
    code("QW7.4", "QW7", "Perkara Ringan dan Rembuk Warga"),
    code("QW8.1", "QW8", "Penanganan BBM Ilegal dan Kejahatan Ekonomi"),
    code(
        "QW8.2",
        "QW8",
        "Penanganan TPPO, Korupsi, Penyelundupan dan Pencucian Uang",
    ),
    code("QW8.3", "QW8", "Penyuluhan KUHP Baru kepada Masyarakat"),
    code("QW8.4", "QW8", "Olah TKP Ilmiah dan Identifikasi Korban"),
];

const fn code(code: &'static str, group: &'static str, title: &'static str) -> QuickWinCode {
    QuickWinCode { code, group, title }
}

pub fn normalize(value: &str) -> String {
    value
        .trim()
        .to_ascii_uppercase()
        .replace(' ', "")
        .replace('—', ".")
}

pub fn find(value: &str) -> Option<&'static QuickWinCode> {
    let normalized = normalize(value);
    CODES.iter().find(|item| item.code == normalized)
}

pub fn group_of(value: &str) -> Option<&'static QuickWinGroup> {
    let normalized = normalize(value);
    let prefix = normalized
        .split_once('.')
        .map(|(prefix, _)| prefix)
        .unwrap_or(normalized.as_str());
    GROUPS.iter().find(|item| item.code == prefix)
}

pub fn group_title(value: &str) -> &'static str {
    group_of(value).map(|group| group.title).unwrap_or("")
}

pub fn title(value: &str) -> &'static str {
    find(value).map(|code| code.title).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_group_and_title() {
        assert_eq!(group_of("qw1.2"), Some(&GROUPS[0]));
        assert_eq!(title("QW3.3"), "Edukasi anti judi daring dan penipuan");
        assert_eq!(group_title("QW8.4"), GROUPS[7].title);
        assert!(find("QW9.1").is_none());
    }
}
