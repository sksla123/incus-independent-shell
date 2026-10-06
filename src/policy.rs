use std::collections::BTreeSet;

pub const BUILTIN_COMMANDS: &[&str] = &["cd", "history", "help", "shell-version", "exit", "logout"];
pub const ALWAYS_ALLOWED_EXTERNAL_COMMANDS: &[&str] = &["bash"];

pub struct AllowedCommands {
    commands: BTreeSet<String>,
}

impl AllowedCommands {
    pub fn parse(contents: &str) -> Result<Self, String> {
        let mut commands = BTreeSet::new();

        for (index, line) in contents.lines().enumerate() {
            let name = line.split('#').next().unwrap_or("").trim();
            if name.is_empty() {
                continue;
            }

            // The file contains command names, not paths or command lines.
            if !name.starts_with(|c: char| c.is_ascii_alphanumeric())
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
            {
                return Err(format!(
                    "line {}: expected one command name, got {name:?}",
                    index + 1
                ));
            }

            // Built-ins and the host shell are always available independently of this file.
            if !BUILTIN_COMMANDS.contains(&name)
                && !ALWAYS_ALLOWED_EXTERNAL_COMMANDS.contains(&name)
            {
                commands.insert(name.to_string());
            }
        }

        Ok(Self { commands })
    }

    pub fn is_allowed(&self, name: &str) -> bool {
        ALWAYS_ALLOWED_EXTERNAL_COMMANDS.contains(&name) || self.commands.contains(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.commands.iter().map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

pub fn classify(argv: &[String], allowed: &AllowedCommands) -> Result<CommandSpec, ()> {
    let cmd = argv.first().ok_or(())?;
    if !allowed.is_allowed(cmd) {
        return Err(());
    }

    Ok(CommandSpec {
        program: cmd.clone(),
        args: argv[1..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_ALLOWED_COMMANDS;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn allowed_commands_receive_all_arguments_unchanged() {
        let allowed = AllowedCommands::parse(DEFAULT_ALLOWED_COMMANDS).unwrap();
        for items in [
            vec!["ip", "link", "set", "eth0", "down"],
            vec!["hostname", "new-name"],
            vec!["resolvectl", "dns", "eth0", "1.1.1.1"],
            vec!["nvidia-smi", "--power-limit=250"],
            vec!["top", "-p", "123"],
            vec!["htop", "--sort-key=CPU%"],
            vec!["bash", "-c", "echo hello"],
            vec!["incus", "list"],
        ] {
            let input = argv(&items);
            let spec = classify(&input, &allowed).unwrap();
            assert_eq!(spec.program, input[0]);
            assert_eq!(spec.args, input[1..]);
        }
    }

    #[test]
    fn custom_list_can_add_and_remove_commands() {
        let defaults = AllowedCommands::parse(DEFAULT_ALLOWED_COMMANDS).unwrap();
        let custom = AllowedCommands::parse("cp\n").unwrap();
        assert!(classify(&argv(&["cp", "-r", "source", "dest"]), &defaults).is_err());
        assert!(classify(&argv(&["cp", "-r", "source", "dest"]), &custom).is_ok());
        assert!(classify(&argv(&["incus", "list"]), &custom).is_err());
    }

    #[test]
    fn comments_and_duplicate_names_are_supported() {
        let allowed = AllowedCommands::parse("# List\r\npwd # comment\r\n\r\npwd\n").unwrap();
        assert_eq!(allowed.names().collect::<Vec<_>>(), vec!["pwd"]);
    }

    #[test]
    fn paths_and_command_lines_are_not_command_names() {
        for input in ["/bin/bash", "./pwd", "ip addr", "pwd;ls", "..", "-option"] {
            assert!(AllowedCommands::parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn builtins_are_separate_from_external_commands() {
        let allowed = AllowedCommands::parse("cd\nexit\npwd\n").unwrap();
        assert_eq!(allowed.names().collect::<Vec<_>>(), vec!["pwd"]);
        assert!(classify(&argv(&["cd"]), &allowed).is_err());
    }

    #[test]
    fn bash_is_always_allowed_and_ignored_in_the_configured_list() {
        for contents in ["", "cp\n", "bash\n"] {
            let allowed = AllowedCommands::parse(contents).unwrap();
            let input = argv(&["bash", "-c", "echo hello"]);
            let spec = classify(&input, &allowed).unwrap();
            assert_eq!(spec.program, "bash");
            assert_eq!(spec.args, input[1..]);
            assert!(!allowed.names().any(|name| name == "bash"));
        }
    }
}
