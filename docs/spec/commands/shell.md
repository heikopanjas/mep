# shell

Default keys: Shift+F9
Source: M guide p. 108 (Table A.3), p. 61 (`askrtn`), p. 89 (Table A.1), p. 52 (Table 6.1); MSJ (print macro, `DoSpawn`); IMPLEMENTATION_PLAN.md §6.7, §8 M4

## Flags
For M4, from p. 108: NOARG, meta form, NULLARG (cursor to end of line as a command; NULLEOL), BOXARG and LINEARG (each line a separate command), TEXTARG and BOXSTR (one command).

## Behavior

### M1
In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

### Later (M4), for reference
- `Shell`: saves the current file and runs the command shell; `Meta Shell`: without saving (p. 108).
- `Arg Shell`: the text from the cursor to the end of the line is a shell command.
- `Arg boxarg/linearg Shell`: each line is a separate command.
- `Arg textarg/streamarg Shell`: the text is the command (MSJ `arg "PRINT TEMP.DAT" shell`).
- `askrtn` decides whether the editor waits for Enter afterwards (p. 61).
- Table 6.1 p. 52: true = shell successful; false = bad argument/program not found.

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
- The MSJ print macro (plan §8 task 17) stubs `shell`; in M1 the real entry returns false.

## Test checklist
- `shell`, `arg shell`, `arg "ls" shell`, `arg down shell` → `not available`, false, nothing run, file not saved.
- `meta shell` → `not available`, meta cleared.
