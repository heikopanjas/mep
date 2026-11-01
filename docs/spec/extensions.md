# C extensions

Source: M guide Chapter 8 pp. 68–85 (§8.2 how extensions work, §8.3 tables and `WhenLoaded`, §8.4 low-level functions, §8.6 `Upper` sample), p. 62 (`load`); MSJ "Programming Extensions" (filter extension, Figs. 3–8); IMPLEMENTATION_PLAN.md §3.4, §8 M4

## Loading
- `load:name` (TOOLS.INI or `assign`, any time; MSJ) loads a shared library: `.so` on Linux, `.dylib` on macOS, `.dll` on Windows. A relative name is relative to the working directory. A name without an extension that does not load as given is tried with the platform's extension added (project choice; ORACLE EXT-1).
- The library must export `cmdTable`, `swiTable` and `WhenLoaded` (p. 70) and the callback table pointer `MepCallbacks` that `exthdr.c` defines (see "Calling the editor").
- On loading, the editor fills `MepCallbacks`, reads `cmdTable` and `swiTable` up to the entry with a null name, registers the functions and switches, then calls `WhenLoaded` (p. 69).
- Errors: the library does not load or lacks a required symbol → `<name>: cannot load extension: <reason>`, the `load` statement fails (an `assign` returns false, TOOLS.INI reports it like other errors). Loading the same library again does nothing.
- Extensions stay loaded until the editor ends.

## Functions (`cmdTable`)
- Each entry: name, function, a reserved field, flags (p. 72). The flags are the argument types and `KEEPMETA`, `CURSORFUNC`, `WINDOWFUNC`, `MODIFIES`, `BOXSTR`, `NULLEOL`, `NULLEOW`, `NUMARG`, `MARKARG`; `ext.h` gives their values. `COLARG` (project addition, `argcol.md`, no manual equivalent) is additive: declaring it hands the extension function a multi-line `ExtArg::Stream` for a column-anchored selection, the same `ExtArg` shape a plain multi-line stream already uses — no new `ExtArg` variant, no ABI break.
- A function becomes known like a built-in one: it can be assigned to keys, used in macros and in `SetKey`. A name that is already a built-in function keeps the built-in one (ORACLE EXT-2); a name that another extension registered is replaced.
- The editor checks the argument against the flags as for built-in functions and converts it before the call (MSJ Fig. 6): a number is a `LINEARG`, a marker a stream, box or line selection, a one-line selection with `BOXSTR` a `TEXTARG`, a null argument with `NULLEOL`/`NULLEOW` a `TEXTARG`.
- The call: `flagType func(unsigned int argData, ARG *pArg, flagType fMeta)` (p. 74). `argData` is the character of the key that invoked the function, 0 for other keys and calls from macros. `fMeta` is −1 if meta is on, 0 otherwise. The return value: 0 is false, anything else true.
- `ARG` (p. 75): `argType` and a union. Positions are 0-based (line `y`, column `x`); `cArg` is the number of `arg` presses.

| argType | Fields |
|---|---|
| `NOARG` | `y`, `x`: the cursor |
| `NULLARG` | `cArg`, `y`, `x`: the anchor |
| `TEXTARG` | `cArg`, `y`, `x`, `pText` (zero-terminated, valid during the call) |
| `LINEARG` | `cArg`, `yStart`, `yEnd` (inclusive) |
| `STREAMARG` | `cArg`, `xStart`, `yStart`, `xEnd`, `yEnd` (the end is not part of the stream) |
| `BOXARG` | `cArg`, `xLeft`, `yTop`, `xRight`, `yBottom` (all inclusive) |

## Switches (`swiTable`)
- Each entry: name, pointer, type (p. 71). `SWI_BOOLEAN`: the pointer is an `int` set to −1 or 0 (`name:` and `noname:`). `SWI_NUMERIC`: an `int` set to the decimal value. `SWI_SPECIAL`: a function `int f(char *value)` called with the text (MSJ `filtcmd`); a return value of 0 makes the assignment fail.
- Extension switches are set like built-in switches, in TOOLS.INI and with `assign`. Invalid values (a value after a Boolean switch, a non-number) are errors as for built-in switches.

## Calling the editor (`ext.h`, `exthdr.c`)
The original linked `EXTHDR.OBJ` into every extension to reach the editor (p. 69). We ship `ext.h` and `exthdr.c`: `exthdr.c` defines the low-level functions below and forwards each through the table `MepCallbacks`, which the editor fills when it loads the library. Compile `exthdr.c` together with the extension.

Handles (`PFILE`) name open files; 0 is no file. Positions are 0-based. All callbacks act at once; changes are undone with the command that called the extension (one `undo` step).

