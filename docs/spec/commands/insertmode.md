# insertmode

Default keys: Ins, Ctrl+V
Source: M guide p. 98 (Table A.3), p. 8 (Ch. 2.3.1), p. 13 (status line field 7), p. 51 (Table 6.1), p. 61 (`enterinsmode`); IMPLEMENTATION_PLAN.md §6.2, §7

## Flags
NOARG (one syntax line). Not KEEPMETA. Not MODIFIES. `Arg Arg` does not apply.

## Behavior

### noarg
Toggles insert mode (p. 98). While on, the status line shows `insert` (p. 8, p. 13). The front end shows a bar cursor in insert mode and a block in overtype, where supported (plan §7); `cursorsize` replaces both with one shape (switches.md). The initial state comes from `enterinsmode` (p. 61). Returns true if insert mode is now on, false if now off (Table 6.1 p. 51).

### meta
No meta form. Meta is ignored and cleared.

## Rejected arguments
Any argument → `invalid argument`, false, insert mode unchanged.

## Open points
- Z's table says "If insert mode was on", the M guide "Insert mode now on". MEP wins: new state.

## Test checklist
- Start (`enterinsmode` off): `insertmode` → true, status `insert`; again → false, status clear.
- `enterinsmode:` → editor starts in insert mode; `insertmode` → false.
- `arg insertmode` → `invalid argument`, false, mode unchanged.
- `meta insertmode` → toggles, meta cleared.
