# setfile

Default keys: F2
Source: M guide pp. 107–108 (Table A.3), p. 12 (Ch. 2.4), p. 13 (nonexistent file prompt), p. 38 (Ch. 4.7), p. 56 (`Save:=Arg Arg Setfile`), p. 61 (`autosave`), p. 62–63 (`backup`, `readonly`), p. 89 (Table A.1), p. 52 (Table 6.1); MSJ (`savefile` macro, print1/print2, pseudo-files); IMPLEMENTATION_PLAN.md §3.3, §3.5, §5.6, §6.9, §6.10, §6.13

## Flags
From pp. 107–108:
- `Setfile` → NOARG.
- `Arg Setfile` → NULLARG. Not NULLEOW: with count 2 the nullarg means "save the current file" (`Arg Arg Setfile`), which a NULLEOW textarg could not be told apart from `Arg Arg textarg Setfile`. The command reads the word at the anchor itself for count 1.
- `Arg textarg Setfile` → TEXTARG.
- `Arg streamarg Setfile` → BOXSTR.
- `Meta` forms of all count-1 forms.
- `Arg Arg Setfile`, `Arg Arg textarg/streamarg Setfile` → count 2 forms.
- No NUMARG (`arg "1" setfile` opens a file named `1`), no LINEARG, no BOXARG.
- Not MODIFIES (it does not change text). Not KEEPMETA.

## Behavior
Loading and saving are I/O requests; the command suspends and finishes on the completion event (plan §3.5). Names are taken literally (no environment-variable expansion; Z's `$PATH:` form is not implemented). Relative names are relative to the current working directory.

Switching away from a modified file saves it first if `autosave` is on (p. 61, plan §6.10); with `autosave` off, the file stays modified in memory. A save failure stops the switch: message, false.

### noarg
Switches to the most recently edited other file, saving the current one per `autosave` (p. 107, p. 38). Repeating toggles between two files. No other file → message, false.

### nullarg (`Arg Setfile`)
Switches to the file whose name starts at the anchor and ends at the first blank (p. 107). Empty → `invalid argument`, false.

### textarg / streamarg
Switches to the named file (p. 108). Already open → it becomes current, with its cursor and window. Not open → loaded. Not on disk → the dialog line asks whether to create it, as at startup (p. 13); yes → new empty buffer with that name, no → false. ORACLE CMD-SETFILE-1.
- Wildcards (`*`, `?`): the original shows a list of matching files (p. 38) in the file-list pseudo-file, which is M3. M1: message `not available`, false.
- `<clipboard>`: opens the pseudo-file filled with the current system clipboard text (plan §3.3, §6.9; clipboard read is an I/O request). It can be edited and saved only under an explicit name.
- Other `<name>` pseudo-files: switched to if they exist (extensions, M4); otherwise a new empty pseudo-file is created (MSJ: pseudo-files as scratch buffers).
- A directory → message, false (Z's change-directory behavior is not implemented).
- Load failure → message, false.

### meta forms (`Meta Setfile`, `Arg Meta Setfile`, `Arg textarg Meta Setfile`, `Arg streamarg Meta Setfile`)
As above, but the current file is not saved (p. 108), even with `autosave` on. Its changes stay in memory and it stays marked modified.

### `Arg Arg Setfile` (save)
Saves the current file under its own name (p. 108; MSJ `savefile`; `Save:=Arg Arg Setfile`, p. 56). Write via temp file and rename (plan §6.10); with `backup:bak` the previous version is renamed to the name with `.bak` replacing its extension. Read-only file: run the `readonly` command with the file name appended, then save; without a `readonly` command, prompt for a new name (p. 63). Pseudo-file or untitled buffer → message (no file name), false. Not modified → saved anyway. Returns true on success, false on failure with a message.

### `Arg Arg textarg/streamarg Setfile` (save as)
Writes the current file's contents to the named file (p. 108; MSJ print2 saves `<clipboard>` as `TEMP.DAT`). The current file keeps its own name and modified state; the written file is not opened. ORACLE CMD-SETFILE-2. Backup and read-only rules apply to the target. Returns true on success.

### `Arg Arg` with meta
Not listed; meta is ignored for the save forms and cleared.

## Rejected arguments
linearg, boxarg (multi-line selections) → `invalid argument`, false.

## Open points
- Prompt when the named file does not exist (taken from the startup behavior, p. 13). ORACLE CMD-SETFILE-1.
- "Save the current file under the name specified": we write a copy and keep the buffer's name. The other reading renames the buffer. ORACLE CMD-SETFILE-2.
- Wildcard lists need the M3 file-list pseudo-file: `not available` in M1 (project decision).
- `.bak` naming "replacing the extension" follows p. 62 ("file name … changed to .BAK"); plan §6.10 asks to confirm; we replace the extension (`a.c` → `a.bak`), and append for names without one (`Makefile` → `Makefile.bak`).

## Test checklist
- Files a, b open, current b; `setfile` → a; again → b.
- Only one file → message, false.
- `autosave` on, modified a, `setfile` → a saved; `autosave` off → a stays modified, not written.
- `meta setfile` with `autosave` on → not saved.
- `arg "b.txt" setfile` → loads b.txt, true; cursor and window remembered on return.
- Missing file → create prompt; `y` → empty buffer named; `n` → false (ORACLE CMD-SETFILE-1).
- `arg setfile` with the cursor on `notes.txt more` → opens `notes.txt`; on a blank → `invalid argument`.
- Streamarg over a file name → opens it.
- `arg "*.c" setfile` → `not available`, false.
- `arg "<clipboard>" setfile` → buffer shows the system clipboard text; reopening after a new `copy` shows the new text.
- `arg arg setfile` → saved; with `backup:bak` the old file becomes `name.bak`; write error (fake `Fs`) → message, false.
- `arg arg setfile` on `<clipboard>` → message, false.
- `arg arg "TEMP.DAT" setfile` on `<clipboard>` then `setfile` → TEMP.DAT written, back to the original file (MSJ print2).
- Read-only file save with `readonly:` unset → prompt for a new name; with `readonly:chmod u+w` → command run with the file name appended.
- `arg "1" setfile` → opens file `1`.
- `arg down setfile` → `invalid argument`.
- Load held (`hold`) → suspended; Esc → false, still on the old file.