| Function | Behavior |
|---|---|
| `PFILE AddFile(char *name)` | Opens a file entry without reading it (MSJ); an open file of that name is returned as is |
| `flagType BadArg(void)` | Shows `invalid argument`; returns false |
| `void CopyBox(PFILE src, PFILE dst, COL xLeft, LINE yTop, COL xRight, LINE yBottom, COL xDst, LINE yDst)` | Inserts the box (inclusive) of `src` into `dst` at (`xDst`, `yDst`), like a box paste |
| `void CopyLine(PFILE src, PFILE dst, LINE yStart, LINE yEnd, LINE yDst)` | Inserts lines `yStart..=yEnd` of `src` before line `yDst` of `dst`; `src` 0 inserts one empty line (p. 80) |
| `void CopyStream(PFILE src, PFILE dst, COL xStart, LINE yStart, COL xEnd, LINE yEnd, COL xDst, LINE yDst)` | Inserts the stream into `dst` at (`xDst`, `yDst`), like a stream paste |
| `void DelBox(PFILE, COL xLeft, LINE yTop, COL xRight, LINE yBottom)` | Deletes the box (inclusive) |
| `void DelFile(PFILE)` | Deletes the whole contents (MSJ) |
| `void DelLine(PFILE, LINE yStart, LINE yEnd)` | Deletes the lines (inclusive) |
| `void DelStream(PFILE, COL xStart, LINE yStart, COL xEnd, LINE yEnd)` | Deletes up to but not including the end (p. 81) |
| `int DoMessage(char *text)` | Shows the text on the dialog line; returns its length (p. 82) |
| `void Display(void)` | Repaints the screen after the command (ORACLE EXT-3) |
| `flagType fExecute(char *macro)` | Runs macro text at once and returns its value (MSJ `DoSpawn`); file and clipboard work completes before it returns; a step that needs an answer from the user fails |
| `LINE FileLength(PFILE)` | Number of lines (`Length` on the status line) |
| `PFILE FileNameToHandle(char *name, char *shortName)` | `""`: the current file; otherwise the open file with that name, or one whose name ends with `shortName` (p. 78); 0 if none |
| `flagType FileRead(char *name, PFILE)` | Replaces the contents with the disk file; a pseudo-file name reads nothing and succeeds (MSJ); false if the file cannot be read |
| `flagType FileWrite(char *name, PFILE)` | Writes the contents to the disk file; false for a pseudo-file name or an error |
| `void GetCursor(COL *x, LINE *y)` | The cursor of the current window |
| `int GetLine(LINE line, char *buf, PFILE)` | Copies the line (tabs expanded, at most `BUFLEN` − 1 bytes, zero-terminated) and returns its length; a line past the end is empty (p. 79) |
| `void KbUnHook(void)`, `void KbHook(void)` | Stop and restart the editor's keyboard reading, so the extension can read the console itself |
| `void MoveCur(COL x, LINE y)` | Moves the cursor of the current window |
| `void pFileToTop(PFILE)` | Makes the file current |
| `void PutLine(LINE line, char *text, PFILE)` | Replaces the line; past the end the file grows (p. 80) |
| `void RemoveFile(PFILE)` | Closes the file without saving; windows showing it switch as with `refresh` |
| `int ReadChar(void)` | Waits for a key; returns its character, 0 for keys without one |
| `char *ReadCmd(void)` | Waits for a key; returns the name of the function assigned to it (valid until the next call) (ORACLE EXT-4) |
| `flagType Replace(char c, COL x, LINE y, PFILE, flagType fInsert)` | Inserts (`fInsert` true) or overwrites the character at (x, y) (p. 80) |
| `flagType SetKey(char *function, char *key)` | Assigns a key like TOOLS.INI `function:key` (p. 82) |

Calls outside an extension function, `WhenLoaded` or a switch function do nothing and return 0.

## Open points
- ORACLE EXT-1: library names, `MepCallbacks`, `exthdr.c` instead of `EXTHDR.OBJ`, exported names, type sizes (`LINE` and `COL` are 32-bit, `flagType` is `int`).
- ORACLE EXT-2: name clashes with built-in functions.
- ORACLE EXT-3: `Display` repaints after the command instead of at once.
- ORACLE EXT-4: `ReadCmd` returns a function name instead of an internal command pointer.

## Test checklist
- `load:` a library with a function and every switch type; `assign` the function to a key; call it with every argument type and meta; check `ARG` fields.
- Every callback, including handles of other files and the current file, and invalid handles.
- `fExecute` with a macro that loads a file, runs `shell` and fails.
- `load:` a missing library, a library without `cmdTable` → message, false.
- Acceptance: the MSJ `filter` extension, compiled with the platform C compiler, sorts a line range through `sort`.
