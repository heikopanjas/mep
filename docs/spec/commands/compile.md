# compile

Default keys: Shift+F3
Source: M guide pp. 94–95 (Table A.3), pp. 35–37 (Ch. 4.5), p. 17 (`Arg Meta Compile`), p. 59 (`maxmsg`), p. 62 (`extmake`), p. 89 (Table A.1), p. 50 (Table 6.1); z-vs-mep.md S7; IMPLEMENTATION_PLAN.md §3.5, §6.7, §8 M4

## Flags
NOARG, NULLARG, TEXTARG, BOXSTR, meta forms. Not KEEPMETA, not MODIFIES.

## Running a compile
- A compile runs a command line through the system shell (`sh -c` on macOS and Linux, `cmd /C` on Windows) in the editor's working directory, in the background: editing goes on while it runs (p. 35 "compiles run in the background and the editor beeps when the compile is completed"). Compiles run one after another in the order they were started.
- Standard output and standard error are read together through one pipe (p. 37 "processes all error messages through a pipe").
- When the compile ends, the editor beeps, puts the complete output into the pseudo-file `<compile>` (replacing what it held), and reads the messages from it (format below). The previous message list is replaced.
  - With messages: the cursor goes to the first one as with `Compile` (p. 36 "the editor moves the cursor to the first error location reported"), unless a command is waiting for an answer; then the dialog line says `<n> compile messages` and `Compile` shows the first one.
  - Without messages: the dialog line says `Compile finished` if the command succeeded, `Compile failed (see <compile>)` otherwise.
- The return value of the starting command is true if the compile was started (Table 6.1: false for a bad argument or a missing `extmake` command). The result of the compile itself is reported as above. ORACLE COMPILE-3.

## Message format
A line of output is a message if it matches one of these forms, tried in order (p. 36; `filename`, `row`, `column`, `message` are placeholders; blanks around the punctuation are optional):

| Form | Example |
|---|---|
| `"filename", row column: message` | `"a.c", 12 5: syntax error` |
| `filename(row, column): message` | `a.c(12, 5): syntax error` |
| `filename(row): message` | `a.c(12): warning C4013` |
| `filename:row:column: message` | `a.c:12:5: error: x` (project addition for modern compilers; ORACLE COMPILE-1) |
| `filename: row: message` | `a.c: 12: error`, also `grep -n` output `a.c:12:text` |
| `filename row column: message` | `a.c 12 5: syntax error` |

Rows and columns in messages count from 1; a missing column means column 1. A relative `filename` is relative to the working directory of the compile. Other lines are not messages. ORACLE COMPILE-1.

## Behavior

### noarg (`Compile`)
Goes to the next message: opens its file if needed, puts the cursor on its row and column and shows the message text on the dialog line (p. 94). True. No message left: `No more compile messages`, false.

### meta (`Meta Compile`)
Skips to the first following message whose file is not the current file (p. 94) and goes there like `Compile`. None: `No more compile messages`, false.

### nullarg (`Arg Compile`)
Compiles the current file with the `extmake` command for its extension (p. 94, p. 62): `%s` in the command is replaced by the file's path. A modified file is saved first (ORACLE COMPILE-2). No `extmake` for the extension: `No extmake command for .<ext>`, false. A pseudo-file: `No file name`, false.

### textarg, one-line selection (`Arg textarg Compile`)
Compiles the named file (p. 94): the `extmake` command for the text's extension, or the one for `text` when there is none (p. 62 example: `Arg foo Compile` runs `make foo`); `%s` is replaced by the text (ORACLE COMPILE-4). Neither exists: `No extmake command for .<ext>`, false.

### `Arg Arg textarg Compile`
Runs the text as the command line (p. 95; for example `arg arg "grep -n main *.c" compile`). Empty text: `invalid argument`.

### `Arg Meta Compile`
Goes back to the previous message (p. 95), at most `maxmsg` messages back from the newest one shown (p. 59). Nothing before: `No previous compile message`, false.

## Rejected arguments
Selections over several lines → `invalid argument`. A number is text: `Arg 5 Compile` compiles the file `5`.

## Open points
- ORACLE COMPILE-1: the gcc form `file:row:column:` and 1-based rows and columns.
- ORACLE COMPILE-2: `Arg Compile` saves a modified current file before compiling.
- ORACLE COMPILE-3: the output pseudo-file `<compile>`, the jump to the first message when the compile ends, return values.
- ORACLE COMPILE-4: `Arg textarg Compile` uses the text's own extension first, then `text`.

## Test checklist
- `arg compile` with `extmake:c cc %s` runs `cc /work/a.c`; output with two messages → `<compile>` filled, cursor at the first, beep; `compile` → second; `compile` → `No more compile messages`.
- Every message form; a column-less form; a line that is no message.
- `meta compile` skips messages of the current file; `arg meta compile` goes back, limited by `maxmsg`.
- `arg "foo" compile` with `extmake:text make %s` → `make foo`; no extmake → message, false.
- `arg arg "grep -n x a.c" compile` → messages from `grep` output.
- Clean compile → `Compile finished`; failed compile without messages → `Compile failed (see <compile>)`.
- A modified current file is saved before `arg compile`.
- A command that cannot start → `Compile failed (see <compile>)` with the error in `<compile>`.
