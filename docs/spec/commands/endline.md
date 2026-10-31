# endline

Default keys: End
Source: M guide pp. 96–97 (Table A.3), p. 26 (Ch. 4.1), p. 19 (text-argument editing), p. 87 (Table A.1), p. 51 (Table 6.1), p. 53 (Blankline macro); IMPLEMENTATION_PLAN.md §6.3

## Flags
CURSORFUNC (Table A.1 p. 87). No arg-kind flags. Not KEEPMETA: `Meta Endline` uses meta. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor to the immediate right of the last nonblank character of the line (p. 96; p. 26 "just to the right of the last printable character"). Trailing blanks are skipped. Empty or all-blank line, or a line beyond the end of the file: column 0. The window follows (plan §6.3). Returns true if moved, false otherwise (Table 6.1 p. 51).

### meta noarg (`Meta Endline`)
"Moves the cursor one character beyond the column corresponding to the rightmost edge of the window" (p. 97): the target column is window left column + window width, which is outside the window, so the window shifts right by `hscroll`. Returns true if moved. ORACLE CMD-ENDLINE-1.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Moves the text cursor to the end of the text (p. 19).

## Rejected arguments
None.

## Open points
- `Meta Endline` target: M says one column beyond the right window edge (forcing a scroll); Z says the rightmost edge itself. MEP wins; we implement M. ORACLE CMD-ENDLINE-1.

## Test checklist
- `abc  ` (two trailing spaces, unedited line), cursor 0 → 3, true; again → false.
- Empty line → column 0, false if already there.
- Line longer than the window → window scrolls right to show the cursor.
- `meta endline` with window columns 0–79 → cursor column 80, window shifted by `hscroll` (ORACLE CMD-ENDLINE-1).
- `arg endline` from column 0 → streamarg over the line text.
- `arg "abc" begline endline` → text cursor at 3.
- No argument kind is rejected (CURSORFUNC; the text case is text editing). Instead test that a pending selection stays pending after `endline` and that `meta endline` clears meta.
