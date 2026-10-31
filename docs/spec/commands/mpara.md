# mpara

Default keys: Ctrl+PgUp
Source: M guide p. 100 (Table A.3), p. 87 (Table A.1), p. 51 (Table 6.1); z-vs-mep.md S4; IMPLEMENTATION_PLAN.md §11 (ORACLE S4)

## Flags
CURSORFUNC (Table A.1 p. 87). No arg-kind flags. Not KEEPMETA: `Meta Mpara` uses meta. `Arg Arg` does not apply.

## Behavior
Blank line and paragraph as in ppara.md. Cursor goes to column 0 (ORACLE CMD-PPARA-1). The window follows.

### noarg
"Moves the cursor to the first blank line preceding the current paragraph, or if currently on a blank line the cursor is positioned before the previous paragraph" (p. 100). ORACLE S4: we implement the M guide (Z instead lands on the first line of the previous paragraph).
- On a nonblank line: go up to the blank line directly above the current paragraph.
- On a blank line: skip blank lines upward, skip the previous paragraph, land on the blank line directly above it.
- If the paragraph starts at line 0 (no blank line above): land on line 0.
Returns true if the cursor moved, false otherwise.

### meta noarg (`Meta Mpara`)
"Moves cursor to the first previous line that has text" (p. 100): the nearest nonblank line above the cursor line. None → no movement, false.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case.

## Open points
- Landing line: ORACLE S4 (M reading implemented).
- Column 0, and line 0 when no blank line precedes: ORACLE CMD-PPARA-1.

## Test checklist
File: `a`,`b`,``,``,`c`,`d`,``,`e` (lines 0–7).
- From (5, 2) → (3, 0), true (ORACLE S4).
- From blank line 3 → line 0 (previous paragraph `a`,`b` starts at line 0), true.
- From line 7 → line 6. From line 6 → line 3.
- From line 1 → line 0; from (0, 0) → false.
- `meta mpara` from line 7 → line 5; from line 3 → line 1; from line 0 → false.
- `arg mpara` → selection extends upward.
- `arg "x" mpara` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
- `meta mpara` → meta form above, meta cleared afterwards.
- `arg down mpara` while in arg mode: the selection extends, the argument is not consumed.
