# argcol

Project addition: not in the 1988 manual. Added at the owner's request, 2026-11-01, for
a GUI-style growable selection ("select a sentence in the middle of a paragraph"): put
the cursor where the sentence starts, then extend down through the lines it wraps onto
without first having to jump to column 0. ORACLE CMD-ARGCOL-1.

Default keys: none. The owner's personal `TOOLS.INI` (and `examples/TOOLS.INI`) binds
it through two macros:
```
Selectdown:=Argcol Down
Selectdown:Shift+Down
Selectup:=Argcol Up
Selectup:Shift+Up
```

Source: no manual citation; mirrors `arg.md`'s anchor/count mechanics
(`IMPLEMENTATION_PLAN.md §6.5`) with one addition: a selection built from an `argcol`
anchor keeps that anchor's column.

## Flags
`RAW | KEEPMETA`, exactly like `arg`. Not a consumer; never rejects.

## Why a separate function, not a change to `arg`
Plain `Alt+A` + `Down`/`Up` already has a manual-documented meaning other code relies
on unchanged: same-column movement always becomes a whole-line (`LINEARG`) selection,
used directly and by numeric-count selections (`Arg "5" Copy`, built in `classify`
without going through the geometric selection logic at all) and by marker selections
(`MARKARG`). Reusing `arg` itself for this would also raise `count` on every repeat
press, which changes what other commands do with `count ≥ 2` (`replace`'s regex mode,
`paste`'s file-insert form) — not something a selection-extending key should ever
trigger. `argcol` is its own function so none of that is disturbed: `down`/`up` are
completely unchanged, and `argcol`'s own repeat presses never raise `count`.

## Behavior

### noarg (arg mode off)
Starts arg mode exactly like `arg` (`arg.md`): the cursor becomes the anchor, count 1.
Additionally marks the pending state so a selection built from it keeps the anchor's
column (see below). Returns true.

### Repeat press, no movement or text pending
Count stays unchanged; the column mark stays set. (Unlike `arg`, `argcol`'s own repeat
press never raises the count — there is no `Argcol Argcol` form to distinguish, so
raising it would only create the `count ≥ 2` hazard above for no benefit.)

### Repeat press after the cursor moved (selection pending)
Keeps the anchor, the selection, and the column mark (ORACLE CMD-ARG-2's rule for
`arg`, applied the same way here).

### Column-anchored selection, once a command consumes it
When the function that finally consumes the pending argument accepts `COLARG` and the
anchor and cursor are on different lines, the selection is the flat character range
between them: `Arg::Stream { start: anchor.min(cursor), end: anchor.max(cursor) }`,
exactly the same stream the dispatcher already builds for an off-column multi-line
movement. Whichever point comes first in the file (by line, then column) is trimmed
from its own column onward (its line break included if it isn't the last line); the
other is trimmed up to (not including) its own column; every line strictly between
them is whole. This is symmetric in anchor/cursor and in direction — pressing
`Argcol Down` or `Argcol Up` just moves the cursor; which point ends up "first" and
which "second" falls out of the result, not out of which key was pressed:
- Moving down (cursor's line below the anchor's): the anchor's line gets the tail,
  the cursor's line gets a prefix up to the cursor's own column, every line strictly
  between is whole.
- Moving up (cursor's line above the anchor's): the mirror image — the cursor's line
  gets the tail (from the cursor's column onward), the anchor's line gets a prefix up
  to the anchor's column.
- Pressing the same direction again just moves the cursor further: the line the
  cursor *was* trimming becomes whole, and the new line the cursor reaches becomes
  the newly trimmed end.
- Reversing direction mid-selection needs no extra state: the range is recomputed from
  the live anchor and cursor every time, the same as every other shape.

This range is carried as an ordinary `Arg::Stream { start, end }` — not a new
geometric shape, and not even a specially-shaped one: it is the *same* stream
`sdelete`/`sinsert`/`execute`/`replace`/`qreplace`'s `Span::Stream` already build for
an off-column multi-line movement, so those needed no new code at all. `copy`,
`ldelete`, `linsert`, `assign` and `shell` gained a stream arm for the multi-line case
specifically to carry it (see each command's own spec and `edit.rs`'s
`stream_window`/`stream_rows` helpers, which were already generic enough to need no
changes either once `selection()` stopped special-casing the endpoints).

### Commands without `COLARG`
Fall through to today's behavior unchanged: same column still becomes a whole-line
selection if the command has `LINEARG`, otherwise a box or an "invalid argument" as
before. The column mark has no effect on them. `sdelete` and `sinsert` are a special
case of this: they have neither `LINEARG` nor `COLARG`, so a same-column movement
already reaches them as a flat stream regardless of `argcol` — granting them `COLARG`
would be a no-op, since the branch it would select produces the exact value its own
`STREAMARG` fallback already does.

### Within one line
If the anchor and cursor end up on the same line (e.g. `Argcol Right Right`, or the
cursor returns to the anchor's line after reversing), the selection is whatever the
ordinary same-line rules give (`STREAMARG`/`BOXSTR`/`BOXARG`) — the column mark only
changes the *different-lines* case.

### Typing, `cancel`, `meta`
Same as `arg`: typing a printable character drops the pending selection and the column
mark and starts a text argument at the cursor (CMD-ARG-2); `cancel` clears the whole
state; `meta` toggles independently (KEEPMETA).

## Rejected arguments
None directly; `argcol` itself never returns false, matching `arg`.

## Open points
- ORACLE CMD-ARGCOL-1: resolved 2026-11-01 — the owner asked for this feature; first
  tried "anchor's line partial, every other line whole, cursor's own line not yet
  included", then corrected it the same day to the simpler, symmetric two-point trim
  described above after a concrete example showed the cursor's own line should be
  partial too, not excluded. Also chose that `sdelete`/`sinsert` should behave the
  same under `argcol` as under plain `arg` (so a deletion/insertion always matches
  what the live highlight shows — this turned out to already be true without
  granting them `COLARG` at all), `linsert` as the exact inverse of the matching
  deletion, and that a downward copy includes the
  trailing line break.
- Tabs inside a column-anchored copy reach the clipboard as spaces (`stream_rows` reads
  displayed, detabbed lines) — matching `Box` and single-line `Stream` copies, unlike
  `Line` copies, which are byte-exact.
- Whether `Shift+Down`/`Shift+Up` reach `mep-term` as distinct keys depends on the
  terminal (same class of risk as the Shift+F3/kitty-protocol gap already tracked in
  `AGENTS.md`'s Pending Verification); the `argcol` mechanism itself does not depend on
  which keys a `TOOLS.INI` happens to bind to it.

## Test checklist
- `argcol down copy` from column `c` on line `L`: clipboard is a stream, line `L`'s
  text from `c` to its end plus its line break, then line `L+1`'s text up to (not
  including) column `c`.
- `argcol down down copy`: line `L`'s tail, the whole of line `L+1`, then line `L+2`'s
  prefix up to column `c`.
- `argcol up copy` / `argcol up up copy`: the mirror, extending upward — the cursor's
  line gets the tail, the anchor's line gets the prefix.
- `argcol down down up copy`: identical to one `argcol down copy` (reversal recomputes
  from the live anchor and cursor, not from history).
- `argcol down down ldelete`: the file loses exactly the copied range, one undo group,
  cursor at the anchor; a `paste` round-trips back to the original file.
- `argcol down down replace "x" enter "y" enter`: matches on the anchor's line before
  the anchor's column are untouched; matches at or after it, on the whole line between,
  and before the cursor's column on the cursor's line, are all replaced; matches at or
  after the cursor's column on the cursor's line are untouched.
- `argcol down down sdelete` deletes the identical range `ldelete` would; plain
  `arg down down sdelete` still does its old column-sweep stream (regression guard).
- `argcol down down linsert`: splits the anchor line at the anchor's column, adds one
  blank line for the line strictly between, and pads the moved text by the cursor's
  column's width — the exact inverse of what `ldelete` would have removed.
- A command without `COLARG` (e.g. `undo`) after `argcol down down` still gets
  "Invalid argument" — the column mark does not leak into commands that do not ask
  for it.
- Repeated `argcol` presses never raise `count` (no `Arg Arg`-style escalation).
- A numeric-count selection (`Arg "5" Copy`) and a marker selection (`Arg "m" Copy`
  after defining `m`) are unaffected even when `argcol` was used earlier in the same
  session.
- The live highlight does not extend to a line before the topmost point or after the
  bottommost point of the selection, even when that line is rendered in the window.
