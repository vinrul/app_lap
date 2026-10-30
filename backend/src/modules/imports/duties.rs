//! Pemetaan tugas Quick Wins dari berkas `DAFTA KEGIATAN QUICK WINS.xlsx`.
//!
//! Kolom `PEJABAT PELAKSANA` berisi pejabat yang wajib melaksanakan sub-kode,
//! sedangkan `PELAKSANA PENGUNGKIT` berisi unit penanggung jawab bila pejabat
//! berhalangan. Keduanya dipakai untuk menentukan unit mana yang sebenarnya
//! bertanggung jawab atas satu kode, sehingga daftar "belum melaksanakan"
//! hanya menampilkan unit yang memang punya tugas tersebut.

use super::qw;

/// Satu baris daftar kegiatan: (kode giat, pejabat pelaksana, pelaksana pengungkit).
const DUTIES: [(&str, &str, &str); 41] = [
    ("QW1.1", "KAPOLDA, KAPOLRES, KAPOLSEK", "DITLANTAS"),
    ("QW1.2", "KAPOLDA, KAPOLRES, KAPOLSEK", "BIRO SDM"),
    ("QW1.3", "KAPOLDA, KAPOLRES, KAPOLSEK", "DITBINMAS"),
    ("QW1.4", "KAPOLDA, KAPOLRES, KAPOLSEK", "DITBINMAS"),
    ("QW1.5", "KAPOLDA, KAPOLRES, KAPOLSEK", "BIRO SDM"),
    ("QW1.6", "KAPOLDA, KAPOLRES, KAPOLSEK", "DIT BINMAS"),
    ("QW1.7", "KAPOLDA, KAPOLRES, KAPOLSEK", "BIRO SDM"),
    ("QW1.8", "KAPOLDA, KAPOLRES, KAPOLSEK", "BIRO SDM"),
    (
        "QW1.9",
        "DIRRESKRIMUM, KAPOLRES, KAPOLSEK",
        "DITRESKRIMUM, SATRESKRIM, UNITRESKRIM",
    ),
    (
        "QW1.10",
        "DIRSAMAPTA, KAPOLRES, KAPOLSEK",
        "DITSAMAPTA, SATSAMAPTA, UNITSAMAPTA",
    ),
    ("QW1.11", "DIRPOLAIR, KAPOLRES", "DITPOLAIR, SATSAMAPTA"),
    ("QW1.12", "", ""),
    ("QW1.13", "KARO SDM, KAPOLRES", "BIRO SDM, BAGSDM"),
    (
        "QW1.14",
        "BIDOROPAM, KAPOLRES",
        "DITSAMAPTA, BIDPROPAM, SATSAMAPTA, SIPROPAM, UNITSAMAPTA",
    ),
    (
        "QW2.1",
        "DIRRESNARKOBA, KAPOLRES",
        "DITRESNARKOBA, SATRESNARKOBA",
    ),
    ("QW2.2", "DIRBINMAS, KAPOLRES", "DITBINMAS, SATBINMAS"),
    (
        "QW2.3",
        "DIRRESNARKOBA, KAPOLRES",
        "DITRESNARKOBA, SATRESNARKOBA",
    ),
    ("QW2.4", "DIRBINMAS, KAPOLRES", "DITBINMAS, SATBINMAS"),
    (
        "QW3.1",
        "DIRSIBER, KAPOLRES, KAPOLSEK",
        "DITSIBER, SPKT, KANITSIBER, SATRESKRIM",
    ),
    (
        "QW3.2",
        "DIRSIBER, KAPOLRES, KAPOLSEK",
        "DITSIBER, SPKT, KANITSIBER, SPKT POLRES, SPKT POLSEK",
    ),
    (
        "QW3.3",
        "KABIDHUMAS, KAPOLRES, KAPOLSEK",
        "DITBINMAS, BIDHUMAS, SATBINMAS, SIHUMAS",
    ),
    ("QW3.4", "DIRSIBER, KAPOLRES", "DITSIBER, KANITSIBER"),
    (
        "QW4.1",
        "KAROOPS, KAPOLRES, KAPOLSEK",
        "BIROOPS, SPKT, SPKT POLRES, SPKT POLSEK",
    ),
    (
        "QW4.2",
        "KA SPKT, KAPOLRES, KAPOLSEK",
        "SPKT, SPKT POLRES, SPKT POLSEK",
    ),
    (
        "QW4.3",
        "KAROOPS, KAPOLRES, KAPOLSEK",
        "BIROOPS, SPKT, SPKT POLRES, SPKT POLSEK",
    ),
    (
        "QW4.4",
        "KA SPKT, KAPOLRES, KAPOLSEK",
        "SPKT, SPKT POLRES, SPKT POLSEK",
    ),
    (
        "QW5.1",
        "DITRESKRIMUM, KAPOLRES, KAPOLSEK",
        "DITRESKRIMUM, SATRESKRIM, UNITRESKRIM",
    ),
    (
        "QW5.2",
        "DITRESKRIMUM, KAPOLRES, KAPOLSEK",
        "DITRESKRIMUM, SPKT, SATRESKRIM, SPKT POLRES, UNITRESKRIM, SPKT POLSEK",
    ),
    (
        "QW5.3",
        "DITRESKRIMUM, KAPOLRES, KAPOLSEK",
        "DITRESKRIMUM, SATRESKRIM, UNITRESKRIM",
    ),
    (
        "QW5.4",
        "DITRESKRIMUM, KAPOLRES, KAPOLSEK",
        "DITRESKRIMUM, RESKRIM, UNITRESKRIM",
    ),
    (
        "QW6.1",
        "KABIDPROPAM, DIRLANTAS, DIRINTEL, KAPOLRES",
        "BIDPROPAM, DITLANTAS (SIM), DITINTELKAM (SKCK), SATLANTAS, SATINTELKAM, SIPROPAM",
    ),
    ("QW6.2", "KABIDPROPAM, KAPOLRES", "BIDPROPAM, SIPROPAM"),
    (
        "QW6.3",
        "ITWASDA, KABIDPROPAM, KAPOLRES",
        "BIDPROPAM, ITWASDA, SIPROPAM",
    ),
    ("QW7.1", "DIRBINMAS, KAPOLRES, KAPOLSEK", "POLRES, POLSEK"),
    ("QW7.2", "KARO SDM, KAPOLRES, KAPOLSEK", "POLRES, POLSEK"),
    ("QW7.3", "KARO OPS, KAPOLRES, KAPOLSEK", "POLSEK"),
    ("QW7.4", "DIRBINMAS, KAPOLRES, KAPOLSEK", "POLRES, POLSEK"),
    (
        "QW8.1",
        "DIRRESKRIMSUS, KAPOLRES, KAPOLSEK",
        "DITRESKRIMUM, DIRESKRIMSUS, SATRESKRIM, UNITRESKRIM",
    ),
    (
        "QW8.2",
        "DIRRESKRIMSUS, KAPOLRES, KAPOLSEK",
        "DITRESKRIMUM, DIRESKRIMSUS, SATRESKRIM, UNITRESKRIM",
    ),
    (
        "QW8.3",
        "DIRRESKRIMSUS, DIRRESKRIMUM, DIRSAMAPTA, KABIDKUM, DIRBINMAS, KAPOLRES, KAPOLSEK",
        "DITRESKRIMUM, DIRESKRIMSUS, DITSAMAPTA, BIDKUM, SATBINMAS, SIKUM",
    ),
    (
        "QW8.4",
        "DIRRESKRIMUM, DIRRESKRIMSUS, KASIINDET, KABIDDOKKES, KABIDLABFOR",
        "DITRESKRIMUM, DIRESKRIMSUS, KASI IDENT, BIDDOKKES, BIDLAFOR",
    ),
];

