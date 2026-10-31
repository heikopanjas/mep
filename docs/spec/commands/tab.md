# tab

Default keys: Tab
Source: M guide p. 109 (Table A.3), p. 60 (`tabstops`), p. 87 (Table A.1), p. 52 (Table 6.1); IMPLEMENTATION_PLAN.md §5.2, §11 (TAB-1)

## Flags
CURSORFUNC (Table A.1 p. 87; Z: "Being a cursor movement function, tab takes no arguments nor mode-command modifiers"). No arg-kind flags. No meta form; not KEEPMETA, so meta is cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor right to the next tab stop: the next cell column that is a multiple of `tabstops` (p. 109, default 4). It moves the cursor only; it never inserts a tab or spaces. Beyond the end of the line it moves into virtual space. The window follows (plan §6.3). Returns true (it always moves; Table 6.1 p. 52).

`tabstops` is independent of the file tab width, which is fixed at 8 for display (plan §5.2, ORACLE TAB-1).

### meta noarg
No meta form; same movement, meta cleared.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case.

## Open points
- TAB-1: display width of file tabs vs `tabstops`.

## Test checklist
- Column 0 → 4; 3 → 4; 4 → 8 (tabstops 4).
- `tabstops:8`: 1 → 8.
- Beyond end of line → moves into virtual space, file unchanged.
- Line containing a tab character: the cursor moves by cells, not by characters.
- `arg tab` → streamarg of four cells.
- `meta tab` → moves, meta cleared.
- `arg "x" tab` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
