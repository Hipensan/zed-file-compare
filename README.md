# zed-anydiff

Compare any two files in Zed — even outside the open project. `zed-anydiff` is a small standalone Rust CLI that pins one file as a "base" and, on demand, opens Zed's **native diff view** (`zed --diff`) against another file. It never implements its own diff renderer.

## Prerequisites

- **Rust** (to build the CLI)
- **Zed** installed with its `zed` CLI on your `PATH`

## Install

```sh
cargo build --release
```

Put the binary anywhere on your `PATH` (Zed's task terminal inherits your shell `PATH`):

```sh
# Linux / macOS
cp target/release/zed-anydiff ~/.cargo/bin/
# Windows
copy target\release\zed-anydiff.exe C:\Users\you\.cargo\bin\
```

### Zed tasks

Add these to your **global** `tasks.json`. Zed reads this file as a JSON **array** of task objects — merge with any tasks you already have rather than replacing the file.

- Linux / macOS: `~/.config/zed/tasks.json`
- Windows: `%APPDATA%\zed\tasks.json`

```json
[
  { "label": "AnyDiff: Set as Base",       "command": "zed-anydiff", "args": ["base", "$ZED_FILE"], "env": { "ZED_ANYDIFF_QUIET": "1" }, "show_summary": false, "show_command": false, "hide": "on_success" },
  { "label": "AnyDiff: Compare with Base", "command": "zed-anydiff", "args": ["compare", "$ZED_FILE"], "env": { "ZED_ANYDIFF_QUIET": "1" }, "show_summary": false, "show_command": false, "hide": "on_success" },
  { "label": "AnyDiff: Show Base",         "command": "zed-anydiff", "args": ["status"], "show_summary": false, "show_command": false },
  { "label": "AnyDiff: Clear Base",        "command": "zed-anydiff", "args": ["clear"], "show_summary": false, "show_command": false, "hide": "on_success" }
]
```

The silence fields keep the Zed-triggered run quiet: `env` sets `ZED_ANYDIFF_QUIET` so the CLI prints nothing, `show_summary`/`show_command` suppress Zed's own `⏵ Task … finished` / `⏵ Command: …` lines, and `hide: "on_success"` closes the terminal tab after the command succeeds (it stays open on failure so errors remain visible). "Show Base" omits `hide` because its output *is* the point.

`$ZED_FILE` is the absolute path of the file currently open, so this works for files outside the project.

### Key bindings (optional)

Bind any task to a key by passing `task_name` to `task::Spawn` in your `keymap.json`:

- Linux / macOS: `~/.config/zed/keymap.json`
- Windows: `%APPDATA%\zed\keymap.json`

Example — `Ctrl+Alt+1` sets the base, `Ctrl+Alt+2` compares:

```json
[
  {
    "context": "Workspace",
    "bindings": {
      "ctrl-alt-1": ["task::Spawn", { "task_name": "AnyDiff: Set as Base" }],
      "ctrl-alt-2": ["task::Spawn", { "task_name": "AnyDiff: Compare with Base" }]
    }
  }
]
```

The `task_name` value must match the task `label` above.

## Usage

In Zed, with a file open:

- `Ctrl+Alt+1` → make the open file the base
- open another file, `Ctrl+Alt+2` → Zed's native diff view opens

Or, without keybindings, open the task modal (`Ctrl+Shift+P` → **Zed: Run Task**) and pick an `AnyDiff:` task.

### CLI

```sh
zed-anydiff base FILE      # persist FILE as the base
zed-anydiff compare FILE   # open zed --diff BASE FILE
zed-anydiff status         # show the current base
zed-anydiff clear          # clear the stored base
```

Exit codes: `0` success, `1` operational error (missing file, no base set, `zed` not on `PATH`), `2` usage error.

**Quiet mode.** Setting `ZED_ANYDIFF_QUIET` (to `1`, `true`, or `yes`) suppresses the success messages (`Base file: …`, `Comparing:` block) while still printing errors to stderr. This is how the Zed tasks stay silent — each task sets it via its `env` field. Run the CLI from a plain shell (no flag) and the output is unchanged.

## State

The base is stored as `{"base": "<absolute path>"}` in the OS cache directory:

- Windows: `%LOCALAPPDATA%\zed-anydiff\state.json`
- Linux: `$XDG_CACHE_HOME/zed-anydiff/state.json`
- macOS: `~/Library/Caches/zed-anydiff/state.json`

`ZED_ANYDIFF_STATE_DIR` overrides the location (used by the test suite so it never touches the real cache).

## Notes

- Diff rendering is always delegated to Zed's built-in `zed --diff`; this tool only orchestrates it.
- `$ZED_FILE` only exists while a file-backed buffer is open; an unsaved buffer has no `$ZED_FILE`.
- A single base slot — no history, no named slots, no swap.
- Paths with spaces and Unicode are safe: arguments are passed to the OS as a vector, never through a shell.
- The `--diff` window is an empty Zed workspace: the two files open as a diff view, not as worktrees, so it inherits Zed's remembered dock layout for empty workspaces (the `default_dock_state` KVP). An empty project panel can therefore appear next to the diff — Zed's standard behavior, not a `zed-anydiff` defect.

## License

MIT — see [LICENSE](LICENSE).
