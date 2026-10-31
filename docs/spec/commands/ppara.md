# ppara

Default keys: Ctrl+PgDn
Source: M guide p. 104 (Table A.3), p. 87 (Table A.1), p. 51 (Table 6.1), p. 57 (Inpara macro); IMPLEMENTATION_PLAN.md §6.3, §11 (S4 concerns `mpara`)

## Flags
CURSORFUNC (Table A.1 lists it under Cursor Movement p. 87; Table 6.1 "Movement occurred / Bad argument"). No arg-kind flags in the syntax lines. Not KEEPMETA: `Meta Ppara` uses meta. `Arg Arg` does not apply.

## Behavior
A blank line is an empty line, a line of only spaces and tabs, or a line beyond the end of the file. A paragraph is a run of nonblank lines (Z: "blocks of text separated by blank lines"). The cursor goes to column 0 of the target line. ORACLE CMD-PPARA-1. The window follows (plan §6.3).

### noarg
"Moves the cursor forward one paragraph and places the cursor on the first line of the new paragraph" (p. 104): skip the rest of the current paragraph (if on one), skip blank lines, stop on the first nonblank line. If there is no next paragraph, the cursor goes to the line just past the end of the file. Returns true if the cursor moved, false if it was already there.

This end-of-file rule follows from the Inpara macro (p. 57: `Mark :>Repeat Indent Ppara Endline Left +>Repeat`), which only stops when `left` fails, i.e. when `ppara` has reached an empty line.

### meta noarg (`Meta Ppara`)
Moves the cursor to the first blank line following the current paragraph (p. 104). From a blank line: the first blank line after the next paragraph. No such line in the file → the line past the end of the file. Returns true if moved.

### In arg mode, no text typed
Same movement; the selection extends (usually a linearg, since the column becomes 0 only when the anchor is in column 0; otherwise a boxarg).

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case. (Table 6.1 "Bad argument" covers it.)

## Open points
- Target column (we use 0) and the landing line when there is no next paragraph. ORACLE CMD-PPARA-1.

## Test checklist
File: `a`,`b`,``,``,`c`,`d`,``,`e` (lines 0–7).
- From (0, 3) → (4, 0), true. From (4, 0) → (7, 0). From (7, 0) → (8, 0) (past end), true; again → false.
- From blank line 2 → (4, 0).
- `meta ppara` from line 0 → line 2; from line 4 → line 6; from line 7 → line 8.
- Whitespace-only line counts as blank.
- Inpara macro (p. 57, with `mark` replaced by `arg mpage` in M1) indents each paragraph's first line and terminates.
- `arg ppara` from (0, 0) → linearg lines 0–4.
- `arg "x" ppara` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
- `meta ppara` → meta form above, meta cleared afterwards.
- `arg down ppara` while in arg mode: the selection extends, the argument is not consumed.
