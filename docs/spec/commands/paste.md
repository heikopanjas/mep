# paste

Default keys: Shift+Ins
Source: M guide p. 102 (Table A.3), pp. 10–11 (Ch. 2.3.5), p. 21 (`Happy New Year`), p. 30 (Ch. 4.2.4 reading a file), p. 88 (Table A.1), p. 51 (Table 6.1); MSJ (`<clipboard>`); z-vs-mep.md S5; IMPLEMENTATION_PLAN.md §3.3, §6.7, §6.9, §8 M4, §11 (ORACLE S5)

## Flags
From p. 102:
- `Paste` → NOARG.
- `Arg Paste` → NULLARG with NULLEOL (text from the anchor to the end of the line).
- `Arg textarg Paste` → TEXTARG.
- `Arg streamarg Paste` → BOXSTR (one-line selection as text).
- `Arg Arg textarg/streamarg Paste` → same flags; count 2 selects the file-insert form.
- `Arg Arg !textarg/!streamarg Paste` → M4.
- No NUMARG (`arg "10" paste` inserts the text `10`, p. 20), no LINEARG, no BOXARG.
- MODIFIES. Not KEEPMETA.

## Behavior
Paste always inserts, whatever the insert/overtype mode. Read-only file: plan §5.6.

### noarg
Reads the system clipboard first (plan §6.9; an I/O request, the command suspends, §3.5). Shape: from the private record if present and its hash matches the text; otherwise text ending in a newline → line, else stream (plan §3.3).
- line: the lines are inserted above the current line (p. 102 "prior to the current line"; p. 11 example). Inserted lines use the file's default terminator.
- box: inserted as a box at the cursor: each row goes into the cursor line and the following lines at the cursor column, shifting text right; short lines are padded; lines beyond the end of the file are appended; a half-cell gap at a wide character is filled with a space (plan §3.2).
- stream: inserted at the cursor; a multi-line stream breaks the line. Beyond the end of the line the line is padded first.
The cursor keeps its line index and column (for a line paste it is then on the first pasted line). ORACLE CMD-PASTE-1. Empty clipboard: nothing inserted. Returns true (Table 6.1 p. 51: always).

### nullarg (`Arg Paste`)
"Inserts the text from the initial cursor position to the end of the line at the initial cursor position" (p. 102). NULLEOL makes this the textarg form, so the text also goes into the clipboard (stream). ORACLE CMD-PASTE-2.

### textarg / streamarg (count 1)
Puts the text into the clipboard as a stream (system clipboard too, `<clipboard>` rewritten) and inserts it at the initial cursor position (the anchor) (p. 102; p. 11 "inserted at the current cursor position"). Cursor unchanged (ORACLE CMD-PASTE-1). Returns true.

### `Arg Arg textarg/streamarg Paste` (file insert)
Treats the text as a file name and inserts the file's contents. The manual contradicts itself: p. 30 "at the current cursor position", p. 102 "above the current line". Project decision (IMPLEMENTATION_PLAN.md §11 S5): at the cursor, as a stream. ORACLE S5. The file is read through an I/O request (plan §3.5) and its bytes are inserted unchanged except that inserted line breaks use the file's default terminator. The clipboard is not changed. Cursor unchanged.
- File not found or unreadable → message naming the file, false. ORACLE CMD-PASTE-3.
- `Arg Arg Paste` with a nullarg (NULLEOL) uses the rest of the line as the file name.

### `Arg Arg !textarg/!streamarg Paste`
Runs the text as a command and inserts its standard output (p. 102). M4 (plan §6.7). M1: message `not available`, false, nothing changes.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
linearg, boxarg (multi-line selections) → `invalid argument`, false.

## Open points
- File insert position: ORACLE S5 (at cursor, per plan).
- Cursor after any paste: ORACLE CMD-PASTE-1.
- Nullarg writes the clipboard: ORACLE CMD-PASTE-2.
- Return value: Table 6.1 says paste always returns true. The plan (§0 rule 5) says every failure returns false with a message. Project decision: false for a rejected argument, for `not available`, and for a failed file read; true otherwise. ORACLE CMD-PASTE-3.

## Test checklist
- `ldelete` two lines, move to (0, 0), `paste` → lines above line 0 (p. 11 example), true.
- Line paste in a CRLF file → CRLF terminators.
- Box paste at (1, 3) over shorter lines → padding; box past the end of the file appends lines.
- Box paste splitting a wide character → space fills the gap.
- Stream paste `ab⏎cd` at (0, 2) of `xyz` → `xyab` / `cdz`.
- External text `foo\n` (no private record) → line paste; `foo` → stream paste.
- Stale private record (hash mismatch) → ignored, newline rule used.
- Empty clipboard → nothing, true.
- `arg "The Philosopher said," paste` → inserted at the cursor, clipboard = that text.
- `arg "10" paste` → text `10` inserted.
- `arg paste` at (0, 2) of `abcd` → `abcdcd`; clipboard `cd` (ORACLE CMD-PASTE-2).
- Streamarg `arg right right paste` → text inserted at the anchor.
- `arg arg "other.txt" paste` → file contents at the cursor (ORACLE S5).
- `arg arg "missing.txt" paste` → message, false (ORACLE CMD-PASTE-3).
- `arg arg "!ls" paste` → `not available`, false.
- `arg down paste` → `invalid argument`.
- Overtype mode → still inserts.
- Clipboard read held (`hold`) → editor suspended, type-ahead queued; Esc cancels → false.
