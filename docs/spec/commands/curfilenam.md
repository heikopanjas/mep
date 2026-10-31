# curfilenam

Default keys: none
Source: M guide p. 96 (Table A.3 "base name of the current file"), p. 88 (Table A.1), p. 30 (Ch. 4.2.3); IMPLEMENTATION_PLAN.md §6.8

## Flags
NOARG, MODIFIES (single syntax line in Table A.3, no argument). No meta form; not KEEPMETA, so meta is ignored and cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Inserts the base name of the current file: the last path component without directory and without its extension (p. 96). The extension is the part after the last `.`, unless the name starts with its only `.`: `a.c` → `a`, `archive.tar.gz` → `archive.tar`, `.bashrc` → `.bashrc`, `Makefile` → `Makefile`. Together with `curfileext` it gives the file name back (`curfilenam curfileext`).
- Pseudo-file `<clipboard>`: `<clipboard>`.
- Untitled buffer: nothing inserted, false.

The text is always inserted at the cursor, also in overtype mode. If the cursor is beyond the end of the line, the line is padded with spaces first. The cursor moves to just after the inserted text. ORACLE CMD-CURTEXT-1. Read-only file: plan §5.6.

Returns true when text was inserted.

## Rejected arguments
Any argument → `invalid argument`, false.

## Open points
- Names with several dots or a leading dot: our rule (above), not covered by the manual. Tied to ORACLE CMD-CURFILEEXT-1.
- Insert vs overtype: ORACLE CMD-CURTEXT-1.

## Test checklist
- `/p/src/main.c` → `main`.
- `archive.tar.gz` → `archive.tar`; `.bashrc` → `.bashrc`; `Makefile` → `Makefile`.
- `<clipboard>` → `<clipboard>`; untitled → false.
- `arg curfilenam` → `invalid argument`.
- `meta curfilenam` → inserted, meta cleared.
