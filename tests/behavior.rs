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
    let output = run_stdin("ls -d . | grep '^\\.$'\n");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, ".\n");
}

#[test]
fn unlisted_pipeline_command_is_denied() {
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

#[test]
fn allowed_bash_arguments_run_unchanged() {
    let output = run_stdin("bash -c 'printf hello'\n");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "hello");
}

#[test]
fn help_separates_external_commands_from_builtins() {
    let output = run_stdin("help\n");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("External commands (configured):"));
    let stdout = stdout
        .split_once("===== Host commands allowed by this shell =====")
        .unwrap()
        .1;
    assert!(stdout
        .contains("Built-ins (always available):\n  cd history help shell-version exit logout"));
    assert!(stdout.contains("Host shell (always available):\n  bash"));
    assert!(!stdout.contains("Parsing rules:"));
    assert!(!stdout.contains("Examples"));
    assert!(!stdout.contains("Usage:"));
    assert!(!stdout.contains("Configuration:"));
    assert!(stdout.contains("Working directly on the host is not recommended."));
    assert!(stdout
        .trim_end()
        .ends_with("If you want to work on the host, you can use bash."));
    assert!(output.stderr.is_empty());
}

#[test]
fn c_option_uses_the_same_policy() {
    let bin = env!("CARGO_BIN_EXE_incus-only-shell");
    let output = Command::new(bin)
        .args(["-c", "bash -c 'printf hello'"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "hello");
}

#[test]
fn pasted_multiline_commands_preserve_distinct_output_order() {
    let output = run_stdin(
        "bash -c 'printf \"first\\n\"'\nbash -c 'printf \"second\\n\"'\nbash -c 'printf \"third\\n\"'\n",
    );

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "first\nsecond\nthird\n"
    );
}

#[test]
fn quoted_multiline_command_runs_as_one_argument() {
    let output = run_stdin("bash -c '\nprintf \"first\\n\"\nprintf \"second\\n\"\n'\n");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "first\nsecond\n");
}

#[test]
fn quoted_newlines_are_preserved_in_single_and_double_quotes() {
    for quoted in ["'first\nsecond'", "\"first\nsecond\""] {
        let input = format!("bash -c 'printf \"%s\\n\" \"$1\"' -- {quoted}\n");
        let output = run_stdin(&input);

        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "first\nsecond\n");
    }
}

#[test]
fn listed_external_command_can_receive_pipeline_output() {
    let output = run_stdin(
        "bash -c 'printf \"hello\\n\"' | bash -c 'read -r value; printf \"%s\\n\" \"$value\"'\n",
    );

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "hello\n");
}

#[test]
fn multiline_pipeline_and_chain_execute_in_order() {
    let output = run_stdin(
        "bash -c 'printf \"match\\nskip\\n\"' \\\n| grep '^match$' &&\nbash -c 'printf \"next\\n\"'\n",
    );

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "match\nnext\n");
}

#[test]
fn failed_pipeline_short_circuits_multiline_and_chain() {
    let output = run_stdin(
        "bash -c 'printf \"skip\\n\"' | grep '^match$' &&\nbash -c 'printf should-not-run'\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn failed_command_short_circuits_the_rest_of_a_long_multiline_chain() {
    let output = run_stdin(
        "bash -c 'printf \"first\\n\"' &&\nbash -c 'printf \"second\\n\"; exit 7' &&\nbash -c 'printf should-not-run'\n",
    );

    assert_eq!(output.status.code(), Some(7));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "first\nsecond\n");
}

#[test]
fn independent_command_runs_after_a_failed_and_chain() {
    let output = run_stdin(
        "bash -c 'exit 7' && bash -c 'printf should-not-run'\nbash -c 'printf resumed'\n",
    );

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "resumed");
}
