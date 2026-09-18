//! zed-anydiff: a VS Code-style "set base / compare with base" workflow
//! for Zed, powered by Zed's native `zed --diff` viewer.
//!
//! It deliberately does not implement any diff renderer itself; it only
//! remembers one base file and asks Zed to show the diff.

mod cli;
mod state;
mod zed;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(cli::run(&args));
}
