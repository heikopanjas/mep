# backtab

Default keys: Shift+Tab
Source: M guide p. 93 (Table A.3), p. 60 (`tabstops`), p. 87 (Table A.1), p. 50 (Table 6.1); IMPLEMENTATION_PLAN.md §5.2, §11 (TAB-1)

## Flags
CURSORFUNC (Table A.1 p. 87). No arg-kind flags. No meta form; meta is cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor left to the previous tab stop: the largest multiple of `tabstops` that is less than the cursor column (p. 93). The window follows. Returns true if it moved, false at column 0 (Table 6.1 p. 50 "Cursor at left margin").

### meta noarg
No meta form; same movement, meta cleared.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case.

## Open points
- TAB-1 as for `tab`.

## Test checklist
- Column 9 → 8; 8 → 4; 1 → 0; 0 → false.
- `tabstops:3`: 7 → 6.
- Window scrolled right with the cursor at its left edge → window shifts.
- `arg backtab` → streamarg.
- `arg "x" backtab` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
- `meta backtab` → moves as noarg, meta cleared.
