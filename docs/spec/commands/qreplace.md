# qreplace

Default keys: Ctrl+\
Source: M guide p. 105 (Table A.3), p. 34 (Ch. 4.4.2), p. 32 (Ch. 4.3.1), p. 88 (Table A.1), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §6.7, §8 M2, §11 (ORACLE S6), §12 (Ctrl+\ is a terminal signal key)

## Flags
For M2, as replace.md: NOARG, NULLARG (not listed in Table A.3 but p. 34 says Qreplace "takes exactly the same syntax" as Replace), LINEARG, BOXARG, STREAMARG, NUMARG, MARKARG, count 2 = regex. MODIFIES.

## Behavior

### M1
In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

### Later (M2), for reference
- Same as `replace`, but asks at each occurrence: Y replaces, N skips, P replaces the rest without asking; `cancel` ends the operation (p. 34).
- Table 6.1 p. 51: true = at least one replacement; false = string not found/invalid pattern.

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
- Table A.3 (p. 105) has no `Arg Qreplace` (nullarg) line; p. 34 implies it. For M2.
- ORACLE S6 applies from M2.

## Test checklist
- `qreplace`, `arg "2" qreplace`, `arg down right qreplace`, `arg arg qreplace` → `not available`, false.
- `meta qreplace` → `not available`, meta cleared.
