# linsert

Default keys: Ctrl+N
Source: M guide p. 99 (Table A.3), pp. 28–29 (Ch. 4.2.1), p. 49 (Head1 macro `Arg "3" Linsert`), p. 63 (`Arg "1" Linsert`), p. 22, p. 88 (Table A.1), p. 51 (Table 6.1); z-vs-mep.md S1; IMPLEMENTATION_PLAN.md §6.5, §6.7

## Flags
From p. 99, p. 29, p. 49:
- `Linsert` → NOARG.
- `Arg Linsert` → NULLARG.
- `Arg linearg Linsert` → LINEARG.
- `Arg numarg Linsert` (macros on p. 49 and p. 63) → NUMARG, converted to a linearg of n lines.
- `Arg boxarg Linsert` → BOXARG.
- `Arg streamarg Linsert` → STREAMARG (one line).
- MARKARG (M2). MODIFIES. No TEXTARG.
- Not KEEPMETA. `Arg Arg` behaves like `Arg`.

## Behavior
The clipboard is not touched. Read-only file: plan §5.6.

### noarg
Inserts one blank line above the current line (p. 99; MEP behavior, divergence S1: Z inserts below). The cursor stays on the same line index and column, so it is now on the new blank line. The new line uses the file's default terminator. Beyond the end of the file: nothing to insert above; the file is unchanged. Returns true.

### nullarg (`Arg Linsert`)
"Inserts or deletes blanks at the beginning of a line to make the first nonblank character appear under the cursor" (p. 99): the leading blanks of the cursor line are changed so its first nonblank character is at the cursor column. Cursor does not move. A blank or empty line has no first nonblank character: no change. Returns true.

### linearg / numarg
Inserts as many blank lines as the range has lines, above the range's upper line (p. 29 "the indicated number of blank lines is inserted"; p. 99 "fill the specified area with blanks"). Cursor: upper line, same column, so text typed next lands on the first blank line (required by the Head1/Head2/Head3 macros, p. 49). Returns true.

### boxarg
Inserts a box of blanks: on each line from the upper to the lower line, box-width spaces are inserted at the left edge; text right of it moves right (p. 29). Lines shorter than the left edge are padded first. Cursor: upper-left corner (ORACLE CMD-BLOCK-1). Returns true.

### streamarg (one line)
Inserts blanks between the two positions on the line (same as a one-line box). Cursor: left position. Returns true.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
Non-numeric textarg → `invalid argument`, false.

## Open points
- Divergence S1 resolved for MEP (above).
- `Arg Linsert` on a blank line: no change, true (project choice).
- Cursor after the box form: ORACLE CMD-BLOCK-1.

## Test checklist
- `a`/`b`, cursor (1, 3), `linsert` → `a`/``/`b`, cursor (1, 3), true.
- CRLF file → new line has CRLF.
- Beyond the end of the file → unchanged.
- `arg linsert` on `      x` with cursor 2 → `  x`; cursor 9 on `  x` → `         x`.
- `arg linsert` on an empty line → unchanged.
- `arg down down linsert` from (0, 0) → three blank lines above line 0; cursor (0, 0).
- `arg "3" linsert` then typing → text on the first blank line (Head1 macro); full Header macro (p. 49) output.
- Box (0, 2)–(2, 5) → three spaces inserted at column 2 on lines 0–2.
- Streamarg on one line → blanks inserted.
- `arg "abc" linsert` → `invalid argument`.
- Clipboard unchanged.
