# Recent Updates & Decisions

This file is the append-only log of project decisions and notable changes, maintained by coding agents following the `recent-updates` skill. Everything below the marker line is user-owned history: slopctl never overwrites it during init or merge.

<!-- {changelog} -->

### 2026-11-01 (v0.18.0, usetabs switch and a re-save fix)

- added a `usetabs` boolean switch (project addition, default off, switches.md, oracle tab-1): the file tab width becomes `tabstops` instead of 8 for display and re-tabbing, and the leading blanks of edited lines are always saved as tabs, one per `tabstops` columns, whatever `entab` says; inner runs still follow `entab`
- rationale: the owner wanted a setting for inserting tabs instead of spaces. Editing stays on spaces (plan §5.2 detabs a line before every edit), so tabs are produced on save; the owner chose a tab width that follows `tabstops`, so a file saved with one tab per indent level reopens at the width it was typed
- cells.rs: `layout(line, tab)` lays out raw file lines with a given tab width; `glyphs` stays for displayed lines, which have no tabs; `detab` takes the tab width; `entab` takes a `Retab` (mode, tab width, leading-tabs flag), which `Switches::retab()` builds and which replaces `entab_mode()`; only the four places that handle raw lines (shown line, detab before edit, drawing, bracket match column) and saving pass the width
- fixed a save bug that predates this work and that `usetabs` made obvious: a successful save cleared every line's edited flag although the buffer still held the unconverted form (spaces, trailing blanks), so a second save wrote lines byte-exact and undid the first save's re-tabbing and trimming. Lines now stay edited from loading on, which keeps saving idempotent; removed the then unused `clear_edited`
- added scenarios for tab display width, leading tabs on save, `tabinsert` plus `usetabs`, and a second save; unit tests for `layout`/`detab` at width 4 and leading-blank re-tabbing
- verified with the real binary: tab tab and save wrote two tab bytes; indenting the next line and saving again kept the first line's tabs
- set `nousetabs:` (the default) in the owner's tools.ini and examples/tools.ini, since their `entab:0` suggests they keep spaces
- scripts/check.sh passes: fmt, clippy, full test suite, 100% coverage
- no version bump: part of the same uncommitted v0.18.0 work

### 2026-11-01 (v0.18.0, tabinsert replaces tabindent)

- replaced the `tabindent` switch from the previous entry with `tabinsert` (project addition, default off, switches.md, oracle cmd-tab-1) after the owner clarified what they wanted: tab should insert at the cursor, not indent the whole line
- with `tabinsert:` and no arg pending, `tab` inserts spaces at the cursor up to the next `tabstops` multiple; `backtab` removes the blanks back to the previous multiple if only spaces or tabs are left of the cursor, and otherwise (text on the left, or column 0) moves as before. Blanks beyond the end of the line are virtual, so there backtab only moves
- the block indent/outdent of a pending selection is unchanged, and a fresh `arg tab` still moves
- a setting for saving indentation as real tab characters is still open: the owner chose tabs written on save, but file tabs always display 8 wide (oracle tab-1), so one tab per `tabstops:4` columns would not reopen at the width it was typed; asked the owner before going further
- `tabindent` never shipped (uncommitted), so it was renamed outright with no compatibility alias; the owner's tools.ini and examples/tools.ini now set `tabinsert:`
- verified with the real binary on "foo bar": two tabs then backtab left 4 leading spaces, tab mid-line padded to column 12, backtab after text only moved the cursor
- scripts/check.sh passes: fmt, clippy, full test suite, 100% coverage
- no version bump: part of the same uncommitted v0.18.0 work

### 2026-11-01 (v0.18.0, tabindent switch)

- added a `tabindent` boolean switch (project addition, default off, switches.md, oracle cmd-tab-1): with it on and no arg pending, `tab` indents and `backtab` outdents the cursor line instead of moving the cursor, rounding the line's leading blanks to the next or previous `tabstops` multiple, with the cursor moving along with the text
- default off keeps the manual's pure cursor movement; the owner asked for it as a tools.ini setting, and it is now on in their tools.ini and in examples/tools.ini
- a fresh `arg` with no movement still moves the cursor, so `arg tab` keeps building a selection; `backtab` returns false when the line has no leading blanks; below the end of the file `tab` creates the line, `backtab` does nothing
- `tab`/`backtab` now share one `tab_key` helper in commands/movement.rs, and `reindent` works one line at a time and returns the old and new width so the cursor can follow
- fixed before it shipped: an outdent below the end of the file asked for the start of a line that does not exist and panicked; outdent now only touches a line that has blanks to remove
- verified with the real binary: on "  beta" with the cursor on the e, tab gave "    beta" with the cursor still on the e
- scripts/check.sh passes: fmt, clippy, full test suite, 100% coverage
- no version bump: part of the same uncommitted v0.18.0 work

