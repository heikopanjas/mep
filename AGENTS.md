# Project Instructions for AI Coding Agents

**Last updated:** 2026-11-01

<!-- {preamble} -->

# ⚠️ Before You Start

Run `/init-session` at the beginning of each new session, OR read this entire file before proceeding.

**DO NOT** make code changes or commits until you have done one of the above.

<!-- {mission} -->

## Mission Statement

MEP is a faithful clean-room reimplementation of Microsoft's MEP text editor (M/MEP, 1988, version 1.01 command set) for modern Windows, macOS and Linux terminals.

The project exists for **system clipboard integration**: an existing Windows 11 port of MEP has no access to the system clipboard. Clipboard support must work in milestone M1, including line, box and stream shapes that survive a round trip through other applications.

Key features: the original argument and selection model, TOOLS.INI configuration with the section named after the executable, macros, multiple files, UTF-8 buffers that are written back byte-exact, and jump scrolling. Later milestones add regex search and replace (M2), windows and sessions (M3), and tools and C ABI extensions (M4).

Non-goals for now: syntax highlighting, GUI, mouse support, Z editor behavior, the original support programs, DOS-era switches.

`IMPLEMENTATION_PLAN.md` is the handoff document with the settled decisions, architecture, milestones and test strategy. Read it before writing code; do not reopen its decisions without asking the owner.

## Technology Stack

- **Language:** Rust, nightly toolchain pinned in `rust-toolchain.toml` (currently `nightly-2026-10-31`, with `clippy`, `llvm-tools-preview` and `rustfmt`), edition 2024. `.rustfmt.toml` needs nightly rustfmt. Move the pin only on purpose, in its own commit
- **Coverage data after a toolchain change:** run `cargo llvm-cov clean` (without `--workspace`) once; stale data from another toolchain produces bogus coverage reports
- **Framework:** none; terminal I/O through `crossterm`, system clipboard through `clipboard-rs` (with its `wayland` feature on Linux; `wl-clipboard-rs` is held at 0.9.4 because 0.9.5 is younger than two weeks), OSC 52 in SSH sessions and when no desktop clipboard exists
- **Version Control:** Git; public remote `origin` at `git@github.com:heikopanjas/mep.git`, `main` tracks `origin/main`
- **Package Manager:** Cargo workspace (`crates/mep-core`, `crates/mep-term`, `crates/mep-clip`, `crates/mep-ext`, `crates/mep`); `Cargo.lock` is committed
- **C extensions:** `crates/mep-ext/c/ext.h` and `exthdr.c` are our replacement for the 1988 `EXT.H`/`EXTHDR.OBJ`; `crates/mep/tests/filter.rs` compiles the MSJ `filter.c` with the platform C compiler (`$CC`, else `cc`, `clang` on Windows), so the check needs a C compiler. The test passes gcc-style flags (`-shared`, `-o`), so `$CC` must be a gcc-style driver: on Windows a `CC=cl.exe` left by a Visual Studio environment fails to link, run the check with `$env:CC = 'clang'`
- **Version manifest:** `[workspace.package] version` in the root `Cargo.toml`; all crates inherit it
- **Coverage tool:** `cargo-llvm-cov`
- **Check script:** `scripts/check.sh` (Linux, macOS; needs `jq`) and `scripts/check.ps1` (Windows), optional `--report` for an HTML coverage report in `target/llvm-cov/html`
- **CI:** `.github/workflows/ci.yml` runs the check script on Ubuntu, macOS and Windows
- **Line endings:** `.gitattributes` (`* text=auto eol=lf`) checks text files out with LF on every platform, as `.editorconfig` and rustfmt's `newline_style = "Unix"` require. A working tree checked out before that rule keeps CRLF (`git ls-files --eol` shows `w/crlf`) and fails `cargo fmt --check`; on a clean tree `git rm --cached -r -q . ; git reset --hard` checks it out again with LF
- **License:** MIT (`LICENSE`, `license = "MIT"` in the workspace manifest)

## Session Protocol

When starting a new session, read this entire file and confirm you have
understood the project instructions before proceeding. Summarize the project
purpose and key conventions briefly. Do not make changes until you have
confirmed your understanding.

<!-- {principles} -->

## Primary Instructions

- Avoid making assumptions. If you need additional context to accurately answer the user, ask the user for the missing information. Be specific about which context you need.
- Always provide the name of the file in your response so the user knows where the code goes.
- Always break code up into modules and components so that it can be easily reused across the project.
- All code you write MUST be fully optimized. ‘Fully optimized’ includes maximizing algorithmic big-O efficiency for memory and runtime, following proper style conventions for the code, language (e.g. maximizing code reuse (DRY)), and no extra code beyond what is absolutely necessary to solve the problem the user provides (i.e. no technical debt). If the code is not fully optimized, you will be fined $100.

