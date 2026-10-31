# help

Default keys: F1
Source: M guide p. 97 (Table A.3), p. 12 (Ch. 2.4), p. 47 (Ch. 6.2.2), p. 89 (Table A.1), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §6.7, §6.8

## Flags
Later: NOARG (one syntax line).

## Behavior

### M1
`help` is in the default key table (plan §6.8) but not in the M1 command set (plan §6.7) or any milestone. It is treated like the later-milestone commands. In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

Table 6.1 (p. 51) says `help` always returns true. In M1 it returns false with `not available`.

### Later, for reference
- Lists all editing functions alphabetically with their current key assignments, as a browsable file; `setfile` returns to the previous file (p. 12, p. 47). `Arg ? Assign` shows the same kind of list (assign.md).

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
- `help` has no milestone in IMPLEMENTATION_PLAN.md. The owner should decide when the key list (and with it `Arg ? Assign`) is built.

## Test checklist
- F1 → `not available`, false, current file unchanged.
- `arg help` → `not available`, false.
- `meta help` → `not available`, meta cleared.
