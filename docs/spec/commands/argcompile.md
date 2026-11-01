# argcompile

Default keys: F5
Source: M guide p. 93 (Table A.3 "Performs the Arg Compile command"), p. 89 (Table A.1), p. 50 (Table 6.1); IMPLEMENTATION_PLAN.md §6.7, §8 M4

## Flags
NOARG. Not KEEPMETA.

## Behavior
### noarg
Same as `Arg Compile` (compile.md): saves a modified current file, then compiles it with the `extmake` command for its extension in the background. True if the compile started; false with a message for a pseudo-file or a missing `extmake` command (Table 6.1).

### meta
No meta form. Ignored and cleared.

## Rejected arguments
Any argument → `invalid argument`.

## Open points
See compile.md (ORACLE COMPILE-2, COMPILE-3).

## Test checklist
- `argcompile` with `extmake:c cc %s` runs `cc /work/a.c`.
- `argcompile` in `<untitled>` → `No file name`, false.
- `arg argcompile` → `invalid argument`.