/// Tingkat unit pada kolom pelaksana pengungkit.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DutyLevel {
    Polda,
    Polres,
    Polsek,
}

/// Kata kunci untuk mencocokkan istilah tugas dengan label unit pada data.
const TERMS: &[(&str, &[&str])] = &[
    ("DITLANTAS", &["LANTAS"]),
    ("DITLANTASSIM", &["LANTAS"]),
    ("BIROSDM", &["BIROSDM", "BAGSDM", "SDM"]),
    ("DITBINMAS", &["BINMAS"]),
    ("DITRESKRIMUM", &["RESKRIMUM", "RESKRIM"]),
    ("DITSAMAPTA", &["SAMAPTA"]),
    ("DITPOLAIR", &["POLAIR"]),
    ("BIDPROPAM", &["PROPAM"]),
    ("DITRESNARKOBA", &["NARKOBA"]),
    ("DITSIBER", &["SIBER"]),
    ("SPKT", &["SPKT"]),
    ("BIDHUMAS", &["HUMAS"]),
    ("BIROOPS", &["KAROOPS", "BIROOPS"]),
    ("DITINTELKAMSKCK", &["INTELKAM"]),
    ("DIRESKRIMSUS", &["RESKRIMSUS"]),
    ("BIDKUM", &["BIDKUM"]),
    ("KASIIDENT", &["INAFIS", "IDENT"]),
    ("BIDDOKKES", &["DOKKES", "DOKPOL"]),
    ("BIDLAFOR", &["LABFOR"]),
    ("ITWASDA", &["ITWASDA"]),
    ("SATRESKRIM", &["RESKRIM"]),
    ("SATSAMAPTA", &["SAMAPTA"]),
    ("SATRESNARKOBA", &["NARKOBA"]),
    ("SATBINMAS", &["BINMAS"]),
    ("KANITSIBER", &["SIBER"]),
    ("SPKTPOLRES", &["SPKT"]),
    ("SIHUMAS", &["HUMAS"]),
    ("SIPROPAM", &["PROPAM"]),
    ("SATLANTAS", &["LANTAS"]),
    ("SATINTELKAM", &["INTELKAM"]),
    ("BAGSDM", &["SDM"]),
    ("POLRES", &["KAPOLRES"]),
    ("SIKUM", &["SIKUM"]),
    ("SPKTPOLSEK", &["SPKT"]),
    ("UNITRESKRIM", &["RESKRIM"]),
    ("UNITSAMAPTA", &["SAMAPTA"]),
    ("POLSEK", &["KAPOLSEK"]),
    ("KAPOLDA", &["KAPOLDA"]),
    ("KAPOLRES", &["KAPOLRES"]),
    ("KAPOLSEK", &["KAPOLSEK"]),
    ("DIRRESKRIMUM", &["RESKRIMUM", "RESKRIM"]),
    ("DIRSAMAPTA", &["SAMAPTA"]),
    ("DIRPOLAIR", &["POLAIR"]),
    ("DIRRESNARKOBA", &["NARKOBA"]),
    ("DIRBINMAS", &["BINMAS"]),
    ("DIRSIBER", &["SIBER"]),
    ("DIRRESKRIMSUS", &["RESKRIMSUS"]),
    ("KABIDHUMAS", &["HUMAS"]),
    ("KABIDPROPAM", &["PROPAM"]),
    ("BIDOROPAM", &["PROPAM"]),
    ("KABIDKUM", &["BIDKUM"]),
    ("KABIDDOKKES", &["DOKKES", "DOKPOL"]),
    ("KABIDLABFOR", &["LABFOR"]),
    ("KAROOPS", &["KAROOPS", "BIROOPS"]),
    ("KAROSDM", &["BIROSDM", "SDM"]),
    ("DIRLANTAS", &["LANTAS"]),
    ("DIRINTEL", &["INTELKAM"]),
    ("KASPKT", &["SPKT"]),
    ("KASIINDET", &["INAFIS", "IDENT"]),
];

