# shell

Default keys: Shift+F9
Source: M guide p. 108 (Table A.3), p. 61 (`askrtn`), p. 89 (Table A.1), p. 52 (Table 6.1); MSJ (print macro, `DoSpawn`); IMPLEMENTATION_PLAN.md §6.7, §8 M4

## Flags
NOARG, NULLARG with NULLEOL, TEXTARG, BOXSTR, LINEARG, BOXARG, meta form. Not KEEPMETA.

## Running commands
- The editor leaves full-screen mode, runs the commands on the terminal with the terminal's input and output, then returns and redraws the whole screen. Keys typed while a command runs go to the command.
- A command line runs through the system shell (`sh -c` on macOS and Linux, `cmd /C` on Windows) in the editor's working directory. The interactive shell is `$SHELL` (fallback `/bin/sh`) or `%COMSPEC%` (fallback `cmd.exe`).
- With `askrtn` on (default), the editor shows `Press Enter to return to the editor` afterwards and waits for Enter (p. 61).
- The editor waits for the commands; this keeps `arg "command" shell` usable inside macros and extensions (MSJ `DoSpawn`).
- Returns true if every command started and exited successfully; false if one could not be started or failed (Table 6.1).

## Behavior

### noarg (`Shell`)
Saves the current file if it is modified and has a name, then runs the interactive shell (p. 108). A failed save stops the command.

### meta (`Meta Shell`)
Runs the interactive shell without saving (p. 108).

### nullarg (`Arg Shell`)
The text from the cursor to the end of the line is the command (p. 108).

### textarg, one-line selection (`Arg textarg Shell`)
The text is the command (MSJ `arg "PRINT TEMP.DAT" shell`).

### linearg, boxarg
Each line of the selection (for a box, its part of the line, trailing blanks removed) is a separate command, run in order (p. 108). Empty lines are skipped.

Empty commands (an empty text, a selection of empty lines) → `invalid argument`, false.

## Rejected arguments
Stream over several lines → `invalid argument`.

## Open points
None.

## Test checklist
- `shell` saves a modified file, then runs the interactive shell; `meta shell` does not save.
- `arg shell` on `ls -l` at the cursor runs `ls -l`.
- `arg "make" shell` runs `make`; failure exit → false.
- Line and box selections run each line; empty lines skipped.
- `noaskrtn:` → no wait for Enter.
- A command inside a macro: the macro continues with the result.
