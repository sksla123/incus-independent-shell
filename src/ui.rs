use crate::policy::{AllowedCommands, ALWAYS_ALLOWED_EXTERNAL_COMMANDS, BUILTIN_COMMANDS};

use std::env;
use std::fs;

pub const DENY_MSG: &str =
    "This command is not permitted on the host.\nRun workloads inside an Incus container.";

pub const BANNER_BEFORE_PATH: &str = "/etc/incus-only-shell/banner-before.txt";
pub const BANNER_AFTER_PATH: &str = "/etc/incus-only-shell/banner-after.txt";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn deny() {
    eprintln!("{DENY_MSG}");
}

pub fn print_help(allowed: &AllowedCommands) {
    println!("{}", help_text(allowed));
}

fn help_text(allowed: &AllowedCommands) -> String {
    let names = allowed.names().collect::<Vec<_>>();
    let external = if names.is_empty() {
        "(none)".to_string()
    } else {
        names.join(" ")
    };
    let builtins = BUILTIN_COMMANDS.join(" ");
    let host_shell = ALWAYS_ALLOWED_EXTERNAL_COMMANDS.join(" ");

    format!(
        r#"
===== Host commands allowed by this shell =====

External commands (configured):
  {external}

Built-ins (always available):
  {builtins}

Host shell (always available):
  {host_shell}

Working directly on the host is not recommended.
If you want to work on the host, you can use bash."#
    )
}

pub fn print_banner(banner: &str) {
    print_optional_file(BANNER_BEFORE_PATH);
    print_text(banner);
    print_optional_file(BANNER_AFTER_PATH);
}

fn print_optional_file(path: &str) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    print_text(&contents);
}

fn print_text(contents: &str) {
    if contents.is_empty() {
        return;
    }

    print!("{contents}");

    if !contents.ends_with('\n') {
        println!();
    }
}

pub fn hostname() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_else(|_| "host".into())
        .trim()
        .to_string()
}

pub fn prompt() -> String {
    let user = env::var("USER").unwrap_or_else(|_| "user".into());
    let cwd = env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "?".into());

    format!("[incus-only-shell] {user}@{}:{cwd}$ ", hostname())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_reflects_custom_and_empty_lists_and_always_shows_bash() {
        for (contents, expected) in [("cp\n", "cp"), ("", "(none)")] {
            let allowed = AllowedCommands::parse(contents).unwrap();
            let text = help_text(&allowed);
            assert!(text.contains(&format!("External commands (configured):\n  {expected}\n")));
            assert!(text.contains("Host shell (always available):\n  bash\n"));
            assert!(!text.contains("Parsing rules:"));
            assert!(!text.contains("Examples"));
            assert!(!text.contains("Configuration:"));
            assert!(text.ends_with("If you want to work on the host, you can use bash."));
            assert!(text.contains(
                "Built-ins (always available):\n  cd history help shell-version exit logout\n"
            ));
        }
    }
}
