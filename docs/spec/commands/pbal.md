# pbal

Default keys: Ctrl+[
Source: M guide p. 103 (Table A.3), p. 88 (Table A.1), p. 51 (Table 6.1); MSJ Fig. 1 ("Balance unmatched parentheses, braces, or brackets"); IMPLEMENTATION_PLAN.md §3.5, §7 (Ctrl+[ = Esc collision)

## Flags
From p. 103:
- `Pbal` → NOARG.
- `Arg Pbal` → NULLARG.
- `Meta Pbal`, `Arg Meta Pbal` → meta forms.
- MODIFIES (the non-meta forms). No TEXTARG, LINEARG, STREAMARG, BOXARG, NUMARG.
- Not KEEPMETA. `Arg Arg` behaves like `Arg`.

## Behavior
Bracket pairs: `()`, `[]` and `{}` (MSJ names braces; the M guide says "parentheses and brackets"). The scan does not look at quotes or comments. It runs in bounded slices and Esc cancels it (plan §3.5).

### noarg
Scans backward from the character before the cursor (the cursor position itself is not included, p. 103) toward the start of the file. A closing character is pushed on a stack; an opening character pops a matching closer. The first opening character with an empty stack is the unbalanced one. An opening character that does not match the closer on top of the stack also ends the scan as the unbalanced one. ORACLE CMD-PBAL-1.

When found:
- it is highlighted until the next command; if its line is not visible, that line is shown on the dialog line with the character highlighted (p. 103);
- the corresponding closing character is put into the file at the cursor, the same way `graphic` puts a character (insert or overtype mode, cursor advances). ORACLE CMD-PBAL-1.
Returns true. Not found: message (project wording, e.g. `no unbalanced bracket`), no change, false (Table 6.1 p. 51 "not balanced"). Read-only file: plan §5.6.

### nullarg (`Arg Pbal`)
Same, scanning forward from the character after the cursor to the end of the file, looking for an unbalanced closing character; the corresponding opening character is put in at the cursor (p. 103).

### meta (`Meta Pbal`)
Like noarg, but the file is not changed: only the highlight or dialog-line display (p. 103). Meta cleared.

### `Arg Meta Pbal`
Like `Arg Pbal` without changing the file (p. 103).

## Rejected arguments
textarg, numarg, streamarg, linearg, boxarg → `invalid argument`, false.

## Open points
- Bracket set (braces included), mismatch handling, and whether the inserted character follows insert/overtype mode. ORACLE CMD-PBAL-1.
- Ctrl+[ arrives as Esc on terminals without the kitty protocol (plan §7); the key binding is a front-end concern.

## Test checklist
- `f(a, b` with cursor at the end → `)` inserted, `(` highlighted, true.
- `x = [1, (2)` → `]` inserted.
- `{ (a) ` → `}` inserted.
- Balanced text `(a)` → message, false, no change.
- Matching bracket on a line outside the window → line shown on the dialog line.
- Cursor on a `(`: the scan starts before it.
- `arg pbal` on `a) b` with cursor at 0 → `(` inserted at 0.
- `meta pbal` → highlight only, file unchanged, meta cleared.
- `arg meta pbal` → forward, file unchanged.
- Mismatch `(]` with the cursor at the end → scan stops at `(` (ORACLE CMD-PBAL-1).
- Insert vs overtype mode for the inserted character.
- `arg "x" pbal` → `invalid argument`.
