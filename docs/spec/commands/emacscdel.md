# emacscdel

Default keys: Backspace
Source: M guide p. 96 (Table A.3), p. 88 (Table A.1 "Deletes character to left, including line breaks"), p. 51 (Table 6.1), p. 118 (Glossary "emacs"); IMPLEMENTATION_PLAN.md §5.3

## Flags
NOARG, MODIFIES. Not KEEPMETA. `Arg Arg` does not apply.

## Behavior

### noarg
"Performs similarly to *Cdelete*, except that at the beginning of a line while in insert mode, *Emacscdel* deletes the line break between the current line and the previous line, joining the two lines together" (p. 96).
- Insert mode, column 0, line > 0: the line break before the current line is deleted; the cursor goes to the join point (old end of the previous line). If the current line is beyond the end of the file, the cursor just moves to the end of the previous line. Returns true.
- Every other case exactly as cdelete.md (overtype at column 0 moves to the end of the previous line without joining; beyond end of line moves to the line end; (0, 0) → false).
Read-only file: plan §5.6.

### In arg mode, text being edited
Acts like `cdelete` on the text. ORACLE CMD-ARG-3.

### With a nullarg or selection
Rejected (`invalid argument`), as for `cdelete` (ORACLE CMD-CDELETE-1).

### meta
No meta form; ignored and cleared.

## Rejected arguments
nullarg, numarg, any selection → `invalid argument`, false.

## Open points
- Text editing behavior (ORACLE CMD-ARG-3); selection behavior (ORACLE CMD-CDELETE-1).
- Joining keeps the previous line's terminator for the joined line; the removed terminator is the previous line's.

## Test checklist
- Insert mode, `ab` / `cd`, cursor (1, 0) → `abcd`, cursor (0, 2), true.
- Overtype, same position → cursor (0, 2), no join.
- Insert mode, mid-line → like `cdelete`.
- (0, 0) → false.
- CRLF file: join removes CRLF; file stays CRLF elsewhere.
- `arg "abc" emacscdel` → text `ab`.
- `arg down emacscdel` → `invalid argument`.
