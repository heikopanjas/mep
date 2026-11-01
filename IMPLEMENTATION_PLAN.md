# MEP — Implementation Plan

Handoff document for the coding agent. Read it completely before writing code. Everything under "Decisions" is settled; do not reopen it without asking the owner.

---

## 0. Ground rules

1. **Clean room.** Never use, search for, or read the leaked Windows NT source code of MEP, and do not read the source of other MEP/Z clones. The behavior comes only from the manuals listed in §2 and from observing a real MEP binary.
2. **The manual is the spec.** For every command and switch, the Microsoft Editor User's Guide (1988) is authoritative. Where this plan and the manual disagree, stop and ask.
3. **Spec first, then tests, then code.** Before implementing a command, write its behavior per argument kind into `docs/spec/commands/<name>.md` (from the manual), then write tests from that spec, then implement.
4. **ORACLE items.** Some behaviors are ambiguous or contradictory in the manual (§11). Implement the manual's reading, mark the code and its test with `// ORACLE: <id>`. Tests stay active (§9.5). The owner will confirm against a real binary.
5. **No panics on user input.** Every command returns `bool` (the original `flagType` TRUE/FALSE) because macros branch on it. Failure = `false` + message on the dialog line.
6. **Dependencies.** Only the crates named in §4.3. Any additional crate (runtime or dev) needs a one-line justification in the PR/commit message and the owner's OK.
7. Small commits, each building and passing `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and the coverage gate (§9.1) on Windows, macOS and Linux.
8. **100% test coverage is a hard acceptance criterion** (§9). No task is done and no commit is acceptable while coverage is below 100%. Code that cannot be covered by a test is redesigned until it can, not excluded. The only exclusions are the OS glue files fixed in §9.1 and test code itself.

---

## 1. Goal

A faithful, private reimplementation of Microsoft's **MEP** text editor (M/MEP, 1988, version 1.01 command set) for modern Windows, macOS and Linux terminals, written in Rust.

The motivation: an existing Windows 11 port of MEP has no access to the system clipboard, which makes it useless for daily work. **System clipboard integration is the reason this project exists** and must work in the first milestone.

Non-goals for now: syntax highlighting, GUI, mouse support, Z editor behavior, the original support programs (UNDEL/RM/EXP, MEGREP, CALLTREE), DOS-era switches that make no sense today.

---

## 2. Sources

All in `docs/sources/` (see `docs/sources/README.md` for conventions):

| File | Role |
|---|---|
| `microsoft-editor-users-guide-1988.md` — Microsoft Editor User's Guide, Feb 1988 (M for DOS / MEP for OS/2), Doc. No. 614510001-100-R00-0288 | **Primary spec.** Commands, arguments, switches, TOOLS.INI, macros, default keys (appendix tables), regex. |
| `msj-1988-notenboom-customizing-m-editor.md` — Notenboom, "Customizing the Features of the M Editor Using Macros and C Extensions", MSJ Sep 1988 | Macro semantics in practice, pseudo-files, extension ABI (cmdTable, swiTable, ARG, callback list), the worked `filter` extension. |
| `z-editor-users-guide-1986.txt` (cleaned) and `z-editor-users-guide-1986-raw.txt` — User's Guide to the Z Editor v7.43, 1986 | Secondary only: clarifies shared concepts (arg model, regex, macros) where the M guide is thin. Z-only behavior is **not** implemented. |
| `z-vs-mep.md` | Divergence catalog and spec traps. MEP wins every conflict. |
| `mep-z-brief.md` | Decision record this plan is built from. |

---

## 3. Decisions (settled)

### 3.1 Identity and behavior
- Name **MEP**; executable `mep` / `mep.exe`. Other names only via user-created links.
- MEP 1988 behavior only. No Z personality, no alternate keymaps (Quick/WordStar, BRIEF, Epsilon) in the first release.
- **TOOLS.INI section = executable name**, as in the original: running as `mep` reads `[mep]`; a link named `mz` reads `[mz]`. Section names match case-insensitively. Multi-name tags (`[m mep]`) and tagged variants (§6.12) as in the original.
- **Selection model** exactly as the original (§6.5). No global box/stream mode.
- **Screen**: no separate command line. Bottom two rows fixed: dialog line, status line. Editing area = terminal size, adapts live on resize. `height`/`width` switches only take effect when set explicitly in TOOLS.INI.
- **Scrolling**: jump scrolling, defaults `vscroll:7`, `hscroll:10`, `hike:4`.
- **Backups**: `backup:` switch with `none` (default) and `bak`. No `undel` mode.
- No support programs. Cross-file search later via `arg arg "grep -n …" compile` (M4).

### 3.2 Encoding
- Buffer stores **UTF-8 bytes unchanged**. Invalid bytes are displayed as a placeholder (one cell, e.g. `�` in the "error" color) and written back **byte-exact**.
- A **column is a display cell**. Box edges, `hscroll`, tabs, cursor column and status position count cells. Wide characters take two cells (`unicode-width`). Zero-width characters attach to the preceding character.
- A box edge that splits a wide character: the character belongs to the box if its **first** cell is inside. Paste fills a resulting half-cell gap with a space.
- No UTF-16, no code pages in the first release.

### 3.3 Clipboard
- `<clipboard>` is a pseudo-file (§6.9) **mapped to the system clipboard**.
- The clipboard carries a **shape**: line, box or stream.
- **Copy** writes two representations at once:
  - plain UTF-8 text for other applications (box: one row per line, trailing padding trimmed, rows joined with the platform newline);
  - a **private format** (custom clipboard type, e.g. `com.panjas.mep.shape` / registered format name `MEP Clipboard Shape`) containing: format version, shape, box width in cells, and a 64-bit hash of the plain text exactly as written.
- **Paste**: if the private format is present **and** its hash matches the current plain text, use its shape (and box width). Otherwise the text came from another app: **ends with a newline → line paste, otherwise → stream paste.**
- **OSC 52** (remote sessions, M3): copy sends plain text only; paste uses the internal clipboard and the newline rule.
- Opening `<clipboard>` with `setfile` shows the **current** system clipboard contents.

### 3.4 Implementation
- **Rust**, nightly toolchain pinned to a dated version in `rust-toolchain.toml` (currently `nightly-2026-10-31`), edition 2024. Cargo workspace (§4). The pin moves only on purpose, in its own commit. Nightly is required by `.rustfmt.toml`.
- **License**: MIT (`LICENSE`).
- Own **byte-based gap buffer** (not `ropey`, which requires valid UTF-8).
- `crossterm` for terminal I/O on all platforms; `clipboard-rs` for the system clipboard; `libloading` for extensions (M4).
- Extensions use a **C ABI** (M4) so they can be written in C like the originals.
- Child processes and background jobs via `std::process`, threads and channels (M4).

### 3.5 Concurrency and large documents

**Goal: the editor never slows down or freezes on large documents.** Reference document: 100 MB, 2 million lines.

- **Deterministic core, concurrent edges.** `mep-core` stays a single-threaded state machine: commands run one after another, so undo, macros, the arg model and scenario tests stay deterministic. Everything that waits runs on worker threads around it.
- **Threads and channels, no async runtime.** Workers are `std::thread` with `std::sync::mpsc` channels. The editor has a handful of long-lived workers, which threads handle well; an async runtime (tokio) adds a large dependency without making large documents faster.
- **Workers:** terminal input (`mep-term`), rendering (frame diff and terminal writes, `mep-term`), file I/O (load, save, backup), system clipboard (`mep-clip`; X11 reads can block), background jobs (M4).
- **The core never blocks on I/O.** A command that needs I/O (`setfile` loading a file, saving, `refresh`, `paste` reading the system clipboard) emits a **request** and **suspends**. While suspended, the editor keeps rendering, shows progress on the dialog line, queues typed keys (type-ahead) and lets Esc cancel. The worker's result arrives as a **completion event**; the command then finishes and returns its `bool` as usual. The macro interpreter is resumable (explicit state, not native recursion), so a macro waits at that step and continues with the command's result.
- **Long-running core work runs in bounded slices**, e.g. a search scans a fixed number of bytes per step. Between steps the editor handles events, so input stays responsive and Esc cancels. The slice size is a constant; tests use a small one.
- **Size rule: the cost of an edit depends on the size of the edit, not the size of the file.**
  - edits at the cursor are gap buffer operations (§4.4 `gap`);
  - the line index is updated incrementally, never rebuilt per edit (§4.4 `lines`);
  - rendering reads only the visible lines; the status line's line count is O(1);
  - undo records deltas (position, removed bytes, inserted bytes), never copies of the buffer;
  - whole-file passes happen only on load, save, search and explicit whole-file commands.
- **Saving** hands an owned copy of the bytes to the I/O worker (one copy in memory, no disk I/O on the core thread).

---

## 4. Architecture

### 4.1 Workspace layout

```
mep/
├── Cargo.toml              # workspace
├── rust-toolchain.toml     # channel = "nightly-YYYY-MM-DD", components = ["clippy", "llvm-tools-preview", "rustfmt"]
├── scripts/check.sh        # fmt + clippy + test + coverage gate (Linux, macOS)
├── scripts/check.ps1       # the same for Windows
├── crates/
│   ├── mep-core/           # all editor logic; no terminal, no OS clipboard
│   ├── mep-term/           # crossterm front end: Terminal trait impl, key decoding
│   ├── mep-clip/           # SystemClipboard trait impls (clipboard-rs; OSC 52 in M3)
│   ├── mep-ext/            # C extensions (M4): ABI structs, callbacks, libloading; c/ext.h, c/exthdr.c, c/filter.c
│   └── mep/                # binary: CLI parsing, wiring, event loop, shell host, job thread
├── docs/
│   ├── sources/            # manuals (provided)
│   ├── spec/commands/      # one file per command, transcribed from the manual
│   ├── spec/switches.md
│   └── oracle.md           # ORACLE items and their confirmed results
└── tests/scenarios/        # scenario test files (§9.3)
```

`mep-core` has `#![forbid(unsafe_code)]`. Unsafe code is only allowed in the extension crate `mep-ext` (M4) and in the excluded glue file `mep/src/sys.rs` (the Windows console control handler).

