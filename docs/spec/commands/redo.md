# redo

Project addition: not in the 1988 manual. The M guide has no redo function
or form; `undo.md`'s open points flagged that `IMPLEMENTATION_PLAN.md §8`
task 12's "Undo/redo of every editing command" wording needed the owner's
confirmation. Resolved 2026-11-01: the owner asked for `redo` as part of a
modern CUA-style keymap (`Ctrl+Z` undo, `Ctrl+Y` redo). ORACLE REDO-1.

Default keys: none. M1's built-in keymap keeps `Ldelete` on `Ctrl+Y`
(`keys::DEFAULT_KEYS`); the owner's personal `TOOLS.INI` binds `Ctrl+Y` to
`redo` there instead.

Source: no manual citation. Mirrors `undo.md`'s mechanics
(`IMPLEMENTATION_PLAN.md §5.7`; plan §3.5 delta records) in reverse.

## Flags
NOARG only, matching `undo`. Not KEEPMETA. `Arg Arg` irrelevant.

## Behavior

### noarg (`Redo`)
Reapplies the most recently undone change in the current file (the inverse
of `undo`: `undo.md`'s noarg form) and restores the cursor to its position
after that change. Repeated `redo` goes forward through further undone
steps, most recently undone first.

Any editing command that modifies the file after an `undo` **clears the
pending redo steps** for that file: the discarded future can no longer be
replayed, matching every mainstream editor's convention. `redo` itself does
not clear them, so `undo redo undo redo` toggles back and forth cleanly
(matching `undo.md`'s own "undo is not recorded, `undo undo` goes back two
steps" rule).

With nothing to redo: no change, no message, true (matching `undo`'s
"nothing to undo" rule, Table 6.1 p. 52, by analogy — there is no manual
citation for `redo` itself).

Redo is per file, like undo: after switching files, `redo` works on the new
current file's own history. `refresh` discards it along with the file's
undo history (matching `undo.md`'s `refresh` rule).

### meta
No meta form. Meta is ignored and cleared, matching `undo`.

## Rejected arguments
Any argument (nullarg, text, numarg, any selection) → `invalid argument`,
false, matching `undo`.

## Open points
- ORACLE REDO-1: whether `redo` should exist at all — resolved 2026-11-01,
  the owner confirmed yes, closing the open point `undo.md` raised against
  `IMPLEMENTATION_PLAN.md §8` task 12.
- Implementation note: `Group` (`crates/mep-core/src/editor.rs`) currently
  stores only the cursor *before* its edits (`cursor: Pos`), which is all
  `undo` needs. `redo` additionally needs the cursor *after* the edits, so
  `Group` needs a second field (e.g. `after: Pos`) populated wherever the
  editor seals a group at the end of a top-level command, or an equivalent
  way to compute it. Left to implementation.
- A `redo: Vec<Group>` stack is kept in `FileState` alongside the existing
  `undo: Vec<Group>`, bounded by the same `undocount` switch.
- Default key: none in the built-in keymap; the owner's personal
  `TOOLS.INI` binds `Ctrl+Y` (dropping the default `Ldelete` binding there).

## Test checklist
- Type `abc`, `undo`, `redo` → `abc` back, cursor where it was right after
  typing.
- `ldelete`, `undo`, `redo` → line removed again; the clipboard is
  unaffected by `redo` itself.
- Three changes, `undo undo undo`, `redo redo redo` → all three reapplied
  in their original order.
- `undo`, then a new edit (not `redo`) → the redo stack is cleared; `redo`
  afterward does nothing (true, no change).
- `undo redo undo redo` → toggles cleanly, no stack corruption.
- `undocount:2`, three changes, three `undo`, three `redo` → only two of
  each take effect; the extra calls return true without change.
- Nothing to redo (fresh file, or an empty redo stack) → true, no change.
- Change in file A, switch to B, `redo` → acts on B's own (empty) redo
  stack; A is unaffected.
- `refresh` after `undo` → the redo stack is discarded along with the undo
  history.
- `arg redo` → `invalid argument`, false.
