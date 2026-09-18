# Zed API Research

Investigation of the current Zed extension/task/action/CLI surface to determine the
cleanest supported integration for `zed-anydiff`.

Date of research: 2026-09-18. All claims below were verified against the linked
sources on that date; nothing is claimed beyond what the sources state.

## 1. Verified findings

### 1.1 `zed --diff <OLD_PATH> <NEW_PATH>` (CLI) — USABLE

- **Source:** <https://zed.dev/docs/reference/cli> (`--diff <OLD_PATH> <NEW_PATH>` section)
- **Introduced by:** PR <https://github.com/zed-industries/zed/pulls/32922> ("Diff view"),
  merged 2025-06-18, closes <https://github.com/zed-industries/zed/issues/4523>.
- **Current behavior:** opens Zed's native diff view comparing two files.
  Can be specified multiple times (`zed --diff A B --diff C D`). Paths may be
  arbitrary; there is no requirement that they belong to an opened project.
- **Usable for this project:** yes — this is the diff engine we delegate to.
  It is the core of the wrapper: `zed --diff <BASE> <TARGET>`.
- **Limitations:** CLI-only entry point; no documented UI action/context-menu
  equivalent. Other relevant CLI flags: `-n/--new`, `-a/--add`, `-r/--reuse`,
  `-e/--existing` (window reuse controls), `--wait`.

### 1.2 Task variables (incl. `$ZED_FILE`) — USABLE

- **Source:** <https://zed.dev/docs/tasks> ("Variables" section)
- **Current behavior:** tasks (defined in global `~/.config/zed/tasks.json` or
  worktree-local `.zed/tasks.json`) run in Zed's integrated terminal with a
  limited set of variables resolved from the current editor state:
  - `ZED_FILE` — absolute path of the currently opened file
  - `ZED_FILENAME`, `ZED_DIRNAME`, `ZED_STEM` — filename pieces
  - `ZED_RELATIVE_FILE`, `ZED_RELATIVE_DIR`, `ZED_WORKTREE_ROOT` — worktree-relative
  - `ZED_SELECTED_TEXT`, `ZED_SYMBOL`, `ZED_ROW`, `ZED_COLUMN`, `ZED_LANGUAGE`
- **Variable quoting (documented):** for paths with spaces, either pass the
  variable via the `args` array (`"args": ["$ZED_FILE"]`) or quote it in the
  command string (`"$ZED_FILE"`).
- **Usable for this project:** yes — a task like
  `zed-anydiff base $ZED_FILE` gives "Set as Base" for the active file with no
  extension code, and works for files outside the project because `ZED_FILE`
  is the file's absolute path, not a worktree-relative one.
- **Limitations:**
  - The variable is only present when a buffer with an on-disk file path is
    active; unsaved buffers have no `ZED_FILE`, and tasks referencing missing
    variables are filtered out of the task modal.
  - Tasks execute in Zed's integrated terminal (the platform's shell), not as
    raw process launches — so quoting/escaping follows the docs' guidance.
  - There is no documented "file selected in project panel" or "file picker
    selection" variable; selection-based workflows are not supported.

### 1.3 Extension API — NOT USABLE for the core workflow

- **Sources:**
  - <https://zed.dev/docs/extensions/developing-extensions>
  - <https://zed.dev/docs/extensions/capabilities>
  - <https://docs.rs/zed_extension_api/latest/zed_extension_api> (v0.7.0)
- **Current behavior:** extensions are Git repos with an `extension.toml`,
  compiled to `wasm32-wasip2`. Documented extension features are: languages,
  themes, icon themes, snippets, debuggers, MCP servers, context servers,
  language servers.
- **External commands:** `zed_extension_api::process::Command` exists and is
  gated by the user-controlled `process:exec` capability
  (`granted_extension_capabilities` setting).
- **What it does NOT provide (verified against the current docs):**
  - No API to obtain the path of the currently active file/buffer.
  - No API to register custom command-palette actions or context menus.
  - No Project Panel extension hooks.
- **Usable for this project:** no, not for the "act on the active file"
  workflow. An extension could exec `zed-anydiff`, but it has no supported way
  to know which file the user had in focus, and it cannot add commands to the
  command palette. Building a UI on this would be an unsupported workaround.
- **Limitations:** sandboxed WASM runtime; `std::env::var` and `cfg` do not
  behave like on host; capabilities must be granted by the user.

### 1.4 Prior art — none found

- **Official extension repo:** `zed-industries/extensions` (1478 extension
  directories in `extensions/` at research time; searched the full tree for
  `diff`/`compare`/`cmp` in directory names — zero matches).
- **Zed hub API:** `https://zed.dev/api/extensions` — no extension matching
  `diff`/`compare` in name or description; the endpoint exposes no search
  parameter, so this list should be treated as partial.
