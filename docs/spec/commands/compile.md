# compile

Default keys: Shift+F3
Source: M guide pp. 94–95 (Table A.3), pp. 35–37 (Ch. 4.5), p. 17 (`Arg Meta Compile`), p. 59 (`maxmsg`), p. 62 (`extmake`), p. 89 (Table A.1), p. 50 (Table 6.1); z-vs-mep.md S7; IMPLEMENTATION_PLAN.md §6.7, §8 M4

## Flags
For M4, from pp. 94–95: NOARG (`Compile`), meta form (`Meta Compile`), NULLARG (`Arg Compile`), TEXTARG and BOXSTR (`Arg textarg/streamarg Compile`), count 2 with TEXTARG/BOXSTR (`Arg Arg textarg/streamarg Compile`), NULLARG with meta (`Arg Meta Compile`).

## Behavior

### M1
In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

### Later (M4), for reference
- `Compile`: next error message; cursor to its file, row and column; message on the dialog line (p. 94). Recognized formats on p. 36.
- `Meta Compile`: first message not for the current file (p. 94).
- `Arg Compile`: compile the current file with the `extmake` command for its extension (p. 94).
- `Arg textarg/streamarg Compile`: compile the named file via `extmake` (p. 94).
- `Arg Arg textarg/streamarg Compile`: run the text as a program whose output is in error format (p. 95; plan: `arg arg "grep -n …" compile`).
- `Arg Meta Compile`: previous message, up to `maxmsg` (p. 95, p. 36).
- Runs in the background (plan §3.5, M4). Table 6.1 p. 50: true = compile successful; false = bad argument/compiler not found.

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
None for M1.

## Test checklist
- `compile`, `arg compile`, `arg "a.c" compile`, `arg arg "make" compile` → `not available`, false, no process started.
- `meta compile`, `arg meta compile` → `not available`, meta cleared.
