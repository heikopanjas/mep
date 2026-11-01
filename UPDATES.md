# Recent Updates & Decisions

This file is the append-only log of project decisions and notable changes, maintained by coding agents following the `recent-updates` skill. Everything below the marker line is user-owned history: slopctl never overwrites it during init or merge.

<!-- {changelog} -->

### 2026-11-01 (v0.14.0, github remote)

- the repository is published at git@github.com:heikopanjas/mep.git as origin, public, with everything including docs/sources; main tracks origin/main
- github's placeholder initial commit was replaced by a force push of the local history
- rationale: the owner chose a public repository with the full history and sources
- no version bump: no code change

### 2026-11-01 (v0.14.0, cursorsize switch)

- new project switch cursorsize: 0 underline, 1 block, for insert and overtype mode; without it the cursor stays a bar in insert mode and a block in overtype mode (oracle cursor-1)
- the meaning comes from the comment in a sample mep tools.ini from tenox7/ntutils; the owner chose one shape for both modes
- sample configurations from tenox7/ntutils may be read; the repository holds binaries and sample configs, no source code
- version bump: 0.13.0 to 0.14.0 (MINOR - new switch)

### 2026-11-01 (v0.13.0, command key)

- new key modifier cmd+ for the macos command key (super elsewhere), reported by terminals with the kitty keyboard protocol; usable in tools.ini and assign, for example copy:cmd+c (oracle key-1)
- fixes keys with the super modifier that typed their plain character before
- an unassigned cmd+ key runs unassigned and never types its character
- rationale: the owner wants command key shortcuts on macos; ghostty reports command as super for combinations it does not bind itself
- version bump: 0.12.1 to 0.13.0 (MINOR - new key names)

### 2026-11-01 (v0.12.1, option key on macos)

- alt keys on macos need the terminal to send option as alt; ghostty works with macos-option-as-alt = left, as confirmed by the owner
- rationale: macos terminals compose characters such as å with option by default; mep receives the character and cannot tell it from typed text, so the setting belongs to the terminal and no code changes

### 2026-11-01 (v0.12.1, performance tests)

- added the plan's performance tests as ignored perf_ tests: loading the 2 million line, 100 MB reference document takes 69 ms (limit 1 s); insert, delete, moves, page down, undo, status and a frame at the start, middle and end stay under 4 ms (limit 16 ms); the slowest search slice takes 9 ms
- a blocking file system and clipboard under the real front end show that esc still cancels a waiting load or paste within about 6 ms
- the first edit after a jump moves the gap of the buffer and costs up to 3.5 ms on the reference document; later edits nearby cost microseconds
- rationale: plan section 9.6 asks for these measurements before each milestone acceptance; timings are too noisy for the ci gate, so the tests are ignored and run by hand in release mode
- version bump: 0.12.0 to 0.12.1 (PATCH - tests only)

### 2026-11-01 (v0.12.0, m4 tools and extensions)

- compile and argcompile run extmake commands or any command line in a background job thread; output goes to the compile pseudo-file, the bell rings, messages in the five manual forms plus file:row:column are browsed with compile, meta compile and arg meta compile up to maxmsg (oracle compile-1 to compile-4)
- shell hands the terminal to the interactive shell or to command lines, waits for enter with askrtn, saves first without meta; the input thread pauses at a gate and the output is held so no frame is drawn meanwhile
- arg arg !command paste inserts the standard output of a command at the cursor
- c extensions: load switch, cmdTable, swiTable with boolean, numeric and special switches, WhenLoaded and all msj fig. 7 callbacks through our ext.h and exthdr.c with a MepCallbacks table (oracle ext-1 to ext-4); fExecute runs macros to the end and serves their io at once
- new crate mep-ext holds all unsafe code for the c abi with libloading 0.9.0; the msj filter extension, compiled from c in the acceptance test, sorts lines through sort
- added signal-hook 0.3.18 (unix) and windows-sys 0.61.2 (windows) with the owner's ok, so ctrl+c during a shell command does not end the editor; mep/src/sys.rs joins the coverage exclusions
- undo groups are kept per file within one command, for extensions that edit several files
- rationale: shell must own the terminal and extensions call back synchronously, so both run on the core thread through a host trait while compiles stay in the background
- version bump: 0.11.0 to 0.12.0 (MINOR - compile, shell, command paste and c extensions)

