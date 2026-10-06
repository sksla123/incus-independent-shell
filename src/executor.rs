use crate::parser::{parse_structure, split_words, Expr, ParseError};
use crate::policy::{classify, AllowedCommands, CommandSpec};
use crate::ui;

use std::env;
use std::io;
use std::process::{Command, ExitStatus, Stdio};

#[derive(Debug, PartialEq, Eq)]
pub enum Flow {
    Continue(i32),
    Exit(i32),
}

pub fn run_input(input: &str, history: &[String], allowed: &AllowedCommands) -> Flow {
    let expressions = match parse_structure(input) {
        Ok(exprs) => exprs,
        Err(ParseError::Incomplete) => {
            eprintln!("Invalid quoting or escape sequence.");
            return Flow::Continue(2);
        }
        Err(ParseError::Denied) => {
            ui::deny();
            return Flow::Continue(126);
        }
    };

    let mut last = 0;

    for expr in expressions {
        match run_expr(expr, history, allowed) {
            Flow::Continue(rc) => last = rc,
            Flow::Exit(rc) => return Flow::Exit(rc),
        }
    }

    Flow::Continue(last)
}

fn run_expr(expr: Expr, history: &[String], allowed: &AllowedCommands) -> Flow {
    match expr {
        Expr::Simple(command) => run_simple(&command, history, allowed),
        Expr::Pipeline { left, right } => Flow::Continue(run_pipeline(&left, &right, allowed)),
        Expr::AndChain(chain) => {
            let mut last = 0;

            for expr in chain {
                match run_expr(expr, history, allowed) {
                    Flow::Continue(rc) => {
                        last = rc;
                        if rc != 0 {
                            break;
                        }
                    }
                    Flow::Exit(rc) => return Flow::Exit(rc),
                }
            }

            Flow::Continue(last)
        }
    }
}

fn run_simple(input: &str, history: &[String], allowed: &AllowedCommands) -> Flow {
    let argv = match split_words(input) {
        Ok(argv) => argv,
        Err(_) => {
            eprintln!("Invalid quoting or escape sequence.");
            return Flow::Continue(2);
        }
    };

    if argv.is_empty() {
        return Flow::Continue(0);
    }

    match argv[0].as_str() {
        "exit" | "logout" => Flow::Exit(0),

        "help" => {
            ui::print_help(allowed);
            Flow::Continue(0)
        }

        "shell-version" => {
            if argv.len() != 1 {
                eprintln!("shell-version: no arguments expected");
                return Flow::Continue(2);
            }

            println!("incus-only-shell {}", ui::VERSION);
            Flow::Continue(0)
        }

        "history" => run_history(&argv[1..], history),
        "cd" => run_cd(&argv[1..]),

        _ => match classify(&argv, allowed) {
            Ok(spec) => Flow::Continue(run_external(spec)),
            Err(_) => {
                ui::deny();
                Flow::Continue(126)
            }
        },
    }
}

fn run_history(args: &[String], history: &[String]) -> Flow {
    let count = match args {
        [] => None,
        [value] => match value.parse::<usize>() {
            Ok(value) => Some(value),
            Err(_) => {
                eprintln!("history: use \"history\" or \"history N\"");
                return Flow::Continue(126);
            }
        },
        _ => {
            eprintln!("history: use \"history\" or \"history N\"");
            return Flow::Continue(126);
        }
    };

    let start = count.map(|n| history.len().saturating_sub(n)).unwrap_or(0);

    for (index, entry) in history.iter().enumerate().skip(start) {
        let rendered = entry.replace('\n', "\n        ");
        println!("{:5}  {}", index + 1, rendered);
    }

    Flow::Continue(0)
}

fn run_cd(args: &[String]) -> Flow {
    if args.len() > 1 {
        eprintln!("cd: too many arguments");
        return Flow::Continue(1);
    }

    let target = args
        .first()
        .cloned()
        .unwrap_or_else(|| env::var("HOME").unwrap_or_else(|_| "/".into()));

    match env::set_current_dir(&target) {
        Ok(()) => Flow::Continue(0),
        Err(err) => {
            eprintln!("cd: {target}: {err}");
            Flow::Continue(1)
        }
    }
}

fn run_external(spec: CommandSpec) -> i32 {
    match make_command(&spec).status() {
        Ok(status) => status_code(status),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            eprintln!("{}: command not installed", spec.program);
            127
        }
        Err(err) => {
            eprintln!("{}: {err}", spec.program);
            126
        }
    }
}

fn run_pipeline(left: &str, right: &str, allowed: &AllowedCommands) -> i32 {
    let left_argv = match split_words(left) {
        Ok(argv) => argv,
        Err(_) => return 2,
    };

    let right_argv = match split_words(right) {
        Ok(argv) => argv,
        Err(_) => return 2,
    };

    let right_spec = match classify(&right_argv, allowed) {
        Ok(spec) => spec,
        Err(_) => {
            ui::deny();
            return 126;
        }
    };

    let left_spec = match classify(&left_argv, allowed) {
        Ok(spec) => spec,
        Err(_) => {
            ui::deny();
            return 126;
        }
    };

    let mut left_cmd = make_command(&left_spec);
    left_cmd.stdout(Stdio::piped());

    let mut left_child = match left_cmd.spawn() {
        Ok(child) => child,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            eprintln!("{}: command not installed", left_spec.program);
            return 127;
        }
        Err(err) => {
            eprintln!("{}: {err}", left_spec.program);
            return 126;
        }
    };

    let Some(stdout) = left_child.stdout.take() else {
        return 126;
    };

    let right_status = make_command(&right_spec)
        .stdin(Stdio::from(stdout))
        .status();

    let _ = left_child.wait();

    match right_status {
        Ok(status) => status_code(status),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            eprintln!("{}: command not installed", right_spec.program);
            127
        }
        Err(err) => {
            eprintln!("{}: {err}", right_spec.program);
            126
        }
    }
}

fn make_command(spec: &CommandSpec) -> Command {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    command
}

fn status_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_available_commands_work_with_an_empty_external_list() {
        let allowed = AllowedCommands::parse("").unwrap();
        for input in [
            "cd .",
            "history",
            "help",
            "shell-version",
            "bash -c 'exit 0'",
        ] {
            assert_eq!(run_input(input, &[], &allowed), Flow::Continue(0));
        }
        for input in ["exit", "logout"] {
            assert_eq!(run_input(input, &[], &allowed), Flow::Exit(0));
        }
        assert_eq!(run_input("pwd", &[], &allowed), Flow::Continue(126));
    }

    #[test]
    fn pipeline_requires_both_external_commands_in_the_list() {
        for commands in ["pwd\n", "grep\n", ""] {
            let allowed = AllowedCommands::parse(commands).unwrap();
            assert_eq!(run_pipeline("pwd", "grep .", &allowed), 126);
        }
    }

    #[test]
    fn bash_is_allowed_in_a_pipeline_without_listing_it() {
        let allowed = AllowedCommands::parse("grep\n").unwrap();
        assert_eq!(
            run_pipeline("bash -c 'printf hello'", "grep hello", &allowed),
            0
        );
    }

    #[test]
    fn configured_pipeline_target_can_be_any_external_command() {
        let allowed = AllowedCommands::parse("cat\n").unwrap();
        assert_eq!(run_pipeline("bash -c 'printf hello'", "cat", &allowed), 0);
    }

    #[test]
    fn bash_pipeline_works_with_an_empty_external_list() {
        let allowed = AllowedCommands::parse("").unwrap();
        assert_eq!(
            run_pipeline("bash -c 'printf hello'", "bash -c 'cat'", &allowed),
            0
        );
    }
}
