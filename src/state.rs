//! Persistent single-base state.
//!
//! Stored as `{"base": "<absolute path>"}` in the OS cache directory
//! (`%LOCALAPPDATA%\zed-anydiff` on Windows, `$XDG_CACHE_HOME/zed-anydiff`
//! on Linux, `~/Library/Caches/zed-anydiff` on macOS), so no hardcoded
//! `/tmp` is involved.
//!
//! The `ZED_ANYDIFF_STATE_DIR` environment variable overrides the location;
//! tests use it so they never touch the real user cache.

use std::fs;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = "state.json";

/// Directory holding `state.json`: `ZED_ANYDIFF_STATE_DIR` if set,
/// otherwise the OS cache directory.
pub fn state_dir() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("ZED_ANYDIFF_STATE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    dirs::cache_dir().ok_or_else(|| "could not determine the OS cache directory".to_string())
}

fn state_file_in(dir: &Path) -> PathBuf {
    dir.join("zed-anydiff").join(STATE_FILE)
}

/// Load the stored base path, if any. A missing file means "no base set".
pub fn load() -> Result<Option<PathBuf>, String> {
    state_dir().and_then(|d| load_in(&d))
}

pub fn load_in(dir: &Path) -> Result<Option<PathBuf>, String> {
    let path = state_file_in(dir);
    match fs::read_to_string(&path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("could not read state file {}: {e}", path.display())),
    }
}

fn parse(text: &str) -> Result<Option<PathBuf>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("could not parse state file: {e}"))?;
    let base = value.get("base").and_then(|b| b.as_str());
    Ok(base.map(PathBuf::from))
}

/// Persist the base path. JSON encoding (not string formatting) keeps paths
/// with quotes, backslashes, or Unicode safe.
pub fn save(base: &Path) -> Result<(), String> {
    state_dir().and_then(|d| save_in(&d, base))
}

pub fn save_in(dir: &Path, base: &Path) -> Result<(), String> {
    let path = state_file_in(dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
    }
    let json = serde_json::json!({ "base": base.to_string_lossy().to_string() });
    let text = serde_json::to_string(&json).map_err(|e| format!("could not encode state: {e}"))?;
    fs::write(&path, text)
        .map_err(|e| format!("could not write state file {}: {e}", path.display()))?;
    Ok(())
}

/// Remove the stored base. Idempotent: clearing when nothing is set is a no-op.
pub fn clear() -> Result<(), String> {
    state_dir().and_then(|d| clear_in(&d))
}

pub fn clear_in(dir: &Path) -> Result<(), String> {
    match fs::remove_file(state_file_in(dir)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not remove state file: {e}")),
    }
}
