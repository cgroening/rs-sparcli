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

use crate::error::{Result, SparcliError};

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

/// Resolves a configured command line from an override, the environment or a
/// built-in default.
///
/// `$EDITOR`, `$VISUAL` and `$PAGER` all follow the same precedence: an
/// explicit override wins, then the first of `keys` that names a non-blank
/// variable, then `default`. Blank and whitespace-only values are treated as
/// unset throughout, so `EDITOR=""` falls through instead of yielding an
/// unusable empty command.
#[must_use]
pub fn resolve_from_env(
    override_command: Option<&str>,
    keys: &[&str],
    default: &str,
) -> String {
    if let Some(command) = override_command
        && !command.trim().is_empty()
    {
        return command.to_string();
    }
    for key in keys {
        if let Ok(value) = std::env::var(key)
            && !value.trim().is_empty()
        {
            return value;
        }
    }
    default.to_string()
}

/// Splits a command line into its program and remaining arguments.
///
/// `label` names the command in the error message (e.g. `"editor"`), so a
/// caller does not have to phrase the same two errors itself.
///
/// # Errors
///
/// Returns [`SparcliError::Config`] if `command` has an unbalanced quote or
/// contains no program at all.
pub(crate) fn program_and_args(
    command: &str,
    label: &str,
) -> Result<(String, Vec<String>)> {
    let argv = split_command(command).ok_or_else(|| {
        SparcliError::Config(format!("unparsable {label} command"))
    })?;
    let (program, args) = argv.split_first().ok_or_else(|| {
        SparcliError::Config(format!("empty {label} command"))
    })?;
    Ok((program.clone(), args.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn an_override_wins_over_the_environment_and_the_default() {
        assert_eq!(resolve_from_env(Some("nano"), &["PATH"], "vi"), "nano");
    }

    #[test]
    fn a_blank_override_falls_through_to_the_default() {
        // An empty `$EDITOR` must not become an unusable empty command.
        assert_eq!(resolve_from_env(Some("   "), &[], "vi"), "vi");
        assert_eq!(resolve_from_env(None, &[], "vi"), "vi");
    }

    #[test]
    fn the_environment_is_consulted_in_key_order() {
        // SAFETY: `SPARCLI_TEST_COMMAND_A`/`_B` exist only for this test; no
        // other test in the crate reads or writes them, so the mutation cannot
        // race a parallel test.
        unsafe {
            std::env::set_var("SPARCLI_TEST_COMMAND_A", "  ");
            std::env::set_var("SPARCLI_TEST_COMMAND_B", "second");
        }
        let keys = ["SPARCLI_TEST_COMMAND_A", "SPARCLI_TEST_COMMAND_B"];
        // The first key is blank, so it is skipped rather than winning.
        assert_eq!(resolve_from_env(None, &keys, "vi"), "second");
        // SAFETY: same exclusive-use argument as above.
        unsafe {
            std::env::remove_var("SPARCLI_TEST_COMMAND_A");
            std::env::remove_var("SPARCLI_TEST_COMMAND_B");
        }
        assert_eq!(resolve_from_env(None, &keys, "vi"), "vi");
    }

    #[test]
    fn program_and_args_splits_off_the_program() {
        let (program, args) = program_and_args("code --wait", "editor")
            .expect("a well-formed command line splits");
        assert_eq!(program, "code");
        assert_eq!(args, vec!["--wait".to_string()]);
    }

    #[test]
    fn program_and_args_names_the_command_in_its_errors() {
        let unbalanced = program_and_args("vi \"oops", "editor");
        assert!(matches!(
            unbalanced,
            Err(SparcliError::Config(message))
                if message == "unparsable editor command"
        ));
        let empty = program_and_args("   ", "pager");
        assert!(matches!(
            empty,
            Err(SparcliError::Config(message))
                if message == "empty pager command"
        ));
    }
}
