# Switches

One section per switch: type, default, accepted values, effect, M1 status and the manual page it comes from (cited as "M guide p. N"). The M1 switch set is fixed in IMPLEMENTATION_PLAN.md §6.13. Where the plan's default differs from the manual, the plan wins and the section says so.

## General rules

Switches are set in TOOLS.INI (one per line, plan §6.12) or at runtime with `Arg textarg Assign` (M guide p. 57; assign.md). Names are case-insensitive (M guide p. 3).

Syntax:
- Numeric: `switchname:numericvalue` (M guide p. 57), e.g. `hscroll:20`. Decimal, except the color switches, which are hexadecimal (`errcolor:0E`, p. 57–58).
- Boolean: `switchname:` turns it on, `noswitchname:` turns it off (M guide p. 60–61), e.g. `case:`, `noaskrtn:`. A value after the colon (`case:on`) is invalid; the manual defines no other form.
- Text: `switchname:textvalue` (M guide p. 62), e.g. `backup:none`.

Leading and trailing blanks around the value are ignored; blanks inside a text value are kept (`readonly:attrib -r`).

Errors (plan §6.12, §6.13):
- An unknown switch name gives an error message and does not abort: TOOLS.INI loading goes on with the next line, `assign` returns false.
- An invalid value gives an error message and leaves the switch unchanged.
- At startup the errors are collected and shown on the dialog line after loading (plan §6.12).
- Message wording is a project choice (e.g. `unknown switch: foo`, `invalid value for tabstops: x`).

Numeric ranges: the manual gives no ranges. Unless a section says otherwise, a numeric switch accepts a decimal integer from 0 to 65535 (project choice). Signs, fractions and hex digits are invalid for decimal switches.

## undocount
- Type: numeric. Default: 10 (M guide p. 60, p. 9; plan §6.13).
- Values: 0–65535. 0 keeps no undo steps.
- Effect: the number of editing steps kept per file for `undo` (plan §5.7). Lowering it drops the oldest steps of every file at once.
- M1: yes.
- Tests: default 10 (eleven changes, only ten undone); `undocount:2` → two; `undocount:x` → error, still 10.

## displaycursor
- Type: boolean. Default: off (M guide p. 61; p. 7).
- Effect: off → the status line position field shows the window's upper-left corner (`Window=(1,1)`, p. 7); on → the cursor position. Both 1-based (plan §5.4).
- M1: yes.
- Tests: default shows the window position; `displaycursor:` shows the cursor position; `displaycursor:yes` → error, unchanged.

## tabstops
- Type: numeric. Default: 4 (M guide p. 60).
- Values: 1–255 (0 has no meaning; project choice).
- Effect: distance between the logical tab stops used by `tab` and `backtab` (p. 93, p. 109). It does not change how tab characters in the file are displayed (8 cells, plan §5.2, ORACLE TAB-1).
- M1: yes.
- Tests: default: `tab` from 0 → 4; `tabstops:8` → 8; `tabstops:0` and `tabstops:four` → error, unchanged.

## tabdisp
- Type: numeric. Default: 32, a space (M guide p. 60).
- Values: the decimal code of a printable single-cell character: 32–126 or a Unicode code point ≥ 160 that is printable and one cell wide. Project decision: the manual says "ASCII value" and meant the DOS code page (where e.g. 250 was a middle dot); we use Unicode code points (plan §3.2: no code pages). See Open points.
- Effect: the character used to draw the cells a tab expands to (plan §5.2).
- M1: yes.
- Tests: default shows tabs as spaces; `tabdisp:183` shows `·`; `tabdisp:7` and `tabdisp:x` → error, unchanged.

## entab
- Type: numeric. Default: 1 (M guide p. 59; plan §5.2).
- Values: 0, 1, 2. Anything else is invalid.
- Effect when an edited line is written (unedited lines are written byte-exact, plan §5.2): 0 = no tabs, white space stays spaces; 1 = runs of several spaces outside quoted strings (quotes `"` or `'`, per the Z guide) become tabs; 2 = all runs of several spaces become tabs (p. 59). Tab positions use the file tab width 8. ORACLE TAB-1.
- M1: yes.
- Tests: default 1: edited line `a        "x   y"` saved with a tab before the quote and spaces inside it; `entab:0` → no tabs; `entab:2` → tabs inside the quotes too; `entab:3` → error, unchanged.

