# Project Instructions for AI Coding Agents

**Last updated:** 2026-10-31

<!-- {preamble} -->

# ⚠️ Before You Start

Run `/init-session` at the beginning of each new session, OR read this entire file before proceeding.

**DO NOT** make code changes or commits until you have done one of the above.

<!-- {mission} -->

## Mission Statement

MEP is a faithful, private clean-room reimplementation of Microsoft's MEP text editor (M/MEP, 1988, version 1.01 command set) for modern Windows, macOS and Linux terminals.

The project exists for **system clipboard integration**: an existing Windows 11 port of MEP has no access to the system clipboard. Clipboard support must work in milestone M1, including line, box and stream shapes that survive a round trip through other applications.

Key features: the original argument and selection model, TOOLS.INI configuration with the section named after the executable, macros, multiple files, UTF-8 buffers that are written back byte-exact, and jump scrolling. Later milestones add regex search and replace (M2), windows and sessions (M3), and tools and C ABI extensions (M4).

Non-goals for now: syntax highlighting, GUI, mouse support, Z editor behavior, the original support programs, DOS-era switches.

`IMPLEMENTATION_PLAN.md` is the handoff document with the settled decisions, architecture, milestones and test strategy. Read it before writing code; do not reopen its decisions without asking the owner.

## Technology Stack

- **Language:** Rust, stable toolchain, edition 2024 (`rust-toolchain.toml` with `llvm-tools-preview`)
- **Framework:** none; terminal I/O through `crossterm`, system clipboard through `clipboard-rs`
- **Version Control:** Git
- **Package Manager:** Cargo workspace (`crates/mep-core`, `crates/mep-term`, `crates/mep-clip`, `crates/mep`)
- **Coverage tool:** `cargo-llvm-cov`
- **License:** not decided; private project

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
4. **Code Review**: The owner reviews every change before it is committed. A commit must build and pass `scripts/check` (fmt, clippy, tests, coverage gate) on Windows, macOS and Linux
5. **Date Changes**: Update the "Last updated" timestamp in this file when making changes
6. **Log Updates**: Add entries to the "Recent Updates & Decisions" log in `UPDATES.md`

### Development Guidelines

- **Clean room**: never use, search for, or read the leaked Windows NT source of MEP or the source of other MEP/Z clones. Behavior comes only from the manuals in `docs/sources/` and from observing a real MEP binary
- **The manual is the spec**: the Microsoft Editor User's Guide (1988) is authoritative for every command and switch. Where `IMPLEMENTATION_PLAN.md` and the manual disagree, stop and ask. Z editor sources are secondary; MEP wins every conflict
- **Spec, then tests, then code**: before implementing a command, write its behavior per argument kind into `docs/spec/commands/<name>.md`, citing manual pages ("M guide p. 102"), then write tests from that spec, then implement
- **ORACLE items**: where the manual is ambiguous, implement the manual's reading and mark code and test with `// ORACLE: <id>`; record the item in `docs/oracle.md`. ORACLE tests stay active
- **No panics on user input**: every command returns `bool`; failure means `false` plus a message on the dialog line
- **Core isolation**: `mep-core` holds all editor logic, has `#![forbid(unsafe_code)]`, and never touches `std::fs`, `std::env`, the clock, the terminal or the OS clipboard directly; it goes through the `Terminal`, `SystemClipboard`, `Fs` and `Env` traits
- **OS glue files** (`mep-term/src/sys.rs`, `mep-clip/src/system.rs`, `mep/src/main.rs`) contain only calls into the OS, no logic
- **Dependencies**: only the crates listed in `IMPLEMENTATION_PLAN.md` §4.3. Any other crate, runtime or dev, needs a one-line justification and the owner's OK. No `clap`; FNV-1a 64 is implemented in-house
- **Byte-exact round trip**: never normalize unedited lines (terminators, tabs, trailing spaces, invalid UTF-8)
- **Small commits**: each one builds and passes `scripts/check`

### Security & Safety

- Never include API keys, tokens, or credentials in code
- Always require explicit human confirmation before commits
- Maintain conventional commit message standards
- Keep change history transparent through commit messages
- Unsafe code is allowed only in the M4 extension crate

### Testing

100% test coverage is a hard acceptance criterion. No task is done and no commit is acceptable while coverage is below 100%. Code that cannot be covered is redesigned until it can, not excluded. Details in `IMPLEMENTATION_PLAN.md` §9.

- Unit tests: per module, in `#[cfg(test)]` modules; property tests (`proptest`) for the gap buffer, line index, cell mapping, key-name round trip and CLI parser
- Scenario tests: `tests/scenarios/*.mep-test`, run by a harness in `mep-core` without a terminal; keys given as function or key names, with `expect` sections for files, cursor, clipboard, frame, message and status
- Platform tests: file I/O and line endings on all three OSes; the clipboard adapter has an `#[ignore]` integration test for manual runs on a desktop session
- Test coverage requirements: 100% lines, functions and regions, gated by `cargo llvm-cov` in `scripts/check` on each platform; the only exclusions are the three OS glue files, and adding one needs the owner's OK
- No `unreachable!()`, `todo!()` or `unimplemented!()` in non-test code; express invariants through types
- Every error path is reached by a test through in-memory fakes (`Fs` with injectable failures, failing `SystemClipboard`, failing `impl Write`)
- Tests assert behavior, not just execution
- Testing framework: built-in `cargo test`, `proptest`, `tempfile`

### Documentation

- Code comments: explain why, not what; mark ambiguous-manual behavior with `// ORACLE: <id>`
- Command specs: one file per command in `docs/spec/commands/<name>.md`, transcribed from the manual with page citations; switches in `docs/spec/switches.md`
- ORACLE items: `docs/oracle.md` with manual text, chosen behavior and a key sequence to confirm against a real MEP
- Sources: `docs/sources/` follows the conventions in `docs/sources/README.md`
- README updates: when user-visible behavior, build steps or CLI options change
- Changelog: maintained in `UPDATES.md` (see the `recent-updates` skill)

<!-- {languages} -->

## Rust Coding Standards

Load the `rust-coding-conventions` skill before writing, reviewing, or refactoring Rust code.
Load the `rust-build-commands` skill when building or running the project.

<!-- {integration} -->

## Commit Protocol

- **NEVER commit automatically** — always wait for explicit user confirmation
- Stage changes, write a conventional commits message (max 50-char subject, 72-char body lines), then commit
- Load the `git-workflow` skill for the full message format, character limits, and examples before committing
- Never add co-authorship trailers or any attribution naming an AI coding agent to commit messages

## Recent Updates & Decisions

Project decisions and notable changes are logged in `UPDATES.md`, below its changelog marker. The log is append-only: add new entries at the top, never edit or delete existing ones, and keep the "Last updated" timestamp in `AGENTS.md` in sync. Load the `recent-updates` skill for the entry format and full rules.

## Semantic Versioning

Automatically bump the project version after every code change and include it in the same commit. Load the `semantic-versioning` skill for the full PATCH/MINOR/MAJOR decision rules.
