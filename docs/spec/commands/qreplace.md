# qreplace

Default keys: Ctrl+\
Source: M guide p. 105 (Table A.3), p. 34 (Ch. 4.4.2), p. 32 (Ch. 4.3.1), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §6.5, §8 M2, §10, §11 (ORACLE S6), §12

## Flags
As replace.md: NOARG, NULLARG (p. 34 "takes exactly the same syntax"), LINEARG, BOXARG, STREAMARG, NUMARG, MARKARG (box, ORACLE S6), COLARG (project addition, argcol.md), count 2 = regular expressions. MODIFIES.

## Behavior
Everything as replace.md (prompts, range, matching, result, undo), except that each match asks first (p. 34): the cursor moves to the match, the match is highlighted and the dialog line asks `Replace? (y/n/p)`.
- `y` replaces this match and goes on.
- `n` skips it and goes on.
- `p` replaces this and all remaining matches without asking.
- Esc (`cancel`) ends the operation; replacements made so far stay.
- Other keys are ignored while asking.
The cursor stays on the last match asked about. Returns true if at least one replacement was made; otherwise false (Table 6.1 p. 51).

### meta
No meta form. Ignored and cleared.

## Rejected arguments
Textarg → `invalid argument`.

## Open points
- ORACLE S6, ORACLE RPL-1 as in replace.md.

## Test checklist
- y, n and p answers; Esc in the middle keeps earlier replacements; other keys ignored.
- No match → false; all answered n → false.
- Regex form with groups.
- `arg "x" qreplace` → invalid argument.