### 2026-11-01 (v0.11.0, m3 clipboard checks postponed, 02:05)

- the wayland, x11 and ssh osc 52 clipboard checks are postponed and listed under pending verification in agents.md
- rationale: they need a linux desktop and a remote session, which are not available yet; m3 stays unaccepted until they pass

### 2026-11-01 (v0.11.0, m3 windows and sessions, 01:52)

- window: up to eight tiled windows with separators in stacolor; arg window splits at the cursor row, arg arg window at the cursor column, window cycles and meta window closes (oracle s10, win-1)
- each window keeps its own file, cursor and window position; files remember their position when switched away; edits show in every window of a file
- information file in the config directory as exe.tmp: written on quit and terminal close unless -t or tmpsav 0, reopens the last file when started without files, restores remembered positions (oracle info-1)
- information command lists open and remembered files in the information pseudo-file
- osc 52 clipboard in ssh sessions and when no desktop clipboard exists, sharing the terminal writer with the screen; clipboard-rs wayland feature on linux, wl-clipboard-rs held at 0.9.4 for the two week rule
- rationale: osc 52 is the only way to reach the local clipboard from a remote shell; reads return what mep wrote, so paste uses the internal clipboard
- version bump: 0.10.0 to 0.11.0 (MINOR - windows, sessions and clipboard adapters)

### 2026-11-01 (v0.10.0, m2 regex search, replace and marks)

- arg arg psearch and msearch search regular expressions with the regex crate on bytes; single-arg searches go through the same engine with the text escaped; an invalid pattern keeps the previous search
- replace and qreplace with prefilled editable prompts, ranges from the cursor, lines, numbers, streams, boxes and markers (always a box, oracle s6), literal or regex replacement with $1 and named groups, slices with esc, one undo step, and y, n, p and esc answers for qreplace
- mark: start of file, 0-based line numbers (oracle s3), define and go to markers with case-insensitive names across files, markfile lookup (read only), and arg mark returning to the position before the last window jump
- marker arguments for copy, ldelete, linsert, sdelete and sinsert become a selection from the marker to the cursor
- dialog line prompts are now editable text with a cursor; the read-only save-as prompt uses them too
- specs for mark, replace and qreplace written; psearch, msearch and markfile specs updated; oracle items S3, S6, MARK-1, MARK-2 and RPL-1 recorded
- added regex 1.13.1
- version bump: 0.9.0 to 0.10.0 (MINOR - regex search, replace and marks)

### 2026-11-01 (v0.9.0, modern regex syntax instead of mep dialect)

- m2 uses the regex crate and its syntax for arg arg searches and replacements; mep's original dialect from chapter 5 of the m guide is dropped
- replacement strings use $0, $1, ${name} and $$; case maps to the case-insensitive flag; matches stay within one line
- removed fancy-regex from the allowed crates and rewrote implementation_plan.md section 10 and the m2 task list
- rationale: owner decision; the original dialect is unfamiliar today, and the regex crate guarantees linear-time matching, which fits the large-document rule of section 3.5

### 2026-11-01 (v0.9.0, pending m1 checks postponed)

- postponed the linux and windows check script runs and the manual checklist on the remaining terminals; listed under Pending Verification in agents.md
- rationale: owner decision; these need machines this session cannot reach

### 2026-11-01 (v0.9.0, m1 verification on macos)

- cargo clippy with -D warnings passes for the whole workspace including tests on x86_64-unknown-linux-gnu and x86_64-pc-windows-msvc; running the tests there is still open
- the ignored system clipboard test passes on macos: a box copied by mep reads as plain text in other apps (pbpaste) and comes back as a box
- the m1 manual checklist ran on macos through a pty: ctrl+c, ctrl+z and ctrl+\ arrive as keys; resize while editing works; a tools.ini key assignment and a macro with -> work; external text pastes as a stream; an edited crlf line keeps crlf and unedited lines stay byte-exact; exit walks the file list
- open: scripts/check.sh on linux and scripts/check.ps1 on windows, the checklist on windows terminal, iterm2, ghostty and a linux terminal

### 2026-11-01 (v0.9.0, m1 tasks 14 and 20 front end and clipboard adapter)

