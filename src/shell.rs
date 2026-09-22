use crate::executor::{self, Flow};
use crate::input::ShellHelper;
use crate::ui;

use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

const SAFE_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
const HISTORY_LIMIT: usize = 2000;

pub fn run() -> i32 {
    env::set_var("PATH", SAFE_PATH);

    let args: Vec<String> = env::args().collect();

    // OpenSSH/login shell form: incus-only-shell -c 'incus list'
    if args.get(1).map(String::as_str) == Some("-c") {
        if args.len() != 3 {
            ui::deny();
            return 126;
        }

        return non_interactive(&args[2]);
    }

    if args.len() != 1 {
        ui::deny();
        return 126;
    }

    if io::stdin().is_terminal() {
        interactive()
    } else {
        let mut input = String::new();
        if io::stdin().read_to_string(&mut input).is_err() {
            return 1;
        }

        non_interactive(&input)
    }
}

fn interactive() -> i32 {
    let history_file = history_path();
    prepare_history_file(&history_file);

    let mut editor = match Editor::<ShellHelper, DefaultHistory>::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("readline: {err}");
            return 1;
        }
    };

    editor.set_helper(Some(ShellHelper));
    let _ = editor.set_max_history_size(HISTORY_LIMIT);
    let _ = editor.load_history(&history_file);

    println!("{}", ui::BANNER);
    println!();

    loop {
        let input = match editor.readline(&ui::prompt()) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => {
                println!();
                return 0;
            }
            Err(err) => {
                eprintln!("readline: {err}");
                return 1;
            }
        };

        if input.trim().is_empty() {
            continue;
        }

        let _ = editor.add_history_entry(input.as_str());
        let _ = editor.append_history(&history_file);

        let history: Vec<String> = editor.history().iter().cloned().collect();

        match executor::run_input(&input, &history) {
            Flow::Continue(_) => {}
            Flow::Exit(rc) => return rc,
        }
    }
}

fn non_interactive(input: &str) -> i32 {
    match executor::run_input(input, &[]) {
        Flow::Continue(rc) | Flow::Exit(rc) => rc,
    }
}

fn history_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".incus-only-shell_history")
}

fn prepare_history_file(path: &PathBuf) {
    if OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .is_ok()
    {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
}