### 2026-11-01 (v0.18.0, word selection macros and tab/backtab block indent)

- added `Selectmword:=Argcol Mword` / `Selectmword:Shift+Ctrl+Left` and `Selectpword:=Argcol Pword` / `Selectpword:Shift+Ctrl+Right` to the owner's tools.ini and examples/tools.ini, for select-by-word. Unlike the earlier `Shift+Left`/`Shift+Right` character-selection macros, `mword`/`pword` can cross line boundaries, so plain `Arg Mword`/`Arg Pword` would risk a box selection (not a flowing text range) whenever a word jump lands on a different column on a different line, since commands like `copy` have `boxarg`. Used `Argcol` instead, which always returns the correct flat stream whenever the anchor and cursor are on different lines, regardless of column; verified against the real binary for both the same-line and cross-line, mismatched-column cases
- gave `tab`/`backtab` a new behavior (project addition, not in the 1988 manual, oracle cmd-tab-1): pressing either while a selection is already pending (anchor differs from the cursor, built any way at all) now indents/outdents every selected line instead of moving to the next/previous tab stop, rounding each line's own leading whitespace to the next/previous `tabstops` multiple, then ends the selection and puts the cursor at column 0 of the first line
- both commands are `cursorfunc`, so the dispatcher always routes them straight to the cursor-move/selection-extend path and never through `classify()`; the new behavior had to be implemented as a direct check inside `tab`/`backtab` themselves (`self.arg`'s anchor vs. the cursor) rather than through the usual arg-kind flag mechanism, since there's no flag that changes cursorfunc's dispatch routing
- accepted tradeoff, discussed with the owner beforehand: using `tab`/`backtab` themselves as one of the movement keys while actively building a selection (e.g. `arg right tab tab copy`) stops extending past the first press once the selection is non-trivial, since the next press now consumes it instead
- added `reindent`/`selected_lines` helpers in commands/movement.rs; updated tab.md, backtab.md, implementation_plan.md and oracle.md; added scenario coverage in movement.mep-test (indent, outdent, rounding from a non-multiple, outdent at column 0, a blank line in the selection, a single-line selection, and the existing plain `arg tab` no-selection case as a regression guard)
- re-verified against the real binary: `shift+down shift+down tab` indents all three lines of a file to 4 spaces, re-selecting and pressing `shift+tab` outdents back to the exact original content
- scripts/check.sh passes: fmt, clippy, full test suite, 100% coverage
- version bump: 0.17.0 to 0.18.0 (minor - new command behavior, backward compatible)

### 2026-11-01 (v0.17.0, character selection macros and an argcol geometry fix, 23:50)

- added `Selectleft:=Arg Left` / `Selectleft:Shift+Left` and `Selectright:=Arg Right` / `Selectright:Shift+Right` to the owner's tools.ini and examples/tools.ini, for character-by-character selection. No code change: `left`/`right` never change the cursor's line, so a selection built purely from them always hits the existing same-line case, which is already a proper character-range (streamarg) selection; verified against the real binary in both directions
- corrected argcol's geometry after the owner worked through a concrete example (cursor at line5 col7, one shift+down): the far end of the selection (wherever the cursor currently is) was wrongly treated as a whole line; it should be trimmed to the cursor's own column too, exactly mirroring how the anchor's own line is trimmed. The corrected rule is symmetric and direction-agnostic: whichever of the anchor and cursor comes first in the file is trimmed from its own column onward, whichever comes second is trimmed up to its own column, everything strictly between is whole
- this simplified the implementation rather than complicating it: `selection()`'s argcol branch in editor.rs collapses to returning the plain two-point stream already computed at the top of the function (no more snapping the far end to column 0); `render.rs`'s live highlight simplified the same way, with no direction-dependent branching left; `stream_window`/`stream_rows`/`insert_stream` and every command's multi-line stream arm needed no changes at all, since they only ever consumed whatever `start`/`end` they were given
- this also proved `sdelete`/`sinsert` never needed the `colarg` flag at all: lacking `linearg`, they already fall back to the exact same flat stream for any multi-line movement regardless of `argcol`, so granting them `colarg` was a no-op; removed it from both in cmd.rs
- updated argcol.md, the per-command notes in copy.md/ldelete.md/linsert.md/sdelete.md/sinsert.md/arg.md, oracle.md's cmd-argcol-1 row, and implementation_plan.md's note to describe the corrected geometry; rewrote select.mep-test's expectations and added a render.rs unit test case plus a scenario case (a selection whose topmost line isn't at the top of the file/window) to close two coverage gaps the fix and the simplification each opened
- re-verified against the real binary with a corrected test paragraph (the earlier one happened to have a short third line that coincidentally clamped to the old, wrong answer): selecting from column 28 down two lines now correctly stops mid-line-three instead of taking it whole
- scripts/check.sh passes: fmt, clippy, full test suite, 100% coverage
- no version bump: still the same uncommitted v0.17.0 work, corrected before ever being committed

