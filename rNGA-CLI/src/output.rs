//! Output formatting.

use chrono::{Local, TimeZone};
use clap::ValueEnum;
use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
use rust_i18n::t;
use serde::Serialize;

/// Output format options.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Pretty table format
    Table,
    /// JSON format
    Json,
    /// TOON format
    Toon,
    /// Plain text format
    #[default]
    Plain,
}

/// Trait for plain text output.
pub trait PlainPrint {
    /// Print as plain text with formatting.
    fn plain_print(&self);
}

/// Trait for table row generation.
pub trait TableRow {
    /// Get table headers.
    fn headers() -> Vec<&'static str>;
    /// Get row data as strings.
    fn row(&self) -> Vec<String>;
}

/// Print items in plain text format.
pub fn print_plain<T: PlainPrint>(items: &[T]) {
    if items.is_empty() {
        println!("{}", t!("no_results"));
        return;
    }
    for item in items {
        item.plain_print();
    }
}

/// Format a Unix timestamp for display.
pub fn format_time(timestamp: i64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }

    let dt = Local.timestamp_opt(timestamp, 0).single();
    match dt {
        Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
        None => "-".to_string(),
    }
}

/// Format a relative time for display.
pub fn format_relative_time(timestamp: i64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }

    let now = Local::now().timestamp();
    let diff = now - timestamp;

    if diff < 60 {
        t!("time_ago_seconds", count = diff).to_string()
    } else if diff < 3600 {
        t!("time_ago_minutes", m = diff / 60, s = diff % 60).to_string()
    } else if diff < 86400 {
        t!(
            "time_ago_hours",
            h = diff / 3600,
            m = (diff % 3600) / 60,
            s = diff % 60
        )
        .to_string()
    } else if diff < 2592000 {
        t!(
            "time_ago_days",
            d = diff / 86400,
            h = (diff % 86400) / 3600,
            m = (diff % 3600) / 60,
            s = diff % 60
        )
        .to_string()
    } else {
        format_time(timestamp)
    }
}

/// Print a table of items with proper formatting for each output mode.
pub fn print_table<T: TableRow + Serialize + PlainPrint>(items: Vec<T>, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&items).unwrap_or_default()
            );
        }
        OutputFormat::Toon => {
            let json_value = serde_json::to_value(&items).unwrap_or_default();
            println!("{}", toon_format::encode_default(&json_value).unwrap_or_default());
        }
        OutputFormat::Table => {
            if items.is_empty() {
                println!("{}", t!("no_results"));
                return;
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(T::headers());
            for item in &items {
                table.add_row(item.row());
            }
            println!("{table}");
        }
        OutputFormat::Plain => {
            print_plain(&items);
        }
    }
}
