# lasttext

Default keys: Ctrl+O
Source: M guide p. 98 (Table A.3), p. 89 (Table A.1), p. 19 (text editing, `right` past the end), p. 51 (Table 6.1); MSJ Fig. 1 ("Redisplay previous text arg for editing"); IMPLEMENTATION_PLAN.md §6.5

## Flags
- NOARG: `Lasttext` is the only syntax line.
- KEEPMETA: the function "is the same as invoking the *Arg* function and then retyping the previous *textarg*" (p. 98); `arg` and typed text both keep meta, so `lasttext` does too.
- With an argument already pending it is not rejected outright; see Behavior.
- Not CURSORFUNC, not MODIFIES.

## Behavior

### The text buffer
"The previous text argument" is the last text that was typed or edited on the dialog line (including text from macro strings in arg mode and text recalled by `lasttext`) and then passed to a function. Texts that come from selections, NULLEOL/NULLEOW or BOXSTR are not recorded. A cancelled text argument is not recorded. ORACLE CMD-LASTTEXT-1.

### noarg (arg mode off)
Enters arg mode with the anchor at the cursor, count 1, and the previous text on the dialog line (`Arg: text`), text cursor at its end, ready for editing or for the next function. Returns true.

If there is no previous text: arg mode starts with an empty text, the same as `arg`. Returns true.

### arg mode on, no text and no movement yet (`Arg Lasttext`, `Arg Arg Lasttext`)
Keeps the current count and fills in the previous text, so `arg arg lasttext psearch` repeats the last search as a regular expression (M2). Returns true. ORACLE CMD-LASTTEXT-1.

### text already being edited
Replaces the text with the previous text. Count unchanged. Returns true.

### selection pending
Not a valid combination: `invalid argument`, false, arg mode ends.

### meta
Meta stays set and reaches the function that consumes the recalled text.

## Rejected arguments
Any pending selection (streamarg, linearg, boxarg shape) → `invalid argument`, false.

## Open points
- Which texts update the buffer, and whether `arg arg lasttext` keeps count 2 or adds another arg (the manual's literal "same as invoking Arg" would give count 3). ORACLE CMD-LASTTEXT-1; we keep the count.
- Table 6.1 lists "Bad argument" as the false case without saying what is bad; we use the pending selection.

## Test checklist
- `arg "abc" psearch`, then `lasttext psearch` → searches `abc` again.
- `lasttext` shows `Arg: abc` on the dialog line, text cursor at the end; `cdelete` edits it.
- No previous text: `lasttext` → arg mode with empty text, true.
- `arg arg lasttext` → count 2 kept (ORACLE CMD-LASTTEXT-1).
- `arg "x"` then `lasttext` → text replaced by the previous text.
- `arg right lasttext` → `invalid argument`, false.
- Text from a streamarg `arg right right psearch` is not recorded; `lasttext` still recalls the older typed text.
- `arg "abc" cancel`, `lasttext` → recalls the text before `abc`.
- `meta lasttext setfile` → meta reaches `setfile` (no save).
