# information

Default keys: Shift+F1
Source: M guide p. 97 (Table A.3), p. 38 (Ch. 4.7), p. 60 (`tmpsav`), p. 12–13 (`/t`, no file), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §6.1, §8 M3

## Flags
NOARG only. Not KEEPMETA, not MODIFIES.

## The information file
- MEP remembers the most recently edited files between sessions, at most `tmpsav` of them, each once (p. 60, p. 38), with the cursor and window position of each (p. 38).
- Project decision: the list is kept in `<config dir>/<exe>.tmp` (for example `~/.config/mep/mep.tmp`, `%APPDATA%\mep\mep.tmp`), one file per line as `path<TAB>line<TAB>column<TAB>top<TAB>left`, most recent first, positions 0-based. The directory is created when needed. ORACLE INFO-1.
- The list is written when the editor quits, also when the terminal goes away: the open disk files first (the current file first), then the earlier entries, cut to `tmpsav`. A file shown in a window is written with that window's position, another open file with the position it had when it was last shown. With `-t` (temporary) or `tmpsav:0` nothing is written (p. 12). Write errors are ignored.
- The layout of multiple windows is not restored (project choice; ORACLE INFO-1).

### Startup
- Without file arguments, MEP opens the first file of the list (p. 13 "the file you were editing when you last exited"). If the list is empty or the file is gone, it starts with an empty untitled buffer.
- Opening a file that is in the list puts the cursor and window where they were (p. 38), unless the remembered cursor line lies past the end of the file (it changed outside the editor).
- The information file is read before TOOLS.INI; a missing or unreadable file is an empty list.

## Behavior

### noarg
Switches to the pseudo-file `<information>` like `setfile` does, saving the current file first per `autosave` (p. 97). It lists the files in memory, each with its length (`path  Length=n`), then the remembered files not in memory, one path per line. `arg setfile` on a line opens the file named at the anchor (p. 38). Returns true (Table 6.1 p. 51).

### meta
No meta form. Ignored and cleared.

## Rejected arguments
Any argument → `invalid argument`.

## Open points
- ORACLE INFO-1: location, format and contents of the information file; window layout not restored.

## Test checklist
- Quit writes the list; the next start without files reopens the first file at its position.
- `-t` and `tmpsav:0` write nothing; `tmpsav:1` keeps one entry.
- A file from the list opened later gets its position back.
- Missing first file → untitled; unreadable or malformed list lines ignored.
- `information` lists open files with lengths and remembered files; `arg setfile` on a line opens it.
- `arg information` → invalid argument.
