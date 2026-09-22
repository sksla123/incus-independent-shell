#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub warn_unrestricted_bash: bool,
}

const HOST_COMMANDS: &[&str] = &[
    "ls",
    "pwd",
    "clear",
    "grep",
    "whoami",
    "id",
    "groups",
    "who",
    "w",
    "uptime",
    "uname",
    "free",
    "vmstat",
    "mpstat",
    "lscpu",
    "ps",
    "ss",
    "ping",
    "traceroute",
    "tracepath",
    "df",
    "du",
    "lsblk",
    "findmnt",
    "mountpoint",
    "blkid",
    "gpu-stat",
    "lspci",
];

pub fn classify(argv: &[String], allow_bash: bool) -> Result<CommandSpec, ()> {
    let cmd = argv.first().ok_or(())?;
    let args = &argv[1..];

    match cmd.as_str() {
        "incus" => Ok(spec("incus", args)),

        "bash" if allow_bash => Ok(CommandSpec {
            program: "/bin/bash".into(),
            args: args.to_vec(),
            warn_unrestricted_bash: true,
        }),

        "ip" if ip_allowed(args) => Ok(spec("ip", args)),
        "hostname" if hostname_allowed(args) => Ok(spec("hostname", args)),
        "resolvectl" if resolvectl_allowed(args) => Ok(spec("resolvectl", args)),
        "nvidia-smi" if nvidia_smi_allowed(args) => Ok(spec("nvidia-smi", args)),

        "top" => {
            let mut out = vec!["-s".into()];
            out.extend_from_slice(args);
            Ok(CommandSpec {
                program: "top".into(),
                args: out,
                warn_unrestricted_bash: false,
            })
        }

        "htop" => {
            let mut out = vec!["--readonly".into()];
            out.extend_from_slice(args);
            Ok(CommandSpec {
                program: "htop".into(),
                args: out,
                warn_unrestricted_bash: false,
            })
        }

        _ if HOST_COMMANDS.contains(&cmd.as_str()) => Ok(CommandSpec {
            program: cmd.clone(),
            args: args.to_vec(),
            warn_unrestricted_bash: false,
        }),

        _ => Err(()),
    }
}

fn spec(program: &str, args: &[String]) -> CommandSpec {
    CommandSpec {
        program: program.into(),
        args: args.to_vec(),
        warn_unrestricted_bash: false,
    }
}

fn ip_allowed(args: &[String]) -> bool {
    const OBJECTS: &[&str] = &["addr", "address", "link", "route", "neigh", "neighbor"];
    const MUTATING: &[&str] = &[
        "add",
        "del",
        "delete",
        "change",
        "replace",
        "set",
        "flush",
        "append",
        "prepend",
        "batch",
        "-batch",
        "--batch",
        "netns",
        "exec",
    ];

    if args.iter().any(|arg| MUTATING.contains(&arg.as_str())) {
        return false;
    }

    args.iter().any(|arg| OBJECTS.contains(&arg.as_str()))
}

fn hostname_allowed(args: &[String]) -> bool {
    match args {
        [] => true,
        [arg] => matches!(
            arg.as_str(),
            "-s"
                | "--short"
                | "-f"
                | "--fqdn"
                | "-d"
                | "--domain"
                | "-i"
                | "--ip-address"
                | "-I"
                | "--all-ip-addresses"
        ),
        _ => false,
    }
}

fn resolvectl_allowed(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        None | Some("status" | "query" | "statistics" | "show-server-state")
    )
}

fn nvidia_smi_allowed(args: &[String]) -> bool {
    const DENIED: &[&str] = &[
        "-pm",
        "--persistence-mode",
        "-pl",
        "--power-limit",
        "-lgc",
        "--lock-gpu-clocks",
        "-lmc",
        "--lock-memory-clocks",
        "-ac",
        "--applications-clocks",
        "-rac",
        "--reset-applications-clocks",
        "-rgc",
        "--reset-gpu-clocks",
        "-rmc",
        "--reset-memory-clocks",
        "-gom",
        "--gpu-operation-mode",
        "-mig",
        "--mig-mode",
        "-r",
        "--gpu-reset",
    ];

    !args.iter().any(|arg| {
        DENIED
            .iter()
            .any(|denied| arg == denied || arg.starts_with(&format!("{denied}=")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn lspci_is_allowed() {
        assert!(classify(&argv(&["lspci", "-Dnnk"]), false).is_ok());
    }

    #[test]
    fn ip_show_is_allowed() {
        assert!(classify(&argv(&["ip", "-br", "addr"]), false).is_ok());
    }

    #[test]
    fn ip_mutation_is_denied() {
        assert!(classify(&argv(&["ip", "link", "set", "eth0", "down"]), false).is_err());
    }

    #[test]
    fn hostname_change_is_denied() {
        assert!(classify(&argv(&["hostname", "new-name"]), false).is_err());
    }

    #[test]
    fn nvidia_read_is_allowed() {
        assert!(classify(&argv(&["nvidia-smi", "-L"]), false).is_ok());
    }

    #[test]
    fn nvidia_mutation_is_denied() {
        assert!(classify(&argv(&["nvidia-smi", "-pl", "250"]), false).is_err());
    }

    #[test]
    fn nvidia_mutation_with_equals_is_denied() {
        assert!(classify(&argv(&["nvidia-smi", "--power-limit=250"]), false).is_err());
    }

    #[test]
    fn bash_requires_explicit_permission() {
        assert!(classify(&argv(&["bash"]), false).is_err());
        assert!(classify(&argv(&["bash"]), true).is_ok());
    }
}
