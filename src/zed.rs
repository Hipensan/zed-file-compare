//! Launching Zed's native diff view.
//!
//! All arguments are passed to `std::process::Command` as a vector — never
//! through a shell — so paths containing spaces or Unicode survive intact.

use std::path::Path;
use std::process::Command;

/// Build the full `zed --diff BASE TARGET` argument list.
///
/// Kept separate from launching so command construction can be unit-tested
/// without spawning the user's Zed application.
pub fn build_diff_args(base: &Path, target: &Path) -> Vec<String> {
    vec![
        "zed".to_string(),
        "--diff".to_string(),
        base.to_string_lossy().into_owned(),
        target.to_string_lossy().into_owned(),
    ]
}

/// Spawn Zed to show the native diff view.
///
pub fn launch_diff(base: &Path, target: &Path) -> Result<(), String> {
    let args = build_diff_args(base, target);
    Command::new(&args[0])
        .args(&args[1..])
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "Could not find the Zed CLI executable.\n\
                 Ensure the `zed` command is installed and available in PATH."
                    .to_string()
            } else {
                format!("failed to launch `zed`: {e}")
            }
        })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn builds_diff_args_in_order() {
        let args = build_diff_args(Path::new("/a/foo.c"), Path::new("/b/bar.c"));
        assert_eq!(
            args,
            vec![
                "zed".to_string(),
                "--diff".to_string(),
                "/a/foo.c".to_string(),
                "/b/bar.c".to_string()
            ]
        );
    }

    #[test]
    fn builds_diff_args_with_spaces_and_unicode() {
        let args = build_diff_args(
            Path::new("D:\\repo 1\\файл.txt"),
            Path::new("C:\\temp\\바라.txt"),
        );
        assert_eq!(
            args,
            vec![
                "zed".to_string(),
                "--diff".to_string(),
                "D:\\repo 1\\файл.txt".to_string(),
                "C:\\temp\\바라.txt".to_string(),
            ]
        );
    }
}
