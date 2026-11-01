# sinsert

Default keys: Ctrl+J
Source: M guide p. 108 (Table A.3), p. 19 (text-argument editing), p. 88 (Table A.1 "Inserts blanks, breaking lines if necessary"), p. 52 (Table 6.1), p. 57 (Indent macro), p. 73 (STREAMARG); IMPLEMENTATION_PLAN.md §6.5

## Flags
From p. 108:
- `Sinsert` → NOARG.
- `Arg Sinsert` → NULLARG.
- `Arg boxarg/linearg/streamarg Sinsert` → STREAMARG: every selection is treated as a stream from the anchor to the cursor (same wording as `sdelete`).
- MODIFIES. MARKARG (M2). No TEXTARG, no NUMARG.
- No `COLARG` (project addition, argcol.md): like `sdelete`, `sinsert` has no `LINEARG`, so any multi-line movement already reaches it as the exact same flat `Arg::Stream`, `argcol`-tagged or not — nothing to opt into.
- Not KEEPMETA. `Arg Arg` behaves like `Arg`.

## Behavior
Insert mode does not matter: `sinsert` always inserts. The clipboard is not touched. Read-only file: plan §5.6.

### noarg
Inserts one blank at the cursor; the rest of the line shifts right. The cursor does not move. Beyond the end of the line there is nothing to shift, and the trailing-space rule (plan §5.3) leaves the line unchanged. Returns true.

### nullarg (`Arg Sinsert`)
Inserts a line break at the anchor, splitting the line (p. 108); the new line uses the file's default terminator. No `softcr` indentation. The cursor stays at the split point on the first line. Beyond the end of the line: an empty line is inserted after the current line. Returns true.

### streamarg / linearg / boxarg
Inserts "a stream of blanks between the initial cursor position and the current cursor position" (p. 108): with start = the earlier position (l1, c1) and end = the later (l2, c2), the text at the start moves to the end position.
- Same line: c2 − c1 spaces are inserted at c1 (Indent macro, p. 57).
- Different lines: the line breaks at c1, l2 − l1 − 1 empty lines follow, and the moved text starts at column c2 of line l2 (padded with c2 spaces).
The cursor goes to the start position. ORACLE CMD-BLOCK-1. Returns true.

### In arg mode, text being edited
Inserts a space at the text cursor (p. 19).

### meta
No meta form. Ignored and cleared.

## Rejected arguments
textarg, numarg → `invalid argument`, false.

## Open points
- Cursor after the block form: ORACLE CMD-BLOCK-1.
- Multi-line expansion (break + empty lines + padding) follows the text of p. 108 and Z ("break the line … and insert the appropriate number of blank lines and spaces").

## Test checklist
- `abc`, cursor 1 → `a bc`, cursor 1, true.
- Beyond end of line → line unchanged, true.
- `arg sinsert` at 1 → `a` / `bc`, cursor (0, 1).
- Streamarg 3 cells at column 0 → three spaces inserted (Indent macro).
- Selection from (0, 2) to (2, 4) → line 0 split at 2, one empty line, then 4 spaces + rest; cursor (0, 2).
- Linearg (same column) from (0, 2) to (1, 2) → split, then 2 spaces + rest.
- `arg "a" sinsert` → `invalid argument`.
- `arg "ab" begline sinsert` → text ` ab`.
- Clipboard unchanged in all forms.
