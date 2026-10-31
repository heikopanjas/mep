# arg

Default keys: Alt+A
Source: M guide p. 93 (Table A.3), pp. 8–9 (Ch. 2.3.3), pp. 16–24 (Ch. 3.2–3.3), p. 50 (Table 6.1), p. 72 (cmdTable flags); MSJ Fig. 5, Fig. 6 and the `selectall` example; IMPLEMENTATION_PLAN.md §6.5

## Flags
`arg` is the argument introducer, not a consumer. It has no arg-kind flags: the dispatcher never classifies an argument for it and never rejects it.
- KEEPMETA: `Arg Meta Compile` and `Arg textarg Meta Setfile` (M guide p. 17) show meta entered inside an argument, and the meta prefix "should occur just before the function that it modifies". `arg` therefore never clears meta, so `Meta Arg Exit` and `Arg Meta Exit` reach `exit` with the same state.
- Not CURSORFUNC, not WINDOWFUNC, not MODIFIES.
- `Arg Arg` is the reason the count exists: each further `arg` raises the count (M guide p. 16 rule 2, p. 17).

## Behavior

### noarg (arg mode off)
Starts arg mode. The cursor position becomes the anchor ("initial cursor position", M guide p. 21) and is highlighted with `hgcolor`. Arg count = 1. The dialog line is not changed until text is typed. Cursor does not move. Returns true (Table 6.1 p. 50: always).

### Arg pressed again before any text or movement
Arg count + 1 (`Arg Arg` = 2). Anchor unchanged. Returns true.

### Arg pressed after the cursor moved (selection pending)
Not described in the manual. Project choice: the count goes up by one; anchor and selection stay. Returns true. ORACLE CMD-ARG-2.

### Arg pressed while a text argument is being edited
Clears the text from the text cursor to the end of the text (M guide p. 19, item 6). The count does not change. Returns true.

### What happens after arg (handled by the arg model, specified here)
- Printable characters (keys bound to `graphic`) start a text argument. The dialog line shows `Arg: ` followed by the text (M guide p. 19). The text is classified when the next non-editing function runs: a valid decimal integer is a numarg if that function accepts NUMARG, a mark name is a markarg (M2 only), anything else is a textarg. A function that does not accept NUMARG receives a numarg as a plain textarg (M guide p. 20); the same holds for markarg (p. 20).
- Text editing on the dialog line (M guide p. 19): `sdelete` deletes the character at the text cursor; `cdelete` deletes the character left of it; `left` and `right` move without deleting, and `right` past the end copies the next character of the previous text argument into the text; `sinsert` inserts a space; `begline`/`endline` go to the start/end of the text; `arg` clears to the end of the text.
- CURSORFUNC functions (before any text is typed) move the cursor and extend the selection. Classification when the next non-cursor function runs follows plan §6.5: no movement = nullarg, same line = streamarg, same column = linearg, otherwise boxarg. The right edge is exclusive (M guide pp. 22, 24).
- `meta` while in arg mode toggles meta and leaves the argument untouched (MSJ, `bigprint` discussion).
- `cancel` ends arg mode (see cancel.md).
- Any other function consumes the argument: the dispatcher checks it against that function's flags, converts numarg to linearg where the function has NUMARG and does not count by itself (plan §6.5, MSJ Fig. 6), and arg mode ends whether the function succeeds or not.
- A macro may end with an open arg; the next function consumes it (MSJ `selectall`; plan §6.14).

### meta forms
`meta` does not change what `arg` does. Meta stays set (KEEPMETA).

## Rejected arguments
None. `arg` never returns false (Table 6.1 p. 50).

## Open points
- `Arg Arg Arg` (count 3 or more): the manual only distinguishes `Arg` and `Arg Arg`. We store the real count and every command treats count ≥ 2 as the `Arg Arg` form. ORACLE CMD-ARG-1: does a third `arg` act like `Arg Arg`, reset to 1, or something else?
- `arg` or typing after the cursor has moved: we keep anchor and selection on `arg` (count + 1). Typing a printable character while a selection is pending discards the selection, puts the anchor back on the cursor position, and starts a text argument with the same count. ORACLE CMD-ARG-2.
- Text-argument editing details not in the manual (ORACLE CMD-ARG-3; we implement the listed choices):
  - typed characters follow the current insert/overtype mode inside the text (the manual lists `sinsert` "insert a space", which suggests overtype, but it does not say);
  - `emacscdel` (Backspace, the default key) acts like `cdelete` inside the text, because the default Backspace key would otherwise be useless there;
  - `quote` inserts the next key literally into the text;
  - any other CURSORFUNC (`up`, `down`, `tab`, …) while text is pending receives the text, rejects it ("invalid argument", false) and arg mode ends;
  - which texts count as "the previous text argument" for `right` past the end: the same buffer as `lasttext` (see lasttext.md).
- MSJ Fig. 5 describes a nullarg as "cursor to end of line" in general; whether it means to end of line or to end of word is per command (NULLEOL / NULLEOW, M guide p. 72).

## Test checklist
- `arg` once: arg mode, count 1, anchor highlighted at the cursor, cursor unchanged, returns true.
- `arg arg`: count 2; `arg arg arg`: count 3, treated as `Arg Arg` by `psearch` (ORACLE CMD-ARG-1).
- `arg` then text `abc`: dialog line `Arg: abc`; `arg` with the text cursor in the middle clears to the end.
- Text editing: `sdelete`, `cdelete`, `left`, `right` past the end (copies from previous text), `sinsert`, `begline`, `endline`, `emacscdel` (ORACLE CMD-ARG-3).
- numarg classification: `arg "10" ldelete` deletes 10 lines; `arg "10" paste` inserts the text `10`.
- Movement after `arg`: same line → streamarg, same column → linearg, other → boxarg; right edge exclusive.
- `arg right right arg`: count 2, selection kept (ORACLE CMD-ARG-2).
- `arg right x`: selection dropped, text `x` started (ORACLE CMD-ARG-2).
- `meta arg exit` and `arg meta exit` give the same result (meta kept).
- `arg` inside a macro left open is consumed by the next function after the macro.
- `arg` at end of file and beyond end of line (virtual space): anchor placed there.
