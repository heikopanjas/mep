# curfile

Default keys: none
Source: M guide p. 96 (Table A.3 "fully-qualified pathname"), p. 88 (Table A.1), p. 30 (Ch. 4.2.3); IMPLEMENTATION_PLAN.md §5.6, §6.8

## Flags
NOARG, MODIFIES (single syntax line in Table A.3, no argument). No meta form; not KEEPMETA, so meta is ignored and cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Inserts the fully qualified path of the current file (p. 96): an absolute path, with `.` and `..` removed, symbolic links not resolved, native separators (`/` on macOS and Linux, `\` on Windows). Non-UTF-8 path bytes are inserted byte-exact on Unix; Windows paths are converted from UTF-16 to UTF-8.
- Pseudo-file: its name, for example `<clipboard>`.
- Untitled buffer: nothing inserted, false.

The text is always inserted at the cursor, also in overtype mode. If the cursor is beyond the end of the line, the line is padded with spaces first. The cursor moves to just after the inserted text. ORACLE CMD-CURTEXT-1. Read-only file: plan §5.6.

Returns true when text was inserted.

## Rejected arguments
Any argument → `invalid argument`, false.

## Open points
- Symlink handling and normalization are project choices (the manual only says "fully-qualified").
- Insert vs overtype: ORACLE CMD-CURTEXT-1.

## Test checklist
- Opened as `src/a.c` with cwd `/home/u/p` → `/home/u/p/src/a.c`.
- Opened as `../x/b.txt` → normalized absolute path.
- Windows-style path in a Windows `Env` → backslashes.
- `<clipboard>` → `<clipboard>`.
- Untitled → false, nothing inserted.
- `arg curfile` → `invalid argument`.
- `meta curfile` → inserted, meta cleared.
