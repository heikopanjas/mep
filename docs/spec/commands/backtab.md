# backtab

Default keys: Shift+Tab
Source: M guide p. 93 (Table A.3), p. 60 (`tabstops`), p. 87 (Table A.1), p. 50 (Table 6.1); IMPLEMENTATION_PLAN.md §5.2, §11 (TAB-1)

## Flags
CURSORFUNC (Table A.1 p. 87). No arg-kind flags. No meta form; meta is cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor left to the previous tab stop: the largest multiple of `tabstops` that is less than the cursor column (p. 93). The window follows. Returns true if it moved, false at column 0 (Table 6.1 p. 50 "Cursor at left margin").

With the `tabinsert` switch on (project addition, switches.md, ORACLE CMD-TAB-1) and only blanks (spaces or tabs) left of the cursor, `backtab` instead removes the blanks between that tab stop and the cursor and moves the cursor there; tabs in the line become spaces first. Blanks beyond the end of the line are virtual, so there it only moves. Returns true. With text left of the cursor, or at column 0, it moves as above.

### meta noarg
No meta form; same movement, meta cleared.

### In arg mode, no text typed
If no selection is pending yet (anchor equals the cursor), same movement; the selection extends. If a selection is already pending (anchor differs from the cursor), see "Block outdent" below instead.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Block outdent with a selection (project addition, not in the 1988 manual)
ORACLE CMD-TAB-1. Mirrors `tab`'s block indent (tab.md) for the same reason (`backtab` is `CURSORFUNC` too, so it has to check for a pending selection itself rather than through `classify()`). If the pending arg's anchor differs from the cursor, every line from the anchor's line to the cursor's line loses leading whitespace down to the previous multiple of `tabstops` (the same rounding `backtab`'s own cursor movement uses), the selection ends, and the cursor goes to column 0 of the first line. A line already at or below the previous multiple (including column 0) is left unchanged — this never removes non-whitespace content. Returns true.

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
- A pending multi-line selection + `backtab` → every covered line loses spaces down to the previous `tabstops` multiple, selection ends, cursor at column 0 of the first line.
- A line already at column 0 → unchanged (no negative indent).
- Indent-then-outdent round-trips back to the original line content.
- `tabinsert:` → "      x" at column 6 gives "    x", cursor 4; a tab left of the cursor is outdented; text on the left → moves only; column 0 → false; virtual space → moves only.
