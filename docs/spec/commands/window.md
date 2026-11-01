# window

Default keys: F6
Source: M guide p. 37 (Ch. 4.6), p. 109 (Table A.3), p. 52 (Table 6.1); z-vs-mep.md S10; IMPLEMENTATION_PLAN.md §8 M3, §11 (ORACLE S10)

## Flags
NOARG (`Window`), NULLARG with count 1 and 2 (`Arg Window`, `Arg Arg Window`), meta form (`Meta Window`). WINDOWFUNC. Not KEEPMETA.

## Windows
- The text area (terminal rows minus the dialog and status lines, or `height`/`width` when set) is tiled into windows. Each window shows a file with its own cursor and window position; several windows may show the same file, and an edit in one shows in all (p. 37).
- Windows are separated by a one-cell line drawn in `stacolor`: a row of `─` between windows above each other, a column of `│` between windows side by side (project choice; ORACLE WIN-1).
- Every window must keep at least 5 lines and 10 columns (p. 37). At most 8 windows (p. 37).
- The status line, the cursor and selections belong to the current window. Switching files with `setfile` changes the file of the current window; each file remembers the cursor and window position it had when it was last shown.
- When the terminal is resized, window sizes are kept where possible and shrunk otherwise; every window keeps its cursor visible.

## Behavior

### noarg (`Window`)
Moves to the next window, to the right of or below the current one, wrapping from the last to the first (p. 109). With only one window: no change, false. Otherwise true.

### nullarg, count 1 (`Arg Window`)
Splits the current window at the cursor row into a window above and a window below; the separator takes the cursor row (p. 109 "splits the current window horizontally at the initial cursor position … at least five lines high"). Both windows show the current file. The upper window keeps its window position and its cursor moves up one line, next to the separator; the lower window starts with the cursor line at its top and keeps the cursor. The lower window becomes current. ORACLE S10, WIN-1.

### nullarg, count 2 (`Arg Arg Window`)
Splits the current window at the cursor column into a left and a right window; the separator takes the cursor column (p. 109 "vertically … at least 10 columns wide"). The left window keeps its window position and its cursor moves left one column; the right window starts with the cursor column at its left edge and keeps the cursor. The right window becomes current.

A split that would leave a window with fewer than 5 lines or 10 columns, or a ninth window → message `Window too small` or `Too many windows`, false (p. 37).

### meta (`Meta Window`)
Closes the current window; the window next to it in the split takes its space (p. 109 "closes the window"; p. 37 "merging it with an adjacent window") and becomes current. The file stays open. With only one window: false.

## Rejected arguments
Textarg, numarg, any selection → `invalid argument`.

## Open points
- ORACLE S10: which split is horizontal and which vertical (p. 37 vs p. 109). We follow p. 109.
- ORACLE WIN-1: separator lines, which window becomes current after a split, and where the cursors of both windows go.

## Test checklist
- `arg window` at row 6 of 12 → two windows, separator row; too close to an edge → `Window too small`.
- `arg arg window` → side by side; narrow → `Window too small`.
- Ninth window → `Too many windows`.
- `window` cycles; single window → false.
- `meta window` merges; single window → false.
- Same file in two windows: edit in one shows in the other; frames drawn with separators.
- `setfile` in one window keeps the other; switching back restores the position.
- Resize shrinks windows.
- `arg "x" window` → invalid argument.
