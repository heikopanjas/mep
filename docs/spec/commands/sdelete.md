# sdelete

Default keys: Del
Source: M guide p. 107 (Table A.3), p. 8 (Ch. 2.3.2), p. 17, p. 28 (Ch. 4.2.1), p. 19 (text-argument editing), p. 21 ("Sdelete always deletes a stream"), p. 10 (Clipboard), p. 52 (Table 6.1), p. 73 (STREAMARG "accepts any kind of cursor movement"); MSJ (pseudo-file `<clipboard>`); IMPLEMENTATION_PLAN.md §6.5, §6.9

## Flags
From p. 107:
- `Sdelete` → NOARG.
- `Arg Sdelete` → NULLARG.
- `Arg streamarg/linearg/boxarg Sdelete` → STREAMARG only: all three are deleted as a stream "from the initial cursor position up to the current cursor position" (p. 107; p. 21). LINEARG and BOXARG are not set; the dispatcher passes any selection as a stream (p. 73; plan §6.5).
- MODIFIES. MARKARG (M2). No TEXTARG, no NUMARG (numeric text is a textarg and is rejected).
- Not KEEPMETA. `Arg Arg` behaves like `Arg`.

## Behavior

### noarg
Deletes the character under the cursor; the rest of the line shifts left. It never deletes a line break, also at the end of the line (p. 28). The clipboard is not touched (p. 107). Cursor does not move. At or beyond the end of the line nothing is deleted. Returns true (also when nothing was deleted). Read-only file: plan §5.6.

### nullarg (`Arg Sdelete`)
Deletes from the cursor through the end of the line including the line break, joining the next line at the cursor position (p. 107, p. 28). If the cursor is beyond the end of the line, the line is first padded with spaces to the cursor column. The deleted text (line break included) goes to the clipboard as a stream (plan §6.9). On the last line of the file there is no following line: only the text to the end of the line is deleted, the last line's terminator state is unchanged, and the clipboard gets the text without a line break. Cursor does not move. Returns true.

### streamarg / linearg / boxarg (any selection)
Deletes the stream from the earlier of anchor and cursor up to (not including) the later one, line breaks included (p. 107). Positions beyond a line end are padded first. A selection made as a linearg (same column, different lines) is still a stream between the two positions, not whole lines. The deleted text goes to the clipboard as a stream; `<clipboard>` is rewritten (MSJ). The cursor goes to the start of the deleted range. ORACLE CMD-BLOCK-1. Returns true.

### In arg mode, text being edited
Deletes the character at the text cursor (p. 19).

### meta
No meta form. Ignored and cleared.

## Rejected arguments
textarg and numarg (e.g. `arg "3" sdelete`) → `invalid argument`, false.

## Open points
- Cursor position after a block delete: start of the deleted range. ORACLE CMD-BLOCK-1.
- `Arg Sdelete` beyond the end of the line pads before joining (p. 107 "joining … at the point of the cursor position"); on the last line no join. Project reading.

## Test checklist
- `abcd`, cursor 1 → `acd`, cursor 1, clipboard unchanged, true.
- End of line → nothing deleted, no join, true.
- `arg sdelete` on `abc`/`def` at 1 → `adef`; clipboard stream `bc` + line break.
- `arg sdelete` at column 5 of `abc` → `abc  def`; clipboard = line break only.
- `arg sdelete` on the last line → text to end deleted, no join.
- Streamarg `arg right right sdelete` on `abcd` at 0 → `cd`; clipboard stream `ab`.
- Linearg `arg down sdelete` from (0, 2) → stream from (0, 2) to (1, 2) deleted; lines joined.
- Box-shaped `arg down right right sdelete` → stream, not a box.
- Selection made upward/leftward → same stream (positions ordered).
- `arg "3" sdelete` → `invalid argument`.
- `arg "abc" left sdelete` → text `ab`.
- Clipboard payload shape `stream`; `<clipboard>` shows the text.
- `undo` restores text; clipboard keeps the deleted text.
