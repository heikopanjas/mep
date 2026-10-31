# newline

Default keys: none
Source: M guide p. 102 (Table A.3), p. 61 (`softcr`), p. 87 (Table A.1 "Moves cursor down to next line"), p. 51 (Table 6.1), p. 49 (Head2/Head3 macros); IMPLEMENTATION_PLAN.md §6.8, §6.13

## Flags
CURSORFUNC (Table A.1 lists it under Cursor Movement, p. 87). No arg-kind flags. Not KEEPMETA: `Meta Newline` uses meta. `Arg Arg` does not apply.

## Behavior
`newline` only moves the cursor; it never inserts a line break, in insert mode either (Z: "the newline function is NOT a graphic character and does NOT cause insertion of a line break"; M: `emacsnewl` "performs similarly to Newline, except that when in insert mode, it breaks the current line", p. 96). On the last line it moves to the line past the end of the file; no line is added.

### noarg
Moves the cursor to the next line. Column:
- `softcr` off: column 0.
- `softcr` on (default) (p. 102):
  1. In a C program the editor indents "based on continuation of lines and on open blocks". Not implemented in M1: all files use rules 2 and 3. ORACLE CMD-NEWLINE-1.
  2. If the new line is blank: the column of the first nonblank character of the previous line (the line the cursor came from). If that line is blank too, column 0.
  3. Otherwise: the first nonblank character of the new line.
The window follows. Returns true (Table 6.1 p. 51: always).

### meta noarg (`Meta Newline`)
Moves the cursor to column 0 of the next line (p. 102, "column 1" in 1-based terms), whatever `softcr` says.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case.

## Open points
- C-specific `softcr` indentation (which extensions count as C, how continuations and open blocks are measured) is not specified precisely enough to implement. M1 uses the generic rules for every file. ORACLE CMD-NEWLINE-1.

## Test checklist
- `softcr` off: from (0, 7) → (1, 0).
- `softcr` on, previous line `    foo`, next line empty → (1, 4).
- `softcr` on, next line `  bar` → (1, 2).
- `softcr` on, both lines blank → column 0.
- Last line → line past the end; file unchanged.
- Insert mode: no line break inserted.
- `meta newline` → column 0, meta cleared.
- Head2/Head3 macros (p. 49) with `linsert` produce the comment header.
- `arg newline` → linearg or boxarg selection.
- `arg "x" newline` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
