# ppage

Default keys: PgDn, Ctrl+C
Source: M guide p. 103 (Table A.3), pp. 26–27 (Ch. 4.1.2), p. 31 (Mark noarg "just as Arg Mpage"), p. 87 (Table A.1), p. 51 (Table 6.1); MSJ ("The editor's ppage function is a window-movement function", `selectall` macro); IMPLEMENTATION_PLAN.md §6.3, §6.5

## Flags
- `Ppage` → NOARG.
- `Arg Ppage` → NULLARG.
- `Arg numarg Ppage` → NUMARG, used as a count of windows (not converted; plan §6.5).
- `Arg streamarg Ppage` (p. 27 only, not in Table A.3) → BOXSTR: the highlighted text must be a decimal number and is used as the count. ORACLE CMD-PPAGE-1.
- WINDOWFUNC (MSJ). No LINEARG, BOXARG, other textarg. Not KEEPMETA. `Arg Arg` has no special meaning.

## Behavior
"Page" = the number of lines in the window (p. 27). The cursor moves with the window and keeps its window row and column; the window top stays ≤ the last line of the file (ORACLE CMD-SCROLL-1).

### noarg
Moves the window and cursor forward by one page (p. 103). Returns true if they moved, false if the window top was already at the last line.

### nullarg (`Arg Ppage`)
Moves to the end of the file (p. 103 "window to the end"; p. 27 "cursor forward to the end of the file"): cursor on the last line of the file, column 0; the window is placed per plan §6.3 (the cursor line `hike` lines from the top when the jump exceeds a window). Column 0 is required by the MSJ `selectall` macro (`arg ppage … arg mpage … arg "endoffile" copy` must give a linearg over the whole file). Returns true if the cursor or window moved.

### numarg
Moves forward by n pages (p. 103, p. 27), clamped. n = 0 → false.

### streamarg (one line)
If the highlighted text is a decimal number, same as numarg with that count; otherwise `invalid argument`, false. ORACLE CMD-PPAGE-1.

### meta
No meta form; ignored and cleared.

## Rejected arguments
Non-numeric textarg, non-numeric streamarg, linearg, boxarg → `invalid argument`, false.

## Open points
- "Moves the cursor forward by the number of pages that you highlight" (p. 27) is unclear. We read it as a highlighted number. ORACLE CMD-PPAGE-1.
- Cursor following the window and the end-of-file limit. ORACLE CMD-SCROLL-1.
- With a selection pending, PgDn consumes it (WINDOWFUNC with arg forms) rather than extending it. This follows the syntax forms; it means PgDn cannot grow a selection.

## Test checklist
- 100-line file, window height 20, top 0, cursor (5, 3) → top 20, cursor (25, 3), true.
- Window top at the last line → false.
- `arg ppage` → cursor (99, 0), true; window placed with `hike`.
- `arg "2" ppage` → +40 lines.
- `arg "0" ppage` → false.
- Streamarg over the text `2` → +2 pages; streamarg over `ab` → `invalid argument`.
- `arg "ab" ppage` → `invalid argument`; `arg down ppage` → `invalid argument`.
- `selectall` macro from MSJ followed by `copy` → whole file in the clipboard as lines (needs M2 `mark`; in M1 test `arg ppage` and `arg mpage` columns only).