- **GitHub search:** no third-party Zed extension or CLI tool implementing the
  "set base / compare with base" workflow was found.
- **Related upstream discussion:**
  <https://github.com/zed-industries/zed/discussions/32992> (2025-06-18) asks
  for a UI "Compare Selected" built on `zed --diff`; no built-in equivalent
  exists.

## 2. Answers to the research questions

| # | Question | Answer |
|---|----------|--------|
| 1 | Can an extension obtain the active file path? | **No** — no such API in the current extension docs. |
| 2 | Can an extension obtain a file selected in editor tab / project panel / file picker? | **No** — no selection-context APIs are documented. |
| 3 | Can a custom command/action receive the current file path? | **No** for extension-registered actions (extensions cannot register actions). **Yes** via *tasks*, which receive `$ZED_FILE` from the active file. |
| 4 | Do Zed tasks expose active-file / selected-file / buffer-path variables? | **Yes** — `ZED_FILE` (absolute path of the currently opened file), plus `ZED_FILENAME`, `ZED_DIRNAME`, `ZED_WORKTREE_ROOT`, `ZED_SELECTED_TEXT`, etc. No "selected file in panel" variable. |
| 5 | Can an extension launch an external command (`zed-anydiff` / `zed --diff`)? | **Yes** — `process:exec` capability + `process::Command` (but pointless here since Q1/Q2 fail). |
| 6 | Can arbitrary files outside the project be passed through? | **Yes** — `ZED_FILE` is an absolute path independent of the worktree, and `zed --diff` accepts arbitrary paths. |
| 7 | Sandbox/capability limitations? | Extensions run as WASM under a user-granted capability system (`process:exec`, `download_file`, `npm:install`); no active-file access exists to begin with. |
| 8 | Existing third-party implementation of this workflow? | **None found** (see 1.4). |

## 3. Ranking of integration approaches (by UX)

1. **Zed tasks (global `tasks.json`) calling the wrapper CLI** — clean,
   documented, no extension code, works for any open file (project or not).
   The wrapper must be on `PATH` (or referenced by absolute path in the task).
2. **Manual CLI** (`zed-anydiff base/compare ...`) — always works; the
   fallback if the user does not want task configuration.
3. **Extension with `process:exec`** — technically possible to exec the
   wrapper, but the extension cannot know the active file or add UI commands,
   so it adds risk with no benefit. Rejected.
4. **Path passed manually / Copy Path workflow** — works but is strictly worse
   than task variables. Rejected for MVP.
5. **Private/internal Zed APIs** — unsupported; explicitly rejected by
   project rules.

## 4. MVP architecture

```
+-----------------------------+
| Zed (documented surfaces)  |
|  - tasks.json tasks        |
|    "AnyDiff: Set as Base"  |   command: zed-anydiff, args: ["base", "$ZED_FILE"]
|    "AnyDiff: Compare with Base"  command: zed-anydiff, args: ["compare", "$ZED_FILE"]
|    "AnyDiff: Show Base" / "AnyDiff: Clear Base"
+--------------+--------------+
               |  (spawned in Zed's integrated terminal)
               v
+-----------------------------+
| zed-anydiff (Rust binary)  |
|  - src/state.rs: state I/O |
|  - src/cli.rs: parse+dispatch
|  - src/zed.rs: build+spawn
+--------------+--------------+
               |  std::process::Command, no shell
               v
+-----------------------------+
| zed --diff BASE TARGET      |  (Zed native diff view)
+-----------------------------+
```

Decisions:

- **State:** one JSON file `{"base": "<absolute path>"}` in the OS cache dir
  via the `dirs` crate (`%LOCALAPPDATA%\zed-anydiff\state.json` on Windows,
  `$XDG_CACHE_HOME/zed-anydiff/state.json` on Linux, `~/Library/Caches/zed-anydiff/state.json`
  on macOS). Single base; compare leaves the base intact; `clear` removes it.
- **Process execution:** `Command::new("zed").arg("--diff").arg(base).arg(target)`
  — argument arrays, never shell concatenation; spaces and Unicode-safe.
- **Errors:** missing base file, missing target file, missing base state,
  `zed` not in PATH — all reported on stderr with exit code 1.
- **Zed layer:** a `zed/tasks.json` shipped in the repo with the four tasks
  (documented in README); users copy it into `~/.config/zed/tasks.json`.
  No extension is built for the MVP.

## 5. Verified limitations (carried into README)

- Task variables exist only while a file-backed buffer is active; unsaved
  buffers cannot be used as base/target via tasks.
- Tasks run in Zed's integrated terminal; their output appears in the
  terminal pane (expected behavior).
- No project-panel context menu or "compare selected file" UI exists today
  (upstream request #32992); the wrapper covers the same need via tasks.
- The diff renderer is Zed's native `--diff` view; `zed-anydiff` renders
  nothing itself.