- task 14: mep-clip writes plain text plus the private shape record in one clipboard-rs call and reads both; image support disabled; an ignored integration test does a real round trip on a desktop session
- task 20: mep-term converts crossterm keys (keypad, shift folding, release events ignored), maps dos colors to ansi with bright backgrounds, and diffs frames, moving the cursor only between non-adjacent changed cells; sys.rs holds only terminal setup
- task 20: the mep binary runs an io and clipboard worker thread, a render thread that draws only the newest frame, and the input thread from main.rs; while a search runs the main loop polls input and ticks slices
- added Event::Close so a failed terminal write ends the editor instead of leaving it waiting for input; terminal setup runs only after the command line parsed, so usage errors print on the normal screen
- added Env::cwd for relative file names and OsEnv with local time from tzif zone files and posix tz rules, without an extra crate; windows uses utc for now
- added scenario cases for the meta form and a rejected argument kind of every m1 command (425 cases in total)
- smoke test through a real pty on macos: typing, saving and quitting work and an edited crlf line keeps crlf
- added clipboard-rs 0.3.5 (no default features) and crossterm 0.29.0
- version bump: 0.8.0 to 0.9.0 (MINOR - terminal front end, clipboard adapter and binary)

### 2026-11-01 (v0.8.0, m1 tasks 8 to 13 and 15 to 19 editor core)

- task 8: render module builds the frame: text window with tabdisp, traildisp, placeholders for invalid bytes and control characters, selection and match highlight in hgcolor, dialog line with arg text, prompts and messages, status line with up to eight fields
- task 9: command table with argument flags per spec, dispatcher with arg-mode text editing, selection extension by cursor functions, argument classification (nullarg, text, numarg to lines, stream, line, box, boxstr, nulleol, nulleow), meta handling, and the scenario harness in crates/mep-core/tests/scenarios.rs
- tasks 10 to 12: movement, editing and undo commands per docs/spec/commands; undo groups per top-level command with deltas and undocount
- task 13: clipboard core with shapes, private record, system clipboard requests, byte-exact internal clipboard for text mep wrote itself, clipboard pseudo-file and one-time fallback message
- task 15: plain search in bounded slices with esc to cancel; regex forms report not yet supported
- tasks 16 to 19: tools.ini sections and statements, assign, initialize, macros with a resumable stack, execute, lasttext, setfile, refresh, exit with autosave, backup, read-only prompt, and the cur insertion functions
- the core never blocks on io: commands suspend on requests and prompts and resume on completions; typed keys wait in a type-ahead queue
- line index carries edited flags; saving trims and re-tabs edited lines only; a file ending with a line break keeps it when lines are added
- 46 oracle items recorded in docs/oracle.md
- coverage: every function the scenario binary uses is covered by scenarios, because unit tests and scenarios build separate copies of the library
- rationale: implementation_plan.md sections 3.5 and 6; open points from the spec agent are listed in the command specs
- version bump: 0.7.0 to 0.8.0 (MINOR - editor core)

### 2026-10-31 (v0.7.0, m1 task 7 switch table and spec files)

- wrote docs/spec/commands (55 m1 commands plus 9 not-available stubs) and docs/spec/switches.md from the m guide, with test checklists and proposed oracle ids
- added the switches module: one table entry per switch with its kind and range; booleans use name: and noname: only, numbers are decimal 0 to 65535 unless the spec narrows them, colors one or two hex digits, tabdisp and traildisp a printable one-cell character code, backup none or bak, free text switches accept any text and clear when empty
- vmbuf, shortnames and savescreen are type-checked but have no effect; values are trimmed; unknown switches and invalid values give a message and leave the value unchanged
- also finished ahead of their tasks: Frame and view scrolling (task 8), clipboard model with fnv-1a record (task 13), tools.ini search path and sections plus the Env trait (task 16), macro parser (task 17), and the clap-based cli parser in the mep crate (task 20)
- added clap 4.6.7 with derive and proptest to the mep crate
- recorded ORACLE INI-1 in docs/oracle.md
- version bump: 0.6.0 to 0.7.0 (MINOR - new switches api)

### 2026-10-31 (v0.6.0, m1 task 6 keys and default keymap)