/// Kata kunci yang gugur bila label memuat istilah lain yang lebih spesifik.
const EXCLUSIONS: &[(&str, &[&str])] = &[("RESKRIM", &["RESKRIMSUS"])];

/// Menormalkan istilah tugas menjadi huruf besar tanpa karakter non-alfanumerik.
fn normalize_term(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}

/// Menentukan tingkat unit dari satu istilah pelaksana pengungkit.
fn level_of(token: &str) -> DutyLevel {
    let term = normalize_term(token);
    if term.contains("POLSEK") || term.starts_with("UNIT") {
        return DutyLevel::Polsek;
    }
    if term.contains("POLRES")
        || term.starts_with("SAT")
        || term.starts_with("SI")
        || term.starts_with("BAG")
        || term.starts_with("KANIT")
    {
        return DutyLevel::Polres;
    }
    DutyLevel::Polda
}

/// Kata kunci pencocokan untuk satu istilah tugas; kosong bila tidak dikenali.
fn keywords_of(token: &str) -> &'static [&'static str] {
    let term = normalize_term(token);
    TERMS
        .iter()
        .find(|(keyword, _)| *keyword == term)
        .map(|(_, keywords)| *keywords)
        .unwrap_or(&[])
}

fn split_tokens(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

/// Baris tugas untuk satu kode, termasuk seluruh sub-kode bila `code` adalah kelompok.
fn duties_for(code: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let normalized = qw::normalize(code);
    if normalized.contains('.') {
        DUTIES
            .iter()
            .filter(|(kode, _, _)| *kode == normalized)
            .copied()
            .collect()
    } else {
        let prefix = format!("{normalized}.");
        DUTIES
            .iter()
            .filter(|(kode, _, _)| kode.starts_with(&prefix))
            .copied()
            .collect()
    }
}

/// Apakah kode tercantum di daftar kegiatan.
fn is_listed(code: &str) -> bool {
    !duties_for(code).is_empty()
}

/// Apakah kode terdaftar di daftar kegiatan (meski tanpa pelaksana).
pub(crate) fn is_assigned(code: &str) -> bool {
    is_listed(code)
}

/// Apakah peran (mis. `KAPOLRES`) termasuk pejabat pelaksana kode tersebut.
pub(crate) fn role_assigned(code: &str, role: &str) -> bool {
    duties_for(code)
        .iter()
        .any(|(_, pelaksana, _)| split_tokens(pelaksana).any(|token| normalize_term(token) == role))
}

/// Kata kunci pejabat pelaksana pada satu tingkat. `None` bila kode tidak punya
/// data tugas sama sekali.
pub(crate) fn pelaksana_keywords(code: &str, level: DutyLevel) -> Option<Vec<&'static str>> {
    if !is_listed(code) {
        return None;
    }
    let mut keywords: Vec<&'static str> = Vec::new();
    for (_, pelaksana, _) in duties_for(code) {
        for token in split_tokens(pelaksana) {
            if pejabat_level(token) != level {
                continue;
            }
            keywords.extend_from_slice(keywords_of(token));
        }
    }
    keywords.sort_unstable();
    keywords.dedup();
    Some(keywords)
}

