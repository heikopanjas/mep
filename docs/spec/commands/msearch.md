# msearch

Default keys: F4
Source: M guide p. 101 (Table A.3), p. 33 (Ch. 4.4 "Backward searches take place from the current cursor position to the beginning of the file"), p. 61 (`case`), p. 88 (Table A.1), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §3.5, §6.3, §6.7, §6.11

## Flags
Same as psearch.md (p. 33 "uses syntax identical to Psearch"): NOARG, NULLARG with NULLEOW, TEXTARG, BOXSTR; `Arg Arg` = regex (M2). Not MODIFIES, not KEEPMETA.

## Behavior
Plain strings in M1, shared search string with `psearch`, `case` switch, sliced and cancellable, no wrap: from the cursor back to the beginning of the file (p. 33).

Finds the match that starts closest before the cursor (a match starting at the cursor is skipped). Found: cursor on the first character of the match, window moved, match highlighted, true. Not found: no movement, message, false.

### noarg
Previous search string backward (p. 101). None → message, false.

### nullarg
Text from the anchor to the first blank (p. 101) becomes the search string. Empty → `invalid argument`, false.

### textarg / streamarg
Search backward for the text (p. 101).

### `Arg Arg` forms (`Arg Arg Msearch`, `Arg Arg textarg/streamarg Msearch`)
The text (or the word at the anchor for a nullarg) is a regular expression in the syntax of the `regex` crate (plan §10, project decision replacing the M guide's dialect), searched backward. `case` off makes it case-insensitive. A match lies within one line. An invalid pattern → the crate's message, false, the previous search is kept. The pattern becomes the previous search, so a plain `msearch` repeats the regular-expression search.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
linearg, boxarg → `invalid argument`, false.

## Open points
- No wrap (p. 33).

## Test checklist
- Cursor at the end of the p. 7 sample, `arg "mind" msearch` → last `mind` before the cursor, true.
- Repeat `msearch` → the one before; at the first → message, false.
- Match starting at the cursor skipped.
- `psearch` then `msearch` share the string.
- Case on/off.
- `arg msearch` on a word; on a blank → `invalid argument`.
- `arg arg "x" msearch` → `not yet supported`.
- `arg down msearch` → `invalid argument`.
