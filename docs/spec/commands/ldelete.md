# ldelete

Default keys: Ctrl+Y
Source: M guide p. 98 (Table A.3), p. 10 (Ch. 2.3.5), pp. 28–29 (Ch. 4.2.1), p. 20 (`Arg 10 Ldelete`), p. 22 ("characters on a single line are always treated as a stream"), p. 88 (Table A.1), p. 51 (Table 6.1); MSJ (`<clipboard>` rewritten on block delete); IMPLEMENTATION_PLAN.md §3.3, §6.5, §6.9

## Flags
From p. 98 and p. 20:
- `Ldelete` → NOARG.
- `Arg Ldelete` → NULLARG.
- `Arg linearg Ldelete` → LINEARG.
- `Arg boxarg Ldelete` → BOXARG (a multi-line box-shaped selection is a box; plan §6.5).
- `Arg streamarg Ldelete` → STREAMARG (one line only, because BOXARG takes the multi-line shapes).
- `Arg numarg Ldelete` (p. 20) → NUMARG, converted by the dispatcher to a linearg of n lines from the cursor line.
- MARKARG (M2). MODIFIES. No TEXTARG.
- Not KEEPMETA. `Arg Arg` behaves like `Arg`.

## Behavior
All forms put the deleted text into the clipboard with its shape, write it to the system clipboard (plan §3.3, §6.9) and rewrite `<clipboard>`. Read-only file: plan §5.6.

### noarg
Deletes the current line and puts it into the clipboard as one line (line shape) (p. 98). The cursor stays on the same line index and column (now on the following line). Cursor beyond the last line: the file is unchanged and the clipboard gets one empty line. ORACLE CMD-LDELETE-1. Returns true.

### nullarg (`Arg Ldelete`)
Deletes from the anchor to the end of the line; does not join (p. 98, p. 29). Clipboard: that text as a stream, no line break. Beyond the end of the line: nothing deleted, clipboard gets an empty stream. Cursor does not move. Returns true.

### linearg / numarg
Deletes the whole lines from the upper to the lower line of the range; clipboard gets them as lines (p. 98). A numarg of n deletes lines cursor … cursor + n − 1 (p. 20), clamped at the end of the file. Cursor: upper line, same column. Returns true.

### boxarg
Deletes the box: on each line from the upper to the lower line, the cells from the left edge up to (not including) the right edge (p. 24, p. 29). Text right of the box shifts left. Lines shorter than the left edge are unchanged. A wide character belongs to the box if its first cell is inside (plan §3.2). Clipboard: box shape, width = box width in cells, rows padded with spaces to that width. Cursor: upper-left corner of the box (ORACLE CMD-BLOCK-1). Returns true.

### streamarg (one line)
Deletes the cells between the two positions on the line. Clipboard: stream (p. 22 "always treated as a stream"). Cursor: the left position. Returns true.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
textarg (non-numeric, e.g. `arg "abc" ldelete`) → `invalid argument`, false.

## Open points
- `ldelete` on a line beyond the end of the file. ORACLE CMD-LDELETE-1.
- Z treats a streamarg as "a box one line high"; the M guide (p. 22) says streams are streams. M wins; the only visible difference is the clipboard shape.
- Cursor after a box delete: ORACLE CMD-BLOCK-1.

## Test checklist
- `alpha`/`beta`/`gamma`, cursor (1, 2), `ldelete` → `alpha`/`gamma`, cursor (1, 2), clipboard line `beta`.
- Beyond the last line → file unchanged, clipboard one empty line (ORACLE CMD-LDELETE-1).
- `arg ldelete` at (0, 2) of `alpha` → `al`; clipboard stream `pha`; no join.
- `down arg down ldelete` (plan §9.3 example) → `alpha`; clipboard lines `beta`,`gamma`; cursor (1, 0).
- `arg "2" ldelete` on line 0 → lines 0–1 deleted; `arg "10" ldelete` near the end → clamped.
- Box from (0, 1) to (2, 3) → cells 1–2 removed on lines 0–2; clipboard box width 2.
- Box over short lines → padding in the clipboard rows; plain-text clipboard rows trimmed (plan §3.3).
- Box edge splitting a wide character.
- Streamarg on one line → stream in the clipboard.
- `arg "abc" ldelete` → `invalid argument`.
- System clipboard write failure (fake) → internal clipboard used, one-time message (plan §6.9); command still true.
- `undo` restores the text.
