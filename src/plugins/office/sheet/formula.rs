//! Q-SHEET Formula Engine
//!
//! Parses and evaluates spreadsheet formulas.

use super::state::CellValue;
use std::collections::HashMap;

// =============================================================================
// CELL REFERENCE
// =============================================================================

/// A reference to a single cell
#[derive(Debug, Clone, Copy)]
pub struct CellRef {
    pub col: usize,
    pub row: usize,
}

/// A range of cells (e.g., A1:D10)
#[derive(Debug, Clone, Copy)]
pub struct CellRange {
    pub start: CellRef,
    pub end: CellRef,
}

impl CellRange {
    /// Iterate over all cells in the range
    pub fn iter(&self) -> impl Iterator<Item = (usize, usize)> {
        let start_col = self.start.col.min(self.end.col);
        let end_col = self.start.col.max(self.end.col);
        let start_row = self.start.row.min(self.end.row);
        let end_row = self.start.row.max(self.end.row);

        (start_row..=end_row).flat_map(move |row| (start_col..=end_col).map(move |col| (col, row)))
    }
}

// =============================================================================
// PARSING
// =============================================================================

/// Parse a cell reference like "A1", "$A$1", "B10"
pub fn parse_cell_ref(s: &str) -> Option<CellRef> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Skip $ signs for absolute references (we treat them the same for now)
    let s = s.replace('$', "");

    // Find where digits start
    let letter_end = s.chars().take_while(|c| c.is_ascii_alphabetic()).count();
    if letter_end == 0 {
        return None;
    }

    let col_str = &s[..letter_end];
    let row_str = &s[letter_end..];

    // Parse column (A=0, Z=25)
    if col_str.len() != 1 {
        return None; // Only support A-Z for now
    }
    let col_char = col_str.chars().next()?.to_ascii_uppercase();
    if !col_char.is_ascii_uppercase() {
        return None;
    }
    let col = (col_char as u8 - b'A') as usize;

    // Parse row (1-indexed in formula, 0-indexed internally)
    let row: usize = row_str.parse().ok()?;
    if row == 0 {
        return None;
    }

    Some(CellRef { col, row: row - 1 })
}

/// Parse a cell range like "A1:D10"
pub fn parse_range(s: &str) -> Option<CellRange> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let start = parse_cell_ref(parts[0])?;
    let end = parse_cell_ref(parts[1])?;

    Some(CellRange { start, end })
}

// =============================================================================
// TOKENIZER
// =============================================================================

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    CellRef(CellRef),
    Range(CellRange),
    Function(String),
    Operator(char),
    OpenParen,
    CloseParen,
    Comma,
    String(String),
}

fn tokenize(formula: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Skip whitespace
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // Operators
        if "+-*/^".contains(c) {
            tokens.push(Token::Operator(c));
            i += 1;
            continue;
        }

        // Comparison operators (for IF)
        if c == '>' || c == '<' {
            // Check for >= or <=
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::Operator(if c == '>' { 'G' } else { 'L' })); // G=>=, L=<=
                i += 2;
            } else {
                tokens.push(Token::Operator(c));
                i += 1;
            }
            continue;
        }

        if c == '=' && i > 0 {
            // = as comparison (not formula start)
            tokens.push(Token::Operator('='));
            i += 1;
            continue;
        }

        // Parentheses
        if c == '(' {
            tokens.push(Token::OpenParen);
            i += 1;
            continue;
        }
        if c == ')' {
            tokens.push(Token::CloseParen);
            i += 1;
            continue;
        }

        // Comma
        if c == ',' {
            tokens.push(Token::Comma);
            i += 1;
            continue;
        }

        // String literal
        if c == '"' {
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                s.push(chars[i]);
                i += 1;
            }
            i += 1; // Skip closing quote
            tokens.push(Token::String(s));
            continue;
        }

        // Number
        if c.is_ascii_digit() || (c == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            if c == '-' {
                i += 1;
            }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            let n: f64 = num_str
                .parse()
                .map_err(|_| format!("Invalid number: {}", num_str))?;
            tokens.push(Token::Number(n));
            continue;
        }

        // Cell reference, range, or function name
        if c.is_ascii_alphabetic() || c == '$' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '$' || chars[i] == ':')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();

            // Check if it's a range (contains :)
            if word.contains(':') {
                if let Some(range) = parse_range(&word) {
                    tokens.push(Token::Range(range));
                    continue;
                }
            }

            // Check if it's a cell reference
            if let Some(cell) = parse_cell_ref(&word) {
                tokens.push(Token::CellRef(cell));
                continue;
            }

            // Must be a function name
            tokens.push(Token::Function(word.to_uppercase()));
            continue;
        }

        // Skip formula start =
        if c == '=' && i == 0 {
            i += 1;
            continue;
        }

        return Err(format!("Unexpected character: {}", c));
    }

    Ok(tokens)
}

