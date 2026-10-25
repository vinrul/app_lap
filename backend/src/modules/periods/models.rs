use serde::{Deserialize, Serialize};

use crate::modules::{
    imports::models::ImportView,
    reports::models::{DraftView, ReportView},
};

#[derive(Clone, Debug, Serialize)]
pub struct PeriodView {
    pub id: String,
    pub start_date: String,
    pub end_date: String,
    pub label: String,
    pub period_type: String,
    pub created_at: String,
    pub created_by: String,
    pub imports: i64,
    pub rows: i64,
    pub reports: i64,
}

#[derive(Debug, Serialize)]
pub struct PeriodDetail {
    pub period: PeriodView,
    pub imports: Vec<ImportView>,
    pub reports: Vec<ReportView>,
    pub draft: Option<DraftView>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePeriodRequest {
    pub start_date: String,
    pub end_date: String,
    pub label: Option<String>,
    #[serde(default)]
    pub period_type: Option<String>,
}
