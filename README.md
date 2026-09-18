# zed-anydiff

Compare any two files in Zed, even outside the current project.

`zed-anydiff` is a small standalone Rust CLI that gives Zed a VS Code-style
"set base / compare with base" workflow. It remembers one base file, and when
you ask it to compare another file, it opens Zed's **native diff view** via
`zed --diff`. It does not implement a diff renderer of its own; all diff
rendering is delegated to Zed's built-in `zed --diff` functionality.

## Why?

Zed can diff two files with `zed --diff OLD_PATH NEW_PATH`, but there is no
built-in way to pin one file as "the base" and keep comparing other files
against it, especially when those files are outside the currently opened
project. `zed-anydiff` fills exactly that gap with a four-command CLI plus a
few Zed tasks, using only documented Zed functionality.

## Features

- **`base FILE`** — persist `FILE` as the base.
- **`compare FILE`** — launch `zed --diff <BASE> <FILE>` in Zed's native diff
  view. The base is **retained** after comparison, so you can compare several
  files against the same base back to back.
- **`status`** — show which file is currently the base.
- **`clear`** — clear the stored base (idempotent).
- Works with files that have **spaces and Unicode** in their names: all
  arguments are passed to `std::process::Command` as a vector, never through a
  shell, so quoting is never an issue.
- Cross-platform: Windows, Linux, and macOS (state lives in the OS cache
  directory).
- Small, single binary with two runtime dependencies (`dirs`, `serde_json`).

## Installation

You need:

1. **Rust** (to build `zed-anydiff`).
2. **Zed** installed with its CLI on your `PATH` (the `zed` command).

Build and install:

```sh
cargo build --release
# Windows
copy target\release\zed-anydiff.exe C:\Users\you\.cargo\bin\
# Linux / macOS
cp target/release/zed-anydiff ~/.cargo/bin/
```

Any directory on your `PATH` works.

## Usage

```sh
zed-anydiff base "D:\docs\original.txt"
zed-anydiff compare "D:\docs\edited.txt"
zed-anydiff status
zed-anydiff clear
```

Examples:

```sh
$ zed-anydiff base "C:\work\repo 1\файл.txt"
Base file: C:\work\repo 1\файл.txt

$ zed-anydiff compare "C:\work\repo 1\другой.txt"
Comparing:
  base:   C:\work\repo 1\файл.txt
  target: C:\work\repo 1\другой.txt
# opens Zed's native diff view comparing the two files

$ zed-anydiff status
Base: C:\work\repo 1\файл.txt

$ zed-anydiff clear
Base: cleared
```

`compare` prints the pair it is comparing and then launches Zed. The base is
left in place, so a second `compare` reuses the same base without re-running
`base`.

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | success |
| `1` | operational error (missing file, no base set, `zed` not on `PATH`, I/O error) |
| `2` | usage error (unknown command or wrong number of arguments) |

### Where the base is stored

The base is stored as `{"base": "<absolute path>"}` in the OS cache directory:

- Windows: `%LOCALAPPDATA%\zed-anydiff\state.json`
- Linux: `$XDG_CACHE_HOME/zed-anydiff/state.json`
- macOS: `~/Library/Caches/zed-anydiff/state.json`

The `ZED_ANYDIFF_STATE_DIR` environment variable overrides this location
(mainly used by the test suite so it never touches the real user cache).

## Zed integration

Zed's **task** system is the supported way to trigger the workflow from
inside the editor: tasks run in Zed's integrated terminal and can use the
`$ZED_FILE` task variable, which is the **absolute path of the currently
opened file** — so it works for files outside the project too.

The repo ships `zed/tasks.json` with four tasks. To install them, add them to
your global task file (`~/.config/zed/tasks.json` on Linux/macOS, or the
platform equivalent — on Windows, Zed's global config under
`%APPDATA%\zed\`). Merge rather than replace if you already have tasks:

```json
{
  "AnyDiff: Set as Base": {
    "command": "zed-anydiff",
    "args": ["base", "$ZED_FILE"]
  },
  "AnyDiff: Compare with Base": {
    "command": "zed-anydiff",
    "args": ["compare", "$ZED_FILE"]
  },
  "AnyDiff: Show Base": {
    "command": "zed-anydiff",
    "args": ["status"]
  },
  "AnyDiff: Clear Base": {
    "command": "zed-anydiff",
    "args": ["clear"]
  }
}
```

Then, with a file open in Zed:

1. Open the task modal (`Ctrl+Shift+P` → "Zed: Run Task").
2. Run **`AnyDiff: Set as Base`** to make the active file the base.
3. Open the other file and run **`AnyDiff: Compare with Base`** — Zed's native
   diff view opens.
4. Run **`AnyDiff: Show Base`** / **`AnyDiff: Clear Base`** to inspect or reset
   the base.

The CLI is fully independent of these tasks, so the command line always works
even if you never install the task configuration.

## How it works

```
zed-anydiff base FILE        zed-anydiff compare FILE
        |                                |
        v                                v
  persist base path          look up base, verify both files
  to state file              exist, then spawn:
        |                          zed --diff BASE TARGET
        v                                |
  state.json                          Zed native diff view
```

- **State** — one JSON file written with `serde_json`, so paths containing
  quotes, backslashes, or Unicode round-trip exactly.
- **Path handling** — user-supplied paths are canonicalized (absolute,
  normalized) and re-verified at compare time, so a deleted base is reported
  as "base file no longer exists" rather than handed to Zed as a dead path.
- **Process execution** — `std::process::Command` with an argument vector; no
  shell string concatenation anywhere, which is what makes spaces and Unicode
  safe. Zed is spawned (not waited on), since `zed --diff` hands the work to
  the GUI.
- **Diff rendering** — none here. Zed's `zed --diff` view renders the diff.

## Limitations

- No diff renderer of any kind; this is a workflow wrapper around `zed --diff`.
- A single base slot — there is no base history, no named slots, no swap.
- Task variables exist only while a file-backed buffer is open; an unsaved
  buffer has no `$ZED_FILE`, so it cannot be used as base/target through the
  Zed tasks.
- Tasks run in Zed's integrated terminal, so their output appears in the
  terminal pane.
- No directory diff, git revision comparison, three-way merge, or binary diff.
- Zed must be installed and its CLI on `PATH`.

## Roadmap

Potential follow-ups (not implemented): recent base history, swap base/target,
compare clipboard against file, multiple named slots, compare open tabs, file
picker, Zed command palette / Project Panel context menu integration (would
require Zed to expose those APIs), and git revision comparison.

## Contributing

See `AGENTS.md` at the repository root for the project's working agreements
(wrapper-only approach, no Zed source modifications, no invented Zed
functionality). Run `cargo fmt`, `cargo clippy`, and `cargo test` before
considering a change complete. The test suite never launches the user's Zed
GUI.

## License

MIT — see [LICENSE](LICENSE).