- added the keys module: Key (code plus ctrl, alt, shift), case-insensitive key-name parser and canonical lowercase printer (ctrl+alt+shift+ order), named keys from m guide section 6.2.1 plus enter and space, f1 to f12, num0 to num9, num+ and num-
- a modifier prefix needs a key after it, so alt++ is alt with the plus key; unknown names fail with "unknown key name 'x'"
- added Keymap with the default column of table a.2; unbound printable keys run graphic, all other unbound keys run unassigned
- every default key name and random key names round-trip through parser and printer
- version bump: 0.5.0 to 0.6.0 (MINOR - new keys api)

### 2026-10-31 (v0.5.0, m1 task 5 file table and file io)

- added the io module: Fs trait, worker requests (load, save) with completions, and serve, which runs a request against an Fs; the core never touches the disk
- saving refuses read-only files, renames the old version to .bak with backup:bak, then writes atomically
- added MemFs, an in-memory Fs with injectable failures per operation and path, public so other crates can test with it
- added the file module: File (name, text, modified, readonly, default crlf terminator), FileName (disk path or pseudo-file), FileTable with stable FileId handles, and lexical path resolution
- added OsFs in mep/src/os.rs; write_atomic keeps the old file permissions and removes its temporary file on failure; tested against tempfile directories
- added tempfile 3.27.0 as a dev dependency of mep, and mep-core as a dependency of mep
- recorded ORACLE FILE-BAK-1 in docs/oracle.md
- version bump: 0.4.0 to 0.5.0 (MINOR - new file and io api)

### 2026-10-31 (v0.4.0, m1 task 4 cell model)

- added the cells module: glyphs with byte range, first cell and width; wide characters take two cells, zero-width characters attach to the previous glyph (one cell at line start), tabs expand to the next multiple of 8, invalid bytes and control characters take one cell each
- cell to byte mapping in both directions, box ranges that take a wide character when its first cell is inside, detab, and entab for saving edited lines
- entab converts a run of two or more spaces reaching a tab stop; entab 1 keeps spaces inside double or single quotes; a space carrying a combining mark is never converted
- added unicode-width 0.2.2 to mep-core, pinned because 0.2.3 is younger than two weeks
- recorded ORACLE TAB-1 in docs/oracle.md
- version bump: 0.3.0 to 0.4.0 (MINOR - new cell model api)

### 2026-10-31 (v0.3.0, m1 task 3 line index and terminators)

- added LineIndex: line starts kept in two stacks split at the last edit, absolute before the split and as distance from the end after it, so edits never shift the rest of the file
- added Text in a new text module: gap buffer plus line index, per-line terminators (none, lf, crlf) and byte-exact to_bytes for saving
- a line ends only at a line feed; a lone carriage return stays line content; a final line break is followed by an empty last line
- GapBuffer::delete now returns the removed bytes so Text validates a range once
- property test: the incremental index after random edits equals a full rebuild
- rationale: size rule from implementation_plan.md section 3.5; the text module is an addition to the section 4.4 module table
- version bump: 0.2.0 to 0.3.0 (MINOR - new line index and text api)

### 2026-10-31 (v0.2.0, concurrency and large documents)

- added implementation_plan.md section 3.5: deterministic single-threaded core with worker threads for input, rendering, file io, clipboard and jobs, using std threads and mpsc channels
- commands needing io suspend and resume on a completion event; the macro interpreter is resumable; long core work such as search runs in bounded slices
- size rule: edit cost depends on the edit, not the file; incremental line index, visible-only rendering, delta undo
- added performance tests (section 9.6) with budgets on a 100 mb, 2 million line reference document, and updated the affected m1 tasks
- rationale: owner requirement that the editor never slows down on large documents; a deterministic core keeps scenario tests and the coverage gate workable, and tokio would add a large dependency without speeding up large documents
- version bump: none (plan only)

### 2026-10-31 (v0.2.0, no commits at task end)

- the commit protocol now states that finishing a plan task never triggers a commit; the agent summarizes and waits for the owner
- rationale: owner decision after the task 2 commit was made without asking; the owner reviews every task before it is committed
- version bump: none (instructions only)

### 2026-10-31 (v0.2.0, m1 task 2 gap buffer)

