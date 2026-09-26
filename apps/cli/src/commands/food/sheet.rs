//! Worksheets as plain grids of text.
//!
//! The parser never sees calamine types. A cell is either empty or the
//! text a reader of the table would see, so the same parser runs on the
//! official workbook and on the hand-written grids in the tests.

use std::io::Cursor;

use anyhow::{anyhow, Context, Result};
use calamine::{Data, Reader, Xlsx};

/// One worksheet with absolute row/column positions (0-based).
#[derive(Debug, Clone, Default)]
pub struct Sheet {
    pub name: String,
    rows: Vec<Vec<Option<String>>>,
}

impl Sheet {
    pub fn new(name: &str, rows: Vec<Vec<Option<String>>>) -> Self {
        Self {
            name: name.to_string(),
            rows,
        }
    }

    /// Build a sheet from string rows; `""` is an empty cell. For tests.
    #[cfg(test)]
    pub fn from_rows(name: &str, rows: &[&[&str]]) -> Self {
        Self::new(
            name,
            rows.iter()
                .map(|row| {
                    row.iter()
                        .map(|c| (!c.is_empty()).then(|| c.to_string()))
                        .collect()
                })
                .collect(),
        )
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    pub fn width(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }

    /// The cell text, or `None` for an empty cell. Text is returned as
    /// stored; callers decide how much whitespace matters.
    pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .and_then(|c| c.as_deref())
    }

    pub fn row_is_blank(&self, row: usize) -> bool {
        self.rows.get(row).is_none_or(|r| {
            r.iter()
                .all(|c| c.as_deref().is_none_or(|s| s.trim().is_empty()))
        })
    }
}

/// A workbook's sheets in file order.
#[derive(Debug, Clone, Default)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

impl Workbook {
    pub fn read_xlsx(bytes: &[u8]) -> Result<Self> {
        let mut book = Xlsx::new(Cursor::new(bytes))
            .map_err(|e| anyhow!("not a readable .xlsx workbook: {e}"))?;
        let names = book.sheet_names();
        let mut sheets = Vec::with_capacity(names.len());
        for name in names {
            let range = book
                .worksheet_range(&name)
                .with_context(|| format!("failed to read sheet {name}"))?;
            let (row0, col0) = range.start().unwrap_or((0, 0));
            let mut rows: Vec<Vec<Option<String>>> =
                vec![Vec::new(); row0 as usize];
            for source_row in range.rows() {
                let mut row: Vec<Option<String>> =
                    vec![None; col0 as usize];
                row.extend(source_row.iter().map(cell_text));
                rows.push(row);
            }
            sheets.push(Sheet::new(&name, rows));
        }
        Ok(Self { sheets })
    }

    pub fn sheet(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.name == name)
    }

    pub fn sheet_names(&self) -> Vec<&str> {
        self.sheets.iter().map(|s| s.name.as_str()).collect()
    }
}

/// What a person reading the table sees in a cell.
///
/// Numbers keep their shortest round-trip form (`0.92`, `1452`), which is
/// how the workbook stores them. Text cells are the table's own notation
/// (`(0)`, `Tr`, `5.0`) and are returned untouched.
fn cell_text(data: &Data) -> Option<String> {
    match data {
        Data::Empty => None,
        Data::String(s) => Some(s.clone()),
        Data::Int(i) => Some(i.to_string()),
        Data::Float(f) => Some(format_number(*f)),
        Data::Bool(b) => Some(b.to_string()),
        Data::DateTime(d) => Some(d.to_string()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => Some(s.clone()),
        Data::Error(e) => Some(format!("#ERROR:{e:?}")),
    }
}

/// Shortest decimal text of a float, without exponent.
pub fn format_number(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

/// Remove every kind of whitespace, including the ideographic space the
/// table uses to pad headers (`食　品　番　号`).
pub fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Full-width ASCII letters and digits to ASCII (`Ｂ１` → `B1`).
pub fn narrow_alnum(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'Ａ'..='Ｚ' | 'ａ'..='ｚ' | '０'..='９' => {
                char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)
            }
            other => other,
        })
        .collect()
}

/// A loose form of a label for matching one table's wording against
/// another's: no whitespace, no hyphen-like marks, ASCII letters/digits
/// and brackets. `β|クリプトキサンチン` and `βクリプトキサンチン` match.
pub fn match_key(s: &str) -> String {
    narrow_alnum(&squash(s))
        .chars()
        .filter(|c| !matches!(c, '|' | '-' | '‐' | '－' | '−' | '・'))
        .map(|c| match c {
            '（' => '(',
            '）' => ')',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_render_like_the_printed_table() {
        assert_eq!(format_number(1452.0), "1452");
        assert_eq!(format_number(0.92), "0.92");
        assert_eq!(format_number(13.5), "13.5");
        assert_eq!(format_number(0.00001), "0.00001");
    }

    #[test]
    fn labels_match_across_wordings() {
        assert_eq!(
            match_key("β|クリプトキサンチン"),
            "βクリプトキサンチン"
        );
        assert_eq!(
            match_key("食\u{3000}品\u{3000}番\u{3000}号"),
            "食品番号"
        );
        assert_eq!(match_key("ビ\nタ\nミ\nン\nＢ１"), "ビタミンB1");
        assert_eq!(
            match_key("利用可能炭水化物\n（単糖当量）"),
            "利用可能炭水化物(単糖当量)"
        );
    }

    #[test]
    fn a_sheet_reports_cells_by_absolute_position() {
        let sheet = Sheet::from_rows("s", &[&["a", ""], &["", "b"]]);
        assert_eq!(sheet.cell(0, 0), Some("a"));
        assert_eq!(sheet.cell(0, 1), None);
        assert_eq!(sheet.cell(1, 1), Some("b"));
        assert_eq!(sheet.cell(5, 5), None);
        assert!(!sheet.row_is_blank(1));
        assert!(sheet.row_is_blank(2));
    }
}
