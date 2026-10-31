# savecur

Default keys: none
Source: M guide p. 107 (Table A.3), p. 32 (Ch. 4.3.2), p. 87 (Table A.1), p. 52 (Table 6.1); IMPLEMENTATION_PLAN.md §6.8

## Flags
CURSORFUNC: "Savecur and Restcur do not take arguments" (p. 32), and M guide p. 72 defines CURSORFUNC as not taking arguments and not affecting highlighting or `Arg`. No arg-kind flags. No meta form; meta is cleared.

## Behavior

### noarg
Saves the current file and cursor position (line, cell column) in a single slot, replacing any earlier saved position (p. 32: "you can save only one position at a time"). Cursor does not move. Returns true (Table 6.1 p. 52: always).

### In arg mode (no text)
Saves the position; the argument stays pending.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

### meta
Ignored and cleared.

## Rejected arguments
Only the pending-text case.

## Open points
- Whether the slot is per file or global; see restcur.md, ORACLE CMD-RESTCUR-1.

## Test checklist
- `savecur` at (5, 3), move away, `restcur` → (5, 3).
- Second `savecur` at (8, 0) replaces the first.
- `arg right savecur` → selection still pending.
- Position beyond end of line is saved exactly.
- Returns true.
- `arg "x" savecur` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
