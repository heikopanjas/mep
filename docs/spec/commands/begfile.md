# begfile

Default keys: none
Source: project addition, not in the 1988 manual. A sample MEP TOOLS.INI (tenox7/ntutils) assigns `Begfile:Ctrl+PgUp`. The movement is that of `Arg Mpage` (M guide p. 100; mpage.md). ORACLE CMD-BEGFILE-1.

## Flags
CURSORFUNC, like `home` (p. 87). No arg-kind flags. Not KEEPMETA, so meta is cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Moves to the beginning of the file: cursor (0, 0), window top 0, left column 0, as `Arg Mpage` (p. 100). Returns true if anything moved, false if the cursor was already at (0, 0).

### meta noarg
No meta form. Meta is ignored and cleared; same movement as noarg.

### In arg mode, no text typed
Same movement; the selection extends to the beginning of the file.

### In arg mode, text being edited
Text is passed to `begfile` and rejected (`invalid argument`, false). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case above.

## Open points
- The 1988 manual has no `begfile`. The name and key come from a sample TOOLS.INI; the movement and the cursor-function kind are our choice. ORACLE CMD-BEGFILE-1.
- Ctrl+PgUp stays `mpara` by default (Table A.2 p. 91); `begfile` needs an assignment.

## Test checklist
- 30-line file, window height 10, cursor (25, 3) → (0, 0), window (0, 0), true; again → false.
- Window scrolled right → left column 0.
- `meta begfile` → same movement, meta cleared.
- `arg begfile copy` from (2, 0) → lines 0–2 in the clipboard.
- `arg "x" begfile` → `invalid argument`, false.
- `begfile:ctrl+pgup` in TOOLS.INI → Ctrl+PgUp runs `begfile`.
