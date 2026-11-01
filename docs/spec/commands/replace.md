# replace

Default keys: Ctrl+L
Source: M guide p. 106 (Table A.3), p. 34 (Ch. 4.4.2), p. 88 (Table A.1), p. 52 (Table 6.1); z-vs-mep.md S6; IMPLEMENTATION_PLAN.md §6.5, §8 M2, §10, §11 (ORACLE S6)

## Flags
NOARG and NULLARG (identical, p. 34), LINEARG, BOXARG, STREAMARG, NUMARG (converted to a linearg), MARKARG (always a box, ORACLE S6), COLARG (project addition, argcol.md). Count 2 (`Arg Arg`) selects regular expressions for every form. MODIFIES.

## Behavior

### Prompts
The dialog line asks for the search string and then the replacement string, each prefilled with the value used last by `replace` or `qreplace` (p. 34). Enter accepts; the text can be edited with the text-argument keys (left, right, begline, endline, sdelete, cdelete and Backspace, typing in insert or overtype mode); the key bound to `arg` clears to the end (p. 34). Esc cancels: no change, false. An empty search string → message, false.

### Range
- noarg, nullarg: from the cursor to the end of the file (p. 34).
- linearg / numarg: the whole lines (p. 34).
- streamarg: from the earlier to the later position.
- boxarg: within the box on each of its lines (p. 34).
- markarg: the box between the cursor and the marker (p. 34). ORACLE S6.
- colarg (project addition, argcol.md): within the column-anchored stream on each line it touches — the anchor's own line only from the anchor's column onward (or up to it, going up), every other line in full.
A match must lie entirely within the range and within one line.

### Matching (plan §10)
- `Arg`: the search string is literal text. `Arg Arg`: it is a regular expression in the syntax of the `regex` crate, and the replacement may use `$0`, `$1`…, `${name}` and `$$`.
- `case` off: case-insensitive.
- An invalid pattern → the crate's message, false, no change.
- A match that is empty is skipped.

### Result
Every match in the range is replaced, without confirmation. Long ranges run in slices (plan §3.5); Esc stops the operation and keeps the replacements made so far. The whole command is one undo step. The cursor does not move. Message `<n> replaced`. Returns true if at least one replacement was made; otherwise message `<text> not found`, false (Table 6.1 p. 52).

### meta
No meta form. Ignored and cleared.

## Rejected arguments
Textarg (a typed text that is not a number and not a marker name) → `invalid argument`.

## Open points
- ORACLE S6: markarg range is a box.
- ORACLE RPL-1: prompt order and editing, the cursor staying put, and the `<n> replaced` message are project choices.

## Test checklist
- noarg from the middle of a file replaces only after the cursor; `arg replace` the same.
- linearg, numarg, streamarg, boxarg ranges; markarg box.
- `arg arg` regex with `$1`; invalid pattern; empty match skipped.
- Literal special characters in the single-arg form.
- `case` switch.
- Prefilled prompts reuse the previous strings; Esc at either prompt cancels; empty search string fails.
- Not found → false and message; undo restores everything in one step.
- Esc during a sliced replace stops it.
- `arg "x" replace` → invalid argument; `meta replace` clears meta.
