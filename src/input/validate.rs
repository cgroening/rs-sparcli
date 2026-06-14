//! Validation callbacks and character filters for text input prompts.

/// Validates a full input value, returning an error message on failure.
pub type Validator = Box<dyn Fn(&str) -> Result<(), String>>;

/// Decides whether a single typed character is accepted.
pub type CharFilter = Box<dyn Fn(char) -> bool>;

/// Rejects empty (whitespace-only) input.
pub fn non_empty() -> Validator {
    Box::new(|value: &str| {
        if value.trim().is_empty() {
            Err("must not be empty".to_string())
        } else {
            Ok(())
        }
    })
}

/// Requires at least `min` characters.
pub fn min_len(min: usize) -> Validator {
    Box::new(move |value: &str| {
        if value.chars().count() < min {
            Err(format!("must be at least {min} characters"))
        } else {
            Ok(())
        }
    })
}

/// Accepts only ASCII digits.
pub fn digits() -> CharFilter {
    Box::new(|ch: char| ch.is_ascii_digit())
}

/// Accepts digits plus a single sign and decimal point.
pub fn decimal() -> CharFilter {
    Box::new(|ch: char| ch.is_ascii_digit() || ch == '.' || ch == '-')
}

/// Accepts only alphabetic characters.
pub fn alpha() -> CharFilter {
    Box::new(|ch: char| ch.is_alphabetic())
}

/// Accepts only alphanumeric characters.
pub fn alnum() -> CharFilter {
    Box::new(|ch: char| ch.is_alphanumeric())
}

/// Rejects whitespace.
pub fn no_space() -> CharFilter {
    Box::new(|ch: char| !ch.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_rejects_blank() {
        assert!(non_empty()("   ").is_err());
        assert!(non_empty()("x").is_ok());
    }

    #[test]
    fn min_len_counts_characters() {
        assert!(min_len(3)("ab").is_err());
        assert!(min_len(3)("abc").is_ok());
    }

    #[test]
    fn digit_filter_accepts_only_digits() {
        assert!(digits()('5'));
        assert!(!digits()('a'));
    }
}
