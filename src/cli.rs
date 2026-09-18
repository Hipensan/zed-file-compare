//! Command parsing and dispatch.
//!
//! Commands:
//!
//!     zed-anydiff base FILE      persist FILE as the base
//!     zed-anydiff compare FILE   launch `zed --diff BASE FILE` (base retained)
//!     zed-anydiff status         show the current base
//!     zed-anydiff clear          clear the stored base

use std::path::{Path, PathBuf};

use crate::state;

const USAGE: &str = "Usage: zed-anydiff <base|compare|status|clear>\n\n\
                     base FILE      persist FILE as the base\n\
                     compare FILE   launch zed --diff BASE FILE (base retained)\n\
                     status         show the current base\n\
                     clear          clear the stored base";

/// Dispatch one invocation and return the process exit code.
pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("{USAGE}");
        return 2;
    }
    let (command, rest) = (args[0].as_str(), &args[1..]);
    match command {
        "base" => cmd_base(rest),
        "status" => cmd_status(),
        "clear" => cmd_clear(),
        other => {
            eprintln!("Unknown command: {other}");
            eprintln!("{USAGE}");
            2
        }
    }
}

fn require_single_path(rest: &[String]) -> Result<PathBuf, String> {
    match rest {
        [] => Err("missing FILE argument".to_string()),
        [path] => Ok(PathBuf::from(path)),
        _ => Err("expected exactly one FILE argument".to_string()),
    }
}

/// Resolve a user-supplied path to an absolute, normalized path.
/// On Windows this also resolves the drive-letter case, so `D:\X\y` and
/// `d:\x\y` refer to the same file.
fn canonicalize(path: &Path, label: &str) -> Result<PathBuf, String> {
    match path.canonicalize() {
        Ok(resolved) => Ok(resolved),
        Err(_) => Err(format!(
            "{label} file does not exist or is not accessible: {}",
            path.display()
        )),
    }
}

fn cmd_base(rest: &[String]) -> i32 {
    let path = match require_single_path(rest) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("{USAGE}");
            return 1;
        }
    };
    let base = match canonicalize(&path, "Base") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    match state::save(&base) {
        Ok(()) => {
            println!("Base file: {}", base.display());
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn cmd_status() -> i32 {
    match state::load() {
        Ok(Some(base)) => {
            println!("Base: {}", base.display());
            0
        }
        Ok(None) => {
            println!("Base: not set");
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn cmd_clear() -> i32 {
    match state::clear() {
        Ok(()) => {
            println!("Base: cleared");
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}
