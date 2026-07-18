//! Splitting a configured command line into an argument vector.
//!
//! `$EDITOR`, `$VISUAL` and `$PAGER` are command *lines*, not program names, so
//! they have to be split before they can be handed to [`std::process::Command`].
//! Splitting on whitespace loses any path containing a space, which is common
//! on macOS (`/Applications/Sublime Text/subl`), so [`split_command`] honors
//! single and double quotes the way a POSIX shell would.
//!
//! This is a lexer only. The result is always passed to `Command::new` as an
//! argument list and never to a shell, so quoting cannot become injection.

/// Splits a command line into its arguments, honoring quotes.
///
/// Single and double quotes group their contents into one argument; a
/// backslash inside double quotes escapes the next character. An unterminated
/// quote yields `None`, which callers treat as "unusable command".
///
/// `code --wait` splits into two arguments; `"/Applications/Sublime Text/subl"`
/// stays a single one; `vi "unterminated` yields `None`.
#[must_use]
pub fn split_command(command: &str) -> Option<Vec<String>> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut chars = command.chars();

    while let Some(ch) = chars.next() {
        match quote {
            Some('"') if ch == '\\' => match chars.next() {
                Some(escaped) => current.push(escaped),
                None => return None,
            },
            Some(open) if ch == open => quote = None,
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                started = true;
            }
            None if ch.is_whitespace() => {
                if started {
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => {
                current.push(ch);
                started = true;
            }
        }
    }

    if quote.is_some() {
        return None;
    }
    if started {
        parts.push(current);
    }
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::split_command;

    #[test]
    fn a_bare_program_becomes_one_argument() {
        assert_eq!(split_command("vi"), Some(vec!["vi".to_string()]));
    }

    #[test]
    fn arguments_are_separated_on_whitespace() {
        assert_eq!(
            split_command("code  --wait\t-n"),
            Some(vec![
                "code".to_string(),
                "--wait".to_string(),
                "-n".to_string(),
            ])
        );
    }

    #[test]
    fn a_double_quoted_path_with_spaces_stays_one_argument() {
        assert_eq!(
            split_command("\"/Applications/Sublime Text/subl\" -w"),
            Some(vec![
                "/Applications/Sublime Text/subl".to_string(),
                "-w".to_string(),
            ])
        );
    }

    #[test]
    fn a_single_quoted_path_with_spaces_stays_one_argument() {
        assert_eq!(
            split_command("'/my editor/bin' -x"),
            Some(vec!["/my editor/bin".to_string(), "-x".to_string()])
        );
    }

    #[test]
    fn a_backslash_escapes_inside_double_quotes() {
        assert_eq!(split_command("\"a\\\"b\""), Some(vec!["a\"b".to_string()]));
    }

    #[test]
    fn an_empty_command_yields_no_arguments() {
        assert_eq!(split_command(""), Some(Vec::new()));
    }

    #[test]
    fn whitespace_only_yields_no_arguments() {
        assert_eq!(split_command("   \t "), Some(Vec::new()));
    }

    #[test]
    fn an_empty_quoted_argument_is_preserved() {
        assert_eq!(
            split_command("vi \"\""),
            Some(vec!["vi".to_string(), String::new(),])
        );
    }

    #[test]
    fn an_unterminated_double_quote_is_rejected() {
        assert_eq!(split_command("vi \"unterminated"), None);
    }

    #[test]
    fn an_unterminated_single_quote_is_rejected() {
        assert_eq!(split_command("vi 'unterminated"), None);
    }

    #[test]
    fn a_trailing_backslash_in_a_quote_is_rejected() {
        assert_eq!(split_command("\"a\\"), None);
    }
}
