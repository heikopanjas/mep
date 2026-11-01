# ORACLE Items

Behaviors where the manual is ambiguous or contradicts itself. The code and its
test carry `// ORACLE: <id>` and implement the manual's reading; the tests stay
active (IMPLEMENTATION_PLAN.md §9.5). The owner confirms each item against a
real MEP and records the result here. If the real binary differs, test and code
change together.

Each item has:

- **Manual text**: the quoted passage with its page ("M guide p. N")
- **Chosen behavior**: what the implementation does
- **Confirm with**: a key sequence to run in a real MEP
- **Result**: filled in by the owner after confirmation

## TAB-1: tab width and entab values

- **Manual text**: "**entab** Controls the degree to which the Microsoft Editor converts multiple spaces to tabs when editing a file. A value of 0 means that tabs are not used to represent white space, 1 means that all multiple spaces outside of quoted strings are converted, 2 means that all multiple spaces are converted to tabs. (Default value: 1)" (M guide p. 59). The width of a tab character in the file is not stated; **tabstops** (p. 60) only controls cursor movement.
- **Chosen behavior**: tabs in the file display to the next multiple of 8 cells (`cells::TAB_WIDTH`). Edited lines are re-tabbed on save: a run of two or more spaces that reaches a multiple of 8 becomes a tab; a single space before a tab stop stays a space. `entab:1` keeps spaces between `"` or `'` quotes (no backslash escapes); `entab:2` converts everywhere. Unedited lines are written byte-exact.
- **Confirm with**: create a file whose line is `a<TAB>b`, open it and check that `b` is in column 9 (status line with `displaycursor:`). Then type `x       y` (x, 7 spaces, y) and `"        "` on new lines, save with `arg arg setfile`, and inspect the bytes with a hex viewer for `entab:0`, `entab:1` and `entab:2`.
- **Result**: open

## FILE-BAK-1: backup file name

- **Manual text**: "A value of **bak** specifies that the file name of the old version of the file will be changed to **.BAK**." (M guide p. 62, Table 7.4 `backup`)
- **Chosen behavior**: the old version is renamed by replacing the file's extension with `bak` (`main.c` → `main.bak`, `Makefile` → `Makefile.bak`, `a.tar.gz` → `a.tar.bak`), replacing an existing backup. A file that did not exist before the save gets no backup.
- **Confirm with**: with `backup:bak`, edit and save `main.c` and `Makefile` and list the directory; save twice to see whether the second save replaces `main.bak`.
- **Result**: open

## INI-1: comments in TOOLS.INI

- **Manual text**: "The Microsoft Editor assumes everything on the line following a semicolon is a comment and ignores it." (M guide p. 55, §7.1). The same manual's example `extmake:asm masm -Mx %s;` (p. 62, Table 7.4) ends a switch value with a semicolon, which that rule would cut off.
- **Chosen behavior**: only a line whose first non-blank character is `;` is a comment. A `;` later in a line is part of the statement. There are no continuation lines (the manual describes none).
- **Confirm with**: put `extmake:asm masm -Mx %s;` and `hike:9 ; comment` into the `[mep]` section, start MEP, run `arg ? assign` or check the hike behavior, and see whether `hike:9 ; comment` is accepted and whether the extmake value keeps its `;`.
- **Result**: open

## Command behavior items

The command specs in `docs/spec/commands/` give the full manual text and reasoning for each item under "Open points". Each entry below names the chosen behavior and a sequence to confirm it. **Result** is open for all of them.

