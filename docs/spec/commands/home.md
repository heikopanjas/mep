# home

Default keys: Ctrl+Home
Source: M guide p. 97 (Table A.3), p. 87 (Table A.1 "upper-left corner of window"), p. 51 (Table 6.1); MSJ Fig. 5 (meta row); IMPLEMENTATION_PLAN.md §6.3

## Flags
CURSORFUNC (Table A.1 p. 87). No arg-kind flags. No meta form in Table A.3; not KEEPMETA, so meta is cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Places the cursor in the upper-left corner of the current window: the window's top line and left column (p. 97). The window does not move. Returns true if the cursor moved, false if it was already there (Table 6.1 p. 51).

### meta noarg
No meta form in the M guide. Meta is ignored and cleared; same movement as noarg. ORACLE CMD-HOME-1.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Text is passed to `home` and rejected (`invalid argument`, false). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case above.

## Open points
- MSJ Fig. 5 says "*home* goes to first nonblank character on a line, while *meta home* goes to column one", which is what the M guide gives for `begline`/`meta begline`. Probably a mix-up in the article; the M guide wins. ORACLE CMD-HOME-1: does `meta home` do anything different from `home`?

## Test checklist
- Window top 20, left 10, cursor (30, 15) → (20, 10), true; again → false.
- Window not scrolled → (0, 0).
- `meta home` → same as `home`, meta cleared (ORACLE CMD-HOME-1).
- `arg home` from (5, 5) with window at (0, 0) → boxarg.
- `arg "x" home` → `invalid argument`, false.
