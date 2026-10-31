# mlines

Default keys: Ctrl+W
Source: M guide p. 100 (Table A.3), p. 60 (`vscroll`), p. 87 (Table A.1), p. 51 (Table 6.1); MSJ (window-movement functions); IMPLEMENTATION_PLAN.md §6.3, §6.5

## Flags
- `Mlines` → NOARG.
- `Arg Mlines` → NULLARG.
- `Arg numarg Mlines` → NUMARG, used as a count (not converted; plan §6.5).
- WINDOWFUNC. No TEXTARG, LINEARG, STREAMARG, BOXARG. Not KEEPMETA. `Arg Arg` has no special meaning.

## Behavior
The cursor keeps its window row; window top never below 0 (ORACLE CMD-SCROLL-1).

### noarg
Moves the window back (toward the start) by `vscroll` lines, or fewer in a small window (p. 100; ORACLE CMD-SCROLL-2). Returns true if the window moved, false at the top.

### nullarg (`Arg Mlines`)
Moves the window until the cursor's line is the bottom line of the window (p. 100). The cursor does not move. If that would need a negative window top, the window top becomes 0. Returns true if the window moved.

### numarg
Moves the window back by n lines, clamped at 0 (p. 100). Returns true if it moved.

### meta
No meta form; ignored and cleared.

## Rejected arguments
Non-numeric textarg, streamarg, linearg, boxarg → `invalid argument`, false.

## Open points
- ORACLE CMD-SCROLL-1 (cursor follows the window), CMD-SCROLL-2 (small window).

## Test checklist
- Window top 20, cursor (25, 0) → window 13, cursor (18, 0), true.
- Window top 3 → 0, cursor up 3.
- Window top 0 → false.
- `arg mlines` with cursor on line 40, window height 20, top 30 → top 21; cursor unchanged.
- `arg mlines` with cursor on line 5, height 20 → top 0.
- `arg "4" mlines` → window −4.
- `arg "x" mlines` → `invalid argument`, false.