### 2026-11-01 (v0.17.0, argcol and a column-anchored selection, 23:30)

- added a real `argcol` command (project addition, no manual equivalent, docs/spec/commands/argcol.md, oracle cmd-argcol-1) after discussing the earlier shift+down/shift+up macros' whole-line limitation with the owner for their real use case: selecting a sentence that wraps across lines in the middle of a paragraph
- `argcol` starts/continues arg mode exactly like `arg` (same anchor/count-keeping rules) but never raises the count itself, and marks the pending state (`ArgState` gained a `col: bool`) so a selection built from it keeps the anchor's column instead of becoming a whole-line selection: the anchor's line is trimmed to the anchor's column (tail going down, head going up), every other line between the anchor and wherever the cursor currently is comes in whole, and the cursor's own line is not part of it yet; pressing the matching movement again extends it by exactly one more whole line; reversing direction recomputes cleanly since nothing but the live anchor and cursor is used
- this is a dedicated new function rather than a change to plain `arg`/`down`/`up`, and is gated by a new `colarg` flag added to the ten commands that already take linearg/boxarg/streamarg (assign, copy, execute, ldelete, linsert, qreplace, replace, sdelete, shell, sinsert): ordinary `arg down`/`arg up` selections, numeric-count selections and marker selections are all completely unaffected, since they either never reach this code path or explicitly opt out of the new marker
- the column-anchored shape needed no new `arg` variant at all: it is carried as an ordinary `arg::stream`, the same type `sdelete`/`sinsert`/`execute`/`replace`'s `span::stream` already use for flat multi-line ranges, so those five needed zero code changes once granted the flag; `copy`, `ldelete` (cut), `linsert` and `assign`/`shell` gained a new multi-line stream arm each; `edit.rs` gained shared `stream_window`/`stream_rows` helpers (and `insert_stream`, factored out of `sinsert` and reused by `linsert`), and `replace.rs`'s `span::window` now delegates to `stream_window` instead of duplicating the same trim rule
- `render.rs`'s live selection highlight (`hgcolor`) was taught the same geometry so what's drawn always matches what a command would actually act on
- owner's decisions along the way: sdelete/sinsert honor the new shape (so a deletion/insertion always matches the highlight); linsert is the exact inverse of the matching deletion; a downward copy includes the trailing line break; a command without colarg falls back to today's behavior unchanged
- updated the owner's tools.ini and examples/tools.ini: `Selectdown`/`Selectup` macro bodies changed from `Arg Down`/`Arg Up` to `Argcol Down`/`Argcol Up`, key bindings (shift+down/shift+up) unchanged
- verified against the real binary, not just scenario tests: opened a wrapped three-line paragraph, moved the cursor to the start of a mid-line sentence, pressed shift+down twice, copied, and saved the clipboard to a real file — got exactly that sentence's tail plus the next whole line, matching the design exactly (including the known, accepted oversell of also grabbing the next sentence's start on that second whole line)
- scripts/check.sh passes: fmt, clippy, the full workspace test suite including new scenario cases in select.mep-test and additions to arg.mep-test/edit.mep-test/config.mep-test/shell.mep-test, and the coverage gate at 100% lines/functions/regions/branches; getting there took real iteration (adding a scenario left the selection pending across a frame check for the "up" direction specifically, since `highlighted()` is a function the scenario harness calls and per the project's own coverage-across-binaries rule must be fully covered by scenarios, not patched over by unit tests alone)
- fixed a second accidental loss of the `<!-- {changelog} -->` marker line in this file during this session's editing; restored it again, this time double-checked
- version bump: 0.16.0 to 0.17.0 (MINOR - new command, new flag, backward compatible)

### 2026-11-01 (v0.16.0, shift+down and shift+up line-selection macros, 22:30)

- added `Selectdown:=Arg Down` / `Selectdown:Shift+Down` and `Selectup:=Arg Up` / `Selectup:Shift+Up` to the owner's tools.ini and to examples/tools.ini, for GUI-style growable line selection
- these are plain macros, no new code: `arg` run again while a selection is already pending keeps the anchor and extends it rather than resetting (oracle cmd-arg-2, already implemented), and a cursor function like `down`/`up` does not consume the pending arg (editor.rs's dispatch only clears `self.arg` for non-cursorfunc functions), so repeated shift+down/shift+up keep extending the same selection; any other key in between ends it, and the next shift+down starts fresh
- this is not quite what was asked for literally: mep's selection model only has four shapes (none, one-line stream, whole lines, box) and pure vertical movement always classifies as whole lines (editor.rs's `selection()`: same column = linearg), so the first shift+down selects the whole current line plus the next line, not "cursor position to end of line" as a partial first line; there is also no live visual highlight while the selection grows (the 1988 model only highlights the arg anchor itself, hgcolor, not a dynamic extent), so the only feedback is the cursor moving until a command like copy consumes the selection
- verified against the real binary (not just reasoned about): two shift+down presses then ctrl+c on a 5-line file, then `arg arg "file" setfile` to save the clipboard to a real file, produced exactly the first three lines
- no version bump: tools.ini macros only, no code changed

### 2026-11-01 (v0.16.0, redo implemented, the cut spec from the prior entry was a mistake, 22:00)

- correction to the prior entry (21:00): docs/spec/commands/cut.md and oracle cut-1 were wrong and have been deleted. `ldelete` already is the manual's cut: p. 98 and ldelete.md already say every form copies the deleted text to the clipboard with its shape before removing it, for exactly the noarg/nullarg/linearg/boxarg/streamarg/numarg shapes a cut needs. No new command was ever needed; added a note to ldelete.md so this is not missed again
- implemented `redo` per redo.md and oracle redo-1: `Group` (editor.rs) gained an `after: Pos` field sealed in `begin_command` once a command finishes (for the file actually being viewed; a second file an extension also touched in the same command keeps `after` defaulted to its own `cursor`, same as before); `FileState` gained a `redo: Vec<Group>` stack; `undo_step` now pushes the popped group onto `redo`; the new `redo_step` reapplies a group's edits forward, sets the cursor to `after`, marks the file modified (undo deliberately leaves `modified` alone per oracle cmd-undo-1, but redo moving forward past a save point needs it set, or a save after undo then redo would leave `modified` false while the buffer no longer matches disk) and pushes the group back onto `undo`; any real edit through `replace` (not `redo` itself, which bypasses it) clears `redo`; the two existing `undo.clear()` sites (reread in files.rs, `fill_pseudo` in info.rs) now also clear `redo`
- registered `redo` in cmd.rs (noarg only, no modifies flag, matching how `undo` itself is classified) and added scenario tests in tests/scenarios/edit.mep-test and files.mep-test (redo after undo, redo of a deleted line, three steps back and forward, a new edit clearing the redo stack, undo/redo toggling, the undocount bound applying to redo too, redo with nothing to redo, `arg redo` rejected, and refresh clearing a pending redo)
- fixed the owner's personal tools.ini (~/.config/mep/tools.ini): ctrl+x now binds `ldelete` (real cut, using the existing command) instead of the nonexistent `cut`; ctrl+y now binds the new `redo` instead of `ldelete`'s old binding there; synced examples/tools.ini in the repo to match (per agents.md, it is the owner's working configuration), including the extmake:c gcc line
- scripts/check.sh passes: fmt, clippy with -D warnings, the full workspace test suite including the new scenarios, and the coverage gate at 100% lines/functions/regions/branches; cargo-llvm-cov was not installed in this session's sandbox and had to be installed first
- version bump: 0.15.2 to 0.16.0 (MINOR - new `redo` command)

### 2026-11-01 (v0.15.2, cut and redo specs for a cua keymap, 21:00)

- the owner asked for a modern cua keymap in their own tools.ini (copy ctrl+c, cut ctrl+x, paste ctrl+v, undo ctrl+z, redo ctrl+y) instead of mep's 1988 defaults
- copy, paste and undo are rebinds of existing commands, applied directly to the owner's tools.ini; cut and redo do not exist in mep at all (the 1988 manual has neither, and undo.md already flagged redo's wording in implementation_plan.md §8 task 12 as needing the owner's confirmation)
- wrote docs/spec/commands/cut.md and redo.md (project additions, oracle cut-1 and redo-1 in docs/oracle.md) per the spec-then-tests-then-code rule; cut is its own command rather than a copy+delete macro because arg mode consumes the selection after the first function, so a macro would copy the right shape but delete only the current line; redo needs a cursor-after field added to editor.rs's Group struct (currently only stores cursor-before, which is all undo needs) — left as an implementation note, not yet coded
- the owner's personal tools.ini (~/.config/mep/tools.ini, not the repo's examples/tools.ini) now has copy:ctrl+c, paste:ctrl+v and undo:ctrl+z added; this dropped ppage:ctrl+c (ppage keeps pgdn), insertmode:ctrl+v (insertmode keeps ins) and plines:ctrl+z (plines now has no key) by the owner's choice; ctrl+x and ctrl+y are untouched (still down and ldelete) until cut and redo are implemented
- no version bump: documentation only, no code changed yet

### 2026-11-01 (v0.15.2, ghostty eats copy and paste keys, 20:00)

- the owner's copy and paste did not work in ghostty on linux, even within a single mep window; ghostty +show-config --default shows it binds ctrl+insert to copy_to_clipboard:mixed and shift+insert to paste_from_selection itself, so neither key reaches mep; freeing both with keybind = ctrl+insert=unbind and keybind = shift+insert=unbind is the fix
- rationale: this is the same class of problem as the cmd+c/cmd+v note already in agents.md: ghostty's own default keybinds claim a key before mep's terminal input ever sees it
- separately, an automated two-process test (arg, down, down, copy, then a fresh process pasting) confirmed the real wayland system clipboard round-trip for a line-shaped selection works correctly end to end once the right keys reach mep
- no version bump: documentation only

### 2026-11-01 (v0.15.2, shell verified on linux, 19:30)

- the owner ran shell by hand on linux: noarg shell saves and drops to the interactive shell, meta shell skips the save, arg shell on ls -l and on a failing ls /does-not-exist both ran correctly, ctrl+c during arg shell on ping localhost returned cleanly, and a line selection of two echo commands ran each in order
- the pending item for shell by hand on linux is done; m4's manual shell and compile checks are now closed on windows and linux, only the shift+f3 kitty-protocol gap on linux and macos stays open under m4
- no version bump: documentation only

### 2026-11-01 (v0.15.2, compile verified on linux, 19:00)

- the owner ran compile by hand on linux with extmake:c gcc -fsyntax-only %s; arg compile shows the compile errors in the editor, gcc's filename:row:column: message form parses correctly (oracle compile-1)
- the pending item for compile by hand on linux is done; shell by hand on linux stays open
- no version bump: documentation only

### 2026-11-01 (v0.15.2, compile verified on windows, 18:30)

- the owner ran compile by hand on windows with extmake:c clang -fsyntax-only %s from a TOOLS.INI in %APPDATA%\mep; arg compile shows the compile errors in the editor
- the pending item for compile by hand on windows is done; shell and compile by hand stay open on linux
- no version bump: documentation only

### 2026-11-01 (v0.15.2, altgr characters on windows, 18:00)

- a character key with both ctrl and alt is typed text when the character is not an ascii letter or digit; on windows altgr+7 on a german keyboard now inserts { instead of being the unbound key ctrl+alt+{
- rationale: windows reports altgr as ctrl+alt and crossterm passes the modifiers through, so braces, brackets, backslash, pipe, at sign and euro could not be typed on such layouts
- trade-off: ctrl+alt with punctuation or non-ascii characters can no longer be bound; no default binding uses them, ctrl+alt with letters and digits still can
- version bump: 0.15.1 to 0.15.2 (PATCH - bug fix)

### 2026-11-01 (v0.15.1, shell verified on windows, 17:00)

- the owner ran shell by hand on windows: arg dir shell, a failing dir command, and ctrl+c during ping -t all behaved as shell.md requires
- the pending items for shell on windows and for ctrl+c during shell on windows are done; shell on linux and compile by hand on linux and windows stay open
- no version bump: documentation only

### 2026-11-01 (v0.15.1, shift f3 on windows, 16:45)

- the owner confirmed by hand that shift+f3 reaches compile on windows; without a compile it shows no more compile messages, as compile.md requires
- the pending shift+f3 item stays open only for linux and macos terminals without the kitty keyboard protocol
- rationale: on windows crossterm reads console key events instead of escape sequences, so the csi 1;2r cursor report clash cannot occur
- no version bump: documentation only

### 2026-11-01 (v0.15.1, ignore object files, 16:15)

- .gitignore ignores *.obj files
- rationale: a filter test run with CC=cl.exe leaves object files in crates/mep, the test working directory, and they showed up as untracked files
- no version bump: repository housekeeping only

### 2026-11-01 (v0.15.1, local windows check notes, 16:00)

- agents.md notes that the filter test needs a gcc-style c compiler in $CC; a CC=cl.exe from a visual studio environment fails to link, CC=clang passes
- agents.md notes how to refresh a working tree that still has crlf files from before the eol=lf rule
- rationale: the first local run of check.ps1 on windows failed on both points although the code and ci were green
- no version bump: documentation only

### 2026-11-01 (v0.15.1, examples folder)

- new examples folder with the owner's TOOLS.INI; it loads without errors and its switches and begfile and endfile bindings take effect
- rationale: a sample configuration for users, and a check that the project additions work together in a real file
- no version bump: no code change

### 2026-11-01 (v0.15.1, ci green on all platforms)

- the check scripts pass in ci on ubuntu, macos and windows, coverage gate included, so the pending item for check.sh on linux and check.ps1 on windows is done
- no version bump: documentation only

### 2026-11-01 (v0.15.1, process tests on every os)

- the runner test in process.rs and the shell test in host.rs run on every os with the host's shell instead of only on unix; cmd gets its own command lines, crlf output and hostname as the interactive program
- rationale: on windows these tests were skipped, so the coverage gate missed the runner and shell code there; the first full windows ci run passed every test and failed only this gate
- ci status: check.sh passes on linux; on windows all tests pass, including the filter extension built with clang
- no version bump: test code only

### 2026-11-01 (v0.15.1, typed paths in the core)

- added typed-path 0.12.3 to mep-core, pinned because 0.12.4 and 0.13.0 are younger than two weeks; the owner approved it
- the core keeps paths as typed-path utf-8 paths in the style of env os instead of std paths in the style of the host; Os::path builds them, the binary converts to std paths in os.rs, host.rs and process.rs
- resolve is now join plus normalize; history parse takes the os; memfs keys files by their slash form so fixtures match unix and windows paths
- scenarios with os windows now show windows paths such as backslash work backslash a.txt
- local_offset builds the zone file path as a string, so its test passes on windows
- rationale: the first windows ci run failed because std joins with a backslash on windows while the tests use unix paths; typed paths make path logic independent of the host
- version bump: 0.15.0 to 0.15.1 (PATCH - windows path fix, no new features)

### 2026-11-01 (v0.15.0, no fail fast in the check scripts)

- check.sh and check.ps1 run the doc tests and the coverage test run with --no-fail-fast
- rationale: the first windows run stopped at the first failing test binary, so the mep-core and scenario tests never ran; one run now shows every failure
- no version bump: no code change

### 2026-11-01 (v0.15.0, lf line endings on windows)

- new .gitattributes with text=auto eol=lf: text files are checked out with lf on every platform
- rationale: the first windows ci run failed in cargo fmt with incorrect newline style in every rust file; the runner checks out with crlf, while rustfmt is set to newline_style unix and .editorconfig to lf
- all tracked files are already lf in the index, so nothing is renormalized
- no version bump: no code change

### 2026-11-01 (v0.15.0, begfile and endfile)

- new project functions begfile and endfile: cursor functions that move to the start and the end of the file, as arg mpage and arg ppage, and extend a selection in arg mode (oracle cmd-begfile-1)
- no default keys: ctrl+pgup and ctrl+pgdn stay mpara and ppara as in table a.2; the owner's sample tools.ini assigns them
- sample configurations may now add a function, not only a switch, as a project addition after asking the owner
- the mission statement no longer calls the project private; the repository is public
- rationale: the owner's sample tools.ini from tenox7/ntutils uses begfile and endfile, and mep reported unknown function
- version bump: 0.14.0 to 0.15.0 (MINOR - new functions)

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
