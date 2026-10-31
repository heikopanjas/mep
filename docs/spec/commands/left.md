# left

Default keys: Left, Ctrl+S
Source: M guide p. 99 (Table A.3), p. 26 (Ch. 4.1 Meta forms), p. 26 (Ch. 4.1.1 scrolling), p. 19 (text-argument editing), p. 87 (Table A.1), p. 51 (Table 6.1), p. 53 (Blankline macro); IMPLEMENTATION_PLAN.md §5.4, §6.3

## Flags
CURSORFUNC (Cursor Movement in Table A.1 p. 87; M guide p. 72: "Executes cursor movement only. Highlighting and the *Arg* function are not affected. The function does not take arguments."). No arg-kind flags, no NUMARG. Not KEEPMETA: the meta form uses meta and clears it. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor one cell column left. Columns are display cells (plan §3.2); on a wide character the cursor moves to the character's first cell. Works the same beyond the end of the line (virtual space, plan §5.4). If the cursor leaves the window, the window shifts left by `hscroll` columns, or fewer in a small window, never below column 0 (p. 99; ORACLE CMD-SCROLL-2). Returns true if the cursor moved, false at column 0 (Table 6.1 p. 51; Blankline macro p. 53 relies on this).

### meta noarg (`Meta Left`)
Moves the cursor to the leftmost column of the window on the same line (p. 26, p. 99). No scrolling. Returns true if it moved, false if it was already there.

### In arg mode, no text typed
Same movement; the selection extends from the anchor to the new position. The argument stays pending.

### In arg mode, text being edited
Moves the text cursor one character left on the dialog line without deleting (p. 19). At the start of the text: no movement, false. The file cursor does not move.

## Rejected arguments
None (CURSORFUNC functions never receive an argument).

## Open points
- "or less if in a small window" (p. 99): we shift by `min(hscroll, window width)`. ORACLE CMD-SCROLL-2.

## Test checklist
- Column 5 → 4, true. Column 0 → stays, false.
- Beyond end of line (column 20 on a 5-cell line) → 19, true.
- Left over a wide character: cursor lands on its first cell.
- Window left column 10, cursor at column 10 → window shifts to 0 (hscroll 10), cursor 9.
- `hscroll:3` → window shifts by 3.
- `meta left` with window left column 10 → cursor at column 10, no scroll; again → false.
- `arg left left`: streamarg of two cells selected, highlight shown.
- `arg "abc" left`: text cursor moves left, file cursor unchanged.
- Blankline macro `endline :>back sdelete left +>back` empties a line and stops at column 0.
- No argument kind is rejected (CURSORFUNC; the text case is text editing). Instead test that a pending selection stays pending after `left` and that `meta left` clears meta.
