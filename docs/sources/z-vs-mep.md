# Z (1986) vs MEP (1988) — Divergence Catalog and Spec Traps

> **Status (2026-10-31):** Z personality dropped. MEP behavior wins every conflict below; no Z switches, no Z keymap. The catalog stays as reference: the Z column explains where M's wording is thin, and rows marked ORACLE still need the real MEP binary because M is ambiguous or contradicts itself.

Derived only from the manuals and the MSJ article. "Z" = Z User's Guide v7.43; "M" = Microsoft Editor User's Guide (M/MEP), Feb 1988; "MSJ" = Notenboom, Sep 1988. Every row marked ORACLE needs confirmation against the real binary.

## 1. Renamed functions

| Z | M | Notes |
|---|---|---|
| pick | copy | M: "Clipboard"; Z: "pick buffer" |
| put | paste | same semantics for line/box/stream payloads |
| push | shell | |
| (predefined macros) curdate, curday, curtime, curuser, curfilenam, curfileext | functions of the same names + curfile | M curfile = full path |
| savecurs / savecur (Z spells both) | savecur | |

Only in M: undo, execute, help, lasttext, graphic, unassigned (as a nameable function), argcompile, copy/paste names.
Only in Z: nothing functional beyond naming.

## 2. Default key bindings

| Function | Z | M (default config) |
|---|---|---|
| arg | ^X | Alt+A |
| meta | ESC | F9 |
| cancel | ^C | ESC |
| assign | ^^ | Alt+= |
| exit | F9 | F8 |
| setfile | ^B | F2 |
| information | F1 | Shift+F1 |
| help | — | F1 |
| initialize | F2 | Shift+F8 |
| refresh | F10 | Shift+F7 |
| compile | ^U | Shift+F3 (argcompile F5) |
| execute | — | F7 |
| psearch / msearch | ^R / ^E | F3 / F4 |
| replace / qreplace | ^O / Ctrl+Enter, ^J | Ctrl+L / Ctrl+\ |
| pick→copy | ^K | Ctrl+Ins, keypad + |
| put→paste | ^G | Shift+Ins |
| ldelete / linsert | ^F / ^D | Ctrl+Y / Ctrl+N |
| sdelete / sinsert | ^S / ^A | Del / Ctrl+J |
| cdelete | Backspace, ^H | Ctrl+G |
| emacscdel | Ctrl+Backspace | Backspace |
| newline | Enter, ^M | — |
| emacsnewl | unassigned | Enter |
| insertmode | ^N | Ins, Ctrl+V |
| begline / endline | End / PgDn (sic) | Home / End |
| home | Home | Ctrl+Home |
| mark | ^P | Ctrl+M |
| ppage / mpage | ^L / ^Q | PgDn, Ctrl+C / PgUp, Ctrl+R |
| plines / mlines | ^T / ^W | Ctrl+Z / Ctrl+W |
| ppara / mpara | Ctrl+PgDn / Ctrl+PgUp | same |
| pword / mword | unassigned (later default added, unnamed) | Ctrl+Right, Ctrl+F / Ctrl+Left, Ctrl+A |
| pbal | ^V | Ctrl+[ |
| push→shell | ^Z | Shift+F9 |
| window | ^Y | F6 |
| setwindow | ^] | Ctrl+] |
| quote | unassigned | Ctrl+P |
| lasttext | — | Ctrl+O |
| undo | — | Alt+Backspace |
| curdate / curtime | Alt+D / Alt+T | unassigned |

M also ships Quick/WordStar, BRIEF and Epsilon keymaps (Table A.2) via MESETUP.

Consequence of the defaults: in M, Enter breaks lines and Backspace joins lines (emacsnewl/emacscdel). In Z, Enter only moves the cursor and Backspace never joins lines, even in insert mode. This is the biggest day-to-day difference.

## 3. Semantic conflicts (M wins; ORACLE rows still need the binary)

| # | Behavior | Z | M | Status |
|---|---|---|---|---|
| S1 | linsert (no arg) | blank line *below* cursor | blank line *above* current line | M |
| S2 | exit with several command-line files | exit saves+quits; *arg exit* advances to next file | exit saves+advances; *arg exit* quits without advancing | M |
| S3 | arg numarg mark | "beginning of the specified line" (base not stated) | line numbers are **0-based** (status line is 1-based) | M — ORACLE (confirm 0-based) |
| S4 | mpara landing | first line of previous paragraph | first blank line preceding current paragraph | M — ORACLE |
| S5 | arg arg textarg put/paste (file insert) | at cursor position | Ch.4: at cursor; Table A.3: above current line (M contradicts itself) | ORACLE |
| S6 | arg markarg replace | stream between cursor and bookmark | "box of text between cursor and marker" | M — ORACLE |
| S7 | meta compile | skip to first message not for current file | same; plus *arg meta compile* = previous message (≤ maxmsg) | M |
| S8 | TOOLS.INI search path | dirs in `USER` env var | dirs in `INIT` env var | M (`INIT`), see trap T1 |
| S9 | TOOLS.INI tags | `[z]`, `[z-x.y]` (DOS version) | `[M]`, `[M-3.20]`, `[M-10.0]`, `[M-10.0R]`, `[M-mono|ega|vga|viking]`; section name = executable name, multi-name tags `[M MEP]` | M |
| S10 | window split naming | arg window = horizontal split at cursor; arg arg = vertical | arg window = "horizontal window (split at the cursor column)"; arg arg = "vertical (split at cursor row)" — wording looks swapped | ORACLE |

