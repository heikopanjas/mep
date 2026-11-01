# psearch

Default keys: F3
Source: M guide p. 104 (Table A.3), pp. 11–12 (Ch. 2.3.6), p. 17 (Arg Arg = regex), pp. 32–33 (Ch. 4.4, 4.4.1), p. 61 (`case`), p. 59 (`noise`), p. 88 (Table A.1), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §3.5, §6.3, §6.7, §6.11

## Flags
From p. 104:
- `Psearch` → NOARG.
- `Arg Psearch` → NULLARG with NULLEOW ("the characters from the initial cursor position to the first blank character").
- `Arg textarg Psearch` → TEXTARG.
- `Arg streamarg Psearch` → BOXSTR (one-line selection as text).
- `Arg Arg …` forms → same flags, count 2 = regular expression (M2).
- No NUMARG (a number is searched as text), no LINEARG, no BOXARG.
- Not MODIFIES. Not KEEPMETA.

## Behavior
M1 searches plain strings only (plan §6.7). There is one search string shared with `msearch` ("the previously defined string", p. 104). Comparison follows `case` (p. 61; default off = case-insensitive, compared character by character with Unicode simple case folding). A match lies within one line.

The search runs from the cursor to the end of the file and does not wrap: "All searches take place from the current cursor position to the end of the file" (p. 32). It starts one character after the cursor, so repeating the search finds the next match. A cursor beyond the end of the line starts on the next line. The search runs in bounded slices; Esc cancels it (plan §3.5) and the command returns false. Progress display via `noise` comes later (plan §6.13).

Found: the cursor moves to the first character of the match, the window moves to show it (cursor line `hike` lines from the top when the jump exceeds a window, plan §6.3), the matched text is highlighted until the next command. Returns true.

Not found: no cursor movement, a message on the dialog line (project wording `<text> not found`), false (p. 104; Table 6.1 p. 51).

### noarg
Searches for the previous search string (p. 33). None defined → message, false.

### nullarg (`Arg Psearch`)
The text from the anchor to the first blank or the end of the line becomes the search string (p. 12, p. 104). Empty (anchor on a blank or beyond the end of the line) → `invalid argument`, false.

### textarg / streamarg
The text becomes the search string, then the search runs (p. 33). The string is remembered even if not found.

### `Arg Arg` forms (`Arg Arg Psearch`, `Arg Arg textarg/streamarg Psearch`)
The text (or the word at the anchor for a nullarg) is a regular expression in the syntax of the `regex` crate (plan §10, project decision replacing the M guide's dialect), searched forward. `case` off makes it case-insensitive. A match lies within one line. An invalid pattern → the crate's message, false, the previous search is kept. The pattern becomes the previous search, so a plain `psearch` repeats the regular-expression search.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
linearg, boxarg (multi-line selections) → `invalid argument`, false.

## Open points
- Wrap: the manual says the search ends at the end of the file (p. 32); plan §6.11 asked to "confirm"; we do not wrap.
- Start position (one character after the cursor) is our reading of "next occurrence" (p. 33).

## Test checklist
- `arg "mind" psearch` from (0, 0) on the p. 7 sample → cursor at the first `mind`, highlighted, true.
- `psearch` again → next occurrence.
- Match at the cursor position is skipped.
- Last occurrence passed → message, false, cursor unchanged (no wrap).
- `nocase:` → `Mind` matches `mind`; `case:` → does not.
- `arg psearch` with the cursor on `mind` → searches `mind`; on a blank → `invalid argument`.
- Streamarg over `mind` → same as textarg.
- `arg "10" psearch` → searches the text `10`.
- `psearch` with no previous string → message, false.
- `arg arg "m?nd" psearch` → `not yet supported`, false, previous string unchanged.
- Match far below → window jumps, cursor `hike` lines from the top.
- Large file, search interrupted by Esc → false, cursor unchanged.
- Match containing a wide character; case folding of non-ASCII letters.
- `arg down psearch` → `invalid argument`.