/// Tingkat pejabat pelaksana: hanya Kapolres/Kapolsek yang bukan tingkat Polda.
fn pejabat_level(token: &str) -> DutyLevel {
    let term = normalize_term(token);
    if term.contains("POLSEK") {
        return DutyLevel::Polsek;
    }
    if term.contains("POLRES") {
        return DutyLevel::Polres;
    }
    DutyLevel::Polda
}

/// Kata kunci satker yang bertanggung jawab pada satu tingkat.
///
/// `None` berarti kode tidak punya data tugas sehingga pemanggil memakai
/// daftar unit lengkap. `Some(vec![])` berarti tidak ada satker pada tingkat
/// tersebut yang bertanggung jawab.
pub(crate) fn satker_keywords(code: &str, level: DutyLevel) -> Option<Vec<&'static str>> {
    if !is_listed(code) {
        return None;
    }
    let mut keywords: Vec<&'static str> = Vec::new();
    for (_, _, pengungkit) in duties_for(code) {
        for token in split_tokens(pengungkit) {
            if level_of(token) != level {
                continue;
            }
            keywords.extend_from_slice(keywords_of(token));
        }
    }
    keywords.sort_unstable();
    keywords.dedup();
    Some(keywords)
}

/// Mencocokkan label unit hasil impor dengan kata kunci tugas.
pub(crate) fn label_matches(label: &str, keywords: &[&str]) -> bool {
    if keywords.is_empty() {
        return false;
    }
    let normalized = normalize_term(label);
    keywords.iter().any(|keyword| {
        if !normalized.contains(keyword) {
            return false;
        }
        !EXCLUSIONS.iter().any(|(term, blockers)| {
            *term == *keyword && blockers.iter().any(|blocker| normalized.contains(blocker))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_pengungkit_per_level() {
        let polda = satker_keywords("QW1.1", DutyLevel::Polda).unwrap();
        assert_eq!(polda, vec!["LANTAS"]);
        assert!(label_matches("Lantas Polda", &polda));
        assert!(!label_matches("Reskrim Polda", &polda));

        let polres = satker_keywords("QW1.9", DutyLevel::Polres).unwrap();
        assert!(label_matches("Reskrim Polres", &polres));
        assert!(!label_matches("SPKT Polres", &polres));
        assert!(!label_matches("Reskrimsus", &["RESKRIM"]));
        assert!(label_matches("Reskrim Polda", &["RESKRIM"]));

        let polsek = satker_keywords("QW4.1", DutyLevel::Polsek).unwrap();
        assert!(label_matches("SPKT", &polsek));
    }

    #[test]
    fn group_codes_union_their_sub_codes() {
        let keywords = satker_keywords("QW2", DutyLevel::Polres).unwrap();
        assert!(keywords.contains(&"NARKOBA"));
        assert!(keywords.contains(&"BINMAS"));
    }

    #[test]
    fn flags_codes_without_satker_assignment() {
        assert_eq!(
            satker_keywords("QW7.1", DutyLevel::Polda).unwrap(),
            Vec::<&str>::new()
        );
        assert!(satker_keywords("QW99", DutyLevel::Polda).is_none());
        assert!(role_assigned("QW1.1", "KAPOLSEK"));
        assert!(!role_assigned("QW1.11", "KAPOLSEK"));
        assert!(is_assigned("QW1.12"));
        assert!(!role_assigned("QW1.12", "KAPOLRES"));
        assert_eq!(
            satker_keywords("QW1.12", DutyLevel::Polda).unwrap(),
            Vec::<&str>::new()
        );
    }
}
