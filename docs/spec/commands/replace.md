# replace

Default keys: Ctrl+L
Source: M guide p. 106 (Table A.3), p. 34 (Ch. 4.4.2), p. 39 (Ch. 5 regex), p. 88 (Table A.1), p. 52 (Table 6.1); z-vs-mep.md S6; IMPLEMENTATION_PLAN.md §6.7, §8 M2, §10, §11 (ORACLE S6)

## Flags
For M2, from p. 106: NOARG and NULLARG (identical, p. 34), LINEARG, BOXARG (a multi-line box-shaped selection is a box, plan §6.5), STREAMARG, NUMARG (converted to a linearg), MARKARG; count 2 (`Arg Arg`) selects regular expressions for every form. MODIFIES.

## Behavior

### M1
In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

### Later (M2), for reference
- Prompts for the search string and the replacement string, showing the previous values (p. 34), then replaces without confirmation from the cursor to the end of the file (noarg, nullarg) or within the selection (linearg, boxarg, streamarg), over n lines (numarg), or between cursor and marker (markarg, a box: p. 34, ORACLE S6).
- `Arg Arg` forms: regular expression with tagged groups `$0`–`$9` (p. 43, plan §10).
- `case` switch applies.
- Table 6.1 p. 52: true = at least one replacement; false = string not found/invalid pattern.

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
- ORACLE S6 (markarg range shape) applies from M2.

## Test checklist
- `replace`, `arg replace`, `arg down replace`, `arg "3" replace`, `arg arg replace` → each `not available`, false, file unchanged, no prompt.
- `meta replace` → `not available`, meta cleared.
