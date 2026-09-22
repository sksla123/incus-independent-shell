use crate::parser::{parse_structure, ParseError};

use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::Helper;

#[derive(Default)]
pub struct ShellHelper;

impl Completer for ShellHelper {
    type Candidate = String;
}

impl Hinter for ShellHelper {
    type Hint = String;
}

impl Highlighter for ShellHelper {}
impl Helper for ShellHelper {}

impl Validator for ShellHelper {
    fn validate(&self, ctx: &mut ValidationContext<'_>) -> rustyline::Result<ValidationResult> {
        match parse_structure(ctx.input()) {
            Err(ParseError::Incomplete) => Ok(ValidationResult::Incomplete),
            _ => Ok(ValidationResult::Valid(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_incomplete(input: &str) -> bool {
        matches!(parse_structure(input), Err(ParseError::Incomplete))
    }

    #[test]
    fn trailing_backslash_requests_more_input() {
        assert!(is_incomplete("incus launch \\"));
    }

    #[test]
    fn open_single_quote_requests_more_input() {
        assert!(is_incomplete("incus exec c1 -- sh -c 'echo hello"));
    }

    #[test]
    fn open_double_quote_requests_more_input() {
        assert!(is_incomplete("incus exec c1 -- sh -c \"echo hello"));
    }

    #[test]
    fn trailing_double_ampersand_requests_more_input() {
        assert!(is_incomplete("incus stop c1 &&"));
    }

    #[test]
    fn independent_complete_commands_do_not_request_more_input() {
        assert!(!is_incomplete("incus list\nincus info\nincus project list"));
    }
}
