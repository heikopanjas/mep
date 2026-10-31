# emacsnewl

Default keys: Enter
Source: M guide p. 96 (Table A.3), p. 88 (Table A.1 "Starts new line, breaking current line"), p. 51 (Table 6.1), p. 61 (`softcr`); IMPLEMENTATION_PLAN.md §5.1, §5.3, §6.13

## Flags
NOARG, MODIFIES. Not CURSORFUNC (Table A.1 lists it under Inserting/Deleting Text). Not KEEPMETA. `Arg Arg` does not apply.

## Behavior

### noarg
"Performs similarly to *Newline*, except that when in insert mode, it breaks the current line at the cursor position" (p. 96).
- Overtype mode: exactly `newline` (newline.md): cursor to the next line, column per `softcr`, no change to the file.
- Insert mode: the line is split at the cursor. The text right of the cursor moves to a new line below, using the file's default terminator (plan §5.1). The cursor goes to the new line at the column `softcr` gives (newline.md rules, with the new line's own text as "the line"); that column is filled with spaces before the moved text. Trailing blanks left on the first half are trimmed per plan §5.3. Cursor beyond the end of the line: the new line is empty. Cursor beyond the end of the file: like `newline`, nothing is inserted.
Read-only file (insert mode): plan §5.6. Returns true (Table 6.1 p. 51: always).

### meta noarg
Not listed. Because `emacsnewl` "performs similarly to Newline", `meta emacsnewl` uses column 0 like `meta newline` (no `softcr` indent). ORACLE CMD-EMACSNEWL-1.

## Rejected arguments
Any argument (nullarg, text, selection) → `invalid argument`, false.

## Open points
- Meta form inherited from `newline`. ORACLE CMD-EMACSNEWL-1.
- C-specific `softcr` rules: ORACLE CMD-NEWLINE-1.

## Test checklist
- Insert mode, `    foo bar`, cursor 8, `softcr` on → `    foo` and `    bar`, cursor (1, 4).
- `softcr` off → `bar` at column 0, cursor (1, 0).
- Insert mode at the end of the line → new empty line, cursor at the softcr column, no trailing spaces written.
- Overtype → cursor moves, file unchanged.
- CRLF file → new line gets CRLF.
- `meta emacsnewl` (insert mode) → cursor (1, 0), text not indented (ORACLE CMD-EMACSNEWL-1).
- `arg emacsnewl` → `invalid argument`.
- `undo` restores the line.
