# right

Default keys: Right, Ctrl+D
Source: M guide p. 107 (Table A.3), p. 26 (Ch. 4.1 Meta forms, 4.1.1 scrolling), p. 19 (text-argument editing), p. 87 (Table A.1), p. 52 (Table 6.1); IMPLEMENTATION_PLAN.md §5.4, §5.5, §6.3

## Flags
CURSORFUNC (Table A.1 p. 87; M guide p. 72). No arg-kind flags, no NUMARG. Not KEEPMETA: the meta form uses meta and clears it. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor one character right: past a wide character it moves by the character's width, beyond the end of the line by one cell (virtual space has no limit, plan §5.4, §5.5). If the cursor leaves the window, the window shifts right by `hscroll` columns, or fewer in a small window (p. 107; ORACLE CMD-SCROLL-2).

Return value (Table 6.1 p. 52): true if the cursor is now over text of the line (new column < line length in cells), false if it is now beyond the end of the line. It is not "cursor moved".

### meta noarg (`Meta Right`)
Moves the cursor to the rightmost column of the window on the same line (p. 26, p. 107). No scrolling. Same return rule as noarg (over text → true, beyond end → false).

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Moves the text cursor one character right (p. 19). Past the end of the current text, it appends the character at that position of the previous text argument (p. 19); if the previous text is not that long, nothing happens and it returns false.

## Rejected arguments
None.

## Open points
- Small-window scroll amount: ORACLE CMD-SCROLL-2.

## Test checklist
- `abc`, cursor 0 → 1, true; cursor 2 → 3 (end of line), false; cursor 3 → 4, false.
- Wide character at column 0 (2 cells) → cursor 2.
- Cursor at the right window edge → window shifts by `hscroll`.
- `meta right` on a short line → rightmost window column, false; on a line longer than the window → true.
- `arg right right` → streamarg.
- Previous text `hello`; `arg "he" right right` → text `hel`, then `hell`.
- Empty line: `right` → false.
- No argument kind is rejected (CURSORFUNC; the text case is text editing). Instead test that a pending selection stays pending after `right` and that `meta right` clears meta.
