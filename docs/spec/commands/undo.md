# undo

Default keys: Alt+Backspace
Source: M guide p. 109 (Table A.3), p. 9 (Ch. 2.3.4), p. 89 (Table A.1), p. 52 (Table 6.1), p. 60 (`undocount`); IMPLEMENTATION_PLAN.md §5.7, §3.5

## Flags
NOARG only (one syntax line). Not KEEPMETA. Not MODIFIES in the sense of recording an undo step (undo does not undo itself). `Arg Arg` irrelevant.

## Behavior

### noarg
Reverses the most recent editing change in the current file and restores the cursor to where it was before that change (plan §5.7). Repeated `undo` goes further back: the next-to-last change, then the one before (p. 9). At most `undocount` steps are kept per file (p. 60, default 10).

An editing change is one top-level command invocation that modified the file; a macro or `execute` run from a key is one step (plan §5.7, ORACLE UNDO-1). Cursor movement alone is not a change. Undo records are deltas (plan §3.5).

With nothing left to undo: no change, no message. Returns true in every case (Table 6.1 p. 52: always).

Undo is per file. After switching files, `undo` works on the new current file's history. `refresh` discards the history of the reloaded file.

The manual itself has no redo; undo itself is not recorded, so `undo undo` goes back two steps (p. 9). The project adds a `redo` command (`redo.md`) that reapplies what `undo` undid; `redo` does not clear what further `undo` calls can still reach, so the two interleave cleanly.

### meta
No meta form. Meta is ignored and cleared.

## Rejected arguments
Any argument (nullarg, text, numarg, any selection) → `invalid argument`, false.

## Open points
- Macro as one undo step: ORACLE UNDO-1.
- The `modified` flag after undoing back to the saved state: we leave the file marked modified. ORACLE CMD-UNDO-1.
- IMPLEMENTATION_PLAN.md §8 task 12 says "Undo/redo of every editing command". The M guide has no redo function or form. Resolved 2026-11-01: the owner confirmed `redo` should exist, as a project addition — see `redo.md` and ORACLE REDO-1.

## Test checklist
- Type `abc`, `undo` → line restored, cursor back where typing started, true.
- `ldelete`, `undo` → line back, cursor restored; clipboard keeps the deleted line.
- Three changes, `undo undo undo` → all reverted in reverse order.
- `undocount:2`, three changes, three `undo` → only two reverted; the third returns true without change.
- Nothing to undo → true, no change.
- Macro that inserts two lines run from a key, `undo` → both removed (ORACLE UNDO-1).
- Change in file A, switch to B, `undo` → B unchanged; back to A, `undo` → A reverted.
- After undoing to the saved state, `modified` still shown (ORACLE CMD-UNDO-1).
- `arg undo` → `invalid argument`, false.
- `meta undo` → undoes, meta cleared.
