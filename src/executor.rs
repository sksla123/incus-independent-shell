use crate::parser::{parse_structure, split_words, Expr, ParseError};
use crate::policy::{classify, CommandSpec};
use crate::ui;

use std::env;
use std::io;
use std::process::{Command, ExitStatus, Stdio};

pub enum Flow {
    Continue(i32),
    Exit(i32),
}

pub fn run_input(input: &str, history: &[String]) -> Flow {
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
        match run_expr(expr, history) {
            Flow::Continue(rc) => last = rc,
            Flow::Exit(rc) => return Flow::Exit(rc),
        }
    }

    Flow::Continue(last)
}

fn run_expr(expr: Expr, history: &[String]) -> Flow {
    match expr {
        Expr::Simple(command) => run_simple(&command, history),
        Expr::Pipeline { left, right } => Flow::Continue(run_pipeline(&left, &right)),
        Expr::AndChain(chain) => {
            let mut last = 0;

            for expr in chain {
                match run_expr(expr, history) {
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

fn run_simple(input: &str, history: &[String]) -> Flow {
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
            let _ = Command::new("incus").arg("--help").status();
            println!("{}", ui::HELP);
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

        _ => match classify(&argv, true) {
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
    if spec.warn_unrestricted_bash {
        eprintln!(
            "\nWARNING: Entering an unrestricted Bash shell on the host.\n\
Commands in this Bash session are NOT filtered by incus-only-shell.\n\
Type \"exit\" to return to incus-only-shell.\n"
        );
    }

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

fn run_pipeline(left: &str, right: &str) -> i32 {
    let left_argv = match split_words(left) {
        Ok(argv) => argv,
        Err(_) => return 2,
    };

    let right_argv = match split_words(right) {
        Ok(argv) => argv,
        Err(_) => return 2,
    };

    if right_argv.first().map(String::as_str) != Some("grep") {
        ui::deny();
        return 126;
    }

    // Built-ins and unrestricted Bash do not participate in filtered pipelines.
    let left_spec = match classify(&left_argv, false) {
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

    let grep_status = Command::new("grep")
        .args(&right_argv[1..])
        .stdin(Stdio::from(stdout))
        .status();

    let _ = left_child.wait();

    match grep_status {
        Ok(status) => status_code(status),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            eprintln!("grep: command not installed");
            127
        }
        Err(err) => {
            eprintln!("grep: {err}");
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
