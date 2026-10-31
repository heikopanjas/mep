# argcompile

Default keys: F5
Source: M guide p. 93 (Table A.3 "Performs the Arg Compile command"), p. 89 (Table A.1), p. 50 (Table 6.1); IMPLEMENTATION_PLAN.md §6.7, §8 M4

## Flags
For M4: NOARG (one syntax line); it behaves as `Arg Compile`.

## Behavior

### M1
In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

### Later (M4), for reference
- Same as `Arg Compile` (compile.md): compiles the current file with the `extmake` command for its extension.
- Table 6.1 p. 50: true = compile successful; false = bad argument/compiler not found.

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
None for M1.

## Test checklist
- `argcompile` → `not available`, false.
- `arg argcompile` → `not available`, false.
- `meta argcompile` → `not available`, meta cleared.
