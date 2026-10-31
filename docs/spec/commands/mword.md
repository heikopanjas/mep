# mword

Default keys: Ctrl+Left, Ctrl+A
Source: M guide p. 101 (Table A.3), p. 27 (Ch. 4.1.3), p. 87 (Table A.1), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §6.3

## Flags
CURSORFUNC (Table A.1 p. 87). No arg-kind flags. Not KEEPMETA: `Meta Mword` uses meta. `Arg Arg` does not apply.

## Behavior
Word definition as in pword.md (ORACLE CMD-PWORD-1). Crosses line boundaries backwards.

### noarg
"Moves the cursor to the beginning of a word. If not in a word or at the first character, use the previous word, otherwise use the current word" (p. 101). So: inside a word but not on its first character → start of that word; on a word's first character or not in a word → start of the previous word. Returns true if moved; false if there is no such word (cursor unchanged).

### meta noarg (`Meta Mword`)
Moves the cursor to the immediate right of the previous word: the nearest word that ends before the cursor (p. 101). Returns true if moved, false if none.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case.

## Open points
- Word characters: ORACLE CMD-PWORD-1.

## Test checklist
- `foo bar`, cursor 5 (inside `bar`) → 4, true.
- Cursor 4 (first char of `bar`) → 0.
- Cursor 3 (blank) → 0.
- First word of a line → start of the last word on the previous nonblank line.
- Before the first word of the file → false.
- `meta mword` at 6 of `foo bar` → 3; at 4 → 3.
- Beyond end of line (column 20 of `foo bar`) → 4.
- `arg mword` → streamarg selection.
- `arg "x" mword` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
- `meta mword` → meta form above, meta cleared afterwards.
