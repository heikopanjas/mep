# curfileext

Default keys: none
Source: M guide p. 96 (Table A.3 "extension of the current file"), p. 88 (Table A.1), p. 30 (Ch. 4.2.3); IMPLEMENTATION_PLAN.md §6.8

## Flags
NOARG, MODIFIES (single syntax line in Table A.3, no argument). No meta form; not KEEPMETA, so meta is ignored and cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Inserts the extension of the current file including its leading dot: `a.c` → `.c`, `archive.tar.gz` → `.gz` (p. 96). Returns true. No extension (`Makefile`, `.bashrc`) or a pseudo-file → nothing inserted, true. Untitled buffer → nothing inserted, false. ORACLE CMD-CURFILEEXT-1.

The text is always inserted at the cursor, also in overtype mode. If the cursor is beyond the end of the line, the line is padded with spaces first. The cursor moves to just after the inserted text. ORACLE CMD-CURTEXT-1. Read-only file: plan §5.6.

## Rejected arguments
Any argument → `invalid argument`, false.

## Open points
- With or without the dot: the manual does not say. We include it so that `curfilenam curfileext` rebuilds the name. ORACLE CMD-CURFILEEXT-1.
- Insert vs overtype: ORACLE CMD-CURTEXT-1.

## Test checklist
- `main.c` → `.c` (ORACLE CMD-CURFILEEXT-1).
- `archive.tar.gz` → `.gz`.
- `Makefile` → nothing, true; `.bashrc` → nothing, true.
- `<clipboard>` → nothing, true; untitled → false.
- `arg curfileext` → `invalid argument`.
- `meta curfileext` → inserted, meta cleared.
