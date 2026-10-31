# MEP — Project Brief

Private clean-room clone of Microsoft's MEP editor (M/MEP 1988), for modern operating systems.

## Sources

- Microsoft Editor User's Guide, Feb 1988 (M for DOS / MEP for OS/2), OS/2 Museum scan — Doc. No. 614510001-100-R00-0288 — primary reference
- Leo A. Notenboom, "Customizing the Features of the M Editor Using Macros and C Extensions", Microsoft Systems Journal, Sep 1988 (scan) — covers M 1.01 functions, pseudo-files, the full extension callback list, a worked `filter` extension
- User's Guide to the Z Editor, Mark Zbikowski, 4 Aug 1986 (v7.43), plain text (raw + cleaned) — secondary: explains shared concepts (arg model, regex, macros) where the M guide is thin; Z-only behavior is not implemented
- Nancy Michell, "Memory Bank: Microsoft Programmer's WorkBench, 1990", MSDN News, Jul/Aug 2001 (background only: M → PWB lineage, MEP's own VM system for loading large files fast)

## Decisions

All open questions are decided (2026-10-31).

### Identity and behavior
- Name: **MEP** (executable `mep`, `mep.exe`). Other names via symbolic links if wanted.
- Behavior: MEP (1988, M 1.01) only. No Z personality, no Z keymap, no Z/MEP switches. Where the M guide is ambiguous, the real MEP binary decides (ORACLE rows in z-vs-mep.md).
- TOOLS.INI: the section read is the executable's own name, as in the original (`[mep]`; a link named `mz` reads `[mz]`). Section names match case-insensitively. Tagged variants (`[mep-<os>]` etc.) and multi-name tags (`[m mep]`) as in the original.
- Selection model: reproduce the original exactly; no global box/stream mode. The cursor geometry relative to the Arg anchor classifies the arg (same position → nullarg; same line → streamarg; same column → linearg; else boxarg). Each command declares in its cmdTable flags which arg kinds it accepts; a multi-line box-shaped selection is a box for Ldelete/Linsert/Copy/Replace and a stream for Sdelete (and per relative position for markarg). Right edge exclusive: box/stream end one column left of the rightmost of anchor and cursor. numarg → linearg and markarg → stream/box/line conversion happens in the dispatcher before the command runs (MSJ Fig. 6), so extensions only see the base kinds.
- Screen layout: no separate command line. Bottom two rows fixed: dialog line (messages, `Arg:` text-argument editing) and status line. Editing area follows the terminal size and adapts live on resize; `height`/`width` apply only when set explicitly in TOOLS.INI (the original 23×80 defaults are dropped).
- Scrolling: jump scrolling with original defaults `vscroll:7`, `hscroll:10`, `hike:4`; smooth scrolling via `vscroll:1 hscroll:1`. `displaycursor` switch for cursor vs window position in the status line.
- Named commands in a command table; arg-first command model as in MEP. No scripting language beyond the original macros.
- No syntax highlighting in the first release.
- Support programs: none (no UNDEL/RM/EXP, MEGREP, CALLTREE) — version control is git's job. Cross-file search via `arg arg "grep -n …" compile`.
- Backups: `backup:` switch with `none` (default) and `bak`. The original `undel` mode is dropped along with UNDEL.

### Encoding
- Buffer stores UTF-8 bytes unchanged. Invalid bytes are displayed as a placeholder and written back byte-exact.
- A column is a display cell: box edges, hscroll, tabs and the status position count cells. Wide characters occupy two cells (`unicode-width`).
- A box edge splitting a wide character: the character belongs to the box if its first cell is inside. Paste fills a half-cell gap with a space.
- No UTF-16 or code pages in the first release; later as conversion on load/save.

### Implementation
- Language: **Rust** (stable toolchain), built with Cargo. Replaces the earlier C17/CMake choice.
- Structure: Cargo workspace with a portable core crate (buffer, arg model, commands, TOOLS.INI, macros) and a thin front-end binary `mep`. The core never touches the terminal or the OS clipboard directly; it talks to traits so tests run headless.
- Text storage: own byte-based gap buffer (not `ropey`, which requires valid UTF-8 and would break byte-exact round-trips).
- Terminal: `crossterm` for both Windows console and VT terminals; resize events; kitty keyboard protocol (keyboard enhancement flags) where the terminal supports it, Esc-timeout fallback otherwise.
- Clipboard: `clipboard-rs` behind a clipboard trait (Windows, macOS, X11, optional Wayland; custom formats supported). OSC 52 as an own small adapter.
- Child processes and background jobs: `std::process`, threads and channels. Synchronous single-thread mode for deterministic tests.
- Extensions: native shared libraries loaded with `libloading`, exposing a **C ABI** so extensions can be written in C as in the original: `#[repr(C)]` ARG, cmdTable and swiTable, `extern "C"` callbacks. File handles (PFILE) are opaque indices into a file table, not pointers. All `unsafe` code stays at this boundary.
- Regex (M2): first try translating the MEP dialect to `regex` / `fancy-regex`; check `~X` and backtracking alternation for exact semantics. If they don't map, write an own matcher.
- Dependencies are kept to the crates named here; each new one needs a reason.
- Windows target `x86_64-pc-windows-msvc` (and `aarch64-pc-windows-msvc`) with the VS 2026 build tools as linker. macOS and Linux with the default Rust targets.

### Clipboard
- `<clipboard>` pseudo-buffer mapped to the system clipboard. System clipboard access is the reason this project exists (the existing Windows 11 port lacks it), so the Windows and macOS adapters ship in M1.
- Clipboard shape (line / box / stream) survives the system clipboard:
  - Copy writes plain text for other apps plus a private format (custom type via `clipboard-rs`) recording the shape, the box width and a hash of the plain text. A box becomes one row per line with trailing padding trimmed in the plain text; the private record keeps the true width.
  - Paste uses the private shape only if the private record is present and its hash matches the current plain text. Otherwise (text from other apps) it classifies: ends with a newline → line paste, otherwise → stream.
  - OSC 52 (SSH): copy goes out as plain text only; paste reads the internal clipboard and applies the newline rule, since terminals often block clipboard reads.
  - `<clipboard>` is a view of the current system clipboard: opening it via setfile shows what is on the system clipboard now.

### Milestones
- M1 = core engine, one window, several files: gap buffer, file load/save (CRLF/LF preserved, "NL" status flag), setfile with the command-line file list and switching files in a single window, full arg model with all six arg kinds plus meta, cursor/insert/delete/copy/paste/undo commands, plain-string psearch/msearch, assign, TOOLS.INI parsing incl. macros with :> => +> -> conditionals, UTF-8 cell model, clipboard with the shape rules, system clipboard on Windows and macOS, crossterm front end, `backup:` switch.
- M2 = regex + replace/qreplace with tagged $0–$9 + markers/markfile.
- M3 = split windows (≤8 tiled, min 5×10), information file, tmpsav session restore, remaining clipboard adapters (Wayland, X11, OSC 52).
- M4 = compile/shell via job queue, error browsing, extensions ABI.

### Process
- Clean room: the leaked NT source is never used. Real MEP binaries serve as behavioral oracles; tests derived from the manuals.
- Later IDE features: compiler-diagnostic parsing, optional clangd.
