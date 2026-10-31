# restcur

Default keys: none
Source: M guide p. 106 (Table A.3), p. 32 (Ch. 4.3.2), p. 87 (Table A.1), p. 52 (Table 6.1); IMPLEMENTATION_PLAN.md §6.8

## Flags
CURSORFUNC (p. 32: no arguments; Table A.1 Cursor Movement p. 87). No arg-kind flags. No meta form; meta is cleared.

## Behavior

### noarg
Moves the cursor to the position saved by `savecur` (p. 106). The window follows (plan §6.3). The slot is kept, so `restcur` can be repeated. Returns true if a position had been saved, false if not (Table 6.1 p. 52). If the saved position belongs to another file, nothing moves and it returns false. ORACLE CMD-RESTCUR-1. If lines were deleted so the saved line is beyond the end of the file, the cursor goes there anyway (virtual space).

### In arg mode (no text)
Same movement; the selection extends to the restored position.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

### meta
Ignored and cleared.

## Rejected arguments
Only the pending-text case.

## Open points
- `restcur` after switching files: we do not switch back and return false. ORACLE CMD-RESTCUR-1.

## Test checklist
- Without `savecur` → false, cursor unchanged.
- `savecur` at (5, 3), `down down`, `restcur` → (5, 3), true; `restcur` again → true.
- Saved position far away → window jumps, cursor line placed per `hike`.
- `savecur` in file A, switch to B, `restcur` → false, still in B (ORACLE CMD-RESTCUR-1).
- `arg restcur` → selection from the anchor to the saved position.
- `arg "x" restcur` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