## traildisp
- Type: numeric. Default: 0 = off (M guide p. 60).
- Values: 0, or a character code as for `tabdisp`.
- Effect: trailing spaces are drawn with this character. Per the manual (p. 60) it "has no effect unless the trailspace switch is turned on". Plan §5.3 describes `traildisp` without that condition; we follow the manual. See Open points.
- M1: yes.
- Tests: default: trailing spaces invisible; `trailspace:` + `traildisp:46` → shown as `.`; `traildisp:46` with `trailspace` off → no change; `traildisp:-1` → error.

## trailspace
- Type: boolean. Default: off (M guide p. 61).
- Effect: off → trailing spaces are removed from a line when that line is edited; unedited lines are never touched (plan §5.3). On → trailing spaces are kept.
- M1: yes.
- Tests: default: typing at the end of `ab  ` trims; `trailspace:` keeps; `trailspace:1` → error, unchanged.

## case
- Type: boolean. Default: off (M guide p. 61).
- Effect: on → `psearch`/`msearch` (and `replace`/`qreplace` in M2) compare case-sensitively; off → case-insensitive (Unicode simple case folding; psearch.md).
- M1: yes.
- Tests: default: `Procedure` matches `procedure`; `case:` → no match; `case:x` → error, unchanged.

## enterinsmode
- Type: boolean. Default: off (M guide p. 61).
- Effect: the editor starts in insert mode instead of overtype mode. Only the start state: setting it later (with `assign` or `initialize`) does not change the current mode (project choice).
- M1: yes.
- Tests: default starts in overtype; `enterinsmode:` in TOOLS.INI starts in insert (`insert` on the status line); `enterinsmode:on` → error, overtype.

## softcr
- Type: boolean. Default: on (M guide p. 61).
- Effect: `newline` and `emacsnewl` choose the column of the new line from the surrounding text (newline.md; C-specific rules ORACLE CMD-NEWLINE-1). Off → column 0.
- M1: yes.
- Tests: default indents after `    foo`; `nosoftcr:` → column 0; `softcr:no` → error, unchanged.

## wordwrap
- Type: boolean. Default: off (M guide p. 61).
- Effect: typing beyond `rmargin` breaks the line (graphic.md, ORACLE CMD-GRAPHIC-1).
- M1: yes.
- Tests: default no break; `wordwrap:` breaks; `wordwrap:1` → error, unchanged.

## rmargin
- Type: numeric. Default: 72 (M guide p. 60).
- Values: 1–65535.
- Effect: the right margin column for `wordwrap` (p. 60). Cell column, 1-based as in the status line; a character placed in cell index ≥ `rmargin` (0-based) is "to the right of the margin".
- M1: yes.
- Tests: default 72; `rmargin:10` with `wordwrap:` breaks at 10; `rmargin:0` and `rmargin:x` → error, unchanged.

## autosave
- Type: boolean. Default: on (M guide p. 61).
- Effect: switching away from a modified file saves it (setfile.md). Off → the file stays modified in memory.
- M1: yes.
- Tests: default saves on `setfile`; `noautosave:` does not; `autosave:x` → error, unchanged.

## askexit
- Type: boolean. Default: off (M guide p. 61).
- Effect: `exit` asks for confirmation before quitting (exit.md).
- M1: yes.
- Tests: default quits without asking; `askexit:` asks; `askexit:y` → error, unchanged.

## backup
- Type: text. Default: `none`. Project decision (IMPLEMENTATION_PLAN.md §3.1, §6.13): the manual's default is `undel` (p. 62), which needs the UNDEL program and is dropped.
- Values: `none`, `bak` (case-insensitive). `undel` and anything else are invalid.
- Effect: `none` → the old file is overwritten (atomic temp file and rename, plan §6.10). `bak` → before saving, the previous version is renamed to the file name with its extension replaced by `.bak` (p. 62 "changed to .BAK"; `Makefile` → `Makefile.bak`), replacing an existing `.bak`.
- M1: yes.
- Tests: default: no `.bak` created; `backup:bak` → `a.c` saved, old content in `a.bak`; `backup:undel` and `backup:` → error, unchanged.

## readonly
- Type: text. Default: empty (no command) (M guide p. 63; plan §6.13).
- Values: any text; empty clears it.
- Effect: when a save would overwrite a read-only file, this command is run with the file name appended (e.g. `readonly:attrib -r`, p. 63; on Unix `readonly:chmod u+w`), then the save is retried. Empty → the user is asked for a new name to save under (p. 63).
- M1: yes. Running the command needs a child process in M1 (plan §3.4 `std::process`).
- Tests: default asks for a new name; `readonly:chmod u+w` runs `chmod u+w <file>` (fake process runner) and saves; a failing command → message, not saved. Invalid value: none exists for a free text switch; test that `readonly:` clears a previously set command.