### 4.2 Core boundaries (traits)

```rust
// mep-core
pub trait Terminal {
    fn size(&self) -> (u16, u16);                 // cols, rows
    fn draw(&mut self, frame: &Frame) -> io::Result<()>;   // full frame; impl diffs
}

pub trait SystemClipboard {
    fn read(&mut self) -> Option<ClipboardPayload>;        // text + optional private record
    fn write(&mut self, payload: &ClipboardPayload) -> bool;
}

pub trait Fs {                                    // all file system access of the core
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write_atomic(&self, path: &Path, data: &[u8]) -> io::Result<()>; // temp + rename
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn metadata(&self, path: &Path) -> io::Result<FileMeta>;      // exists, readonly
    fn read_dir_names(&self, dir: &Path) -> io::Result<Vec<OsString>>; // case-insensitive TOOLS.INI lookup
}

pub trait Env {                                   // environment and clock
    fn var(&self, name: &str) -> Option<OsString>;
    fn now(&self) -> LocalDateTime;
    fn os(&self) -> Os;                           // Windows / MacOs / Linux
}

pub enum Event { Key(Key), Resize(u16, u16), Completed(Completion) } // Completion: a worker's result for a request (§3.5)
```

The editor is a state machine: `Editor::handle(event) -> Outcome` (`Continue` / `Quit`, plus the requests for workers that the event produced), then `Editor::render(&mut Frame)`. The binary owns the loop: receive the next event from the input thread or a worker → `handle` → dispatch requests to workers → `render` → hand the frame to the render thread (§3.5). The `Fs` and `SystemClipboard` traits are called by the workers, not by the core thread. Tests use an in-memory clipboard, an in-memory `Fs` with injectable failures (so every I/O error path is testable), a fixed `Env`, and render into a `Frame` without any terminal. `mep-core` never calls `std::fs`, `std::env` or the system clock directly.

### 4.3 Allowed crates

| Crate | Where | Why |
|---|---|---|
| `crossterm` | mep-term | Windows console + VT, resize events, keyboard enhancement (kitty protocol) |
| `clipboard-rs` | mep-clip | System clipboard incl. custom formats on Windows, macOS, X11 (Wayland optional) |
| `unicode-width` | mep-core | Cell widths |
| `libloading` | mep-ext (M4) | Loading native extensions |
| `signal-hook` | mep, Unix only (M4) | Ignoring SIGINT/SIGQUIT/SIGTSTP in the editor while `shell` runs a program (`mep/src/sys.rs`) |
| `windows-sys` | mep, Windows only (M4) | `SetConsoleCtrlHandler` for the same on Windows (`mep/src/sys.rs`) |
| `regex` | M2, mep-core | Regular expressions in the crate's own syntax (§10) |
| `typed-path` | mep-core | Unix and Windows paths chosen by `Env::os()` instead of the host, so path logic and tests behave the same on every platform; held at 0.12.3 (0.12.4 and 0.13.0 are younger than two weeks) |
| `tempfile` (dev) | tests | Temp dirs for file I/O tests |
| `proptest` (dev) | mep-core tests | Property tests for gap buffer and cell mapping |
| `anyhow` | any crate | Error handling for fallible operations outside the command interface (commands still return `bool`, §0 rule 5) |
| `clap` | mep | Command-line parsing (§6.1) |

