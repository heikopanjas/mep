# assign

Default keys: Alt+=
Source: M guide p. 93 (Table A.3), pp. 46–50 (Ch. 6.2–6.3), p. 57 (Ch. 7.4: switches set with `Arg textarg Assign`), p. 50 (Table 6.1); MSJ "The Keyboard" (`arg exit:alt+x assign`), `load:` via assign; IMPLEMENTATION_PLAN.md §6.12, §6.13, §6.14

## Flags
Derived from the syntax lines on p. 93:
- `Arg Assign` → NULLARG with NULLEOL: the text from the anchor to the end of the line, line break excluded, arrives as a textarg.
- `Arg textarg Assign` → TEXTARG.
- `Arg streamarg Assign` → BOXSTR: the highlighted text of a one-line selection arrives as a textarg.
- `Arg linearg Assign` → LINEARG.
- `Arg boxarg Assign` → BOXARG.
- `Arg ? Assign` → TEXTARG with the exact text `?`.
- No NOARG line, no NUMARG (a numeric text is a plain textarg and fails as an assignment), no STREAMARG beyond one line (a multi-line box-shaped selection is a BOXARG because BOXARG is set).
- Not KEEPMETA, not CURSORFUNC. Not MODIFIES (it changes editor state, not text).
- `Arg Arg` has no special meaning; count 2 behaves like count 1.

## Behavior
Every form feeds one or more lines to the same parser that reads TOOLS.INI statements (plan §6.12):
- `function:key` assigns a function or macro to a key (M guide p. 46); `unassigned:key` removes an assignment (p. 47); `graphic:key` makes a key insert its character (p. 48). Names are case-insensitive (p. 3).
- `name:=item item …` defines or replaces a macro (p. 49; plan §6.14). Limit 1024 macros.
- `switch:value`, `switch:` and `noswitch:` set switches (p. 57; switches.md). `load:` is accepted and stored (M4).
Assignments last for the session only (p. 46). Assigning a key does not remove other keys bound to the same function (p. 46).

### nullarg (`Arg Assign`)
The text from the anchor to the end of the line (no line break) is one statement. Cursor does not move. Returns true if the statement was applied, false otherwise.

### textarg
The text is one statement. `arg exit:alt+x assign` (MSJ). Returns true on success. On failure: false and an error message on the dialog line naming the problem (unknown function or macro name, unknown key name, unknown switch, invalid switch value, macro syntax error). Message wording is a project choice.

### textarg `?` (`Arg ? Assign`)
"Displays the current function assignments for all functions and macros" (p. 93). This needs a browsable list (a pseudo-file like the help list), which is not in M1. M1: message `not available`, returns false.

### streamarg (one line)
The highlighted text is one statement (BOXSTR). Same results as textarg.

### linearg
Each line of the range is a separate statement; blank lines are ignored (p. 93). Lines are applied top to bottom.

### boxarg
Each line of the box (the cells between the left edge and the exclusive right edge, trailing blanks trimmed) is a separate statement (p. 93). Blank box lines are ignored as for linearg.

### Multi-line result
All valid statements are applied; invalid ones are skipped. Returns true only if every non-blank statement succeeded; otherwise false with the message of the first failure. ORACLE CMD-ASSIGN-1.

### meta
No meta form. Meta is ignored and cleared.

## Rejected arguments
- noarg → `invalid argument`, false.
- numarg is not rejected as such: it becomes a textarg (`arg "5" assign`) and fails as an assignment with the parser's message.

## Open points
- Multi-line assign with some invalid lines (apply the rest, return false, first error shown). ORACLE CMD-ASSIGN-1.
- `Arg ? Assign` needs a pseudo-file listing; M1 reports `not available`. The listing arrives with `help` (not in the plan's scope). Project decision.
- The manual prints `Header;ALT+H` (p. 50, marked [sic]); the parser does not accept `;` as a separator, because `;` starts a comment in TOOLS.INI (p. 55).
- Whether a trailing `; comment` is stripped from an assign text the same way as in TOOLS.INI: we strip it outside quoted macro strings, because assign and TOOLS.INI share one parser (plan §6.12).

## Test checklist
- `arg "savecur:ctrl+b" assign` → ctrl+b runs `savecur`, returns true.
- `arg "unassigned:ctrl+a" assign` → ctrl+a shows `ctrl+a is not assigned to any editor function`.
- `arg "graphic:ctrl+l" assign` then ctrl+l inserts a form feed (0x0C).
- `arg "halfscreen:=meta up arg \"11\" plines begline" assign` defines a macro; assigning it to a key works.
- `arg "hscroll:20" assign`, `arg "case:" assign`, `arg "nocase:" assign` set switches.
- Invalid: unknown function `arg "foo:alt+x" assign`, unknown key `arg "exit:alt+banana" assign`, bad switch value `arg "tabstops:x" assign`: each false with a message.
- nullarg: line `exit:alt+x` with the cursor on `e` → applied.
- streamarg over `exit:alt+x` → applied.
- linearg over three lines with a blank line in between → two statements applied, blank ignored.
- linearg with one invalid line → valid lines applied, false, first error shown (ORACLE CMD-ASSIGN-1).
- boxarg over a column of statements → each row applied.
- `arg "?" assign` → `not available`, false.
- noarg `assign` → `invalid argument`, false.
- `meta arg "exit:alt+x" assign` → applied, meta cleared.
