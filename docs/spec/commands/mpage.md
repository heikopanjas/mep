# mpage

Default keys: PgUp, Ctrl+R
Source: M guide p. 100 (Table A.3), p. 27 (Ch. 4.1.2 "direct inverse of Ppage … accepts the same syntax"), p. 31 (Mark), p. 87 (Table A.1), p. 51 (Table 6.1); MSJ `selectall`; IMPLEMENTATION_PLAN.md §6.3, §6.5

## Flags
Same as ppage.md: NOARG, NULLARG, NUMARG (count), BOXSTR streamarg as count (p. 27 "accepts the same syntax"; ORACLE CMD-PPAGE-1), WINDOWFUNC. Not KEEPMETA. `Arg Arg` has no special meaning.

## Behavior
Cursor moves with the window, keeping its window row and column; the window top stays ≥ 0 (ORACLE CMD-SCROLL-1).

### noarg
Moves the window and cursor back by one page (p. 100). If fewer lines remain above, moves by what remains. Returns true if they moved, false at the top.

### nullarg (`Arg Mpage`)
Moves to the beginning of the file: cursor (0, 0), window top 0, left column 0 (p. 100; p. 27). Column 0 is required by MSJ `selectall`. Returns true if anything moved.

### numarg
Back by n pages, clamped at the top.

### streamarg (one line)
Numeric highlighted text → count; otherwise `invalid argument`. ORACLE CMD-PPAGE-1.

### meta
No meta form; ignored and cleared.

## Rejected arguments
Non-numeric textarg/streamarg, linearg, boxarg → `invalid argument`, false.

## Open points
- ORACLE CMD-SCROLL-1, CMD-PPAGE-1.

## Test checklist
- Window height 20, top 40, cursor (45, 2) → top 20, cursor (25, 2), true.
- Top 5, cursor (8, 0) → top 0, cursor (3, 0).
- Top 0 → false.
- `arg mpage` from (50, 7) → (0, 0), window (0, 0), true; again → false.
- `arg "2" mpage` → −40 lines.
- `arg "x" mpage` → `invalid argument`.
