# endfile

Default keys: none
Source: project addition, not in the 1988 manual. A sample MEP TOOLS.INI (tenox7/ntutils) assigns `Endfile:Ctrl+PgDn`. The movement is that of `Arg Ppage` (M guide p. 103, p. 27; ppage.md). ORACLE CMD-BEGFILE-1.

## Flags
CURSORFUNC, like `home` (p. 87). No arg-kind flags. Not KEEPMETA, so meta is cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Moves to the end of the file, as `Arg Ppage` (p. 103): cursor on the last line of the file, column 0; the window is placed per IMPLEMENTATION_PLAN.md §6.3 (the cursor line `hike` lines from the top when the jump exceeds a window). Returns true if the cursor moved, false if it was already there.

### meta noarg
No meta form. Meta is ignored and cleared; same movement as noarg.

### In arg mode, no text typed
Same movement; the selection extends to the last line.

### In arg mode, text being edited
Text is passed to `endfile` and rejected (`invalid argument`, false). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case above.

## Open points
- The 1988 manual has no `endfile`. The name and key come from a sample TOOLS.INI; the movement and the cursor-function kind are our choice. ORACLE CMD-BEGFILE-1.
- Ctrl+PgDn stays `ppara` by default (Table A.2 p. 91); `endfile` needs an assignment.

## Test checklist
- 30-line file, window height 10, cursor (2, 3) → (29, 0), true; again → false.
- `meta endfile` → same movement, meta cleared.
- `arg endfile copy` from (1, 0) → lines 1 to the last in the clipboard.
- `arg "x" endfile` → `invalid argument`, false.
- `endfile:ctrl+pgdn` in TOOLS.INI → Ctrl+PgDn runs `endfile`.
