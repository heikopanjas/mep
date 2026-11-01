# copy

Default keys: Ctrl+Ins, Num+
Source: M guide p. 95 (Table A.3), p. 29 (Ch. 4.2.2), p. 20 (markarg example), p. 22 (streams), p. 88 (Table A.1), p. 50 (Table 6.1); MSJ (print macro Fig. 2: noarg = current line, nullarg = cursor to end of line; `<clipboard>`); IMPLEMENTATION_PLAN.md §3.3, §6.5, §6.9

## Flags
From p. 95:
- `Copy` → NOARG.
- `Arg Copy` → NULLARG.
- `Arg boxarg Copy` → BOXARG (multi-line box shapes are boxes; plan §6.5).
- `Arg linearg Copy` → LINEARG.
- `Arg streamarg Copy` → STREAMARG (one line).
- `Arg textarg Copy` → TEXTARG.
- `Arg numarg Copy` → NUMARG, converted to a linearg of n lines from the cursor line.
- `Arg markarg Copy` → MARKARG (M2; in M1 a mark name is a textarg).
- `Argcol down/up Copy` → COLARG (project addition, argcol.md): a multi-line stream whose anchor line is trimmed to the anchor's column, every other line whole.
- Not MODIFIES. Not KEEPMETA. `Arg Arg` behaves like `Arg`.

## Behavior
Every form replaces the clipboard with the copied text and its shape, writes it to the system clipboard as plain text plus the private shape record (plan §3.3; a write failure falls back to the internal clipboard with a one-time message, §6.9), and rewrites `<clipboard>` (MSJ). The file and the cursor do not change.

### noarg
Copies the current line, line shape (p. 95). On a line beyond the end of the file: one empty line. Returns true.

### nullarg (`Arg Copy`)
Copies from the anchor to the end of the line, line break not included (p. 95), stream shape. Beyond the end of the line: empty stream. Returns true.

### textarg
Copies the text itself, stream shape (p. 95). Returns true.

### numarg
n lines starting with the cursor line, line shape (p. 95, p. 29), clamped at the end of the file. Returns true.

### linearg
The whole lines of the range, line shape. Lines beyond the end of the file are not included. Returns true.

### streamarg (one line)
The cells between the two positions, stream shape (p. 22). Returns true.

### boxarg
The box from the left edge up to (not including) the right edge on each line, box shape with the box width; short rows padded with spaces in the internal clipboard; the plain text trims the padding (plan §3.3). Wide characters per plan §3.2. Returns true.

### colarg (project addition, argcol.md)
The column-anchored stream, stream shape: whichever of the anchor and cursor comes first in the file is trimmed to its own column onward, whichever comes second is trimmed up to its own column, every line strictly between them is whole. Tabs reach the clipboard as spaces, like `boxarg`. Returns true.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
None of the user argument kinds is rejected. An internal failure (none expected) would return false (Table 6.1 p. 50 "Bad argument").

## Open points
- One-line selections are streams (p. 22), so `copy` of a streamarg gives stream shape, not a one-line box.
- `Arg markarg Copy` is M2 (plan §6.7); in M1 `arg "P1" copy` copies the text `P1`.

## Test checklist
- `copy` on `beta` → clipboard line `beta`, file unchanged, true.
- `copy` beyond the last line → clipboard one empty line.
- `arg copy` at (0, 2) of `alpha` → stream `pha`.
- `arg "Happy" copy` → stream `Happy`.
- `arg "2" copy` → two lines, line shape.
- `arg down copy` → two lines; `arg right right copy` → stream of two cells.
- Box over lines of different length → box shape, width kept; plain text rows trimmed, joined with the platform newline; private record hash matches.
- Paste after copy reproduces the shape (paste.md).
- `<clipboard>` opened after `copy` shows the copied text.
- Failing system clipboard → internal clipboard used, one-time message, true.
- `meta copy` → copies, meta cleared.
