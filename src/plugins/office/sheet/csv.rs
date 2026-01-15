//! Q-SHEET CSV Import/Export
//!
//! Handles loading and saving CSV files with formula preservation.

use super::state::{CellValue, SheetState, MAX_COLS};
use std::fs;
use std::path::Path;

// =============================================================================
// CSV LOADING
// =============================================================================

/// Load a CSV file into a SheetState
pub fn load_csv(path: &Path) -> Result<SheetState, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut state = SheetState::new();
    state.file_path = Some(path.to_path_buf());

    // Parse formula comments first
    let mut formulas: Vec<(usize, usize, String)> = Vec::new();

    for line in content.lines() {
        if line.starts_with("# Q-SHEET formula:") {
            // Parse formula comment: "# Q-SHEET formula: B4=B2-B3"
            if let Some(formula_part) = line.strip_prefix("# Q-SHEET formula:") {
                let formula_part = formula_part.trim();
                if let Some(eq_pos) = formula_part.find('=') {
                    let cell_ref = &formula_part[..eq_pos];
                    let formula = &formula_part[eq_pos..];

                    if let Some(cell) = super::formula::parse_cell_ref(cell_ref) {
                        formulas.push((cell.col, cell.row, formula.to_string()));
                    }
                }
            }
        }
    }

    // Parse CSV data (skip comment lines)
    let mut row = 0;
    for line in content.lines() {
        if line.starts_with('#') {
            continue;
        }

        let fields = parse_csv_line(line);
        for (col, field) in fields.into_iter().enumerate() {
            if col >= MAX_COLS {
                break;
            }

            let trimmed = field.trim();
            if !trimmed.is_empty() {
                // Check if this cell has a formula
                let has_formula = formulas.iter().any(|(c, r, _)| *c == col && *r == row);

                let value = if has_formula {
                    // Find the formula
                    if let Some((_, _, formula)) =
                        formulas.iter().find(|(c, r, _)| *c == col && *r == row)
                    {
                        // Parse as number for cached value
                        let cached = trimmed.parse::<f64>().unwrap_or(0.0);
                        CellValue::Formula {
                            formula: formula.clone(),
                            cached,
                        }
                    } else {
                        parse_cell_value(trimmed)
                    }
                } else {
                    parse_cell_value(trimmed)
                };

                if !value.is_empty() {
                    state.cells.insert((col, row), value);
                }
            }
        }
        row += 1;
    }

    state.row_count = row.max(100);
    state.modified = false;

    // Recalculate all formulas with actual cell references
    state.recalculate();

    Ok(state)
}

/// Parse a single CSV line, handling quoted fields
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    // Escaped quote
                    current.push('"');
                    chars.next();
                } else {
                    // End of quoted field
                    in_quotes = false;
                }
            } else {
                current.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            fields.push(current);
            current = String::new();
        } else {
            current.push(c);
        }
    }

    fields.push(current);
    fields
}

/// Parse a cell value from a string
fn parse_cell_value(s: &str) -> CellValue {
    if s.is_empty() {
        return CellValue::Empty;
    }

    // Try parsing as number
    if let Ok(n) = s.parse::<f64>() {
        return CellValue::Number(n);
    }

    // Otherwise it's text
    CellValue::Text(s.to_string())
}

// =============================================================================
// CSV SAVING
// =============================================================================

/// Save a SheetState to a CSV file
pub fn save_csv(state: &SheetState, path: &Path) -> Result<(), String> {
    let mut output = String::new();

    // Collect formula comments
    let mut formulas: Vec<(usize, usize, String)> = state
        .cells
        .iter()
        .filter_map(|((col, row), value)| {
            if let CellValue::Formula { formula, .. } = value {
                Some((*col, *row, formula.clone()))
            } else {
                None
            }
        })
        .collect();

    // Sort formulas by row then column for consistent output
    formulas.sort_by(|a, b| (a.1, a.0).cmp(&(b.1, b.0)));

    // Write formula comments
    for (col, row, formula) in &formulas {
        let cell_addr = format!("{}{}", SheetState::col_to_letter(*col), row + 1);
        output.push_str(&format!("# Q-SHEET formula: {}{}\n", cell_addr, formula));
    }

    // Find the extent of data
    let max_row = state.cells.keys().map(|(_, r)| *r).max().unwrap_or(0);
    let max_col = state.cells.keys().map(|(c, _)| *c).max().unwrap_or(0);

    // Write data rows
    for row in 0..=max_row {
        let mut fields: Vec<String> = Vec::new();

        for col in 0..=max_col {
            let value = state.get_cell(col, row);
            let display = match value {
                CellValue::Empty => String::new(),
                CellValue::Text(s) => escape_csv_field(s),
                CellValue::Number(n) => {
                    if n.fract() == 0.0 && n.abs() < 1e10 {
                        format!("{}", *n as i64)
                    } else {
                        format!("{}", n)
                    }
                }
                CellValue::Formula { cached, .. } => {
                    if cached.fract() == 0.0 && cached.abs() < 1e10 {
                        format!("{}", *cached as i64)
                    } else {
                        format!("{}", cached)
                    }
                }
                CellValue::Error(e) => format!("#ERR:{}", e),
            };
            fields.push(display);
        }

        // Trim trailing empty fields
        while fields.last().map(|s| s.is_empty()).unwrap_or(false) {
            fields.pop();
        }

        output.push_str(&fields.join(","));
        output.push('\n');
    }

    fs::write(path, output).map_err(|e| format!("Failed to write file: {}", e))
}

/// Escape a field for CSV output
fn escape_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_line() {
        let fields = parse_csv_line("a,b,c");
        assert_eq!(fields, vec!["a", "b", "c"]);

        let fields = parse_csv_line("\"hello, world\",b,c");
        assert_eq!(fields, vec!["hello, world", "b", "c"]);

        let fields = parse_csv_line("\"a\"\"b\",c");
        assert_eq!(fields, vec!["a\"b", "c"]);
    }

    #[test]
    fn test_escape_csv_field() {
        assert_eq!(escape_csv_field("hello"), "hello");
        assert_eq!(escape_csv_field("hello, world"), "\"hello, world\"");
        assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
