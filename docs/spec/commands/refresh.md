# refresh

Default keys: Shift+F7
Source: M guide p. 106 (Table A.3), p. 89 (Table A.1 "Rereads file, discarding edits"), p. 52 (Table 6.1); MSJ Fig. 1 ("Reread or discard current file"); IMPLEMENTATION_PLAN.md §3.5, §6.10

## Flags
From p. 106:
- `Refresh` → NOARG.
- `Arg Refresh` → NULLARG.
- No text or selection forms. Not KEEPMETA. Not MODIFIES in the undo sense (it replaces the buffer). `Arg Arg` behaves like `Arg`.

## Behavior
Both forms ask for confirmation on the dialog line first (p. 106), whether or not the file is modified. Only `y`/`Y` confirms; any other key (and `cancel`) declines: no change, false (Table 6.1 p. 52 "Canceled").

### noarg
After confirmation, rereads the current file from disk, discarding all edits since the last save (p. 106). The read is an I/O request (plan §3.5). The `modified` flag is cleared and the file's undo history is dropped. The cursor keeps its position (it may now be beyond the end of a shorter file); the window is clamped. `<clipboard>` rereads the system clipboard. A file that does not exist on disk (new or untitled buffer) → message, false. Read failure → message, buffer unchanged, false. Returns true when the file was read in.

### nullarg (`Arg Refresh`)
After confirmation, discards the current file from memory, without saving, and switches to the most recently edited other file (p. 106). If there is no other file, an empty untitled buffer takes its place (project choice; M1 starts with an untitled buffer when no file is given, plan §6.1). Returns true.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
textarg, numarg, streamarg, linearg, boxarg → `invalid argument`, false.

## Open points
- Confirmation: plan §6.10 says "with confirmation if modified … confirm in the manual". The manual (p. 106) asks in every case. Because the plan defers to the manual here, we always ask.
- `Arg Refresh` as the only file: untitled buffer (project choice).

## Test checklist
- Edit, `refresh`, `y` → disk contents back, `modified` cleared, true.
- `refresh`, `n` → unchanged, false; `refresh`, Esc → false.
- Unmodified file → prompt still shown.
- File deleted on disk → message, false, buffer unchanged.
- Read error (fake `Fs`) → message, false.
- `undo` after `refresh` → nothing to undo.
- `<clipboard>`: `refresh` shows the current system clipboard.
- Files a, b; in b `arg refresh`, `y` → b discarded unsaved, a current; b no longer in the file list.
- Only file → `arg refresh` → untitled buffer.
- `arg "x" refresh` → `invalid argument`.