### Working Together

This file (`AGENTS.md`) is the primary instructions file for AI coding assistants working on this project. Agent-specific instruction files (such as `CLAUDE.md`, `.github/copilot-instructions.md`, `.cursorrules`) reference this document, maintaining a single source of truth.

When initializing a session or analyzing the workspace, refer to instruction files in this order:

1. `AGENTS.md` (this file - primary instructions and single source of truth)
2. Agent-specific reference file (if present - points back to AGENTS.md)

### Update Protocol (CRITICAL)

**PROACTIVELY update this file (`AGENTS.md`) as we work together.** Whenever you make a decision, choose a technology, establish a convention, or define a standard, you MUST update AGENTS.md immediately in the same response.

**Update ONLY this file (`AGENTS.md`)** when coding standards, conventions, or project decisions evolve. Do not modify agent-specific reference files unless the reference mechanism itself needs changes.

**When to update** (do this automatically, without being asked):

- Technology choices (build tools, languages, frameworks)
- Directory structure decisions
- Coding conventions and style guidelines
- Architecture decisions
- Naming conventions
- Build/test/deployment procedures

**How to update AGENTS.md:**

- Maintain the "Last updated" timestamp at the top
- Add content to the relevant section (Project Overview, Coding Standards, etc.)
- Log every change in the "Recent Updates & Decisions" log in `UPDATES.md` with:
  - Date (with time if multiple updates per day)
  - Brief description
  - Reasoning for the change
- New log entries go directly below the changelog marker in `UPDATES.md`, newest first; load the `recent-updates` skill for the full rules
- Preserve the AGENTS.md structure: title header → timestamp → main instructions

## Best Practices

### When Updating This Repository

1. **Maintain Consistency**: Keep code style consistent across the codebase
2. **Test First**: Write tests before implementing features when applicable
3. **Document Changes**: Update documentation when changing functionality
4. **Code Review**: The owner reviews every change before it is committed. A commit must build and pass the check script (fmt, clippy, tests, coverage gate) on Windows, macOS and Linux
5. **Date Changes**: Update the "Last updated" timestamp in this file when making changes
6. **Log Updates**: Add entries to the "Recent Updates & Decisions" log in `UPDATES.md`

### Development Guidelines

- **Clean room**: never use, search for, or read the leaked Windows NT source of MEP or the source of other MEP/Z clones. Behavior comes only from the manuals in `docs/sources/` and from observing a real MEP binary
- **Sample configurations**: the owner's TOOLS.INI files from https://github.com/tenox7/ntutils (binaries and sample configs, no source code) may be read; a switch or function they use that the 1988 manual lacks is added only as a project addition with an ORACLE item, after asking the owner
- **The manual is the spec**: the Microsoft Editor User's Guide (1988) is authoritative for every command and switch. Where `IMPLEMENTATION_PLAN.md` and the manual disagree, stop and ask. Z editor sources are secondary; MEP wins every conflict
- **Spec, then tests, then code**: before implementing a command, write its behavior per argument kind into `docs/spec/commands/<name>.md`, citing manual pages ("M guide p. 102"), then write tests from that spec, then implement
- **ORACLE items**: where the manual is ambiguous, implement the manual's reading and mark code and test with `// ORACLE: <id>`; record the item in `docs/oracle.md`. ORACLE tests stay active
- **No panics on user input**: every command returns `bool`; failure means `false` plus a message on the dialog line
- **Core isolation**: `mep-core` holds all editor logic, has `#![forbid(unsafe_code)]`, and never touches `std::fs`, `std::env`, the clock, the terminal or the OS clipboard directly; it goes through the `Terminal`, `SystemClipboard`, `Fs` and `Env` traits
- **Paths in the core**: `mep-core` uses `typed_path::Utf8TypedPathBuf` (re-exported as `mep_core::Utf8TypedPathBuf`), never `std::path`; `Os::path` builds a path in the style of `Env::os()`, so a Windows path joins with `\` on any host. The binary converts to `std::path` only in `mep/src/os.rs`, `host.rs` and `process.rs`. `MemFs` keys files by their `/` form, so a fixture `/work/a.txt` matches either style
- **OS glue files** (`mep-term/src/sys.rs`, `mep-clip/src/system.rs`, `mep/src/main.rs`, `mep/src/sys.rs`) contain only calls into the OS, no logic; they are the only files excluded from coverage
- **Unsafe code** lives only in `mep-ext` (C extension ABI) and in `mep/src/sys.rs` (Windows console control handler); every unsafe block has a `// SAFETY:` comment
- **Dependencies**: only the crates listed in `IMPLEMENTATION_PLAN.md` §4.3, which include `anyhow`, `clap`, `regex`, `typed-path` (mep-core, held at 0.12.3), `libloading` (mep-ext), `signal-hook` (mep, Unix) and `windows-sys` (mep, Windows). Any other crate, runtime or dev, needs a one-line justification and the owner's OK. FNV-1a 64 is implemented in-house. A crate is added to a `Cargo.toml` only when code first uses it
- **Large documents** (`IMPLEMENTATION_PLAN.md` §3.5): the editor must never slow down or freeze on large files. The core stays a deterministic single-threaded state machine; input, rendering, file I/O, clipboard and jobs run on `std::thread` workers with `mpsc` channels (no async runtime). Commands that need I/O suspend and resume on a completion event. The cost of an edit depends on the size of the edit, not the size of the file
- **Local time**: `curdate`/`curtime` read `/etc/localtime` or `TZ` with the TZif parser in `crates/mep/src/tz.rs` (no time crate); Windows falls back to UTC until a solution is approved
- **Byte-exact round trip**: never normalize unedited lines (terminators, tabs, trailing spaces, invalid UTF-8)
- **Small commits**: each one builds and passes the check script
- **Script names**: shell scripts for Linux and macOS end in `.sh`, PowerShell scripts for Windows in `.ps1`

