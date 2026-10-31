# begline

Default keys: Home
Source: M guide p. 93 (Table A.3), pp. 7, 26 (Ch. 2.3, 4.1), p. 19 (text-argument editing), p. 87 (Table A.1), p. 50 (Table 6.1), p. 57 (Indent macro); MSJ Fig. 5 (see home.md); IMPLEMENTATION_PLAN.md §6.3

## Flags
CURSORFUNC (Table A.1 p. 87). No arg-kind flags. Not KEEPMETA: `Meta Begline` uses meta. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor to the first nonblank character of the line (p. 93; p. 7 "first nonblank character"). Blank means space or tab. On an empty or all-blank line, and on a line beyond the end of the file, the cursor goes to column 0. The window follows the cursor horizontally (plan §6.3). Returns true if the cursor moved, false if it was already there (Table 6.1 p. 50).

### meta noarg (`Meta Begline`)
Moves the cursor to column 0 (p. 93). Returns true if moved.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Moves the text cursor to the start of the text (p. 19). Returns true if it moved.

## Rejected arguments
None.

## Open points
- All-blank line: the manual has no first nonblank character; we use column 0.

## Test checklist
- `    int x;`, cursor 8 → 4, true; at 4 → false.
- Line starting with a tab: target is the first nonblank cell after the tab expansion.
- Empty line → column 0; already 0 → false.
- `meta begline` from 8 → 0, true.
- Long line scrolled right: `begline` brings the window back so the cursor is visible.
- `arg begline` from column 8 on `    int x;` → streamarg cells 4–7.
- `arg "abc" begline` → text cursor at 0.
- Indent macro (p. 57): `meta begline arg right right right right right sinsert` inserts five spaces at line start.
- No argument kind is rejected (CURSORFUNC; the text case is text editing). Instead test that a pending selection stays pending after `begline` and that `meta begline` clears meta.
