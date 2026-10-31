# down

Default keys: Down, Ctrl+X
Source: M guide p. 96 (Table A.3), p. 26 (Ch. 4.1, 4.1.1), p. 87 (Table A.1), p. 50 (Table 6.1); IMPLEMENTATION_PLAN.md §5.4, §6.3

## Flags
CURSORFUNC (Table A.1 p. 87; M guide p. 72). No arg-kind flags. Not KEEPMETA. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor down one line, same cell column. The cursor may move beyond the last line of the file (plan §5.4); no lines are added. If the cursor leaves the window, the window moves down by `vscroll` lines, or fewer in a small window (p. 96; ORACLE CMD-SCROLL-2). Returns true if moved (Table 6.1 p. 50). Because there is no lower limit, `down` always moves.

### meta noarg (`Meta Down`)
Moves the cursor to the bottom line of the window, same column, no scrolling (p. 26, p. 96). Returns true if moved, false if already there.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Text is passed to `down` and rejected (`invalid argument`, false). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case above.

## Open points
- Small-window scroll amount: ORACLE CMD-SCROLL-2.
- The manual does not say whether `down` stops at the end of the file. The plan allows the cursor beyond the last line (§5.4); we never stop.

## Test checklist
- Line 0 → 1, true. Last line → one past the last line, true; file unchanged; status length unchanged.
- Cursor on the bottom window row → window moves by 7.
- `meta down` → bottom window row, true; again → false.
- `arg down` → linearg; `arg down right` → boxarg.
- `arg "x" down` → `invalid argument`, false.
