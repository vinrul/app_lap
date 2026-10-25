use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LabeledCount {
    pub label: String,
    pub count: i64,
}

impl LabeledCount {
    pub fn new(label: impl Into<String>, count: i64) -> Self {
        Self {
            label: label.into(),
            count,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct UnitBreakdown {
    pub label: String,
    pub rows: i64,
    pub polsek: i64,
    pub personel: i64,
    pub bibit: i64,
    pub area_m2: f64,
    pub jamaah: i64,
}

/// Mengagregasi baris per unit (satker Polda atau polres).
///
/// Item berupa `(label, polsek, personel, bibit, luas_m2, jamaah)`. Label
/// dibandingkan tanpa membedakan huruf besar/kecil; `polsek` dihitung unik
/// untuk menunjukkan banyak polsek yang melapor di bawah polres tersebut.
pub fn unit_breakdown<I, L, P>(items: I, limit: usize) -> Vec<UnitBreakdown>
where
    I: IntoIterator<Item = (L, P, i64, i64, f64, i64)>,
    L: AsRef<str>,
    P: AsRef<str>,
{
    let mut groups: BTreeMap<String, (BTreeSet<String>, UnitBreakdown)> = BTreeMap::new();
    for (label, polsek, personel, bibit, area_m2, jamaah) in items {
        let label = label.as_ref().trim();
        if label.is_empty() {
            continue;
        }
        let entry = groups
            .entry(label.to_lowercase())
            .or_insert_with(|| {
                let mut breakdown = UnitBreakdown::default();
                breakdown.label = label.to_string();
                (BTreeSet::new(), breakdown)
            });
        entry.1.rows += 1;
        entry.1.personel += personel;
        entry.1.bibit += bibit;
        entry.1.area_m2 += area_m2;
        entry.1.jamaah += jamaah;
        let polsek = polsek.as_ref().trim();
        if !polsek.is_empty() {
            entry.0.insert(polsek.to_lowercase());
        }
    }

    let mut result: Vec<UnitBreakdown> = groups
        .into_values()
        .map(|(polsek, mut breakdown)| {
            breakdown.polsek = polsek.len() as i64;
            breakdown.area_m2 = (breakdown.area_m2 * 100.0).round() / 100.0;
            breakdown
        })
        .collect();
    result.sort_by(|left, right| {
        right
            .rows
            .cmp(&left.rows)
            .then_with(|| left.label.cmp(&right.label))
    });
    if limit > 0 {
        result.truncate(limit);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_units_case_insensitively() {
        let items = [
            ("Polresta Denpasar", "Polsek Kuta", 2, 10, 100.5, 50),
            ("polresta denpasar", "Polsek Mengwi", 3, 20, 200.25, 25),
            ("Satker Polda", "", 1, 5, 0.0, 0),
        ];
        let result = unit_breakdown(items, 10);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Polresta Denpasar");
        assert_eq!(result[0].rows, 2);
        assert_eq!(result[0].polsek, 2);
        assert_eq!(result[0].personel, 5);
        assert_eq!(result[0].bibit, 30);
        assert_eq!(result[0].area_m2, 300.75);
        assert_eq!(result[0].jamaah, 75);
        assert_eq!(result[1].label, "Satker Polda");
        assert_eq!(result[1].polsek, 0);
    }

    #[test]
    fn respects_limit_and_ignores_blank_labels() {
        let items = [
            ("A", "", 1, 0, 0.0, 0),
            ("B", "", 2, 0, 0.0, 0),
            ("C", "", 3, 0, 0.0, 0),
            ("  ", "", 4, 0, 0.0, 0),
        ];
        let result = unit_breakdown(items, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "A");
        assert_eq!(result[1].label, "B");
    }
}
