use std::env;
use std::fs;

pub const DENY_MSG: &str =
    "This command is not permitted on the host.\nRun workloads inside an Incus container.";

pub const BANNER_BEFORE_PATH: &str = "/etc/incus-only-shell/banner-before.txt";
pub const BANNER_AFTER_PATH: &str = "/etc/incus-only-shell/banner-after.txt";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const BANNER: &str = r#"This host shell is restricted.

Use this shell primarily for Incus commands.
Do not run workloads or perform system administration on the host.

Run applications and arbitrary commands inside your Incus containers.

The "bash" command is available when you explicitly need a normal host shell.
Commands executed inside bash are not filtered by this wrapper.

Type "help" to see available commands."#;

pub const HELP: &str = r#"
===== Host commands allowed by this shell =====

Navigation:
  ls cd pwd clear history grep

Identity / session:
  whoami id groups who w uptime hostname uname

CPU / memory / process monitoring:
  free vmstat mpstat lscpu ps top htop

Network monitoring:
  ss ping traceroute tracepath
  ip addr|address|link|route|neigh
  resolvectl status|query|statistics

Disk / filesystem monitoring:
  df du lsblk findmnt mountpoint blkid

GPU monitoring:
  gpu-stat
  nvidia-smi
  lspci

GPU PCI address example:
  lspci -Dnnk | grep -EA3 'VGA|3D|Display'

Incus:
  incus ...

Shell:
  shell-version
      Show the incus-only-shell version.

Host shell:
  bash
      Start an unrestricted Bash shell on the host.

Parsing rules:
  A normal newline outside quotes separates commands.

  Backslash followed by newline continues the same command.

  A newline inside single or double quotes remains inside the argument.

  One top-level pipeline per command is allowed only when the final command is grep.

  && runs the next command only when the previous command succeeds.

Examples:
  incus list

  incus launch \
    images:debian/13/cloud \
    c1

  incus exec c1 -- sh -c '
  echo hello
  id
  ip route
  '

  incus exec c1 -- sshd -T | grep passwordauthentication

  incus stop c1 && incus start c1

  lspci -Dnnk | grep -EA3 'VGA|3D|Display'
"#;

pub fn deny() {
    eprintln!("{DENY_MSG}");
}

pub fn print_banner() {
    print_optional_file(BANNER_BEFORE_PATH);
    println!("{BANNER}");
    print_optional_file(BANNER_AFTER_PATH);
}

fn print_optional_file(path: &str) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

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
