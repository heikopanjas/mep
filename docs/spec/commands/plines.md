# plines

Default keys: Ctrl+Z
Source: M guide p. 103 (Table A.3), p. 60 (`vscroll` "The Mlines and Plines functions also use this value"), p. 87 (Table A.1), p. 51 (Table 6.1), p. 49 (Halfscreen macro); MSJ (window-movement functions); IMPLEMENTATION_PLAN.md §6.3, §6.5

## Flags
From the syntax lines on p. 103:
- `Plines` → NOARG.
- `Arg Plines` → NULLARG.
- `Arg numarg Plines` → NUMARG, handled by the command as a count (not converted to linearg; plan §6.5).
- WINDOWFUNC (window movement; MSJ: window-movement functions "may take arguments").
- No TEXTARG, LINEARG, STREAMARG, BOXARG. Not KEEPMETA. `Arg Arg` has no special meaning.

## Behavior
The cursor keeps its row in the window: when the window moves by d lines, the cursor line moves by d lines and the column is unchanged. The window top never goes below 0 and never past the last line of the file. ORACLE CMD-SCROLL-1.

### noarg
Moves the window forward (toward the end of the file) by `vscroll` lines, or fewer in a small window (p. 103; ORACLE CMD-SCROLL-2). Returns true if the window moved, false if it could not move (Table 6.1 p. 51 "Movement occurred").

### nullarg (`Arg Plines`)
Moves the window so the cursor's line is the top line (p. 103). The cursor does not move. Returns true if the window moved, false if the cursor line was already at the top.

### numarg (`Arg numarg Plines`)
Moves the window forward by n lines (p. 103), clamped as above. n = 0 → no movement, false. Halfscreen (p. 49): `meta up arg "11" plines begline`.

### meta
No meta form. Meta is ignored and cleared.

## Rejected arguments
Non-numeric textarg, streamarg, linearg, boxarg → `invalid argument`, false (Table 6.1 "Bad argument").

## Open points
- Cursor behavior while the window moves (keeps its window row) and the limit at the end of the file (window top ≤ last line). ORACLE CMD-SCROLL-1.
- "or less if in a small window": ORACLE CMD-SCROLL-2.

## Test checklist
- 100-line file, window top 0, cursor (3, 5) → window top 7, cursor (10, 5), true.
- `vscroll:3` → moves by 3.
- Window top at the last line → false, nothing changes.
- `arg plines` with cursor on line 12, window top 5 → window top 12, cursor unchanged, true; again → false.
- `arg "11" plines` → window +11, cursor +11.
- `arg "0" plines` → false.
- Halfscreen macro result: cursor at the top row, column 0, window 11 lines further.
- `arg "abc" plines` → `invalid argument`, false; `arg down plines` → `invalid argument`.
- `meta plines` → same as `plines`, meta cleared.
