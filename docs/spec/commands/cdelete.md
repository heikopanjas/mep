# cdelete

Default keys: Ctrl+G
Source: M guide p. 94 (Table A.3), p. 19 (text-argument editing), p. 88 (Table A.1 "Deletes character to left, excluding line breaks"), p. 50 (Table 6.1); IMPLEMENTATION_PLAN.md §5.3, §5.4

## Flags
NOARG, MODIFIES. The one syntax line has no argument. Not KEEPMETA. `Arg Arg` does not apply.

## Behavior

### noarg
p. 94:
- Cursor beyond the end of the line: the cursor moves to the immediate right of the last character of the line; nothing is deleted. Returns true.
- Cursor in column 0, line > 0: the cursor moves to the end of the previous line (right of its last character); no line break is deleted. Returns true.
- Cursor at (0, 0): nothing happens, false (Table 6.1 p. 50 "Cursor not moved").
- Otherwise, insert mode: the character left of the cursor is deleted, the line gets shorter, the cursor moves left by that character's width.
- Otherwise, overtype mode: the character left of the cursor is replaced by blanks (one per cell of its width, so later columns do not move) and the cursor moves onto them.
The line is edited, so trailing blanks may be trimmed (plan §5.3). Read-only file: plan §5.6. Returns true when the cursor moved.

### In arg mode, text being edited
Deletes the character left of the text cursor (p. 19). False at the start of the text.

### In arg mode with a nullarg or a selection
Rejected (`invalid argument`). ORACLE CMD-CDELETE-1.

### meta
No meta form; ignored and cleared.

## Rejected arguments
nullarg, numarg, textarg (outside text editing), any selection → `invalid argument`, false.

## Open points
- Z says that with an argument being entered, `cdelete` acts as cursor-left (it would extend a selection). The M guide does not say. We reject the argument per the flag model. ORACLE CMD-CDELETE-1.
- Z also moves the cursor to column 0 when only blanks are left of it; this is not in the M guide and is not implemented.
- "Immediate right of the last character": we use the line length (trailing blanks of an unedited line count).

## Test checklist
- Overtype `abcd`, cursor 2 → `a cd`, cursor 1, true.
- Insert mode → `acd`, cursor 1.
- Column 0 of line 1 → cursor at end of line 0, no join, true.
- (0, 0) → false.
- Cursor at column 10 of `abc` → cursor 3, nothing deleted, true.
- Overtype on a wide character left of the cursor → two blanks, cursor on the first.
- Overtype at the end of `abc` (cursor 3) → `ab` after trailing trim (trailspace off), cursor 2.
- `arg "abc" cdelete` → text `ab`.
- `arg right cdelete` → `invalid argument` (ORACLE CMD-CDELETE-1).
- `meta cdelete` → deletes, meta cleared.
