mod executor;
mod input;
mod parser;
mod policy;
mod shell;
mod ui;

fn main() {
    std::process::exit(shell::run());
}