## vscroll
- Type: numeric. Default: 7 (M guide p. 60; plan §3.1).
- Values: 1–65535.
- Effect: lines the window jumps when the cursor leaves it vertically (plan §6.3), and the amount for `plines`/`mlines` without argument (p. 60). Limited in a small window (ORACLE CMD-SCROLL-2).
- M1: yes.
- Tests: default 7; `vscroll:1` (smooth scrolling); `vscroll:0` → error, unchanged.

## hscroll
- Type: numeric. Default: 10 (M guide p. 59).
- Values: 1–65535.
- Effect: columns the window shifts when the cursor leaves it horizontally (plan §6.3).
- M1: yes.
- Tests: default 10; `hscroll:20` (p. 57 example); `hscroll:0` and `hscroll:x` → error, unchanged.

## hike
- Type: numeric. Default: 4 (M guide p. 59).
- Values: 0–65535; at use it is clamped to the window height − 1.
- Effect: after a jump of more than one window (search, `arg ppage`, `restcur` …), the cursor line is placed `hike` lines from the top (plan §6.3; p. 59 "ending line position of the cursor").
- M1: yes.
- Tests: default: search far down puts the match on window row 4; `hike:0` → row 0; `hike:-2` → error, unchanged.

## height
- Type: numeric. Default: the terminal height minus the dialog and status lines. Project decision (IMPLEMENTATION_PLAN.md §3.1, §6.3): the manual's default is 23 (p. 59); we use the terminal size and the switch only takes effect when set explicitly.
- Values: 1–65535.
- Effect: number of lines of the editing window, not counting the dialog and status lines (p. 59). Clamped to the terminal (plan §6.3); keeps applying after a resize.
- M1: yes.
- Tests: default follows the terminal (and a resize); `height:10` on a 30-row terminal → 10-line window; `height:100` → clamped; `height:0` → error, unchanged.

## width
- Type: numeric. Default: the terminal width. Project decision (IMPLEMENTATION_PLAN.md §3.1): the manual's default is 80 (p. 60).
- Values: 1–65535.
- Effect: columns used for display (p. 60), clamped to the terminal.
- M1: yes.
- Tests: default follows the terminal; `width:40` → 40 columns; `width:x` → error, unchanged.

## fgcolor
- Type: numeric, hexadecimal. Default: `07`, light gray on black (M guide p. 59).
- Values: one or two hex digits `0`–`FF` (case-insensitive); the high digit is the background, the low digit the text color, per Table 7.1 (p. 58). `7` = `07`. Background values `8`–`F` stand for `0`–`7` with blinking text in the manual (p. 58); we render them as the bright ANSI background (plan §6.4).
- Effect: color of the editing window text.
- M1: yes.
- Tests: default `07`; `fgcolor:1E` → yellow on blue; `fgcolor:G0`, `fgcolor:100`, `fgcolor:` → error, unchanged.

## hgcolor
- Type: numeric, hexadecimal. Default: `70`, black on light gray (M guide p. 59).
- Values: as `fgcolor`.
- Effect: color of highlighted text (selection, search match, `pbal` match).
- M1: yes.
- Tests: default `70`; `hgcolor:1F`; `hgcolor:XYZ` → error, unchanged.

## stacolor
- Type: numeric, hexadecimal. Default: `03`, cyan on black (M guide p. 60).
- Values: as `fgcolor`.
- Effect: color of the status line.
- M1: yes.
- Tests: default `03`; `stacolor:70`; `stacolor:3G` → error, unchanged.

## infcolor
- Type: numeric, hexadecimal. Default: `06`, brown on black (M guide p. 59).
- Values: as `fgcolor`.
- Effect: color of informative messages on the dialog line.
- M1: yes.
- Tests: default `06`; `infcolor:0A`; `infcolor:-1` → error, unchanged.

## errcolor
- Type: numeric, hexadecimal. Default: `04`, red on black (M guide p. 59).
- Values: as `fgcolor`.
- Effect: color of error messages.
- M1: yes.
- Tests: default `04`; `errcolor:0E` (p. 57 example, light yellow on black); `errcolor:0EE` → error, unchanged.

