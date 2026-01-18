//! Q-SHEET XLSX Import/Export
//!
//! Handles loading and saving Excel files.

use super::state::{CellValue, SheetState, MAX_COLS};
use calamine::{open_workbook, Reader, Xlsx};
use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

// =============================================================================
// XLSX LOADING
// =============================================================================

/// Load an XLSX file into a SheetState
pub fn load_xlsx(path: &Path) -> Result<SheetState, String> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e| format!("Failed to open Excel file: {}", e))?;

    let mut state = SheetState::new();
    state.file_path = Some(path.to_path_buf());

    // Get the first sheet
    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Ok(state); // Empty workbook
    }

    let range = workbook
        .worksheet_range(&sheet_names[0])
        .map_err(|e| format!("Failed to read worksheet: {}", e))?;

    let mut max_row = 0;
    for (row_idx, row) in range.rows().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx >= MAX_COLS {
                break;
            }

            let value = match cell {
                calamine::Data::Empty => CellValue::Empty,
                calamine::Data::String(s) => {
                    if s.is_empty() {
                        CellValue::Empty
                    } else if s.starts_with('=') {
                        // Formula stored as string (calamine doesn't preserve formulas well)
                        CellValue::Formula {
                            formula: s.clone(),
                            cached: 0.0,
                        }
                    } else {
                        CellValue::Text(s.clone())
                    }
                }
                calamine::Data::Float(n) => CellValue::Number(*n),
                calamine::Data::Int(n) => CellValue::Number(*n as f64),
                calamine::Data::Bool(b) => CellValue::Number(if *b { 1.0 } else { 0.0 }),
                calamine::Data::Error(e) => CellValue::Error(format!("{:?}", e)),
                calamine::Data::DateTime(dt) => {
                    // Convert Excel datetime to a readable string
                    CellValue::Text(dt.to_string())
                }
                calamine::Data::DateTimeIso(s) => CellValue::Text(s.clone()),
                calamine::Data::DurationIso(s) => CellValue::Text(s.clone()),
            };

            if !value.is_empty() {
                state.cells.insert((col_idx, row_idx), value);
                max_row = max_row.max(row_idx);
            }
        }
    }

    state.row_count = (max_row + 1).max(100);
    state.modified = false;

    // Recalculate formulas
    state.recalculate();

    Ok(state)
}

// =============================================================================
// XLSX SAVING
// =============================================================================

/// Save a SheetState to an XLSX file
pub fn save_xlsx(state: &SheetState, path: &Path) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Create formats
    let number_format = Format::new().set_num_format("0.##");

    // Find the extent of data
    let max_row = state.cells.keys().map(|(_, r)| *r).max().unwrap_or(0);
    let max_col = state.cells.keys().map(|(c, _)| *c).max().unwrap_or(0);

    // Write data
    for row in 0..=max_row {
        for col in 0..=max_col {
            let value = state.get_cell(col, row);
            let row_u32 = row as u32;
            let col_u16 = col as u16;

            match value {
                CellValue::Empty => {}
                CellValue::Text(s) => {
                    worksheet
                        .write_string(row_u32, col_u16, s)
                        .map_err(|e| format!("Failed to write cell: {}", e))?;
                }
                CellValue::Number(n) => {
                    worksheet
                        .write_number_with_format(row_u32, col_u16, *n, &number_format)
                        .map_err(|e| format!("Failed to write cell: {}", e))?;
                }
                CellValue::Formula { formula, .. } => {
                    // Write formula (without leading =)
                    let formula_str = formula.strip_prefix('=').unwrap_or(formula);
                    worksheet
                        .write_formula(row_u32, col_u16, formula_str)
                        .map_err(|e| format!("Failed to write formula: {}", e))?;
                }
                CellValue::Error(e) => {
                    worksheet
                        .write_string(row_u32, col_u16, format!("#ERR:{}", e))
                        .map_err(|e| format!("Failed to write cell: {}", e))?;
                }
            }
        }
    }

    // Set column widths
    for col in 0..=max_col {
        let width = state.col_widths[col] as f64;
        worksheet
            .set_column_width(col as u16, width)
            .map_err(|e| format!("Failed to set column width: {}", e))?;
    }

    workbook
        .save(path)
        .map_err(|e| format!("Failed to save Excel file: {}", e))
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xlsx_roundtrip() {
        let mut state = SheetState::new();
        state
            .cells
            .insert((0, 0), CellValue::Text("Hello".to_string()));
        state.cells.insert((1, 0), CellValue::Number(42.0));
        state.cells.insert((0, 1), CellValue::Number(3.14159));

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("qsheet_test.xlsx");

        // Save
        save_xlsx(&state, &temp_file).expect("Failed to save");

        // Load
        let loaded = load_xlsx(&temp_file).expect("Failed to load");

        // Verify
        assert_eq!(loaded.get_cell(0, 0).display(), "Hello");
        assert_eq!(loaded.get_cell(1, 0).as_number(), Some(42.0));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }
}
