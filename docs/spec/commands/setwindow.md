# setwindow

Default keys: Ctrl+]
Source: M guide p. 108 (Table A.3), p. 88 (Table A.1 "Redisplays window"), p. 52 (Table 6.1); MSJ Fig. 1 ("Redisplay or reorient text in window"); IMPLEMENTATION_PLAN.md §6.2, §7

## Flags
From p. 108:
- `Setwindow` → NOARG.
- `Meta Setwindow` → meta form.
- `Arg Setwindow` → NULLARG.
- WINDOWFUNC. No TEXTARG, LINEARG, STREAMARG, BOXARG, NUMARG. Not KEEPMETA. `Arg Arg` has no special meaning.

## Behavior

### noarg
Redisplays the entire screen (p. 108): the front end discards its previous frame and repaints every cell (plan §7 frame diff reset), which repairs a terminal garbled by other output. Nothing else changes. Returns true.

### meta noarg (`Meta Setwindow`)
Redisplays the line the cursor is on: that terminal row is rewritten even if the frame diff shows no change. Returns true.

### nullarg (`Arg Setwindow`)
"Adjusts the window so that the initial cursor position becomes the home position (upper-left corner)" (p. 108): window top = anchor line, window left column = anchor column. The cursor stays where it is (for a nullarg that is the anchor, now at the window's upper-left). Returns true.

### `Arg Meta Setwindow`
Not listed. The arg form applies; meta is ignored and cleared.

## Rejected arguments
textarg, numarg, streamarg, linearg, boxarg → `invalid argument`, false (Table 6.1 "Bad argument").

## Open points
None beyond the mapping of "redisplay" to a full repaint request.

## Test checklist
- `setwindow` → full repaint requested (frame test sees the reset), returns true.
- `meta setwindow` → cursor row repaint requested, meta cleared.
- `arg setwindow` with cursor (30, 12) → window top 30, left column 12; cursor unchanged.
- `arg setwindow` at (0, 0) → window (0, 0), true.
- `arg "x" setwindow` → `invalid argument`, false; `arg down setwindow` → `invalid argument`.
