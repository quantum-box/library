//! Rendering. Every command produces a `serde_json::Value`; this module
//! decides whether the user sees that JSON verbatim or a table.
//!
//! `--json` is the contract for anything reading the output as data —
//! scripts, and agents shelling out to the CLI. The table form exists for
//! a human reading a terminal and is explicitly not stable.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
}

pub fn print_json(value: &Value) {
    match serde_json::to_string_pretty(value) {
        Ok(rendered) => println!("{rendered}"),
        Err(_) => println!("{value}"),
    }
}

/// A column-aligned table. Widths come from the content, so a narrow
/// result stays narrow instead of padding out to a fixed layout.
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: &[&str]) -> Self {
        Self {
            headers: headers.iter().map(|h| h.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    pub fn push(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    pub fn print(&self) {
        if self.rows.is_empty() {
            println!("(none)");
            return;
        }

        let widths = self.column_widths();
        println!("{}", join_padded(&self.headers, &widths));
        for row in &self.rows {
            println!("{}", join_padded(row, &widths));
        }
    }

    fn column_widths(&self) -> Vec<usize> {
        let mut widths: Vec<usize> = self
            .headers
            .iter()
            .map(|header| header.chars().count())
            .collect();
        for row in &self.rows {
            for (index, cell) in row.iter().enumerate() {
                if index < widths.len() {
                    widths[index] = widths[index].max(cell.chars().count());
                }
            }
        }
        widths
    }
}

fn join_padded(cells: &[String], widths: &[usize]) -> String {
    let last = cells.len().saturating_sub(1);
    cells
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            // The final column is never padded, so a table copied out of
            // a terminal carries no trailing whitespace.
            if index == last {
                return cell.clone();
            }
            let width = widths.get(index).copied().unwrap_or(0);
            let padding = width.saturating_sub(cell.chars().count());
            format!("{cell}{}", " ".repeat(padding))
        })
        .collect::<Vec<_>>()
        .join("  ")
}

/// Read one field as a display string. Missing and null both render as
/// `-`, so a table never has a hole in it.
pub fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        None | Some(Value::Null) => "-".to_string(),
        Some(Value::String(text)) if text.is_empty() => "-".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

/// Read one field, collapsing it to a single line and trimming it, for
/// columns like `description` that can carry a paragraph.
pub fn short_field(value: &Value, key: &str, limit: usize) -> String {
    let text = field(value, key).replace(['\n', '\r'], " ");
    if text.chars().count() <= limit {
        return text;
    }
    let head: String = text.chars().take(limit.saturating_sub(1)).collect();
    // Trimming keeps the ellipsis flush against the text when the cut
    // lands on a space.
    format!("{}…", head.trim_end())
}

pub fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_missing_field_renders_as_a_dash() {
        let value = json!({ "name": "docs" });

        assert_eq!(field(&value, "description"), "-");
        assert_eq!(field(&value, "name"), "docs");
    }

    #[test]
    fn an_explicit_null_renders_as_a_dash() {
        let value = json!({ "description": null });

        assert_eq!(field(&value, "description"), "-");
    }

    #[test]
    fn a_long_field_is_truncated_to_one_line() {
        let value = json!({ "description": "line one\nline two" });

        assert_eq!(short_field(&value, "description", 10), "line one…");
    }

    #[test]
    fn a_short_field_survives_intact() {
        let value = json!({ "description": "brief" });

        assert_eq!(short_field(&value, "description", 10), "brief");
    }

    #[test]
    fn the_last_column_carries_no_trailing_padding() {
        let widths = vec![5, 5];
        let cells = vec!["ab".to_string(), "cd".to_string()];

        assert_eq!(join_padded(&cells, &widths), "ab     cd");
    }
}