- added GapBuffer in mep-core: byte storage with insert and delete at a byte offset and slice access, bytes kept unchanged including invalid utf-8
- slices returns the parts before and after the gap as two slices, so reads work on a shared reference and never move the gap
- the gap grows to at least the content size with a 64 byte minimum, so storage at least doubles and inserts cost amortized constant time
- out-of-range offsets and reversed ranges return an anyhow error and leave the buffer unchanged
- mep-core re-exports anyhow Result and defines the require macro
- unit tests plus proptest property tests compare random insert and delete sequences against a Vec model
- version bump: 0.1.0 to 0.2.0 (MINOR - new gap buffer api)

### 2026-10-31 (v0.1.0, sh extension for shell scripts)

- renamed scripts/check to scripts/check.sh and updated ci, agents.md and the implementation plan
- shell scripts for linux and macos end in .sh, powershell scripts for windows in .ps1
- rationale: owner decision; the extension shows which platform a script is for
- version bump: none (build tooling only)

### 2026-10-31 (v0.1.0, branch coverage gate, cli parser in mep)

- the coverage gate also requires 100% branch coverage; the check scripts collect with --branch and fail when the json summary reports any uncovered branch
- test modules are excluded from coverage with coverage(off), enabled through a test-only coverage_attribute feature gate
- bundled short command-line flags are accepted
- the cli parser moves from mep-core to the mep binary crate, so clap stays out of the core
- rationale: owner decisions; cargo-llvm-cov has no branch threshold option, and assertions in tests add branches a passing test never takes
- version bump: none (build configuration and plan only)

### 2026-10-31 (v0.1.0, mit license, pinned nightly, dash-only cli)

- licensed the project under mit with a LICENSE file and license metadata in all crates
- pinned rust-toolchain.toml to nightly-2026-10-31
- command-line options use only - and -- on all systems; the original slash forms are dropped and a slash argument is always a file
- updated implementation_plan.md for the nightly pin, the license, anyhow and clap in the allowed crates, and the dash-only cli
- rationale: owner decisions; a dated nightly keeps ci from breaking on a new nightly without a code change
- version bump: none (license and build configuration only)

### 2026-10-31 (v0.1.0, nightly toolchain and anyhow clap)

- switched rust-toolchain.toml to nightly with clippy, llvm-tools-preview and rustfmt; check scripts and ci use the toolchain file only
- added anyhow and clap to the allowed crates
- check.ps1 calls cargo directly with a quoted double dash and checks the exit code after each step
- rationale: owner decision; nightly also makes .rustfmt.toml effective without a separate fmt toolchain
- rationale: switching toolchains left stale llvm-cov data that produced bogus reports; a full cargo llvm-cov clean fixes it
- version bump: none (build configuration only)

### 2026-10-31 (v0.1.0, m1 task 1 workspace setup)

- created the cargo workspace with mep-core, mep-term, mep-clip and mep crates, a stable rust-toolchain.toml with clippy and llvm-tools-preview, and a committed Cargo.lock
- added scripts/check and scripts/check.ps1 running fmt, clippy, doc tests and the llvm-cov gate at 100% lines, functions and regions, plus a --report option for html output
- added a github actions workflow running the check script on ubuntu, macos and windows
- added docs/spec/switches.md, docs/oracle.md and tests/scenarios skeletons
- rationale: fmt runs on nightly rustfmt because stable rustfmt ignores brace_style, trailing_comma, imports_granularity and other .rustfmt.toml options
- rationale: the coverage gate fails when no function is instrumented, so mep-core ships a tested version function that the -V option will use
- rationale: anyhow and clap from the coding skill are not used because the plan limits crates to its allowed list
- version bump: none (0.1.0 is the first manifest version)

### 2026-10-31 (v0.1.0, fill agents.md placeholders)

- filled mission statement, technology stack, code review, development guidelines, security, testing and documentation sections of agents.md
- content taken from implementation_plan.md: clean-room rule, manual as spec, spec then tests then code, oracle items, core isolation behind traits, allowed crates, 100% coverage gate
- license left undecided; the plan calls the project private and names no license
- rationale: agents.md still held template placeholders while the plan had already settled these points
- version bump: none (documentation only, no crate exists yet)

### 2025-10-05 (v0.1.0, initial setup)

- initial AGENTS.md setup
- established core coding standards and conventions
- defined repository structure and governance principles
