# meta

Default keys: F9
Source: M guide p. 100 (Table A.3), p. 16–17 (Ch. 3.2 rule 3), p. 51 (Table 6.1), p. 13 (status line field 8), p. 72 (KEEPMETA); MSJ `bigprint` example and Fig. 5; IMPLEMENTATION_PLAN.md §6.6

## Flags
KEEPMETA (by nature: it is the meta prefix). No arg-kind flags: `meta` does not consume, check or change a pending argument ("this action does not affect the selected argument", MSJ). Not CURSORFUNC, not MODIFIES.

## Behavior

### noarg
Toggles the meta flag. The status line shows `meta` while it is set (M guide p. 13 field 8; p. 17 calls it `(meta)`). Returns the new state: true if meta is now on, false if now off (Table 6.1 p. 51; MSJ `bigprint`: "it has a return value to indicate the new state"). Cursor does not move.

### With an argument pending (nullarg, text, selection)
Same toggle. The argument, its count, anchor, selection and text stay exactly as they were, so `Arg Meta Compile` and `Arg textarg Meta Setfile` work (M guide p. 17).

### Effect on the next function
The next function that is not KEEPMETA receives the meta state (fMeta, M guide p. 75) and clears meta after it runs (plan §6.6), whether or not it has a meta form. Functions without a meta form ignore it.

## Rejected arguments
None.

## Open points
- The Z guide's Table 6.1 says "If meta was on / was off", which could mean the old state. MEP (M guide p. 51 "Meta now on") and MSJ agree on the new state; we implement the new state.
- `cancel` clears meta in our implementation (see cancel.md, ORACLE CMD-CANCEL-1).

## Test checklist
- `meta`: status shows `meta`, returns true; `meta meta`: status clear, second call returns false.
- `arg "x" meta`: text argument still `x`, count unchanged, meta on.
- `arg right right meta`: selection unchanged.
- `meta up` moves to the top of the window and clears meta; `meta graphic` (type `a`) inserts `a` and clears meta.
- MSJ `bigprint` structure: `meta +>nometa …` branches on the returned state (with and without meta pressed before the macro).
- `meta arg exit` keeps meta through `arg`.