// =============================================================================
// EVALUATOR
// =============================================================================

/// Evaluate a formula and return the result
pub fn evaluate(formula: &str, cells: &HashMap<(usize, usize), CellValue>) -> Result<f64, String> {
    let tokens = tokenize(formula)?;
    if tokens.is_empty() {
        return Ok(0.0);
    }

    eval_expression(&tokens, cells, &mut 0)
}

fn eval_expression(
    tokens: &[Token],
    cells: &HashMap<(usize, usize), CellValue>,
    pos: &mut usize,
) -> Result<f64, String> {
    let mut left = eval_term(tokens, cells, pos)?;

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Operator('+') => {
                *pos += 1;
                let right = eval_term(tokens, cells, pos)?;
                left += right;
            }
            Token::Operator('-') => {
                *pos += 1;
                let right = eval_term(tokens, cells, pos)?;
                left -= right;
            }
            Token::Operator('>') => {
                *pos += 1;
                let right = eval_term(tokens, cells, pos)?;
                left = if left > right { 1.0 } else { 0.0 };
            }
            Token::Operator('<') => {
                *pos += 1;
                let right = eval_term(tokens, cells, pos)?;
                left = if left < right { 1.0 } else { 0.0 };
            }
            Token::Operator('G') => {
                // >=
                *pos += 1;
                let right = eval_term(tokens, cells, pos)?;
                left = if left >= right { 1.0 } else { 0.0 };
            }
            Token::Operator('L') => {
                // <=
                *pos += 1;
                let right = eval_term(tokens, cells, pos)?;
                left = if left <= right { 1.0 } else { 0.0 };
            }
            Token::Operator('=') => {
                *pos += 1;
                let right = eval_term(tokens, cells, pos)?;
                left = if (left - right).abs() < f64::EPSILON {
                    1.0
                } else {
                    0.0
                };
            }
            _ => break,
        }
    }

    Ok(left)
}

fn eval_term(
    tokens: &[Token],
    cells: &HashMap<(usize, usize), CellValue>,
    pos: &mut usize,
) -> Result<f64, String> {
    let mut left = eval_factor(tokens, cells, pos)?;

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Operator('*') => {
                *pos += 1;
                let right = eval_factor(tokens, cells, pos)?;
                left *= right;
            }
            Token::Operator('/') => {
                *pos += 1;
                let right = eval_factor(tokens, cells, pos)?;
                if right == 0.0 {
                    return Err("DIV/0".to_string());
                }
                left /= right;
            }
            _ => break,
        }
    }

    Ok(left)
}

fn eval_factor(
    tokens: &[Token],
    cells: &HashMap<(usize, usize), CellValue>,
    pos: &mut usize,
) -> Result<f64, String> {
    if *pos >= tokens.len() {
        return Err("Unexpected end of formula".to_string());
    }

    match &tokens[*pos] {
        Token::Number(n) => {
            *pos += 1;
            Ok(*n)
        }
        Token::CellRef(cell) => {
            *pos += 1;
            let value = cells.get(&(cell.col, cell.row));
            match value {
                Some(CellValue::Number(n)) => Ok(*n),
                Some(CellValue::Formula { cached, .. }) => Ok(*cached),
                Some(CellValue::Text(s)) => s.parse().ok().ok_or_else(|| "VALUE".to_string()),
                _ => Ok(0.0),
            }
        }
        Token::OpenParen => {
            *pos += 1;
            let result = eval_expression(tokens, cells, pos)?;
            if *pos < tokens.len() && matches!(tokens[*pos], Token::CloseParen) {
                *pos += 1;
            }
            Ok(result)
        }
        Token::Function(name) => {
            *pos += 1;
            eval_function(name, tokens, cells, pos)
        }
        Token::Operator('-') => {
            *pos += 1;
            let value = eval_factor(tokens, cells, pos)?;
            Ok(-value)
        }
        _ => Err(format!("Unexpected token at position {}", pos)),
    }
}

