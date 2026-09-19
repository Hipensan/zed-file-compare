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
use crate::zed;

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
        "compare" => cmd_compare(rest),
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

/// Whether the invocation was requested quietly, e.g. by a Zed task that sets
/// `ZED_ANYDIFF_QUIET=1` in its `env`. Errors are always printed regardless.
fn quiet() -> bool {
    matches!(
        std::env::var("ZED_ANYDIFF_QUIET"),
        Ok(v) if v == "1" || v.eq_ignore_ascii_case("true")
    )
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
            if !quiet() {
                println!("Base file: {}", base.display());
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn cmd_compare(rest: &[String]) -> i32 {
    let path = match require_single_path(rest) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("{USAGE}");
            return 1;
        }
    };
    let base = match state::load() {
        Ok(Some(b)) => b,
        Ok(None) => {
            eprintln!("Error: no base file selected.");
            eprintln!("Run:");
            eprintln!("  zed-anydiff base <FILE>");
            return 1;
        }
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    // Re-verify at compare time: the base may have been deleted since it was
    // stored, and we should say so rather than handing a dead path to Zed.
    if !base.is_file() {
        eprintln!("Error: base file no longer exists: {}", base.display());
        eprintln!("Run:");
        eprintln!("  zed-anydiff base <FILE>");
        return 1;
    }
    let target = match canonicalize(&path, "Target") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    if !quiet() {
        println!("Comparing:");
        println!("  base:   {}", base.display());
        println!("  target: {}", target.display());
    }
    // The base is deliberately left in place so further `compare` calls
    // reuse it.
    match zed::launch_diff(&base, &target) {
        Ok(()) => 0,
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use super::*;

    /// Serializes tests that redirect `ZED_ANYDIFF_STATE_DIR`: the env var
    /// is process-wide, so two concurrent tests could clobber each other's
    /// state location.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Sandbox {
        dir: PathBuf,
        prev: Option<std::ffi::OsString>,
    }

    impl Sandbox {
        fn new() -> Self {
            let n = NEXT.fetch_add(1, Ordering::SeqCst);
            let dir = std::env::temp_dir()
                .join(format!("zed-anydiff-cli-test-{}-{n}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            let prev = std::env::var_os("ZED_ANYDIFF_STATE_DIR");
            std::env::set_var("ZED_ANYDIFF_STATE_DIR", &dir);
            Sandbox { dir, prev }
        }

        fn file(&self, name: &str, content: &str) -> PathBuf {
            let p = self.dir.join(name);
            fs::write(&p, content).unwrap();
            p
        }

        fn argv(&self, args: &[&str]) -> Vec<String> {
            args.iter().map(|s| s.to_string()).collect()
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var("ZED_ANYDIFF_STATE_DIR", v),
                None => std::env::remove_var("ZED_ANYDIFF_STATE_DIR"),
            }
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn with_sandbox(f: impl FnOnce(&Sandbox)) {
        let _guard = match ENV_LOCK.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let sb = Sandbox::new();
        f(&sb);
    }

    #[test]
    fn base_persists_and_status_reports_it() {
        with_sandbox(|sb| {
            let foo = sb.file("foo.c", "a");
            assert_eq!(run(&sb.argv(&["base", foo.to_str().unwrap()])), 0);
            assert_eq!(state::load().unwrap(), Some(foo.canonicalize().unwrap()));
            assert_eq!(run(&sb.argv(&["status"])), 0);
        });
    }

    #[test]
    fn base_with_spaces_and_unicode() {
        with_sandbox(|sb| {
            let path = sb.file("файл with space.txt", "a");
            assert_eq!(run(&sb.argv(&["base", path.to_str().unwrap()])), 0);
            assert_eq!(state::load().unwrap(), Some(path.canonicalize().unwrap()));
        });
    }

    #[test]
    fn base_missing_file_errors() {
        with_sandbox(|sb| {
            let missing = sb.dir.join("does-not-exist.c");
            assert_eq!(run(&sb.argv(&["base", missing.to_str().unwrap()])), 1);
            assert_eq!(state::load().unwrap(), None);
        });
    }

    #[test]
    fn compare_without_base_errors() {
        with_sandbox(|sb| {
            let bar = sb.file("bar.c", "b");
            assert_eq!(run(&sb.argv(&["compare", bar.to_str().unwrap()])), 1);
        });
    }

    #[test]
    fn compare_with_missing_target_errors() {
        with_sandbox(|sb| {
            let foo = sb.file("foo.c", "a");
            run(&sb.argv(&["base", foo.to_str().unwrap()]));
            let missing = sb.dir.join("nope.c");
            assert_eq!(run(&sb.argv(&["compare", missing.to_str().unwrap()])), 1);
        });
    }

    #[test]
    fn compare_with_deleted_base_errors() {
        with_sandbox(|sb| {
            let foo = sb.file("foo.c", "a");
            run(&sb.argv(&["base", foo.to_str().unwrap()]));
            fs::remove_file(&foo).unwrap();
            let bar = sb.file("bar.c", "b");
            assert_eq!(run(&sb.argv(&["compare", bar.to_str().unwrap()])), 1);
        });
    }

    #[test]
    fn compare_without_zed_reports_missing_executable() {
        // `zed` is not guaranteed absent on every machine; pin PATH to the
        // empty sandbox dir so `zed` cannot be found without spawning Zed.
        with_sandbox(|sb| {
            let foo = sb.file("foo.c", "a");
            let bar = sb.file("bar.c", "b");
            run(&sb.argv(&["base", foo.to_str().unwrap()]));
            let prev_path = std::env::var_os("PATH");
            std::env::set_var("PATH", &sb.dir);
            let code = run(&sb.argv(&["compare", bar.to_str().unwrap()]));
            match prev_path {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
            assert_eq!(code, 1);
        });
    }

    #[test]
    fn clear_removes_base() {
        with_sandbox(|sb| {
            let foo = sb.file("foo.c", "a");
            run(&sb.argv(&["base", foo.to_str().unwrap()]));
            assert_eq!(run(&sb.argv(&["clear"])), 0);
            assert_eq!(state::load().unwrap(), None);
        });
    }

    #[test]
    fn quiet_flag_is_respected() {
        with_sandbox(|sb| {
            let foo = sb.file("foo.c", "a");
            // Without the flag the base is still persisted; the quiet flag only
            // gates stdout, which we assert here via `quiet()` directly since
            // capturing a child's stdout is not what `run` exposes.
            assert!(!quiet());
            std::env::set_var("ZED_ANYDIFF_QUIET", "1");
            assert!(quiet());
            assert_eq!(run(&sb.argv(&["base", foo.to_str().unwrap()])), 0);
            assert_eq!(state::load().unwrap(), Some(foo.canonicalize().unwrap()));
            std::env::remove_var("ZED_ANYDIFF_QUIET");
            assert!(!quiet());
        });
    }

    #[test]
    fn unknown_command_exits_2() {
        assert_eq!(run(&["frobnicate".to_string()]), 2);
        assert_eq!(run(&[]), 2);
    }
}
