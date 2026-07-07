//! Recursive-descent evaluator for calculator-mode arithmetic.
//!
//! Backs [`NumberInput::calculator`](super::NumberInput::calculator) and is an
//! implementation detail, not part of the public API.

/// An error from parsing or evaluating a numeric expression.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CalcError {
    /// The field did not contain a number.
    #[error("not a number")]
    NotANumber,
    /// A numeric literal failed to parse.
    #[error("invalid number")]
    InvalidNumber,
    /// A division by zero was attempted.
    #[error("division by zero")]
    DivisionByZero,
    /// A `(` had no matching `)`.
    #[error("missing closing parenthesis")]
    MissingParenthesis,
    /// Input remained after a complete expression.
    #[error("unexpected trailing input")]
    TrailingInput,
}

/// Parses a number, accepting `,` or `.` as the decimal separator.
pub(crate) fn parse_number(text: &str) -> Option<f64> {
    let normalized = text.trim().replace(',', ".");
    if normalized.is_empty() {
        return None;
    }
    normalized.parse().ok()
}

/// Evaluates an arithmetic expression with `+ - * / ( )`.
///
/// Accepts `,` or `.` as the decimal separator.
///
/// # Errors
///
/// Returns a [`CalcError`] on malformed input or division by zero.
pub(crate) fn eval(expr: &str) -> Result<f64, CalcError> {
    let normalized = expr.replace(',', ".");
    let mut parser = Calc {
        chars: normalized.chars().collect(),
        pos: 0,
    };
    let value = parser.expression()?;
    parser.skip_spaces();
    if parser.pos != parser.chars.len() {
        return Err(CalcError::TrailingInput);
    }
    Ok(value)
}

/// A minimal recursive-descent arithmetic parser.
struct Calc {
    chars: Vec<char>,
    pos: usize,
}

impl Calc {
    /// Parses `term (('+'|'-') term)*`.
    fn expression(&mut self) -> Result<f64, CalcError> {
        let mut value = self.term()?;
        loop {
            self.skip_spaces();
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    value += self.term()?;
                }
                Some('-') => {
                    self.pos += 1;
                    value -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    /// Parses `factor (('*'|'/') factor)*`.
    fn term(&mut self) -> Result<f64, CalcError> {
        let mut value = self.factor()?;
        loop {
            self.skip_spaces();
            match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    value *= self.factor()?;
                }
                Some('/') => {
                    self.pos += 1;
                    let divisor = self.factor()?;
                    if divisor == 0.0 {
                        return Err(CalcError::DivisionByZero);
                    }
                    value /= divisor;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    /// Parses a number, parenthesized expression or unary minus.
    fn factor(&mut self) -> Result<f64, CalcError> {
        self.skip_spaces();
        match self.peek() {
            Some('(') => {
                self.pos += 1;
                let value = self.expression()?;
                self.skip_spaces();
                if self.peek() != Some(')') {
                    return Err(CalcError::MissingParenthesis);
                }
                self.pos += 1;
                Ok(value)
            }
            Some('-') => {
                self.pos += 1;
                Ok(-self.factor()?)
            }
            _ => self.number(),
        }
    }

    /// Parses a decimal number literal.
    fn number(&mut self) -> Result<f64, CalcError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse().map_err(|_| CalcError::InvalidNumber)
    }

    /// Returns the current character without consuming it.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Skips spaces.
    fn skip_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_respects_precedence() {
        assert_eq!(eval("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(eval("(2 + 3) * 4").unwrap(), 20.0);
    }

    #[test]
    fn eval_reports_division_by_zero() {
        assert_eq!(eval("1 / 0"), Err(CalcError::DivisionByZero));
    }

    #[test]
    fn eval_accepts_comma_decimal() {
        assert_eq!(eval("1,5 + 1,5").unwrap(), 3.0);
    }

    #[test]
    fn eval_rejects_trailing_input() {
        assert_eq!(eval("1 2"), Err(CalcError::TrailingInput));
    }

    #[test]
    fn eval_rejects_unbalanced_parenthesis() {
        assert_eq!(eval("(1 + 2"), Err(CalcError::MissingParenthesis));
    }

    #[test]
    fn parse_number_accepts_comma_and_trims() {
        assert_eq!(parse_number("  3,5 "), Some(3.5));
        assert_eq!(parse_number(""), None);
    }
}
