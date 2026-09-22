use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_stdin(input: &str) -> Output {
    let bin = env!("CARGO_BIN_EXE_incus-only-shell");

    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start incus-only-shell");

    child
        .stdin
        .as_mut()
        .expect("stdin unavailable")
        .write_all(input.as_bytes())
        .expect("failed to write test input");

    child.wait_with_output().expect("failed to collect output")
}

#[test]
fn multiple_commands_execute_in_order_without_intermediate_prompt() {
    let output = run_stdin("pwd\npwd\npwd\n");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], lines[1]);
    assert_eq!(lines[1], lines[2]);
    assert!(!stdout.contains("[incus-only-shell]"));
}

#[test]
fn backslash_newline_is_one_command() {
    let output = run_stdin("ls \\\n-d \\\n.\n");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), ".\n");
}

#[test]
fn assignment_is_denied() {
    let output = run_stdin("CONTAINER_NAME=c1\n");

    assert_eq!(output.status.code(), Some(126));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("This command is not permitted on the host."));
}

#[test]
fn semicolon_wrapping_is_denied() {
    let output = run_stdin("pwd; pwd\n");

    assert_eq!(output.status.code(), Some(126));
    assert!(output.stdout.is_empty());
}

#[test]
fn single_grep_pipeline_is_allowed() {
    let output = run_stdin("ls / | grep tmp\n");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.lines().any(|line| line == "tmp"));
}

#[test]
fn arbitrary_pipeline_is_denied() {
    let output = run_stdin("ls / | cat\n");

    assert_eq!(output.status.code(), Some(126));
}

#[test]
fn double_ampersand_runs_next_command_on_success() {
    let output = run_stdin("pwd && pwd\n");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], lines[1]);
}

#[test]
fn double_ampersand_short_circuits_on_failure() {
    let output = run_stdin("grep definitely-not-present /dev/null && pwd\n");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn double_ampersand_can_continue_on_next_line() {
    let output = run_stdin("pwd &&\npwd\n");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], lines[1]);
}

#[test]
fn single_ampersand_is_denied() {
    let output = run_stdin("pwd & pwd\n");

    assert_eq!(output.status.code(), Some(126));
    assert!(output.stdout.is_empty());
}

#[test]
fn shell_version_is_available() {
    let output = run_stdin("shell-version\n");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("incus-only-shell {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn shell_version_rejects_arguments() {
    let output = run_stdin("shell-version extra\n");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("shell-version: no arguments expected"));
}
