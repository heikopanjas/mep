# unassigned

Default keys: every key that has no other default binding (plan §6.8; MSJ "The Keyboard")
Source: M guide pp. 47–48 (Ch. 6.2.3), p. 22 (Figure 3.1 screen text), MSJ Fig. 1 ("Print error message"); IMPLEMENTATION_PLAN.md §6.8

## Flags
- KEEPMETA and no arg-kind flags: the pending argument is not consumed. Evidence: the screen on M guide p. 22 shows a streamarg still highlighted together with the message `alt+p is not assigned to any editor function`.
- Not CURSORFUNC, not MODIFIES.

## Behavior

### Any state (noarg, meta, any argument pending)
Shows `<key> is not assigned to any editor function` on the dialog line, where `<key>` is the key name in the parser's lowercase spelling (`alt+p`, `ctrl+a`, `shift+f5`, M guide p. 22). Nothing else changes: arg mode, count, anchor, selection, text argument and meta stay as they were. Cursor does not move.

Returns false (the key did nothing). ORACLE CMD-UNASSIGNED-1.

### Called by name (macro, `execute`)
There is no key. The message uses the key that started the current top-level command; if there is none (`-e` at startup), the message is `unassigned is not assigned to any editor function`. ORACLE CMD-UNASSIGNED-1.

### Assignments
`unassigned:key` in TOOLS.INI or with `assign` removes a key's binding (M guide p. 47).

## Rejected arguments
None; an argument stays pending.

## Open points
- Return value: not in Table 6.1. We return false. Also the message when invoked from a macro. ORACLE CMD-UNASSIGNED-1.

## Test checklist
- Press an unbound key (`alt+p`): message `alt+p is not assigned to any editor function`, returns false, nothing changes.
- `arg right right alt+p`: message shown, selection still highlighted; `sdelete` afterwards deletes the selection.
- `arg "abc" alt+p`: text still pending.
- `meta alt+p`: meta still set.
- `arg "unassigned:ctrl+a" assign`, then ctrl+a → message with `ctrl+a`.
- Macro `x:=unassigned` run from a key → message names that key.