## noise
- Type: numeric. Default: 50 (M guide p. 59).
- Values: 0–65535; 0 turns the display off.
- Effect (manual): number of lines counted at a time while searching or loading, shown in the lower-right corner (p. 59). M1: accepted and stored; the progress display comes later (plan §6.13).
- Tests: default 50 stored; `noise:0` stored; `noise:x` → error, unchanged.

## tmpsav
- Type: numeric. Default: 20 (M guide p. 60).
- Values: 0–65535.
- Effect (M3): number of files remembered between sessions in the information file (p. 60, p. 38). M1: accepted and stored, no effect.
- Tests: default 20; `tmpsav:5` stored; `tmpsav:x` → error, unchanged.

## markfile
- Type: text. Default: empty (M guide p. 63).
- Values: a file name; empty clears it.
- Effect (M2): file searched for markers not defined in memory, lines `markername filename line column` (p. 63). M1: accepted and stored, no effect.
- Tests: default empty; `markfile:marks.txt` stored; no invalid value for a free text switch, test that `markfile:` clears it.

## maxmsg
- Type: numeric. Default: 10 (M guide p. 59).
- Values: 0–65535.
- Effect (M4): number of compile messages kept for `Arg Meta Compile` (p. 59, p. 95). The manual restricts it to an OS/2 `[M-10.0]` section; we accept it in any section. M1: accepted and stored, no effect.
- Tests: default 10; `maxmsg:20` stored; `maxmsg:x` → error, unchanged.

## extmake
- Type: text. Default: none (M guide p. 62).
- Values: `extension commandline`, e.g. `extmake:c cl /c %s`; the special extension `text` is used for `Arg textarg Compile` (p. 62). May appear several times; a later line for the same extension replaces the earlier one. A value without a command after the extension is invalid.
- Effect (M4): command run by `compile` for files with that extension; `%s` is replaced by the file name or text argument. M1: accepted and stored, no effect.
- Tests: default none; `extmake:c cc -c %s` and `extmake:text make %s` both stored; `extmake:c` and `extmake:` → error.

## askrtn
- Type: boolean. Default: on (M guide p. 61).
- Effect (M4): after `shell`, wait for Enter before returning (p. 61). M1: accepted and stored, no effect.
- Tests: default on; `noaskrtn:` (p. 61 example) stored as off; `askrtn:x` → error, unchanged.

## load
- Type: text. Default: none (M guide p. 62; MSJ "load:filter.mxt").
- Values: a file name of an extension library.
- Effect (M4): loads a C-ABI extension; may be set at any time, also with `assign` (MSJ). M1: accepted and stored, nothing is loaded and no message is shown.
- Tests: default none; `load:filter.so` stored, nothing loaded; `load:` (empty) → error, unchanged.

## vmbuf
- Type: numeric. Default: 128 (M guide p. 60).
- Effect: none (DOS/OS2 virtual-memory buffer). Accepted silently (plan §6.13): a valid value produces no message and has no effect.
- Values: 0–65535; other values give the normal invalid-value error (project choice, so a typo is still reported).
- Tests: default accepted without message; `vmbuf:256` no message, no effect; `vmbuf:x` → error.

## shortnames
- Type: boolean. Default: on (M guide p. 61).
- Effect: none (alternate file by base name, DOS-era). Accepted silently.
- Tests: `shortnames:` and `noshortnames:` no message, no effect; `shortnames:x` → error.

## savescreen
- Type: boolean. Default: on (M guide p. 61).
- Effect: none (DOS screen save/restore; the alternate screen handles this, plan §7). Accepted silently.
- Tests: `savescreen:` and `nosavescreen:` no message, no effect; `savescreen:1` → error.

## Open points
- Numeric ranges (0–65535 by default, minimum 1 for `tabstops`, `rmargin`, `vscroll`, `hscroll`, `height`, `width`) are project choices; the manual gives none.
- `tabdisp`/`traildisp` values: the manual's "ASCII value" referred to the DOS code page; we interpret values as Unicode code points. The owner may prefer a code page 437 mapping for values 128–255 so old TOOLS.INI files keep their look.
- `traildisp` without `trailspace`: manual (p. 60) says no effect; plan §5.3 does not mention the condition. We follow the manual.
- Boolean switches with a value (`case:on`): rejected, because the manual only defines `name:` and `noname:`.
- `vmbuf`, `shortnames`, `savescreen` are type-checked: "accept silently" (plan §6.13) is read as "no message for valid values".
- `enterinsmode` changed at runtime does not switch the current mode.
- `maxmsg` accepted outside an `[M-10.0]`-style section.
