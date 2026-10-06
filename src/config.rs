use crate::policy::AllowedCommands;

use std::fs;
use std::io;
use std::path::Path;

pub const CONFIG_DIR: &str = "/etc/incus-only-shell";
pub const DEFAULT_BANNER: &str = include_str!("../config/banner.txt");
pub const DEFAULT_ALLOWED_COMMANDS: &str = include_str!("../config/allowed-commands.txt");

pub struct ShellConfig {
    pub banner: String,
    pub allowed_commands: AllowedCommands,
}

impl ShellConfig {
    pub fn load() -> Result<Self, String> {
        Self::load_from_dir(Path::new(CONFIG_DIR))
    }

    fn load_from_dir(dir: &Path) -> Result<Self, String> {
        let commands_path = dir.join("allowed-commands.txt");
        let contents = match fs::read_to_string(&commands_path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                DEFAULT_ALLOWED_COMMANDS.to_string()
            }
            Err(err) => return Err(format!("{}: {err}", commands_path.display())),
        };
        let allowed_commands = AllowedCommands::parse(&contents)
            .map_err(|err| format!("{}: {err}", commands_path.display()))?;

        let banner_path = dir.join("banner.txt");
        let banner = match fs::read_to_string(&banner_path) {
            Ok(contents) => contents,
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    eprintln!("{}: {err}; using the default banner", banner_path.display());
                }
                DEFAULT_BANNER.to_string()
            }
        };

        Ok(Self {
            banner,
            allowed_commands,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "incus-only-shell-config-{}-{}",
                std::process::id(),
                NEXT_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn write(&self, name: &str, contents: &[u8]) {
            fs::write(self.0.join(name), contents).unwrap();
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_files_use_embedded_defaults() {
        let dir = TestDir::new();
        let config = ShellConfig::load_from_dir(&dir.0).unwrap();
        assert_eq!(config.banner, DEFAULT_BANNER);
        assert!(config.allowed_commands.is_allowed("incus"));
        assert!(config.allowed_commands.is_allowed("bash"));
        assert!(config.allowed_commands.is_allowed("lspci"));
        assert!(!config.allowed_commands.is_allowed("cp"));
    }

    #[test]
    fn external_files_replace_defaults() {
        let dir = TestDir::new();
        dir.write("banner.txt", b"Custom host banner");
        dir.write(
            "allowed-commands.txt",
            b"# Custom commands\ncp\n\npwd # location\n",
        );
        let config = ShellConfig::load_from_dir(&dir.0).unwrap();
        assert_eq!(config.banner, "Custom host banner");
        assert!(config.allowed_commands.is_allowed("cp"));
        assert!(config.allowed_commands.is_allowed("pwd"));
        assert!(!config.allowed_commands.is_allowed("incus"));
        assert!(config.allowed_commands.is_allowed("bash"));
    }

    #[test]
    fn empty_files_are_intentional_overrides() {
        let dir = TestDir::new();
        dir.write("banner.txt", b"");
        dir.write("allowed-commands.txt", b"# Built-ins and Bash only\n");
        let config = ShellConfig::load_from_dir(&dir.0).unwrap();
        assert!(config.banner.is_empty());
        assert_eq!(config.allowed_commands.names().count(), 0);
        assert!(config.allowed_commands.is_allowed("bash"));
    }

    #[test]
    fn invalid_command_file_does_not_restore_defaults() {
        let dir = TestDir::new();
        dir.write("allowed-commands.txt", b"pwd\nip addr\n");
        let err = ShellConfig::load_from_dir(&dir.0).err().unwrap();
        assert!(err.contains("allowed-commands.txt"));
        assert!(err.contains("line 2"));
    }

    #[test]
    fn unreadable_command_file_does_not_restore_defaults() {
        let dir = TestDir::new();
        fs::create_dir(dir.0.join("allowed-commands.txt")).unwrap();
        assert!(ShellConfig::load_from_dir(&dir.0).is_err());
    }

    #[test]
    fn invalid_utf8_banner_uses_default() {
        let dir = TestDir::new();
        dir.write("banner.txt", &[0xff]);
        let config = ShellConfig::load_from_dir(&dir.0).unwrap();
        assert_eq!(config.banner, DEFAULT_BANNER);
    }

    #[test]
    fn external_list_controls_multiline_pipeline_and_and_chain() {
        use crate::executor::{run_input, Flow};

        let dir = TestDir::new();
        dir.write("allowed-commands.txt", b"cat\n");
        let config = ShellConfig::load_from_dir(&dir.0).unwrap();
        let allowed = &config.allowed_commands;

        assert_eq!(
            run_input(
                "bash -c 'printf \"first\\n\"' \\\n| cat &&\nbash -c 'printf \"second\\n\"'\n",
                &[],
                allowed,
            ),
            Flow::Continue(0)
        );
        assert_eq!(run_input("pwd\n", &[], allowed), Flow::Continue(126));
        assert_eq!(
            run_input("bash -c 'printf hello' | grep hello\n", &[], allowed),
            Flow::Continue(126)
        );
        assert_eq!(
            run_input("pwd && bash -c 'exit 7'\n", &[], allowed),
            Flow::Continue(126)
        );
    }
}