fn eval_function(
    name: &str,
    tokens: &[Token],
    cells: &HashMap<(usize, usize), CellValue>,
    pos: &mut usize,
) -> Result<f64, String> {
    // Expect opening paren
    if *pos >= tokens.len() || !matches!(tokens[*pos], Token::OpenParen) {
        return Err(format!("Expected ( after function {}", name));
    }
    *pos += 1;

    // Collect arguments
    let mut args: Vec<f64> = Vec::new();
    let mut ranges: Vec<CellRange> = Vec::new();

    while *pos < tokens.len() && !matches!(tokens[*pos], Token::CloseParen) {
        // Check for range first
        if let Token::Range(range) = &tokens[*pos] {
            ranges.push(*range);
            *pos += 1;
        } else {
            let value = eval_expression(tokens, cells, pos)?;
            args.push(value);
        }

        // Skip comma
        if *pos < tokens.len() && matches!(tokens[*pos], Token::Comma) {
            *pos += 1;
        }
    }

    // Skip closing paren
    if *pos < tokens.len() && matches!(tokens[*pos], Token::CloseParen) {
        *pos += 1;
    }

    // Expand ranges into values
    for range in ranges {
        for (col, row) in range.iter() {
            let value = cells.get(&(col, row));
            if let Some(n) = value.and_then(|v| v.as_number()) {
                args.push(n);
            }
        }
    }

    // Execute function (Lotus 1-2-3 style @ functions)
    // Note: Lotus uses @ prefix but we handle both = and @ style internally
    match name {
        // =====================================================================
        // AGGREGATE FUNCTIONS
        // =====================================================================
        "SUM" => Ok(args.iter().sum()),
        "AVG" | "AVERAGE" => {
            if args.is_empty() {
                Ok(0.0)
            } else {
                Ok(args.iter().sum::<f64>() / args.len() as f64)
            }
        }
        "COUNT" => Ok(args.len() as f64),
        "MIN" => args
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .ok_or_else(|| "No values".to_string()),
        "MAX" => args
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .ok_or_else(|| "No values".to_string()),

        // =====================================================================
        // MATH FUNCTIONS
        // =====================================================================
        "ABS" => {
            if args.is_empty() {
                Err("ABS requires an argument".to_string())
            } else {
                Ok(args[0].abs())
            }
        }
        "SQRT" => {
            if args.is_empty() {
                Err("SQRT requires an argument".to_string())
            } else if args[0] < 0.0 {
                Err("SQRT of negative".to_string())
            } else {
                Ok(args[0].sqrt())
            }
        }
        "INT" => {
            if args.is_empty() {
                Err("INT requires an argument".to_string())
            } else {
                Ok(args[0].floor())
            }
        }
        "MOD" => {
            if args.len() < 2 {
                Err("MOD requires 2 arguments".to_string())
            } else if args[1] == 0.0 {
                Err("DIV/0".to_string())
            } else {
                Ok(args[0] % args[1])
            }
        }
        "ROUND" => {
            if args.is_empty() {
                Err("ROUND requires an argument".to_string())
            } else {
                let decimals = args.get(1).copied().unwrap_or(0.0) as i32;
                let factor = 10f64.powi(decimals);
                Ok((args[0] * factor).round() / factor)
            }
        }
        "EXP" => {
            if args.is_empty() {
                Err("EXP requires an argument".to_string())
            } else {
                Ok(args[0].exp())
            }
        }
        "LN" => {
            if args.is_empty() {
                Err("LN requires an argument".to_string())
            } else if args[0] <= 0.0 {
                Err("LN of non-positive".to_string())
            } else {
                Ok(args[0].ln())
            }
        }
        "LOG" => {
            if args.is_empty() {
                Err("LOG requires an argument".to_string())
            } else if args[0] <= 0.0 {
                Err("LOG of non-positive".to_string())
            } else {
                Ok(args[0].log10())
            }
        }
        "PI" => Ok(std::f64::consts::PI),
        "RAND" => Ok(rand::random::<f64>()),
        "SIN" => {
            if args.is_empty() {
                Err("SIN requires an argument".to_string())
            } else {
                Ok(args[0].sin())
            }
        }
        "COS" => {
            if args.is_empty() {
                Err("COS requires an argument".to_string())
            } else {
                Ok(args[0].cos())
            }
        }
        "TAN" => {
            if args.is_empty() {
                Err("TAN requires an argument".to_string())
            } else {
                Ok(args[0].tan())
            }
        }
        "ASIN" => {
            if args.is_empty() {
                Err("ASIN requires an argument".to_string())
            } else if args[0] < -1.0 || args[0] > 1.0 {
                Err("ASIN out of range".to_string())
            } else {
                Ok(args[0].asin())
            }
        }
        "ACOS" => {
            if args.is_empty() {
                Err("ACOS requires an argument".to_string())
            } else if args[0] < -1.0 || args[0] > 1.0 {
                Err("ACOS out of range".to_string())
            } else {
                Ok(args[0].acos())
            }
        }
        "ATAN" => {
            if args.is_empty() {
                Err("ATAN requires an argument".to_string())
            } else {
                Ok(args[0].atan())
            }
        }
        "ATAN2" => {
            if args.len() < 2 {
                Err("ATAN2 requires 2 arguments".to_string())
            } else {
                Ok(args[0].atan2(args[1]))
            }
        }
        "POWER" => {
            if args.len() < 2 {
                Err("POWER requires 2 arguments".to_string())
            } else {
                Ok(args[0].powf(args[1]))
            }
        }
        "SIGN" => {
            if args.is_empty() {
                Err("SIGN requires an argument".to_string())
            } else if args[0] > 0.0 {
                Ok(1.0)
            } else if args[0] < 0.0 {
                Ok(-1.0)
            } else {
                Ok(0.0)
            }
        }

        // =====================================================================
        // LOGICAL FUNCTIONS
        // =====================================================================
        "IF" => {
            if args.len() >= 3 {
                Ok(if args[0] != 0.0 { args[1] } else { args[2] })
            } else if args.len() == 2 {
                Ok(if args[0] != 0.0 { args[1] } else { 0.0 })
            } else {
                Err("IF requires 2-3 arguments".to_string())
            }
        }
        "TRUE" => Ok(1.0),
        "FALSE" => Ok(0.0),
        "AND" => {
            if args.is_empty() {
                Ok(1.0)
            } else {
                Ok(if args.iter().all(|&x| x != 0.0) {
                    1.0
                } else {
                    0.0
                })
            }
        }
        "OR" => {
            if args.is_empty() {
                Ok(0.0)
            } else {
                Ok(if args.iter().any(|&x| x != 0.0) {
                    1.0
                } else {
                    0.0
                })
            }
        }
        "NOT" => {
            if args.is_empty() {
                Err("NOT requires an argument".to_string())
            } else {
                Ok(if args[0] == 0.0 { 1.0 } else { 0.0 })
            }
        }

        // =====================================================================
        // STATISTICAL FUNCTIONS
        // =====================================================================
        "STDEV" | "STD" => {
            if args.len() < 2 {
                Err("STDEV requires at least 2 values".to_string())
            } else {
                let mean = args.iter().sum::<f64>() / args.len() as f64;
                let variance =
                    args.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (args.len() - 1) as f64;
                Ok(variance.sqrt())
            }
        }
        "VAR" => {
            if args.len() < 2 {
                Err("VAR requires at least 2 values".to_string())
            } else {
                let mean = args.iter().sum::<f64>() / args.len() as f64;
                let variance =
                    args.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (args.len() - 1) as f64;
                Ok(variance)
            }
        }

        // =====================================================================
        // FINANCIAL FUNCTIONS (basic)
        // =====================================================================
        "PMT" => {
            // PMT(rate, nper, pv) - payment for a loan
            if args.len() < 3 {
                Err("PMT requires 3 arguments (rate, nper, pv)".to_string())
            } else {
                let rate = args[0];
                let nper = args[1];
                let pv = args[2];
                if rate == 0.0 {
                    Ok(-pv / nper)
                } else {
                    Ok(-pv * rate * (1.0 + rate).powf(nper) / ((1.0 + rate).powf(nper) - 1.0))
                }
            }
        }
        "FV" => {
            // FV(rate, nper, pmt) - future value
            if args.len() < 3 {
                Err("FV requires 3 arguments (rate, nper, pmt)".to_string())
            } else {
                let rate = args[0];
                let nper = args[1];
                let pmt = args[2];
                if rate == 0.0 {
                    Ok(-pmt * nper)
                } else {
                    Ok(-pmt * ((1.0 + rate).powf(nper) - 1.0) / rate)
                }
            }
        }
        "PV" => {
            // PV(rate, nper, pmt) - present value
            if args.len() < 3 {
                Err("PV requires 3 arguments (rate, nper, pmt)".to_string())
            } else {
                let rate = args[0];
                let nper = args[1];
                let pmt = args[2];
                if rate == 0.0 {
                    Ok(-pmt * nper)
                } else {
                    Ok(-pmt * (1.0 - (1.0 + rate).powf(-nper)) / rate)
                }
            }
        }

        _ => Err(format!("Unknown function: {}", name)),
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell_ref() {
        let cell = parse_cell_ref("A1").unwrap();
        assert_eq!(cell.col, 0);
        assert_eq!(cell.row, 0);

        let cell = parse_cell_ref("B10").unwrap();
        assert_eq!(cell.col, 1);
        assert_eq!(cell.row, 9);

        let cell = parse_cell_ref("$A$1").unwrap();
        assert_eq!(cell.col, 0);
        assert_eq!(cell.row, 0);
    }

    #[test]
    fn test_parse_range() {
        let range = parse_range("A1:D5").unwrap();
        assert_eq!(range.start.col, 0);
        assert_eq!(range.start.row, 0);
        assert_eq!(range.end.col, 3);
        assert_eq!(range.end.row, 4);
    }

    #[test]
    fn test_simple_formula() {
        let cells = HashMap::new();
        assert_eq!(evaluate("=1+2", &cells).unwrap(), 3.0);
        assert_eq!(evaluate("=10-3", &cells).unwrap(), 7.0);
        assert_eq!(evaluate("=2*3", &cells).unwrap(), 6.0);
        assert_eq!(evaluate("=10/2", &cells).unwrap(), 5.0);
    }

    #[test]
    fn test_cell_reference() {
        let mut cells = HashMap::new();
        cells.insert((0, 0), CellValue::Number(10.0));
        cells.insert((1, 0), CellValue::Number(5.0));

        assert_eq!(evaluate("=A1", &cells).unwrap(), 10.0);
        assert_eq!(evaluate("=A1+B1", &cells).unwrap(), 15.0);
    }

    #[test]
    fn test_sum_function() {
        let mut cells = HashMap::new();
        cells.insert((0, 0), CellValue::Number(1.0));
        cells.insert((0, 1), CellValue::Number(2.0));
        cells.insert((0, 2), CellValue::Number(3.0));

        assert_eq!(evaluate("=SUM(A1:A3)", &cells).unwrap(), 6.0);
    }

    #[test]
    fn test_if_function() {
        let cells = HashMap::new();
        assert_eq!(evaluate("=IF(1>0,10,20)", &cells).unwrap(), 10.0);
        assert_eq!(evaluate("=IF(1<0,10,20)", &cells).unwrap(), 20.0);
    }
}