### Security & Safety

- Never include API keys, tokens, or credentials in code
- Always require explicit human confirmation before commits
- Maintain conventional commit message standards
- Keep change history transparent through commit messages
- Unsafe code is allowed only in the M4 extension crate

### Testing

100% test coverage is a hard acceptance criterion. No task is done and no commit is acceptable while coverage is below 100%. Code that cannot be covered is redesigned until it can, not excluded. Details in `IMPLEMENTATION_PLAN.md` §9.

- Unit tests: per module, in `#[cfg(test)] #[coverage(off)]` modules (the crate root needs `#![cfg_attr(test, feature(coverage_attribute))]`); property tests (`proptest`) for the gap buffer, line index, cell mapping, key-name round trip and CLI parser
- Scenario tests: `tests/scenarios/*.mep-test`, run by a harness in `mep-core` without a terminal; keys given as function or key names, with `expect` sections for files, cursor, clipboard, frame, message and status
- Platform tests: file I/O and line endings on all three OSes; the clipboard adapter has an `#[ignore]` integration test for manual runs on a desktop session
- Test coverage requirements: 100% lines, functions, regions and branches, gated by `cargo llvm-cov --branch` in the check script on each platform; the only exclusions are the three OS glue files and test code itself, and adding a file needs the owner's OK
- No `unreachable!()`, `todo!()` or `unimplemented!()` in non-test code; express invariants through types
- Every error path is reached by a test through in-memory fakes (`Fs` with injectable failures, failing `SystemClipboard`, failing `impl Write`)
- Tests assert behavior, not just execution
- Performance tests (`IMPLEMENTATION_PLAN.md` §9.6) are `#[ignore]` tests named `perf_` in `crates/mep-core/tests/perf.rs` (reference document: 2 million lines, 100 MB) and `crates/mep/tests/perf.rs` (blocking file system and clipboard); run them with `cargo test --release --workspace -- --ignored perf_` before accepting a milestone. A debug build refuses to measure
- Scenario tests: format documented at the top of `crates/mep-core/tests/scenarios.rs`; `~` marks an empty body line, `key:NAME` forces a key event when NAME is also a function, `-- fail`, `-- inject`, `hold`/`-- release` reach I/O error and suspension paths; `-- env XDG_CONFIG_HOME=/cfg` enables the information file at `/cfg/mep/mep.tmp`, `-- temporary` stands for `-t`; `-- os windows` also gives Windows path rules, so status lines and the information file show `\work\a.txt`; `-- run [fail] CMD` scripts command output for `compile`/`paste`/`shell`, `-- expect log` checks what ran, `-- expect bell`; `load:probe.so` loads the scripted `probe` extension whose text argument is a callback command, `-- readkeys` feeds `ReadChar`/`ReadCmd`; macros in `-- tools.ini` stand in for macro text with quotes (harness keys cannot escape quotes)
- Coverage across test binaries: unit tests and the scenario harness build separate copies of `mep-core`, and the gate takes each function's best copy. A function the scenarios call must be fully covered by scenarios; a fallback that scenarios cannot reach belongs in a separate function covered by unit tests
- Avoid generic parameters (`impl Fn`, `W: Write`) on functions whose tests pass different types: each instantiation is measured on its own. Use `&mut dyn FnMut` or `Box<dyn Write>`
- Testing framework: built-in `cargo test`, `proptest`, `tempfile`
- `proptest-regressions/` files from real failures are committed; seeds produced only by placeholder code during the red phase of test-first work are deleted