| ID | Manual text (page) | Chosen behavior | Confirm with |
|---|---|---|---|
| S4 | `mpara`: "first blank line preceding the current paragraph" (p. 100) | On a text line: the blank line above the paragraph; on a blank line: the blank line above the previous paragraph; line 0 if none | Three paragraphs, cursor in the third, `mpara` twice |
| S5 | `Arg Arg textarg Paste`: "at the current cursor position" (p. 30) vs "above the current line" (p. 102) | Inserted at the cursor as a stream | `alt+a alt+a "file" shift+ins` in mid-line |
| UNDO-1 | One undo step per command (p. 9); macros not mentioned | A macro or `execute` run from a key is one step | Macro typing two characters, then `alt+bksp` |
| MACRO-1 | Macro return value not stated (Table 6.1) | Value of the last function run | Macro `right left left` at column 1 followed by `+>`/`->` |
| STATUS-1 | Status fields (p. 13) and `Length=0` vs "minimum value is 1" (p. 7) | `name (type modified NL) Length=n  Window=(r,c) insert meta`; `Cursor=` with `displaycursor`; `Length` counts lines without the empty line after a final break, minimum 1 | Open an empty file and a file with a final line break; toggle `insertmode` and `meta` |
| CMD-ARG-1 | Only `Arg` and `Arg Arg` are described (p. 16) | A third `arg` keeps counting; count ≥ 2 acts as `Arg Arg` | `alt+a alt+a alt+a "x" f3` |
| CMD-ARG-2 | `arg` after moving, typing after moving: not described | `arg` raises the count and keeps the selection; typing drops the selection and starts text at the cursor | `alt+a right right alt+a f3`; `alt+a right x` |
| CMD-ARG-3 | Text editing keys (p. 19) | Typing follows insert mode; Backspace acts as `cdelete`; `quote` inserts literally; other cursor functions reject the text | `alt+a "abc" home x` in both modes; `alt+a "x" up` |
| CMD-ASSIGN-1 | Multi-line `assign` errors not described (p. 93) | Valid lines apply; false with the first error | Linearg `assign` over one good and one bad line |
| CMD-BLOCK-1 | Cursor after block commands not stated | Start or upper-left corner of the region | `alt+a` box then `ctrl+y`, `ctrl+n` |
| CMD-CANCEL-1 | Whether `cancel` clears meta (p. 93) | Clears meta; cursor stays | `f9 esc`, check status line |
| CMD-CDELETE-1 | `cdelete` with a pending argument | `invalid argument` | `alt+a ctrl+g` |
| CMD-CURDATE-1 | `Jun-27-1987` (p. 95) vs `27-Jun-1987` (p. 88) | `Mon-DD-YYYY` (Table A.3) | `curdate` |
| CMD-CURFILEEXT-1 | Extension format not stated (p. 96) | With the dot; nothing for no extension or pseudo-files | `curfileext` in `a.c` and `Makefile` |
| CMD-CURTEXT-1 | Insert vs overtype not stated | Always inserts, cursor after the text | `curday` in overtype mode mid-line |
| CMD-EMACSNEWL-1 | `meta emacsnewl` not listed | Column 0 like `meta newline` | Insert mode, `f9 enter` on an indented line |
| CMD-ENDLINE-1 | "one character beyond … the rightmost edge of the window" (p. 97) | Column = window left + width; the window scrolls | `f9 end` |
| CMD-EXECUTE-1 | Meta during `execute` not stated | Meta is cleared before the text runs | `f9 alt+a "begline" f7` |
| CMD-EXIT-1 | Other modified files on quit not described | Confirmation prompt when other files are modified | Modify two files, `f8` |
| CMD-GRAPHIC-1 | Word wrap details (p. 60) | Break at the last blank before the cursor word when the typed character reaches `rmargin`; new line indented like the old one | `wordwrap:`, `rmargin:10`, type a long sentence |
| CMD-HOME-1 | `meta home` not listed (p. 97) | Same as `home` | `f9 ctrl+home` |
| CMD-INITIALIZE-1 | `Arg Initialize` word (p. 98) | Reads `[exe]` and `[exe-word]` | `alt+a` on the word `c`, `shift+f8` |
| CMD-INITIALIZE-2 | Reset before rereading not stated | Statements apply on top of the current state | Assign a key, `shift+f8`, check the key |
| CMD-LASTTEXT-1 | Which texts are recalled (p. 98) | Typed texts passed to a function; `arg arg lasttext` keeps count 2 | `alt+a "x" f3 ctrl+o` |
| CMD-LDELETE-1 | `ldelete` beyond the end of the file | File unchanged, one empty line in the clipboard | Cursor below the last line, `ctrl+y`, `shift+ins` |
| CMD-NEWLINE-1 | C-specific `softcr` indentation (p. 102) | Not implemented; generic rules for all files | `.c` file with an open brace, `newline` |
| CMD-PASTE-1 | Cursor after paste not stated | Cursor does not move | `shift+ins` with a stream |
| CMD-PASTE-2 | `Arg Paste` clipboard effect (p. 102) | The text also goes into the clipboard | `alt+a shift+ins`, then `shift+ins` elsewhere |
| CMD-PASTE-3 | Paste failure value (Table 6.1 says always true) | False when a file insert fails | `alt+a alt+a "missing" shift+ins` in a macro with `->` |
| CMD-PBAL-1 | Brace handling and insert mode (p. 103) | `()`, `[]`, `{}` with stack matching; inserted character follows insert mode | `{x` then `ctrl+[` |
| CMD-PPAGE-1 | `Arg streamarg Ppage` (p. 27) | A highlighted number is the page count | Highlight `2`, `pgdn` |
| CMD-PPARA-1 | Paragraph movement column and file end | Column 0; past the last paragraph the cursor goes past the end of the file | Inpara macro (p. 57) |
| CMD-PWORD-1 | Word definition not stated | Letters, digits, `_` | `ctrl+right` over `a_b.c` |
| CMD-RESTCUR-1 | `restcur` after switching files | False, nothing moves | `savecur`, `f2`, `restcur` |
| CMD-SCROLL-1 | Cursor during window scrolling | Cursor keeps its window row; window top ≤ last line | `ctrl+z` near the end of a file |
| CMD-SCROLL-2 | "or less if in a small window" (p. 96) | Jump limited to window size − 1 | `height:5`, cursor down past the window |
| CMD-SETFILE-1 | Missing file on `setfile` (p. 13) | Ask to create it | `alt+a "new" f2` |
| CMD-SETFILE-2 | `Arg Arg textarg Setfile` (p. 108) | Writes a copy; the buffer keeps its name and state | `alt+a alt+a "copy" f2`, check status |
| CMD-UNASSIGNED-1 | Return value and message from a macro | False; uses the key that started the command | Macro calling `unassigned` |
| S3 | `Arg numarg Mark`: "the first line of the file is line 0" (p. 31) vs the 1-based status line | Line numbers start at 0; `markfile` lines and columns too | `alt+a "0" ctrl+m`, `alt+a "5" ctrl+m`, compare with the status line |
| S6 | `Arg markarg Replace`: "the box of text between the cursor position and the marker" (p. 34) | A mark argument to `replace`/`qreplace` is always a box; for `copy`, `ldelete` and others it is a stream, lines or a box by position | Define a marker, move down and right, `alt+a "m" ctrl+l` |
| MARK-1 | `Arg Mark`: "location prior to the last scrolling operation" (p. 99) vs "previous cursor position" (p. 31) | Restores window and cursor before the last command that moved the window top; repeating toggles | `alt+a pgdn`, `alt+a ctrl+m` twice |
| MARK-2 | `markfile` lines `markername filename line column` (p. 63); bases and paths not stated | Line and column 0-based; relative names relative to the markfile's directory; malformed lines ignored | A markfile with a relative name and line 0, then `alt+a "name" ctrl+m` |
| RPL-1 | `Replace` prompts (p. 34) | Search prompt then replacement prompt, both prefilled, cursor at the start, editing with the text-argument keys; the cursor stays put; message `<n> replaced` | `ctrl+l`, edit both prompts, Enter |
| CMD-UNDO-1 | `modified` after undoing to the saved state | Stays modified | Edit, save, edit, `alt+bksp`, check status |
