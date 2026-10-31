# execute

Default keys: F7
Source: M guide p. 97 (Table A.3), p. 51 (Table 6.1), p. 12 (`/e` option uses macro syntax), pp. 48–53 (Ch. 6.3 macro syntax and conditionals); MSJ `fExecute` (Fig. 7, `DoSpawn`); IMPLEMENTATION_PLAN.md §6.1 (`-e`), §6.14

## Flags
From the syntax lines on p. 97:
- `Arg Execute` → NULLARG with NULLEOL: "the line from the initial cursor position to the end" arrives as a textarg.
- `Arg textarg Execute` → TEXTARG.
- `Arg streamarg Execute` → STREAMARG. STREAMARG accepts any cursor movement (M guide p. 73), so a multi-line box-shaped selection arrives as a stream; line breaks inside it separate items like spaces.
- `Arg linearg Execute` → LINEARG: the text of all lines, joined with spaces.
- No NOARG, no BOXARG, no NUMARG (numeric text is a textarg: `arg "5" execute` fails as an unknown function).
- Not KEEPMETA. Not MODIFIES itself; the executed functions modify as they do.
- `Arg Arg` has no special meaning.

## Behavior
The text is parsed with the macro syntax of plan §6.14 (functions and macro names, quoted strings, `:>label`, `=>label`, `+>label`, `->label`) and run like an anonymous macro. Quoted strings go through the arg model if arg mode is on, otherwise through `graphic`. The screen is not redrawn until input is required (plan §6.14).

### nullarg
Executes the text from the anchor to the end of the line. Cursor: wherever the executed commands leave it.

### textarg / streamarg / linearg
Executes the given text.

### Parse errors
The whole text is parsed before anything runs. An unknown function or macro name, an unterminated string or a jump to a missing label → message naming the problem, nothing runs, returns false.

### Return value
The value of the last function executed (Table 6.1 p. 51 "Last command successful / Last command failed"; same rule as macros, ORACLE MACRO-1). An empty text runs nothing and returns true.

### Undo
The whole execution is one undo step, like a macro invoked from a key (ORACLE UNDO-1).

### `-e` and extensions
The `-e` startup option (plan §6.1) and the `fExecute` callback (M4) run text through the same code.

### meta
No meta form. Meta is cleared before the text runs. ORACLE CMD-EXECUTE-1.

## Rejected arguments
- noarg → `invalid argument`, false.
- boxarg cannot occur (a box-shaped selection arrives as a stream).

## Open points
- A pending meta when `execute` runs: cleared before the text runs (the first executed function does not see it), or handed to the first function. We clear it. ORACLE CMD-EXECUTE-1.
- Return value = last function executed: ORACLE MACRO-1. Undo granularity: ORACLE UNDO-1.
- Parse-before-run is a project choice; the manual only says each function "must be previously defined" (p. 49).

## Test checklist
- `arg "down down" execute` → cursor two lines down, returns true.
- `arg "left" execute` at column 0 → returns false (last function failed).
- `arg "arg \"abc\" psearch" execute` → finds `abc`.
- `arg "endline :>back sdelete left +>back" execute` (Blankline, p. 53) → line emptied.
- nullarg: line contains `down down`, cursor on the first `d` → two lines down.
- linearg over two lines `down` / `down` → two lines down.
- streamarg over `down down` → two lines down.
- Unknown name `arg "frobnicate" execute` → message, false, nothing executed.
- `arg "5" execute` → unknown function, false.
- noarg `execute` → `invalid argument`, false.
- Editing executed text then `undo` → one undo step reverts all of it (ORACLE UNDO-1).
- `meta arg "begline" execute` → meta cleared before `begline` runs (ORACLE CMD-EXECUTE-1).
