# information

Default keys: Shift+F1
Source: M guide p. 97 (Table A.3), p. 38 (Ch. 4.7), p. 89 (Table A.1), p. 51 (Table 6.1), p. 60 (`tmpsav`); MSJ Fig. 1 ("Display file history"); IMPLEMENTATION_PLAN.md §6.7, §8 M3

## Flags
For M3: NOARG only (one syntax line).

## Behavior

### M1
In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

Table 6.1 (p. 51) says `information` always returns true. In M1 it returns false with `not available`, following plan §6.7.

### Later (M3), for reference
- Saves the current file and switches to the information file: the files in memory and the files edited before, up to `tmpsav` entries, active files with their current lengths (p. 97, p. 38). `Arg Setfile` on a listed name opens it (p. 38).

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
- M1 return value differs from Table 6.1 (false instead of always true) because the command does nothing.

## Test checklist
- `information` → `not available`, false, current file not saved, no switch.
- `arg information` → `not available`, false.
- `meta information` → `not available`, meta cleared.
