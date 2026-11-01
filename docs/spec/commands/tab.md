# tab

Default keys: Tab
Source: M guide p. 109 (Table A.3), p. 60 (`tabstops`), p. 87 (Table A.1), p. 52 (Table 6.1); IMPLEMENTATION_PLAN.md §5.2, §11 (TAB-1)

## Flags
CURSORFUNC (Table A.1 p. 87; Z: "Being a cursor movement function, tab takes no arguments nor mode-command modifiers"). No arg-kind flags. No meta form; not KEEPMETA, so meta is cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Moves the cursor right to the next tab stop: the next cell column that is a multiple of `tabstops` (p. 109, default 4). It moves the cursor only; it never inserts a tab or spaces. Beyond the end of the line it moves into virtual space. The window follows (plan §6.3). Returns true (it always moves; Table 6.1 p. 52).

With the `tabinsert` switch on (project addition, switches.md, ORACLE CMD-TAB-1), `tab` instead inserts spaces at the cursor up to that tab stop and moves the cursor there, in insert and overtype mode alike. Beyond the end of the line or file the line is padded or created first, as when typing. Returns true.

`tabstops` is independent of the file tab width, which is 8 for display (plan §5.2, ORACLE TAB-1) unless the project switch `usetabs` makes it `tabstops` (switches.md). With `usetabs`, the spaces `tab` inserts under `tabinsert` are saved as tabs.

### meta noarg
No meta form; same movement, meta cleared.

### In arg mode, no text typed
If no selection is pending yet (anchor equals the cursor, e.g. the arg was just started), same movement; the selection extends. If a selection is already pending (anchor differs from the cursor), see "Block indent" below instead.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Block indent with a selection (project addition, not in the 1988 manual)
ORACLE CMD-TAB-1. Because `tab` is `CURSORFUNC`, the dispatcher always hands it straight to the cursor-move/selection-extend path; it never goes through `classify()` the way a selection-consuming command like `copy` does. To still let `Tab` act on a selection that's already there, `tab` itself checks for one: if the pending arg's anchor differs from the cursor right now, it indents every line from the anchor's line to the cursor's line (inclusive, whichever is smaller first) instead of moving to the next tab stop, then ends the selection (clears the pending arg) and places the cursor at column 0 of the first indented line. Returns true.

Each line is indented independently: its own leading whitespace (after `detab`) is measured, and spaces are added at the start of the line to bring that width up to the next multiple of `tabstops` — the same rounding `tab`'s own cursor movement uses, so a line already indented by 2 columns with `tabstops:4` gains 2 spaces (to reach 4), not a flat 4. A blank line's "indent" is its entire (blank) width. This has nothing to do with `COLARG`/`argcol`'s column-anchored shape — it triggers on any pending selection, built however (plain `Arg`, `Argcol`, or a marker-free movement), since only the line range matters here, not the shape.

The one behavioral cost: using `Tab` itself as one of the movement keys while still building a selection (e.g. `Arg Right Tab Tab Copy`) no longer works past the first `Tab` press once the selection becomes non-trivial, since that `Tab` now indents and ends it instead of extending it further. Accepted tradeoff — nobody is likely to keep using `Tab` as a selection-building key once it indents on sight.

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
- A pending multi-line selection (any shape) + `tab` → every covered line gains spaces up to the next `tabstops` multiple of its own indent, selection ends, cursor at column 0 of the first line.
- A line already indented to 2 columns with `tabstops:4` → gains 2 spaces, not 4.
- A blank line in the selection → gains a full `tabstops` worth of spaces.
- A selection confined to one line still gets indented.
- `tabinsert:` → `tab` at column 1 of "abc" gives "a   bc", cursor 4; `arg tab` still moves; below the end of the file the line is created.
