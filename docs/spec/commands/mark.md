# mark

Default keys: Ctrl+M
Source: M guide pp. 99–100 (Table A.3), p. 27 (Ch. 4.1.3), pp. 31–32 (Ch. 4.3, 4.3.1), p. 20 (markarg), p. 63 (`markfile`), p. 51 (Table 6.1); MSJ `selectall`; z-vs-mep.md S3; IMPLEMENTATION_PLAN.md §6.5, §8 M2, §11 (ORACLE S3)

## Flags
NOARG (`Mark`), NULLARG (`Arg Mark`), NUMARG used as a line number (COUNT, not converted), TEXTARG and BOXSTR (`Arg textarg/streamarg Mark`), count 2 with TEXTARG (`Arg Arg textarg Mark`). No MARKARG: a text argument naming a marker is the marker's name here. Not KEEPMETA, not MODIFIES.

## Markers
- A marker is a name and a position (file, line, cell column). Names are any text without blanks; they compare without regard to case (project choice). Any number of markers can be defined (p. 31).
- Markers keep their line and column; edits elsewhere do not move them (project choice, as in the original's line/column records of `markfile`).
- `markfile` (switch, p. 63) names a file with lines `markername filename line column`, searched when a marker is not defined in memory. Line and column are 0-based like `Arg numarg Mark` (ORACLE S3). A relative file name is relative to the directory of the markfile. Lines that do not have four fields with numeric line and column are ignored. The file is only read; defining a marker never writes it (p. 99).

## Behavior

### noarg
Moves the cursor to the beginning of the file, (0, 0), like `Arg Mpage` (p. 31, p. 99). Returns true.

### nullarg (`Arg Mark`)
Returns to the window and cursor position before the last jump of the window (a command that moved the window top, such as a search, a page command or `mark`) (p. 99 "the location prior to the last scrolling operation"; p. 31 "moves you back to the previous cursor position"). Repeating it switches back and forth (p. 31). No earlier position → no movement, false. ORACLE MARK-1.

### numarg (`Arg numarg Mark`)
Moves the cursor to column 0 of line numarg; line numbers start at 0 (p. 31). ORACLE S3. Lines beyond the end of the file are allowed (virtual space). Returns true.

### textarg / streamarg (`Arg textarg Mark`)
Moves to the marker: switches to its file if needed (with `autosave`, like `setfile`) and puts the cursor at its position; the window follows with `hike` (plan §6.3). Unknown marker: looked up in `markfile`; still unknown, or no `markfile` → message `<name>: mark not found`, false. Returns true when it moved.

### `Arg Arg textarg Mark`
Defines (or redefines) the marker at the initial cursor position (the anchor) in the current file (p. 99). Returns true.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
Linearg and boxarg → `invalid argument`.

## Open points
- ORACLE S3: 0-based line numbers.
- ORACLE MARK-1: `Arg Mark` restores the position before the last window jump (p. 99 wording) and toggles (p. 31).
- ORACLE MARK-2: `markfile` line and column are 0-based and a relative file name is relative to the markfile's directory; neither is stated in the manual.

## Test checklist
- `mark` from the middle of a file → (0, 0).
- `arg "5" mark` → line 5, column 0; `arg "0" mark` → line 0.
- `arg arg "p1" mark` at a position, move away, `arg "p1" mark` → back; names ignore case.
- Marker in another file → switches files.
- Unknown marker → message, false; found in `markfile` → moves there; malformed markfile lines ignored; unreadable markfile → not found.
- `arg mark` after a search jump → back; again → forward; with no jump → false.
- `arg down mark` → invalid argument.
- `meta mark` → meta cleared.
