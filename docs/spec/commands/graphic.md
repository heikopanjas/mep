# graphic

Default keys: all printable keys (plan §6.8; MSJ "The Keyboard")
Source: M guide p. 48 (Ch. 6.2.4), pp. 7–8 (Ch. 2.3, 2.3.1 overtype and insert), p. 98 (Insertmode entry), p. 60 (`rmargin`), p. 61 (`wordwrap`); MSJ (argData: "The graphic function … simply places the value of argData as a character into the file"); IMPLEMENTATION_PLAN.md §3.2, §5.2–§5.4, §6.13

## Flags
NOARG, MODIFIES. Not in Table A.3 or Table 6.1. In arg mode, printable keys do not run `graphic` as a command: they edit the text argument (arg.md). Not KEEPMETA (meta is ignored and cleared). `Arg Arg` does not apply.

## Behavior

### noarg
Puts the character of the key that invoked it (argData) into the file at the cursor:
- Insert mode: inserts it, shifting the rest of the line right (p. 98).
- Overtype mode (the default, p. 7): replaces the character under the cursor; a wide character is replaced as a whole. At or beyond the end of the line it is appended.
- Beyond the end of the line: the line is first padded with spaces up to the cursor column (plan §5.4). Beyond the end of the file: empty lines are appended up to the cursor line first, using the file's default terminator (plan §5.1).
- A line containing tabs is converted to its expanded form before the edit (plan §5.2, ORACLE TAB-1).
The cursor moves right by the character's cell width. The window follows. The line is now edited, so the trailing-space rule (plan §5.3) applies to it.

Keys assigned with `graphic:key` (p. 48) insert the key's character code: Ctrl+letter gives 0x01–0x1A (Ctrl+L = form feed 0x0C, as on p. 48). A key with no character (function keys, cursor keys) inserts nothing and returns false without a message.

Read-only file: subject to plan §5.6; declined → no change, false.

Returns true when a character was placed. Not in Table 6.1; project choice.

### Word wrap
With `wordwrap` on, "any character typed to the right of this margin causes a line break" (p. 60): when the inserted character ends up at a cell column ≥ `rmargin`, the line is broken at the last blank before the word containing the cursor; that word moves to a new line indented to the first nonblank column of the broken line, and the cursor stays after the typed character. A word with no blank before it is not broken. ORACLE CMD-GRAPHIC-1.

### meta
No meta form. Meta is ignored and cleared.

## Rejected arguments
Not reachable through the dispatcher in arg mode (typing goes to the text argument). Called by name with an argument (`arg "x" graphic` in a macro) → `invalid argument`, false.

## Open points
- Word-wrap details (column test, where the break goes, indentation of the new line, the Z variant "space past margin or any character past rmargin+4"). M p. 60 wins over Z; the remaining details: ORACLE CMD-GRAPHIC-1.
- Return value not in Table 6.1: we return true when a character was placed.

## Test checklist
- Overtype `abc`, cursor 1, type `X` → `aXc`, cursor 2, true.
- Insert mode: → `aXbc`, cursor 2.
- Cursor at column 6 on `abc` → `abc   X`.
- Cursor on line 5 of a 2-line file → lines 2–4 empty, `X` on line 5 at its column; new lines use the file's terminator (CRLF file stays CRLF).
- Overtype on a wide character → whole character replaced; cursor moves by the new character's width.
- Line with a tab: line expanded, edit applied, saved per `entab` (ORACLE TAB-1).
- `graphic:ctrl+l` then ctrl+l → 0x0C inserted.
- `graphic:f5` then F5 → nothing, false.
- `wordwrap:` `rmargin:10`, type past column 10 → break before the word (ORACLE CMD-GRAPHIC-1).
- `meta a` → `a` placed, meta cleared.
- Read-only file → per plan §5.6.