## 4. M-only switches
undocount (10), displaycursor, savescreen, shortnames, tabdisp (32), traildisp (0), hgcolor (70), maxmsg (10), noise (50), vmbuf (128), width (80), load (extension).
Shared: entab 1, errcolor 4, fgcolor 7, height 23, hike 4, hscroll 10, infcolor 6, rmargin 72, stacolor 3, tabstops 4, tmpsav 20, vscroll 7, askexit off, askrtn on, autosave on, case off, enterinsmode off, softcr on, trailspace off, wordwrap off, backup undel, extmake, markfile, readonly. Z-only (not implemented): bgcolor 0, debug 0 (macro tracing).

## 5. Command line
Z: `z [/D] [files]`. M: `m [/D] [/e command] [/t] [files]` — /e runs a macro-syntax command at startup, /t marks files temporary (not recorded in the information file). Both: no file → reopen last edited file; several files → exit walks the list.

## 6. Extension ABI (M only; MSJ adds what the guide omits)
- Required exports: `swiTable[]` (name, ptr-or-handler, SWI_BOOLEAN | SWI_NUMERIC | SWI_SPECIAL), `cmdTable[]` (name, fn, reserved 0, flags), `WhenLoaded()`.
- cmdTable flags: KEEPMETA, CURSORFUNC, WINDOWFUNC, NOARG, TEXTARG, BOXSTR, NULLARG, NULLEOL, NULLEOW, LINEARG, STREAMARG, BOXARG, NUMARG, MARKARG, MODIFIES.
- Command signature: `flagType fn(unsigned argData, ARG *pArg, flagType fMeta)`; ARG = argType + union {noarg, textarg{cArg,y,x,pText}, nullarg{cArg,y,x}, linearg{yStart,yEnd}, streamarg, boxarg{xLeft,yTop,xRight,yBottom}}.
- Callbacks (MSJ Fig. 7): AddFile, BadArg, CopyBox, CopyLine, CopyStream, DelBox, DelFile, DelLine, DelStream, DoMessage, Display, fExecute, FileLength, FileNameToHandle, FileRead, FileWrite, GetCursor, GetLine, KbHook, KbUnHook, MoveCur, pFileToTop, PutLine, RemoveFile, ReadChar, ReadCmd, Replace, SetKey.
- Loaded via `load:` switch (any time, incl. via assign). Original file extensions: .mxt (real mode), .dll (OS/2).

## 7. Pseudo-files (MSJ)
Names in angle brackets; behave like files but cannot be saved without an explicit filename. Editor-created ones have special meaning: `<clipboard>` is rewritten on every copy or block delete. Extensions create their own (`<filter-file>`) with FileNameToHandle → AddFile → FileRead. M's help list and information file are presented as browsable files too.

## 8. Macro semantics worth testing
- A pending arg is passed to the first function in the macro that takes an argument; a macro may end with an "open arg" consumed by whatever runs next.
- The screen is not redrawn during a macro until input is required (switching files inside a macro is invisible).
- meta is a toggle whose return value is the new state (MSJ `bigprint` relies on this).
- Labels `:>l`, jumps `=>l`, `+>l` (if TRUE), `->l` (if FALSE); bare `=>`/`+>`/`->` exits the macro.
- M: max 1024 macros; one definition per line.

## 9. Regex dialect (Z and M agree)
`\ ? [..] [~..] ^ $ X* X+ X@ X# (a!b!c) ~X X^n {..} :letter`; `*`/`+` minimal, `@`/`#` maximal; alternation tries left to right with backtracking; `{}` tag groups → `$0`–`$9` in replacement; predefined `:a :b :c :d :f :h :i :n :p :q :w :z`. Arg arg = regex, single arg = literal.

## 10. Spec traps
- T1: Z's `USER` env var is a directory list, but on Unix `USER` is the login name. Don't use it. Use `INIT`, then the platform config dir, then home.
- T2: Legacy terminal encoding collides with defaults: Ctrl+M = Enter (M's mark), Ctrl+[ = Esc (M's pbal), Ctrl+H = Backspace, Ctrl+I = Tab, ^J = LF; Alt+key arrives as an Esc prefix (M's cancel *is* Esc). Ctrl+Backspace, Ctrl+Enter, Shift/Ctrl+Ins and keypad +/− are often indistinguishable. The VT backend needs the kitty keyboard protocol (CSI u) with an Esc-timeout fallback.
- T3: Z manual typos (relevant only when using Z as a secondary source): "X+ is shorthand for XX+" (should be XX*); `:h` = `[0-9a-zA-Z]#` (M fixes to `[0-9a-fA-F]#`); refers to a nonexistent `<switch>` command (means assign); "[a-zA-z0-9]".
- T4: M curuser: Ch.4 says USER, Table A.3 says MAILNAME.
- T5: M 250-character line limit — do not reproduce.
- T6: Trailing spaces are trimmed unless `trailspace`; entab:1 retabs only outside quotes on output; unedited lines must round-trip byte-exact.
- T7: Status line fields (M, up to 8): file, type, "modified", "NL" (LF-only file), length, position, "insert", "meta".