### Documentation

- Code comments: explain why, not what; mark ambiguous-manual behavior with `// ORACLE: <id>`
- Command specs: one file per command in `docs/spec/commands/<name>.md`, transcribed from the manual with page citations; switches in `docs/spec/switches.md`
- ORACLE items: `docs/oracle.md` with manual text, chosen behavior and a key sequence to confirm against a real MEP
- Sources: `docs/sources/` follows the conventions in `docs/sources/README.md`
- README updates: when user-visible behavior, build steps or CLI options change
- Examples: `examples/` holds sample configurations; `examples/TOOLS.INI` is the owner's working configuration and uses the project additions `begfile`, `endfile` and `cursorsize`. Keep it loading without errors when switches or functions change
- Changelog: maintained in `UPDATES.md` (see the `recent-updates` skill)

<!-- {languages} -->

## Rust Coding Standards

Load the `rust-coding-conventions` skill before writing, reviewing, or refactoring Rust code.
Load the `rust-build-commands` skill when building or running the project.

Project overrides of the skill:

- `anyhow` and `clap` are allowed. Commands still return `bool` (§0 rule 5); `anyhow` is for fallible operations outside the command interface
- Every crate root carries `#![allow(clippy::bool_comparison)]` for the explicit `== true` / `== false` style
- The `require!` macro is added to a crate only when that crate first uses it

<!-- {integration} -->

## Commit Protocol

- **NEVER commit automatically** — always wait for explicit user confirmation
- This includes the end of every plan task: finish the task, summarize it, then stop and wait for the owner's commit instruction. An earlier request to commit after each task does not carry over
- Stage changes, write a conventional commits message (max 50-char subject, 72-char body lines), then commit
- Load the `git-workflow` skill for the full message format, character limits, and examples before committing
- Never add co-authorship trailers or any attribution naming an AI coding agent to commit messages

## Pending Verification

Postponed by the owner on 2026-11-01; M1 is not accepted until these pass:

- The M1 manual checklist (`IMPLEMENTATION_PLAN.md` §8) on Windows Terminal, iTerm2, Ghostty and a Linux terminal; it passed on macOS through a pty
- macOS terminals type characters with Option unless told otherwise, so Alt keys (`arg` is Alt+A) need Option sent as Alt: Ghostty `macos-option-as-alt = left` (confirmed by the owner), iTerm2 "Left Option key: Esc+", Terminal.app "Use Option as Meta key"
- The Command key reaches MEP as `cmd+` (TOOLS.INI `copy:cmd+c`) only in kitty-protocol terminals and only for combinations the terminal does not bind; in Ghostty free them with `keybind = cmd+c=unbind` etc.; not yet tried on a real terminal

Also postponed by the owner on 2026-11-01; M3 is not accepted until these pass:

- Copy and paste on Wayland and X11 desktops, and OSC 52 copies over SSH reaching the local clipboard

M4 checks still open:

- `shell` by hand on Linux; `compile` by hand on Linux and on Windows. `shell` passed by hand on Windows (`cmd /C`: a command, a failing command, Ctrl+C during `ping -t` with `SetConsoleCtrlHandler`); the extension acceptance test (`crates/mep/tests/filter.rs`) passed in CI on Linux and on Windows with clang, macOS passed
- Shift+F3 (`compile`) on terminals without the kitty keyboard protocol: crossterm reads the xterm sequence `CSI 1;2R` as a cursor position report and drops it. Open on Linux and macOS only: on Windows crossterm reads console key events, and Shift+F3 reaches `compile` (`No more compile messages` without a compile)

## Recent Updates & Decisions

Project decisions and notable changes are logged in `UPDATES.md`, below its changelog marker. The log is append-only: add new entries at the top, never edit or delete existing ones, and keep the "Last updated" timestamp in `AGENTS.md` in sync. Load the `recent-updates` skill for the entry format and full rules.

## Semantic Versioning

Automatically bump the project version after every code change and include it in the same commit. Load the `semantic-versioning` skill for the full PATCH/MINOR/MAJOR decision rules.
