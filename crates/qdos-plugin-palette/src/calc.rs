//! Calculator expression parser for command palette
//!
//! Supports: +, -, *, /, ^ (power), parentheses, decimals

use std::iter::Peekable;
use std::str::Chars;

/// Evaluate a mathematical expression
/// Returns None if the input is not a valid expression
pub fn evaluate(input: &str) -> Option<f64> {
    let input = input.trim();

    // Quick check: must start with digit, '(', or '-' (for negative numbers)
    let first = input.chars().next()?;
    if !first.is_ascii_digit() && first != '(' && first != '-' && first != '.' {
        return None;
    }

    // Don't parse things that look like file paths or other non-math
    if input.contains('/') && input.chars().filter(|c| *c == '/').count() > 1 {
        return None; // Probably a path like /usr/bin
    }

    let mut parser = Parser::new(input);
    let result = parser.parse_expression()?;

    // Make sure we consumed all input
    parser.skip_whitespace();
    if parser.chars.peek().is_some() {
        return None;
    }

    // Check for NaN or infinity
    if result.is_nan() || result.is_infinite() {
        return None;
    }

    Some(result)
}

/// Check if input looks like a calculator expression (without fully parsing)
pub fn looks_like_expression(input: &str) -> bool {
    let input = input.trim();
    if input.is_empty() {
        return false;
    }

    let first = input.chars().next().unwrap();
    if !first.is_ascii_digit() && first != '(' && first != '-' && first != '.' {
        return false;
    }

    // Must contain at least one operator or be a simple number
    let has_operator = input
        .chars()
        .any(|c| matches!(c, '+' | '-' | '*' | '/' | '^' | '(' | ')'));
    let is_number = input
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == '-');

    has_operator || is_number
}

struct Parser<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .chars
            .peek()
            .map(|c| c.is_whitespace())
            .unwrap_or(false)
        {
            self.chars.next();
        }
    }

    fn parse_expression(&mut self) -> Option<f64> {
        self.parse_additive()
    }

    // Addition and subtraction (lowest precedence)
    fn parse_additive(&mut self) -> Option<f64> {
        let mut left = self.parse_multiplicative()?;

        loop {
            self.skip_whitespace();
            match self.chars.peek() {
                Some('+') => {
                    self.chars.next();
                    let right = self.parse_multiplicative()?;
                    left += right;
                }
                Some('-') => {
                    self.chars.next();
                    let right = self.parse_multiplicative()?;
                    left -= right;
                }
                _ => break,
            }
        }

        Some(left)
    }

    // Multiplication and division
    fn parse_multiplicative(&mut self) -> Option<f64> {
        let mut left = self.parse_power()?;

        loop {
            self.skip_whitespace();
            match self.chars.peek() {
                Some('*') => {
                    self.chars.next();
                    let right = self.parse_power()?;
                    left *= right;
                }
                Some('/') => {
                    self.chars.next();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return None; // Division by zero
                    }
                    left /= right;
                }
                _ => break,
            }
        }

        Some(left)
    }

    // Exponentiation (right associative)
    fn parse_power(&mut self) -> Option<f64> {
        let base = self.parse_unary()?;

        self.skip_whitespace();
        if self.chars.peek() == Some(&'^') {
            self.chars.next();
            let exp = self.parse_power()?; // Right associative
            Some(base.powf(exp))
        } else {
            Some(base)
        }
    }

    // Unary minus
    fn parse_unary(&mut self) -> Option<f64> {
        self.skip_whitespace();
        if self.chars.peek() == Some(&'-') {
            self.chars.next();
            let value = self.parse_unary()?;
            Some(-value)
        } else {
            self.parse_primary()
        }
    }

    // Numbers and parentheses
    fn parse_primary(&mut self) -> Option<f64> {
        self.skip_whitespace();

        match self.chars.peek()? {
            '(' => {
                self.chars.next(); // consume '('
                let value = self.parse_expression()?;
                self.skip_whitespace();
                if self.chars.next() != Some(')') {
                    return None; // Missing closing paren
                }
                Some(value)
            }
            c if c.is_ascii_digit() || *c == '.' => self.parse_number(),
            _ => None,
        }
    }

    fn parse_number(&mut self) -> Option<f64> {
        let mut num_str = String::new();

        // Integer part
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.chars.next();
            } else {
                break;
            }
        }

        // Decimal part
        if self.chars.peek() == Some(&'.') {
            num_str.push('.');
            self.chars.next();

            while let Some(&c) = self.chars.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    self.chars.next();
                } else {
                    break;
                }
            }
        }

        if num_str.is_empty() || num_str == "." {
            return None;
        }

        num_str.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-10
    }

    #[test]
    fn test_simple_numbers() {
        assert!(approx_eq(evaluate("42").unwrap(), 42.0));
        assert!(approx_eq(evaluate("3.14").unwrap(), 3.14));
        assert!(approx_eq(evaluate("-5").unwrap(), -5.0));
    }

    #[test]
    fn test_addition() {
        assert!(approx_eq(evaluate("2+3").unwrap(), 5.0));
        assert!(approx_eq(evaluate("2 + 3").unwrap(), 5.0));
    }

    #[test]
    fn test_subtraction() {
        assert!(approx_eq(evaluate("5-3").unwrap(), 2.0));
        assert!(approx_eq(evaluate("3-5").unwrap(), -2.0));
    }

    #[test]
    fn test_multiplication() {
        assert!(approx_eq(evaluate("4*3").unwrap(), 12.0));
    }

    #[test]
    fn test_division() {
        assert!(approx_eq(evaluate("12/4").unwrap(), 3.0));
        assert!(evaluate("5/0").is_none()); // Division by zero
    }

    #[test]
    fn test_power() {
        assert!(approx_eq(evaluate("2^3").unwrap(), 8.0));
        assert!(approx_eq(evaluate("2^3^2").unwrap(), 512.0)); // Right associative: 2^(3^2) = 2^9
    }

    #[test]
    fn test_parentheses() {
        assert!(approx_eq(evaluate("(2+3)*4").unwrap(), 20.0));
        assert!(approx_eq(evaluate("2*(3+4)").unwrap(), 14.0));
    }

    #[test]
    fn test_precedence() {
        assert!(approx_eq(evaluate("2+3*4").unwrap(), 14.0)); // Not 20
        assert!(approx_eq(evaluate("2*3+4").unwrap(), 10.0));
    }

    #[test]
    fn test_complex() {
        assert!(approx_eq(evaluate("(2+3)*(4-1)").unwrap(), 15.0));
        assert!(approx_eq(evaluate("-2 + 3 * 4").unwrap(), 10.0));
    }

    #[test]
    fn test_not_expression() {
        assert!(evaluate("hello").is_none());
        assert!(evaluate("").is_none());
        assert!(evaluate("/usr/bin/test").is_none());
    }
}