Hash for the clipboard record: implement FNV-1a 64 yourself (a few lines) rather than adding a crate.

Command-line parsing uses `clap` (§6.1).

Tool (not a crate dependency): `cargo-llvm-cov` for coverage measurement and the gate (§9.1).

### 4.4 Core modules (`mep-core`)

| Module | Responsibility |
|---|---|
| `gap` | `GapBuffer`: byte storage, insert/delete at byte offset, slice access. |
| `lines` | Line index over the gap buffer (line start offsets), line terminator per line; updated incrementally on edits (§3.5). |
| `cells` | UTF-8 decoding with invalid-byte handling, cell widths, cell column ↔ byte offset, tab expansion, placeholder rendering. |
| `file` | `File` (one open file or pseudo-file), `FileTable` with `FileId` handles, load/save, line-ending detection, modified/readonly flags, backup. |
| `view` | Window state: top line, left column, cursor (line, cell col); scrolling rules. |
| `arg` | Arg accumulation state, anchor, text arg editing, classification, numarg/markarg conversion. |
| `cmd` | `CmdDesc` table (name, fn, flags), dispatcher. |
| `commands/*` | Implementations, grouped: `movement`, `edit`, `clipboard`, `search`, `files`, `misc`. |
| `keys` | `Key` type, key-name parser/printer (`alt+x`, `ctrl+g`, `shift+f2`, `num+`…), keymap (key → function name), default MEP bindings. |
| `switches` | Switch table (name, type, default, setter), validation, messages. |
| `ini` | TOOLS.INI locating and parsing, section selection, tags. |
| `macros` | Macro definitions, parser, interpreter (labels, jumps, return values). |
| `undo` | Per-file undo records as deltas (§3.5), `undocount`. |
| `clip` | Clipboard model: shape + content, conversion to/from `ClipboardPayload`, the paste-shape rules. |
| `render` | Builds the `Frame`: text area, dialog line, status line, colors. |
| `editor` | Top-level `Editor`: file table, current file, view, arg state, keymap, macros, switches, message line. |

---

## 5. Data model details

### 5.1 Lines and terminators
- The file's bytes go into the gap buffer **as they are**, including line terminators.
- A line ends at `\n`. A `\r` directly before `\n` is part of the terminator, not of the line content.
- Each file has a **default terminator**: CRLF if any line ends in CRLF, else LF. Files with only LF endings show **"NL"** in the status line. New lines use the default terminator. Unedited lines keep their exact bytes, so mixed-ending files round-trip unchanged.
- A missing terminator on the last line is preserved.

