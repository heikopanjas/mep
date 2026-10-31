# mark

Default keys: Ctrl+M
Source: M guide pp. 99–100 (Table A.3), p. 27 (Ch. 4.1.3), pp. 31–32 (Ch. 4.3, 4.3.1), p. 20 (markarg), p. 63 (`markfile`), p. 87 (Table A.1), p. 51 (Table 6.1); MSJ `selectall`; z-vs-mep.md S3; IMPLEMENTATION_PLAN.md §6.7, §8 M2, §11 (ORACLE S3)

## Flags
For M2, from pp. 99–100: NOARG (`Mark`), NULLARG (`Arg Mark`), NUMARG handled as a line number (not converted), TEXTARG and BOXSTR (`Arg textarg/streamarg Mark`), count 2 with TEXTARG (`Arg Arg textarg Mark`). CURSORFUNC is not set (it takes arguments).

M1: no flags apply; see Behavior.

## Behavior

### M1
In M1 the command is in the command table so it can be assigned, used in macros and listed, but it does nothing: every invocation (any argument kind, with or without meta) shows `not available` on the dialog line and returns false (IMPLEMENTATION_PLAN.md §6.7). A pending argument and meta are consumed and cleared. The file, cursor and clipboard do not change.

### Later (M2), for reference
- `Mark`: moves the window to the beginning of the file (p. 99; p. 31 "just as Arg Mpage does").
- `Arg Mark`: restores the window to its location before the last scrolling operation (p. 99).
- `Arg numarg Mark`: cursor to the beginning of line numarg; lines are numbered from 0 (p. 31). ORACLE S3.
- `Arg Arg textarg Mark`: defines a marker at the initial cursor position (p. 99).
- `Arg textarg Mark`, `Arg streamarg Mark`: moves to the marker; unknown markers are looked up in the `markfile` file (p. 100, p. 63).
- Table 6.1 p. 51: true = definition/move successful; false = bad argument/not found.

## Rejected arguments
In M1 there is no `invalid argument` case: the dispatcher accepts every argument kind for this entry so that all invocations report `not available`. The flags under "Flags" take effect in the milestone that implements the command.

## Open points
- ORACLE S3 (0-based line numbers) applies from M2.
- In M1 a mark name typed as an argument is an ordinary textarg (no markarg classification); MSJ `selectall` therefore cannot run in M1.

## Test checklist
- `mark` → `not available`, false, cursor and window unchanged.
- `arg mark`, `arg "5" mark`, `arg arg "p1" mark`, `arg right mark` → each `not available`, false, arg mode ended.
- `meta mark` → `not available`, meta cleared.
- Macro `x:=mark +>yes` → branch not taken.
