# initialize

Default keys: Shift+F8
Source: M guide p. 98 (Table A.3), pp. 63–65 (Ch. 7.5 tags and load order), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §3.1, §6.12

## Flags
From the syntax lines on p. 98:
- `Initialize` → NOARG.
- `Arg Initialize` → NULLARG with NULLEOW: "the continuous string of nonblank characters, starting with the initial cursor position" arrives as a textarg.
- `Arg textarg Initialize` → TEXTARG.
- `Arg streamarg Initialize` → BOXSTR (one-line selection as text).
- No LINEARG, BOXARG or NUMARG (numeric text is a tag name: `arg "2" initialize` reads `[mep-2]`).
- Not KEEPMETA, not MODIFIES. `Arg Arg` has no special meaning.

## Behavior
Project decision (IMPLEMENTATION_PLAN.md §3.1, §6.12): the manual's `[M]` section is the section named after the executable (`[mep]`, or `[mz]` for a link named `mz`), and `[M-text]` is `[<exe>-text]`. Section names match case-insensitively; multi-name tags (`[m mep]`, `[mep-c mep-cpp]`) match if any name matches. The DOS/OS/2 and display tags of p. 64–65 are not read. TOOLS.INI is located as in plan §6.12; reading it is an I/O request (the command suspends, plan §3.5).

Statements are applied on top of the current state (assignments, macros, switches); nothing is reset to defaults first. ORACLE CMD-INITIALIZE-2.

### noarg
Reads all statements from `[<exe>]`, then `[<exe>-<os>]` (plan §6.12, the replacement for the original's OS tag loaded at startup). Returns true if TOOLS.INI was found and every statement was valid.

### nullarg
Takes the nonblank word starting at the anchor as the tag text and behaves like the textarg form. If the anchor is on a blank, the word is empty → `invalid argument`, false.

### textarg / streamarg
Reads `[<exe>]` and then `[<exe>-text]` (p. 98). Example from p. 63: `arg "C" initialize` reads `[mep]` and `[mep-c]`.

### Errors
- TOOLS.INI not found → message, false.
- `[<exe>-text]` not present → message naming the section, false (the `[<exe>]` statements are still applied).
- Invalid statements: the valid ones are applied; message of the first error; false (Table 6.1 p. 51 "Bad argument"). Same collection rule as at startup (plan §6.12).

### `-d`
`-d` only suppresses reading TOOLS.INI at startup (M guide p. 12); an explicit `initialize` still reads it.

### meta
No meta form. Meta is ignored and cleared.

## Rejected arguments
linearg, boxarg (multi-line selections) → `invalid argument`, false.

## Open points
- `Arg Initialize` (nullarg): p. 98 says it uses the word "as the tag name", without mentioning `[M]`; the textarg form reads `[M]` and `[M-text]`. NULLEOW turns the nullarg into a textarg, so a command built on these flags cannot tell them apart. We read both sections for both forms. ORACLE CMD-INITIALIZE-1.
- Whether `initialize` first resets assignments, macros and switches to their defaults. We apply on top of the current state. ORACLE CMD-INITIALIZE-2.
- `initialize` after starting with `-d`: we read TOOLS.INI (project decision, the user asked explicitly).

## Test checklist
- noarg with `[mep]` containing `hscroll:20` → switch set, true.
- noarg reads `[mep-macos]` (with the test `Env` reporting macOS) after `[mep]`.
- Executable named `mz` → reads `[mz]`, not `[mep]`.
- `[m mep]` multi-name tag matches.
- `arg "C" initialize` with `[mep-c]` defining `Header:=…` → macro defined; `[mep]` also read.
- nullarg with the cursor on the word `c` → same as textarg `c` (ORACLE CMD-INITIALIZE-1).
- nullarg on a blank → `invalid argument`, false.
- streamarg over `c` → same as textarg.
- Section `[mep-pascal]` missing → message, false.
- No TOOLS.INI → message, false.
- One invalid line in the section → others applied, false, first error shown.
- Assignment made with `assign`, then `initialize` → still present (ORACLE CMD-INITIALIZE-2).
- linearg `arg down initialize` → `invalid argument`, false.
- Read failure injected in the fake `Fs` → message, false.
