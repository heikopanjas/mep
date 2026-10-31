# quote

Default keys: Ctrl+P
Source: M guide p. 105 (Table A.3), p. 48 (Ch. 6.2.4), p. 89 (Table A.1), p. 51 (Table 6.1); IMPLEMENTATION_PLAN.md §3.5

## Flags
NOARG, MODIFIES. Not KEEPMETA. `Arg Arg` does not apply. In text-argument editing it is accepted as an editing function (see below).

## Behavior

### noarg
"Reads one keystroke from the keyboard and treats it literally" (p. 105): the next key event is taken without looking up its binding, and its character is put into the file exactly as `graphic` would (insert or overtype, padding, cursor advance; graphic.md). Esc after `quote` inserts 0x1B; Ctrl+L inserts 0x0C. A key with no character inserts nothing. While waiting, the editor keeps rendering (plan §3.5). In a macro, the key is read from the keyboard, not from the macro. Returns true (Table 6.1 p. 51: always).

### In arg mode, text being edited
The next key's character goes into the text argument literally. ORACLE CMD-ARG-3.

### meta
No meta form. Ignored and cleared.

## Rejected arguments
nullarg or a selection pending → `invalid argument`, false (no key is read).

## Open points
- `quote` inside text-argument editing: ORACLE CMD-ARG-3.
- Table 6.1 says "always" true; we keep true even when the key has no character.

## Test checklist
- `quote ctrl+l` → 0x0C inserted, cursor +1, true.
- `quote f8` → nothing inserted (F8 is not run), true.
- `quote esc` → 0x1B inserted, no cancel.
- Insert vs overtype mode both follow graphic.md.
- `arg "a" quote ctrl+a` → text `a\x01` (ORACLE CMD-ARG-3).
- `arg quote` → `invalid argument`, false.
