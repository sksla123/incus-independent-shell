#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Simple(String),
    Pipeline { left: String, right: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    Incomplete,
    Denied,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum QuoteState {
    Plain,
    Single,
    Double,
}

pub fn split_words(input: &str) -> Result<Vec<String>, ParseError> {
    shlex::split(input).ok_or(ParseError::Denied)
}

pub fn parse_structure(input: &str) -> Result<Vec<Expr>, ParseError> {
    let mut state = QuoteState::Plain;
    let mut chars = input.chars().peekable();
    let mut out = Vec::new();
    let mut current = String::new();
    let mut pipe_left: Option<String> = None;

    while let Some(ch) = chars.next() {
        match state {
            QuoteState::Plain => match ch {
                '\'' => {
                    current.push(ch);
                    state = QuoteState::Single;
                }
                '"' => {
                    current.push(ch);
                    state = QuoteState::Double;
                }
                '\\' => match chars.next() {
                    Some('\n') => {
                        // Shell-style line continuation: remove both characters.
                    }
                    Some(next) => {
                        current.push('\\');
                        current.push(next);
                    }
                    None => return Err(ParseError::Incomplete),
                },
                '\n' => finish_expr(&mut out, &mut pipe_left, &mut current)?,
                '|' => {
                    if pipe_left.is_some() || current.trim().is_empty() {
                        return Err(ParseError::Denied);
                    }

                    pipe_left = Some(current.trim().to_string());
                    current.clear();
                }
                ';' | '&' | '<' | '>' | '`' => return Err(ParseError::Denied),
                '\r' => {}
                _ => current.push(ch),
            },

            QuoteState::Single => {
                current.push(ch);
                if ch == '\'' {
                    state = QuoteState::Plain;
                }
            }

            QuoteState::Double => match ch {
                '"' => {
                    current.push(ch);
                    state = QuoteState::Plain;
                }
                '\\' => match chars.next() {
                    Some('\n') => {
                        // Backslash-newline is also a continuation inside double quotes.
                    }
                    Some(next) => {
                        current.push('\\');
                        current.push(next);
                    }
                    None => return Err(ParseError::Incomplete),
                },
                _ => current.push(ch),
            },
        }
    }

    if state != QuoteState::Plain {
        return Err(ParseError::Incomplete);
    }

    finish_expr(&mut out, &mut pipe_left, &mut current)?;
    Ok(out)
}

fn finish_expr(
    out: &mut Vec<Expr>,
    pipe_left: &mut Option<String>,
    current: &mut String,
) -> Result<(), ParseError> {
    let right = current.trim();

    if let Some(left) = pipe_left.take() {
        if right.is_empty() {
            return Err(ParseError::Denied);
        }

        out.push(Expr::Pipeline {
            left,
            right: right.to_string(),
        });
    } else if !right.is_empty() {
        out.push(Expr::Simple(right.to_string()));
    }

    current.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newline_splits_commands() {
        assert_eq!(
            parse_structure("incus list\nincus info").unwrap(),
            vec![
                Expr::Simple("incus list".into()),
                Expr::Simple("incus info".into()),
            ]
        );
    }

    #[test]
    fn backslash_newline_continues_command() {
        assert_eq!(
            parse_structure("incus launch \\\n  images:debian/13/cloud \\\n  c1")
            .unwrap(),
            vec![Expr::Simple(
                "incus launch   images:debian/13/cloud   c1".into()
            )]
        );
    }

    #[test]
    fn quoted_newline_stays_inside_command() {
        let input = "incus exec c1 -- sh -c '\necho hello\nid\nip route\n'";
        assert_eq!(
            parse_structure(input).unwrap(),
            vec![Expr::Simple(input.into())]
        );

        let argv = split_words(input).unwrap();
        assert_eq!(argv[0], "incus");
        assert_eq!(argv[6], "\necho hello\nid\nip route\n");
    }

    #[test]
    fn three_pasted_commands_stay_separate_and_ordered() {
        assert_eq!(
            parse_structure("incus list\nincus info\nincus project list").unwrap(),
            vec![
                Expr::Simple("incus list".into()),
                Expr::Simple("incus info".into()),
                Expr::Simple("incus project list".into()),
            ]
        );
    }

    #[test]
    fn double_quoted_newline_stays_inside_command() {
        let input = "incus exec c1 -- sh -c \"echo hello\nid\"";
        assert_eq!(
            parse_structure(input).unwrap(),
            vec![Expr::Simple(input.into())]
        );

        let argv = split_words(input).unwrap();
        assert_eq!(argv[6], "echo hello\nid");
    }

    #[test]
    fn quoted_pipe_is_literal() {
        let input = "incus exec c1 -- sh -c 'echo a | grep a'";
        assert_eq!(
            parse_structure(input).unwrap(),
            vec![Expr::Simple(input.into())]
        );
    }

    #[test]
    fn one_top_level_pipeline_is_allowed() {
        assert_eq!(
            parse_structure("incus list | grep c1").unwrap(),
            vec![Expr::Pipeline {
                left: "incus list".into(),
                right: "grep c1".into(),
            }]
        );
    }

    #[test]
    fn two_top_level_pipes_are_denied() {
        assert_eq!(
            parse_structure("incus list | grep c1 | grep RUNNING"),
            Err(ParseError::Denied)
        );
    }

    #[test]
    fn shell_operators_are_denied() {
        for input in [
            "incus list; id",
            "incus list && id",
            "incus list > /tmp/x",
            "incus list < /tmp/x",
            "`id`",
        ] {
            assert_eq!(parse_structure(input), Err(ParseError::Denied));
        }
    }

    #[test]
    fn unfinished_quote_is_incomplete() {
        assert_eq!(
            parse_structure("incus exec c1 -- sh -c 'echo hello"),
            Err(ParseError::Incomplete)
        );
    }

    #[test]
    fn trailing_backslash_is_incomplete() {
        assert_eq!(
            parse_structure("incus launch \\"),
            Err(ParseError::Incomplete)
        );
    }
}