### 5.2 Tabs (ORACLE: TAB-1)
- Tab characters in the file are displayed expanded to the next multiple of 8 cells. `tabdisp` (default 32 = space) is the character used to draw the expansion.
- When a line containing tabs is **edited**, it is converted to its expanded form (spaces) for editing.
- On save, **edited** lines are re-tabbed according to `entab` (0 = no tabs, 1 = convert leading/inner runs of spaces outside quotes to tabs, 2 = everywhere; confirm the exact meaning of each value in the manual). Unedited lines are written byte-exact.
- `tabstops` (default 4) controls the `tab`/`backtab` cursor movement, not file tab width.
- **Project addition** (`usetabs`, owner's request 2026-11-01, not in the manual): with `usetabs` on, the file tab width is `tabstops` instead of 8, for display and for re-tabbing, and the leading blanks of edited lines are always saved as tabs, whatever `entab` says. Inner runs still follow `entab`.
- A line counts as edited from loading on, not just since the last save: the buffer keeps the unconverted form (spaces, trailing blanks), so every save must convert it again. Clearing the flags after a save made a second save write the unconverted form.

### 5.3 Trailing spaces
- Unless `trailspace` is on, trailing spaces are removed from a line **when that line is edited**. Unedited lines are never touched.
- `traildisp` (default 0 = off) makes trailing spaces visible with the given character.

### 5.4 Cursor and virtual space
- Cursor position = (line index, cell column). The cursor may be **beyond the end of the line** and beyond the last line; MEP allows this, and box selections depend on it.
- Inserting text beyond the end of a line pads the line with spaces first.
- The status line shows positions **1-based**; internally everything is 0-based.

### 5.5 Line length
- No line length limit (the original's 250-character limit is not reproduced).

### 5.6 Files and handles
- `FileId` is an index into `FileTable` (stable for the file's lifetime). This later becomes the extension ABI's PFILE.
- Pseudo-files: names in angle brackets (`<clipboard>`, later `<information>`, extension-created ones). They behave like files but cannot be saved without an explicit filename.
- Read-only files (no write permission) set the `readonly` state; editing them is allowed only after the user confirms per the manual's behavior.

### 5.7 Undo
- One undo step per **top-level command invocation**. A macro invoked from a key is one step (ORACLE: UNDO-1).
- `undocount` (default 10) limits the number of steps kept per file.
- Undo restores text and cursor position.

---

## 6. Behavior specification (M1 scope)

The manual is the detailed spec; this section fixes structure and the points the manual leaves to the implementer.

### 6.1 Command-line interface
```
mep [options] [file ...]

  -d, --no-tools-ini         do not read TOOLS.INI
  -e, --execute <command>    execute <command> (macro syntax) after startup
  -t, --temporary            files are temporary (not recorded in the information file; effective from M3)
  -h, --help                 print usage to stdout and exit 0
  -V, --version              print version to stdout and exit 0
  --                         end of options; all following arguments are files
```
- Short and long options work on **all** systems. `--execute=<command>` is accepted as well as `--execute <command>`.
- Options start with `-` or `--` on **all** systems, Windows included. The original `/d`, `/e`, `/t` forms are **not** accepted; an argument starting with `/` is always a file name.
- Short flags may be bundled (`-dt` = `-d -t`).
- `-e` may be given several times; the commands run in order.
- Unknown option or missing `<command>`: message and usage to stderr, exit code 2.
- The parser lives in the `mep` binary crate (`crates/mep/src/cli.rs`) and is a pure function (`args → Result<CliOptions, CliError>`, built on `clap`'s `try_parse_from`) so every case is unit-tested.
- No file: reopen the last edited file (M3; in M1 start with an empty untitled buffer).
- Several files: `exit` walks the list (§6.10).

### 6.2 Screen layout
```
row 0 .. rows-3   text window
row rows-2        dialog line   (messages; "Arg:" prompt and text-arg editing)
row rows-1        status line
```
Status line fields (up to 8, in this order; confirm format against the manual): file name, file type, `modified`, `NL` (LF-only file), length in lines, position (`displaycursor` decides cursor vs window position), `insert`, `meta`.

### 6.3 Scrolling
- Cursor leaves the window vertically → window jumps by `vscroll` lines.
- Cursor leaves horizontally → window jumps by `hscroll` columns.
- After a jump of more than one window (search, `home`, line-number moves), the cursor line is placed `hike` lines from the top.
- On terminal resize: keep the cursor line visible, clamp, redraw. `height`/`width` from TOOLS.INI override the terminal size (clamped to it).

### 6.4 Colors
Color switches take **DOS text attributes in hex** (high nibble background, low nibble foreground; e.g. `hgcolor:70` = black on light grey). Map the 16 DOS colors to the 16 ANSI colors. Switches: `fgcolor` (07), `hgcolor` (70, highlighted selection), `stacolor` (03, status), `infcolor` (06), `errcolor` (04). The values are hex digits `0`–`F` per M guide Table 7.1 (p. 58); background values `8`–`F` mean the colors `0`–`7` with blinking text — render them as the bright ANSI background instead (terminals rarely blink). Example from the manual: `errcolor:0E` = light yellow on black.

### 6.5 Arguments
- `arg` starts argument mode at the cursor (the **anchor**); further `arg` presses increase the arg count (`arg arg` = 2, used for regex and other variants).
- Typing printable characters in arg mode starts a **text argument**, edited on the dialog line. Cursor-movement commands (flag `CURSORFUNC`) instead extend a **selection**, highlighted with `hgcolor`.
- Classification when the next non-cursor function runs:

| Situation | Arg kind |
|---|---|
| `arg` then function, no movement, no text | `nullarg` |
| text typed | `textarg` (a number → `numarg` if the function accepts it; a mark name → `markarg`, M2) |
| cursor moved, same line | `streamarg` (characters on one line) |
| cursor moved, same column | `linearg` (whole lines) |
| cursor moved, other line and column | `boxarg` |
| no `arg` at all | `noarg` |

- Right edge is **exclusive**: a box/stream ends one column left of the rightmost of anchor and cursor.
- Each command declares accepted kinds in its `CmdDesc` flags: `NOARG, TEXTARG, NULLARG, LINEARG, STREAMARG, BOXARG, NUMARG, MARKARG`, plus modifiers `BOXSTR` (one-line box → textarg), `NULLEOL` / `NULLEOW` (nullarg → textarg from cursor to end of line / word), `KEEPMETA`, `CURSORFUNC`, `WINDOWFUNC`, `MODIFIES`.
- A multi-line box-shaped selection is interpreted per command: `ldelete`, `linsert`, `copy`, `replace` get a box; `sdelete` gets a stream. (Check each command's entry in the manual.)
- **Conversion in the dispatcher** before the command runs: `numarg` → `linearg` (n lines from the cursor); `markarg` → stream/box/line depending on relative position (M2). Commands never see `numarg`/`markarg`.
- A command called with an arg kind it does not accept: message "invalid argument" and return `false`.
- `cancel` clears arg mode and the selection.
- `lasttext` recalls the previous text argument for editing.
- **Project addition** (`argcol.md`, owner's request 2026-11-01, not in the manual): `argcol` starts/continues arg mode like `arg` but never raises the count, and marks the pending state so that whenever the cursor ends up on a *different line* from the anchor, the selection is always the flat two-point `Arg::Stream { start: anchor.min(cursor), end: anchor.max(cursor) }` (whichever point comes first in the file trimmed to its own column, whichever comes second trimmed up to its own column, everything strictly between whole) rather than the table's "same column → linearg" row, gated by a new `COLARG` flag. The table above is unchanged for plain `arg`.
- **Project addition** (`tab.md`/`backtab.md`, owner's request 2026-11-01, not in the manual): `tab` and `backtab` are `CURSORFUNC`, so the dispatcher never routes them through `classify()` above; instead each checks `self.arg` itself. If a selection is already pending when the key is pressed (anchor differs from the cursor), `tab` indents and `backtab` outdents every line from the anchor's line to the cursor's line — rounding each line's own leading whitespace to the next/previous `tabstops` multiple — then clears the pending arg and places the cursor at column 0 of the first line, instead of moving to the next/previous tab stop. No new flag; applies regardless of the selection's shape (plain `arg`, `argcol`, or any other movement that left anchor != cursor). With the project switch `tabinsert` on and no arg pending, `tab` inserts spaces at the cursor up to the next tab stop and `backtab` removes blanks back to the previous one when only blanks are left of the cursor.

### 6.6 Meta
`meta` toggles the meta flag and **returns the new state**. Most commands clear meta after running; commands with `KEEPMETA` leave it. The status line shows `meta` while set.

### 6.7 Command set for M1

From the M 1.01 function list (MSJ Fig. 1) plus the M text-insertion functions. Each gets a spec file in `docs/spec/commands/`.

| Group | Commands |
|---|---|
| Argument and control | `arg`, `meta`, `cancel`, `assign`, `execute`, `initialize`, `lasttext`, `unassigned`, `undo` |
| Cursor movement | `left`, `right`, `up`, `down`, `begline`, `endline`, `home`, `pword`, `mword`, `plines`, `mlines`, `ppage`, `mpage`, `ppara`, `mpara` (ORACLE S4), `begfile`, `endfile` (project additions, ORACLE CMD-BEGFILE-1), `tab`, `backtab`, `newline`, `savecur`, `restcur` |
| Window (single window) | `setwindow` |
| Editing | `graphic`, `insertmode`, `quote`, `cdelete`, `emacscdel`, `emacsnewl`, `sdelete`, `sinsert`, `ldelete`, `linsert` (MEP: inserts **above**), `pbal` |
| Clipboard | `copy`, `paste` (ORACLE S5 for `arg arg textarg paste`; the `arg arg !textarg paste` / `arg arg !streamarg paste` form runs a command and inserts its stdout at the cursor, M guide p. 102 — M4, reports "not available" in M1) |
| Search | `psearch`, `msearch` — plain strings only in M1 (`arg arg` regex form returns "not yet supported" until M2) |
| Files | `setfile`, `refresh`, `exit` |
| Text insertion | `curdate`, `curday`, `curtime`, `curuser`, `curfile`, `curfilenam`, `curfileext` |

`curuser`: use `MAILNAME` if set, else `USER` (macOS/Linux) or `USERNAME` (Windows). (Manual contradicts itself between USER and MAILNAME.)

Later milestones: `mark`, `replace`, `qreplace` (M2); `window`, `information` (M3); `compile`, `argcompile`, `shell` (M4). Until then they exist in the table and report "not available".

### 6.8 Default key bindings (MEP)

The complete default keymap is the **Default** column of Table A.2 (M guide pp. 90–92). No default uses the `cmd+` modifier, a project addition for the macOS Command key in kitty-protocol terminals (assign.md, ORACLE KEY-1). It is reproduced here; the manual wins if they ever differ. The Quick/WordStar, BRIEF and EPSILON columns are not implemented in the first release.

| Function | Keys | Function | Keys |
|---|---|---|---|
| arg | Alt+A | meta | F9 |
| cancel | Esc | assign | Alt+= |
| exit | F8 | setfile | F2 |
| initialize | Shift+F8 | refresh | Shift+F7 |
| execute | F7 | psearch / msearch | F3 / F4 |
| replace / qreplace | Ctrl+L / Ctrl+\ | copy | Ctrl+Ins, Num+ |
| paste | Shift+Ins | ldelete / linsert | Ctrl+Y / Ctrl+N |
| sdelete / sinsert | Del / Ctrl+J | cdelete | Ctrl+G |
| emacscdel | Backspace | emacsnewl | Enter |
| insertmode | Ins, Ctrl+V | begline / endline | Home / End |
| home | Ctrl+Home | mark | Ctrl+M |
| ppage / mpage | PgDn, Ctrl+C / PgUp, Ctrl+R | plines / mlines | Ctrl+Z / Ctrl+W |
| ppara / mpara | Ctrl+PgDn / Ctrl+PgUp | pword / mword | Ctrl+Right, Ctrl+F / Ctrl+Left, Ctrl+A |
| pbal | Ctrl+[ | setwindow | Ctrl+] |
| quote | Ctrl+P | lasttext | Ctrl+O |
| undo | Alt+Backspace | information / help | Shift+F1 / F1 |
| compile / argcompile | Shift+F3 / F5 | shell | Shift+F9 |
| window | F6 | left / right | Left, Ctrl+S / Right, Ctrl+D |
| up / down | Up, Ctrl+E / Down, Ctrl+X | tab / backtab | Tab / Shift+Tab |
| graphic | all printable keys | | |

No default key: `curdate`, `curday`, `curfile`, `curfileext`, `curfilenam`, `curtime`, `curuser`, `newline`, `savecur`, `restcur`. Everything else: `unassigned`.

The WordStar-style control keys are part of the default map: Ctrl+E/X/S/D move the cursor, Ctrl+A/F move by word, Ctrl+R/C by page. Ctrl+S and Ctrl+Q are XON/XOFF flow control in many terminals and must be freed in raw mode (§7).

### 6.9 Clipboard behavior
- `copy` with an arg copies the selection with its shape; `noarg` copies the current line (line shape) — confirm in the manual.
- Block deletes (`ldelete`, `sdelete` with an arg, box delete) also put the deleted text into the clipboard, as in the original.
- `paste` inserts according to the shape: line → whole lines above/below per the manual; box → as a box at the cursor (pad with spaces as needed); stream → at the cursor.
- Every write to the clipboard goes to the system clipboard via `SystemClipboard::write` (§3.3). Every paste first reads the system clipboard.
- If the system clipboard is unavailable (no display, error), fall back to the internal clipboard silently and show a one-time message.
- `<clipboard>` pseudo-file: opening it fills it with the current clipboard text; it can be edited and saved under a real name.

### 6.10 Files and exit (MEP semantics, divergence S2)
- `setfile` variants (from the manual; key ones): `setfile` (noarg) switches to the previous file; `arg "name" setfile` opens/switches to a file; `arg arg setfile` saves the current file; `arg arg "name" setfile` saves under that name. Confirm every variant and the `meta` forms in the manual.
- `autosave` (default on): switching away from a modified file saves it.
- `exit`: saves the current file if modified and **advances to the next file** from the command line; at the last file it quits. `arg exit` quits without advancing. `meta exit` quits without saving. `askexit` (default off) asks before quitting.
- `refresh` rereads the file from disk (with confirmation if modified) or discards it (`arg refresh`) — confirm in the manual.
- Saving: write to a temp file in the same directory and rename over the original; with `backup:bak`, the previous version is first renamed to `name.bak` (replacing the extension, as the original did — confirm).

### 6.11 Search (M1: plain strings)
- `arg "text" psearch` searches forward for the literal text; `psearch` without arg repeats the last search; `msearch` searches backward.
- `case` switch (default off = case-insensitive).
- Search wraps per the manual (confirm). Found text is highlighted; cursor moves to the match, scrolling with `hike`.

### 6.12 TOOLS.INI
- **Location** (first found wins): directories listed in the `INIT` environment variable (`;` on Windows, `:` elsewhere) → `%APPDATA%\mep\` (Windows) / `~/.config/mep/` (macOS, Linux; honor `XDG_CONFIG_HOME`) → home directory. File name `TOOLS.INI`, matched case-insensitively on case-sensitive filesystems (`tools.ini` also accepted).
- **Sections**: `[name]`, several names per tag (`[m mep]`), case-insensitive. The editor reads the section(s) matching its executable name. Tagged variants: implement `[mep-<os>]` with `<os>` = `windows`, `macos`, `linux`, read after the plain section. DOS/OS/2 tags from the original (`[M-3.20]`, `[M-10.0]`, display tags) are ignored.
- **Lines**: `function:key` (assignment), `name:=sequence` (macro), `switch:value`. Comments start with `;`. Continuation lines per the manual (confirm whether a trailing `\` is supported).
- Errors in TOOLS.INI are collected and shown on the dialog line after startup; they never abort.
- `initialize` rereads the editor sections.
- `assign`: `arg "text" assign` applies one TOOLS.INI-style line at runtime.

### 6.13 Switches for M1

| Switch | Default | M1 |
|---|---|---|
| `undocount` | 10 | yes |
| `displaycursor` | off (status shows window position; on = cursor position) | yes |
| `tabstops` | 4 | yes |
| `tabdisp` | 32 | yes |
| `entab` | 1 | yes (ORACLE TAB-1) |
| `traildisp` | 0 | yes |
| `trailspace` | off | yes |
| `case` | off | yes |
| `enterinsmode` | off | yes |
| `softcr` | on | yes (auto-indent on `emacsnewl`/`newline` per manual) |
| `wordwrap` / `rmargin` | off / 72 | yes |
| `autosave` | on | yes |
| `askexit` | off | yes |
| `backup` | none | yes (`none`, `bak`) |
| `readonly` | none (text switch: command run before overwriting a read-only file, file name appended; without it the user is asked for a new name) | yes |
| `vscroll` / `hscroll` / `hike` | 7 / 10 / 4 | yes |
| `height` / `width` | terminal | yes, only if set |
| `fgcolor` `hgcolor` `stacolor` `infcolor` `errcolor` | 7 / 70 / 3 / 6 / 4 | yes |
| `noise` | 50 | accept, use for search progress later |
| `tmpsav`, `markfile`, `maxmsg`, `extmake`, `askrtn`, `load` | — | accept and store; used in M2–M4 |
| `vmbuf`, `shortnames`, `savescreen` | — | accept silently, no effect (DOS-era) |

Unknown switch → error message, no abort.

### 6.14 Macros
- Definition: `name:=item item …`, one per line. Items: function or macro names, quoted strings, flow control.
- A quoted string is entered as if typed: in arg mode it becomes text argument input; otherwise each character goes through `graphic`.
- Flow control: `:>label` defines a label; `=>label` jumps; `+>label` jumps if the previous function returned true; `->label` if false. A bare `=>`, `+>` or `->` exits the macro (with the condition).
- A pending argument is passed to the first function in the macro that accepts arguments. A macro may end with an **open arg**, consumed by whatever runs next (MSJ `selectall` example — must work).
- The screen is **not redrawn** during macro execution until input is required.
- The macro's return value is the value of the last function executed (ORACLE: MACRO-1).
- Macros can call macros. Recursion depth limit 256 → error message.
- Limit: 1024 macros (as the original), configurable constant.
- `execute`: `arg "item item …" execute` runs the text as a macro sequence. This is also the `fExecute` callback for extensions (M4).

---

## 7. Front end (`mep-term`, `mep`)

- Enter raw mode and the alternate screen; restore on exit **and on panic** (install a panic hook).
- Raw mode must stop the terminal from turning Ctrl+C, Ctrl+Z, Ctrl+\, Ctrl+S, Ctrl+Q into signals or flow control: MEP binds several of these. Verify on macOS and Linux.
- Request keyboard enhancement (kitty protocol: disambiguate escape codes, report alternate keys) when supported. Without it, document the collisions: Ctrl+M = Enter, Ctrl+[ = Esc, Ctrl+H = Backspace, Ctrl+I = Tab; Alt+key arrives as an Esc prefix. Check how crossterm reports Ctrl+J vs Enter in legacy mode.
- Lone Esc vs. Esc prefix: rely on crossterm's parsing; verify that a single Esc (cancel) reacts without noticeable delay.
- Key mapping: convert crossterm `KeyEvent` to `mep_core::keys::Key`. Keypad `+`/`-` (`num+`, `num-`) only when the terminal reports them distinctly.
- Rendering: the core produces a full `Frame`; `mep-term` diffs it against the previous frame and writes only changes. Cursor shape: block in overtype, bar in insert mode (if supported); the project switch `cursorsize` (0 underline, 1 block) sets one shape for both modes (docs/spec/switches.md, ORACLE CURSOR-1).
- Resize events → `Event::Resize`.
- Threads (§3.5): an input thread runs the blocking `crossterm::event::read()` loop and sends events into the main channel; a render thread receives frames, drops all but the newest when it falls behind, diffs and writes. I/O and clipboard workers receive requests and send completions into the same channel.
- The binary wires `mep-term` + `mep-clip` (`clipboard-rs` adapter; fallback to in-memory if construction fails) into the core.

**Testability split (required for the coverage gate):**
- `mep-term` is split into pure, tested code and a minimal OS glue file `mep-term/src/sys.rs`:
  - pure: crossterm `KeyEvent` → `Key` conversion, frame diffing, writing the diff as crossterm commands into any `impl Write` (tests write into a `Vec<u8>` and compare the bytes), DOS attribute → ANSI color mapping, cursor-shape selection;
  - `sys.rs`: only enabling/disabling raw mode, alternate screen, keyboard enhancement, panic hook installation, `crossterm::event::read()`, `terminal::size()`. No logic, no branches beyond forwarding errors.
- `mep-clip` is split the same way: payload ↔ clipboard-rs content encoding is pure and tested; `mep-clip/src/system.rs` only constructs the `clipboard-rs` context and calls get/set.
- `mep/src/main.rs` only collects `std::env::args_os()`, builds the real `Fs`/`Env`/`Terminal`/`SystemClipboard` and calls a library function `run(...)` in `mep/src/lib.rs`, which is tested with fakes.
- Real `Fs` and `Env` implementations live in `mep/src/os.rs` (thin wrappers around `std::fs`, `std::env`, the clock).
- Worker loops are plain functions over channels and trait objects, so tests drive them with fake `Fs`, `SystemClipboard` and `impl Write` and no real terminal or OS clipboard.

---

## 8. Milestones and tasks

### M1 — Usable daily editor (one window, several files)

Order matters. Every task's "done when" **implicitly includes**: tests green on all three platforms and the coverage gate (§9.1) at 100%.

| # | Task | Done when |
|---|---|---|
| 1 | **Workspace setup**: crates, toolchain file (incl. `llvm-tools-preview`), fmt/clippy config, `docs/` structure, `scripts/check.sh` and `scripts/check.ps1` running fmt, clippy, tests and the coverage gate; CI running the same script on Windows, macOS and Linux. | Empty crates build; the coverage gate runs and passes on all three platforms. |
| 2 | **Gap buffer** (`gap`). | Unit + property tests: random insert/delete sequences match a `Vec<u8>` model. |
| 3 | **Line index and terminators** (`lines`, §5.1), updated incrementally on edits (§3.5). | Load→save round-trip byte-exact for LF, CRLF, mixed, no final newline, empty file, invalid UTF-8; property test: index after random edits equals a full rebuild. |
| 4 | **Cell model** (`cells`, §3.2, §5.2). | Tests for ASCII, wide CJK, combining marks, invalid bytes, tabs: cell↔byte mapping both directions, box edge splitting a wide char. |
| 5 | **File table and file I/O** (`file`) on top of the `Fs` trait: load, save via `write_atomic`, `backup:bak`, readonly detection, untitled buffer. Load and save are requests with completion events (§3.5). Real `Fs` in `mep/src/os.rs`. | Core tests with the in-memory `Fs`, including every injected failure (read error, write error, rename error, readonly); real `Fs` tested against `tempfile` directories. |
| 6 | **Keys and default keymap** (`keys`, §6.8): key-name parser/printer. | Round-trip of every name in the default table; parser rejects garbage with a message. |
| 7 | **Switch table** (`switches`, §6.13). | Every switch parses its type; unknown switch → error. |
| 8 | **View, render, status/dialog lines** (`view`, `render`, §6.2–6.4). Rendering reads only visible lines (§3.5). | Golden frame tests (text snapshots of `Frame`) for scrolling, hike, resize, status fields. |
| 9 | **Command table, dispatcher, arg model, meta** (`cmd`, `arg`, §6.5–6.6). | Scenario tests: every arg kind classified correctly; numarg→linearg; invalid-arg message; selection highlight in frame. |
| 10 | **Movement commands**. | Scenario tests per command from its spec file. |
| 11 | **Editing commands** incl. insert/overtype, virtual space padding, trailing-space rule, `softcr`, `wordwrap`. | Scenario tests per command. |
| 12 | **Undo** (§5.7), storing deltas (§3.5). | Undo/redo of every editing command restores bytes and cursor; `undocount` limit. |
| 13 | **Clipboard core** (`clip`, §3.3, §6.9): shapes, payload with private record, FNV-1a hash, paste rules, block-delete-to-clipboard, `<clipboard>` pseudo-file. Clipboard reads and writes are requests with completion events (§3.5). In-memory `SystemClipboard` for tests. | Scenario tests: box/line/stream copy-paste; external text classified by trailing newline; stale private record (hash mismatch) ignored. |
| 14 | **`mep-clip`**: `clipboard-rs` adapter writing text + custom format in one call; reading both. | Manual test on Windows and macOS: copy a box in MEP, paste into another app (plain text), copy it back, paste in MEP as a box; copy text in another app, paste in MEP as stream/line. |
| 15 | **Plain search** (§6.11), running in bounded slices (§3.5). | Scenario tests: forward, backward, repeat, case switch, not-found message, Esc cancels a running search. |
| 16 | **TOOLS.INI** (`ini`, §6.12) + `assign` + `initialize`. | Tests: location order, section by executable name, `[m mep]`, `[mep-<os>]`, comments, errors collected. |
| 17 | **Macros** (`macros`, §6.14) + `execute` + `lasttext`. Resumable interpreter (§3.5). | Tests reproduce the MSJ examples (`savefile`, `print` structure with `shell` stubbed, `bigprint`/`selectall` open-arg chain), jumps, labels, recursion limit, a macro suspended at a file load continuing with the command's result. |
| 18 | **Multiple files** (§6.10): file list from CLI, `setfile` variants, `autosave`, `exit` walking the list, `refresh`, `askexit`. | Scenario tests. |
| 19 | **Text insertion functions** (`curdate` …). | Tests with injected clock/env. |
| 20 | **Front end** (§7) and binary wiring, CLI (§6.1), input/render/I/O/clipboard threads (§3.5). | Key conversion, frame diff and color mapping fully unit-tested; worker loops tested with fakes; `run(...)` tested with fakes; every CLI case tested (each option short/long form, a `/`-prefixed argument treated as a file, `--execute=`, repeated `-e`, `--`, bundled short flags, unknown option, missing argument, help, version). Manual checklist below passes on Windows Terminal, macOS Terminal/iTerm2/Ghostty, a Linux terminal. |

**M1 acceptance:**
- The check scripts pass on Windows, macOS and Linux, including the coverage gate at 100% lines, functions, regions and branches.
- Every command spec file in `docs/spec/commands/` has scenario tests for every accepted arg kind, the `meta` form, and at least one rejected arg kind.
- Every switch in §6.13 has tests for its default, a valid value, and an invalid value.
- Every key name in the default table round-trips through parser and printer.
- The performance tests (§9.6) meet their budgets on the reference document.

**M1 acceptance checklist (manual, owner):**
- Open, edit, save a C++ and a Rust file; CRLF file stays CRLF; unedited lines unchanged (`git diff` shows only edited lines).
- Box copy → paste into a browser text field; text copied in a browser → paste in MEP.
- All six arg kinds and `meta` behave per manual on Windows and macOS.
- A TOOLS.INI with key assignments and a macro using `+>`/`->` works.
- Resize the terminal while editing; nothing breaks.
- Ctrl+C / Ctrl+Z / Ctrl+\ do not kill or suspend the editor.

### M2 — Regex, replace, marks
- Regular expressions with the `regex` crate (§10); MEP's original dialect is not implemented.
- `arg arg` forms of `psearch`/`msearch`.
- `replace`, `qreplace`; with `arg arg`, regex patterns and `$0`, `$1`…, `${name}` in the replacement.
- `mark` and `markarg` (conversion in the dispatcher), `markfile` switch (persisted marks).

### M3 — Windows and sessions
- `window`: up to 8 tiled windows, minimum 5 rows × 10 columns; horizontal/vertical split (ORACLE S10 for naming).
- Information file (`<information>` pseudo-file, file history), reopen last file on start, `tmpsav` session restore, `-t` flag.
- Remaining clipboard adapters: Wayland (via `clipboard-rs` feature), X11, OSC 52 for remote sessions.

### M4 — Tools and extensions
- `arg arg !textarg paste` / `arg arg !streamarg paste`: run the text as a command and insert its stdout at the initial cursor position (M guide p. 102).
- `shell`, `compile`, `argcompile` via a background job queue (threads + channels); `askrtn`; compiler-output parsing and error browsing (`compile` without arg steps to the next error; `meta compile`, `arg meta compile`, `maxmsg`).
- `extmake` switch.
- Extension ABI crate: `#[repr(C)]` `ARG`, `cmdDesc`, `swiDesc`; required exports `cmdTable`, `swiTable`, `WhenLoaded`; the callback set from MSJ Fig. 7 (`AddFile`, `BadArg`, `CopyBox`, `CopyLine`, `CopyStream`, `DelBox`, `DelFile`, `DelLine`, `DelStream`, `DoMessage`, `Display`, `fExecute`, `FileLength`, `FileNameToHandle`, `FileRead`, `FileWrite`, `GetCursor`, `GetLine`, `KbHook`, `KbUnHook`, `MoveCur`, `pFileToTop`, `PutLine`, `RemoveFile`, `ReadChar`, `ReadCmd`, `Replace`, `SetKey`) as `extern "C"`. PFILE = opaque handle wrapping `FileId`. `load:` switch loads via `libloading`. Acceptance: the MSJ `filter` extension, ported to modern C with a provided `ext.h`, compiles and works (sorting a line range through `sort`).

---

## 9. Testing strategy

### 9.1 Coverage gate (hard acceptance criterion)

The gate is part of the check scripts (`scripts/check.sh`, `scripts/check.ps1`) and CI and runs on **each** platform separately (platform-specific `cfg` code is covered on its own platform):

```
cargo llvm-cov --workspace --branch --no-report
cargo llvm-cov report --branch \
  --ignore-filename-regex '(mep-term[/\\]src[/\\]sys\.rs|mep-clip[/\\]src[/\\]system\.rs|mep[/\\]src[/\\]main\.rs)$' \
  --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
# plus: zero uncovered branches in `cargo llvm-cov report --branch --json --summary-only`
# (cargo-llvm-cov has no --fail-under-branches)
```

Rules:
- **Exclusions are exactly these four files** (§7): `mep-term/src/sys.rs`, `mep-clip/src/system.rs`, `mep/src/main.rs` and `mep/src/sys.rs` (signal handling while `shell` runs, M4). They contain only calls into the OS, no logic. Adding a file to the list requires the owner's OK. The Linux clipboard path in `system.rs` is additionally exercised by an `#[ignore]` integration test run manually.
- 100% **lines, functions, regions and branches**. Branch coverage needs nightly (`--branch`); the toolchain is pinned to a nightly (§3.4).
- **Test code is not measured**: every crate root with unit tests has `#![cfg_attr(test, feature(coverage_attribute))]` and every test module `#[cfg(test)] #[coverage(off)]`. Otherwise each assertion adds a branch that a passing test never takes. Integration test crates put `#![feature(coverage_attribute)]` and `#![coverage(off)]` at their root.
- No `unreachable!()` / `todo!()` / `unimplemented!()` arms in non-test code: they are uncovered regions by definition. Express invariants through types instead.
- Every error path is reached by a test: I/O failures through the in-memory `Fs`, clipboard failures through a failing fake `SystemClipboard`, terminal write failures through a failing `impl Write`.
- Tests assert behavior, not just execution: a test that runs code without checking its result does not count as coverage in review.
- A coverage report (`cargo llvm-cov --html`) is produced by `scripts/check.sh --report` (or `scripts/check.ps1 --report`) for finding gaps.

### 9.2 Unit and property tests
Per module. Property tests (`proptest`) for the gap buffer, line index, cell mapping, key-name round-trip and the CLI parser.

### 9.3 Scenario tests
A small text format in `tests/scenarios/*.mep-test`, run by a test harness in `mep-core` (no terminal):

```
== name: ldelete with linearg
-- tools.ini
[mep]
-- file a.txt
alpha
beta
gamma
-- keys
down arg down ldelete
-- expect file a.txt
alpha
-- expect cursor 1 0
-- expect clipboard line
beta
gamma
```

Keys are given as function names (as in macros) or key names (`alt+a`). Optional sections: `expect frame` (text snapshot), `expect message`, `expect status`. Write the harness in task 9 and use it from then on. The harness completes worker requests immediately against its in-memory `Fs` and clipboard; an optional `hold` directive keeps a request pending so tests can check the suspended state (rendering, type-ahead, Esc).

### 9.4 Platform tests
File I/O and line endings run on all three OSes. The clipboard adapter has an `#[ignore]` integration test for manual runs on a desktop session.

### 9.5 Oracle confirmation
`docs/oracle.md` lists each ORACLE item with: manual text, chosen behavior, and a key sequence the owner can run in a real MEP to confirm. ORACLE tests are **active** (not ignored) and assert the manual's reading, so the coverage gate holds. If the real binary behaves differently, the owner records the result and both test and code are changed.

### 9.6 Performance tests
`#[ignore]` tests built in release mode (`cargo test --release -- --ignored perf_`), run manually and before each milestone acceptance; timing is too noisy for the CI gate. They generate the reference document (§3.5) and measure with `std::time::Instant`:
- load into the buffer and line index: under 1 s;
- each of: insert a character, delete a character, move to the last line, page down, render a frame, at the start, middle and end of the document: under 16 ms (one frame at 60 Hz);
- one search slice: under 16 ms;
- the core thread is never blocked by file or clipboard I/O (checked with a fake `Fs` that blocks until released).

---

## 10. Regular expressions (M2)

Project decision (2026-11-01): MEP's original regex dialect (`?`, `[~..]`, minimal `*`/`+`, `@`/`#`, `(a!b)`, `~X`, `{..}` tags, `:a`–`:z` classes; M guide Chapter 5) is **not** implemented. Patterns use the syntax of the `regex` crate (Perl-like: `.`, `[^..]`, `*?`, `|`, `(?:..)`, `\d`, `\w`, named groups). This deliberately departs from the manual (§0 rule 2).

- `arg arg` = regular expression, single `arg` = literal text (escaped with `regex::escape`), as in the original.
- The `case` switch selects case-sensitive matching; off = the case-insensitive flag.
- A match lies within one line, as for plain search (§6.11); `^` and `$` are line start and end.
- Replacement strings use the crate's syntax: `$0` whole match, `$1`… groups, `${name}`, `$$` a literal `$`.
- An invalid pattern: the crate's error message on the dialog line, the command returns false.
- `regex` guarantees linear-time matching, so a pattern can never stall the editor on a large document (§3.5). A pattern is compiled once per search or replace, not per line.

---

## 11. ORACLE items (confirm against a real MEP)

| ID | Question | Implement per |
|---|---|---|
| S3 | `arg numarg mark`: line numbers 0-based? | M guide (0-based) |
| S4 | `mpara`: lands on first blank line preceding the paragraph? | M guide |
| S5 | `arg arg textarg paste` (file insert): at cursor or above current line? | M guide Ch. 4 (at cursor) |
| S6 | `arg markarg replace`: stream or box? | M guide (box) |
| S10 | `arg window` / `arg arg window`: which is horizontal/vertical? | M guide wording; confirm |
| TAB-1 | File tab width 8; exact `entab` values | §5.2 |
| UNDO-1 | Macro = one undo step? | §5.7 |
| MACRO-1 | Macro return value = last function's value? | §6.14 |

Available oracles: the existing Windows 11 MEP port, M for DOS / MEP for OS/2 under emulation.

---

## 12. Known traps

- **INIT, not USER**: TOOLS.INI search path comes from `INIT`. Never read `USER` as a directory list (on Unix it is the login name).
- **Terminal key collisions**: see §7. Test with and without keyboard enhancement.
- **Signals and flow control**: Ctrl+C (ppage), Ctrl+Z (plines), Ctrl+\ (qreplace), Ctrl+S (left) are editor keys.
- **Byte-exact round-trip**: never normalize unedited lines (terminators, tabs, trailing spaces, invalid UTF-8).
- **No 250-char line limit.**
- **Status line is 1-based, internals 0-based.**
- **Executable name** decides the TOOLS.INI section: get it from `std::env::args_os().next()` (file stem, case-insensitive), not from `current_exe()`. Started through a link `mz → mep`, `args_os().next()` is `mz` (the name typed), while `current_exe()` resolves the link to `.../mep`. Using the typed name keeps the original rule: a link named `mz` reads `[mz]`. If `args_os()` is empty, fall back to `mep`.
