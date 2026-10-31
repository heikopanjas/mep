# window

Default keys: F6
Source: M guide p. 109 (Table A.3), p. 37 (Ch. 4.6), p. 88 (Table A.1), p. 52 (Table 6.1); z-vs-mep.md S10; IMPLEMENTATION_PLAN.md §6.7, §8 M3, §11 (ORACLE S10)

## Flags
For M3, from p. 109: NOARG (`Window`), NULLARG with count 1 and count 2 (`Arg Window`, `Arg Arg Window`), meta form (`Meta Window`). WINDOWFUNC.

## Behavior

### M1
In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

### Later (M3), for reference
- `Window`: moves to the next window to the right of or below the current one.
- `Arg Window`: splits horizontally at the initial cursor position; `Arg Arg Window`: splits vertically. p. 37 says "horizontal window (split at the cursor column)" and "vertical window (split at the cursor row)", which looks swapped against p. 109. ORACLE S10.
- `Meta Window`: closes the current window, merging it with an adjacent one.
- Up to 8 windows; minimum 5 lines and 10 columns, otherwise the command fails (p. 37).
- Table 6.1 p. 52: true = successful split, join or move; false = any error.

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
- ORACLE S10 applies from M3.

## Test checklist
- `window`, `arg window`, `arg arg window`, `arg "x" window` → `not available`, false, screen unchanged.
- `meta window` → `not available`, meta cleared.
