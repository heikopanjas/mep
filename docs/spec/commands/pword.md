# pword

Default keys: Ctrl+Right, Ctrl+F
Source: M guide p. 104 (Table A.3), p. 27 (Ch. 4.1.3), p. 87 (Table A.1), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §6.3

## Flags
CURSORFUNC (Table A.1 p. 87). No arg-kind flags. Not KEEPMETA: `Meta Pword` uses meta. `Arg Arg` does not apply.

## Behavior
Word definition (shared with `mword`; the manual does not define a word): a maximal run of word characters, where a word character is a Unicode letter, a Unicode digit or `_`. Everything else, including punctuation and line breaks, separates words. ORACLE CMD-PWORD-1.

The search crosses line boundaries. Positions beyond the end of a line count as non-word. The window follows (plan §6.3).

### noarg
Moves the cursor forward to the beginning of the next word (p. 104 "forward one word and places the cursor on the beginning of the new word"). If the cursor is inside a word, that word is skipped first. Returns true if moved; false if there is no next word before the end of the file (cursor unchanged).

### meta noarg (`Meta Pword`)
Moves the cursor to the immediate right of the current word, or, if the cursor is not in a word, to the immediate right of the next word (p. 104). Returns true if moved; false if there is no such word.

### In arg mode, no text typed
Same movement; the selection extends.

### In arg mode, text being edited
Rejected (`invalid argument`). ORACLE CMD-ARG-3.

## Rejected arguments
Only the pending-text case.

## Open points
- Word characters: letters, digits and `_` (our choice). The search code (`Arg Psearch`) uses "up to the first blank" instead, so the two definitions differ on purpose. ORACLE CMD-PWORD-1.

## Test checklist
- `foo bar`, cursor 0 → 4, true.
- `foo.bar` → `.` separates: from 0 → 4.
- `foo_bar baz` → `_` is part of the word: from 0 → 8.
- Last word of a line → first word of the next nonempty line.
- Last word of the file → false, cursor unchanged.
- `meta pword` inside `foo` (cursor 1) → 3; on the blank at 3 of `foo bar` → 7.
- Unicode word `größe bar` → whole `größe` is one word.
- `arg pword` → streamarg (same line) or boxarg/linearg when it crosses lines.
- `arg "x" pword` → `invalid argument`, false, arg mode ended (ORACLE CMD-ARG-3).
- `meta pword` → meta form above, meta cleared afterwards.
