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
