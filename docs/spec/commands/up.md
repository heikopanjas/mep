# up

Default keys: Up, Ctrl+E
Source: M guide p. 109 (Table A.3), p. 26 (Ch. 4.1, 4.1.1), p. 17 (`Meta Up` example), p. 87 (Table A.1), p. 52 (Table 6.1), p. 49 (Halfscreen macro); IMPLEMENTATION_PLAN.md §5.4, §6.3

## Flags
CURSORFUNC (Table A.1 p. 87; M guide p. 72). No arg-kind flags. Not KEEPMETA. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor up one line, same cell column (no clamping to the line length; virtual space, plan §5.4). If the cursor leaves the window, the window moves up by `vscroll` lines, or fewer in a small window, never above line 0 (p. 109; ORACLE CMD-SCROLL-2). Returns true if moved, false on line 0 (Table 6.1 p. 52).

### meta noarg (`Meta Up`)
Moves the cursor to the top line of the window, same column, no scrolling (p. 26, p. 109). Returns true if moved, false if already on the top line.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Not a text-editing function: the text is passed to `up`, which rejects it (`invalid argument`, false, arg mode ends). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case above.

## Open points
- Small-window scroll amount: ORACLE CMD-SCROLL-2.

## Test checklist
- Line 3 → 2, column kept, true. Line 0 → false.
- From column 30 on a long line up to a short line → column stays 30 (virtual space).
- Cursor on the top window row with window top 10 → window top 3 (`vscroll` 7), cursor line 9.
- Window top 2 → window top 0 (clamped).
- `meta up` → top window row, true; again → false.
- `arg up` → linearg of two lines.
- `arg "x" up` → `invalid argument`, false.
