use serde::{Deserialize, Serialize};

/// Configuration for the photo index (appendix) at the end of the book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendixConfig {
    /// Enable photo index at end of book
    #[serde(default = "default_active")]
    pub active: bool,
    /// Number of columns in the listing
    #[serde(default = "default_columns")]
    pub columns: u32,
    /// Reference style: "positions" (page.slot) or "counter" (sequential number)
    #[serde(default = "default_ref_mode")]
    pub ref_mode: String,
    /// Show page-number headers between pages
    #[serde(default = "default_page_separator")]
    pub page_separator: bool,
    /// Strip timestamps from filenames in the listing
    #[serde(default = "default_strip_timestamps")]
    pub strip_timestamps: bool,
    /// Title of the appendix
    #[serde(default = "default_label_title")]
    pub label_title: String,
    /// "Page" label text
    #[serde(default = "default_label_page")]
    pub label_page: String,
    /// Date format string; placeholders: {day} {month} {year} {hour} {min}
    #[serde(default = "default_date_format")]
    pub date_format: String,
    /// Month abbreviations (12 entries, Jan–Dec)
    #[serde(default = "default_date_months")]
    pub date_months: Vec<String>,
}

impl Default for AppendixConfig {
    fn default() -> Self {
        Self {
            active: default_active(),
            columns: default_columns(),
            ref_mode: default_ref_mode(),
            page_separator: default_page_separator(),
            strip_timestamps: default_strip_timestamps(),
            label_title: default_label_title(),
            label_page: default_label_page(),
            date_format: default_date_format(),
            date_months: default_date_months(),
        }
    }
}

fn default_active() -> bool {
    false
}

fn default_columns() -> u32 {
    7
}

fn default_ref_mode() -> String {
    "positions".to_string()
}

fn default_page_separator() -> bool {
    false
}

fn default_strip_timestamps() -> bool {
    true
}

fn default_label_title() -> String {
    "Photo Index".to_string()
}

fn default_label_page() -> String {
    "Page".to_string()
}

fn default_date_format() -> String {
    "{day}. {month} {year} {hour}:{min}".to_string()
}

fn default_date_months() -> Vec<String> {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}
