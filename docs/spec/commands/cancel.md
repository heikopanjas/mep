# cancel

Default keys: Esc
Source: M guide p. 93 (Table A.3), p. 9 (Ch. 2.3.4), p. 23 (Figure 3.2 screen text "Argument cancelled"), p. 34 (Qreplace terminated by Cancel), p. 50 (Table 6.1); MSJ `bigprint`; IMPLEMENTATION_PLAN.md §3.5, §6.5

## Flags
Special: the dispatcher does not classify or reject any argument for `cancel`; it discards whatever is pending (nullarg, text, numarg, any selection). No NUMARG conversion. Not KEEPMETA (see Open points). Not CURSORFUNC, not MODIFIES. `Arg Arg` makes no difference.

## Behavior

### noarg (nothing pending)
"Cancels the current operation in progress" (p. 93). With no argument and no operation running, nothing changes except that meta is cleared. Returns true (Table 6.1 p. 50: always).

### Any argument pending (`Arg … Cancel`)
Ends arg mode, removes the anchor highlight and the selection, discards the text argument (it does not become the "previous text argument" for `lasttext`, see lasttext.md). Cursor stays where it is (it is not moved back to the anchor). Dialog line shows `Argument cancelled` (M guide p. 23, Figure 3.2). Returns true.

### Operation in progress
A suspended or sliced operation (file load, save, clipboard read, search, plan §3.5) is aborted; the interrupted command finishes with false. `qreplace` (M2) stops at its prompt (p. 34). A pending prompt (create file? confirm refresh?) is answered "no". Returns true.

### meta
Clears meta. See Open points.

## Rejected arguments
None.

## Open points
- Whether `cancel` also clears a pending meta is not stated. Project choice: yes, meta is cleared (cancel is not KEEPMETA, so the general rule of plan §6.6 applies). ORACLE CMD-CANCEL-1.
- Cursor after cancelling a selection: the manual does not say; we leave the cursor where it is. Covered by CMD-CANCEL-1 as a second question.
- The Z guide (`<cancel>` entry) says cancel "never takes an argument nor a mode-command modifier"; consistent with the above.

## Test checklist
- `cancel` with nothing pending: returns true, no message, no change.
- `arg cancel`: arg mode off, message `Argument cancelled`, returns true.
- `arg "abc" cancel`: text discarded; `lasttext` afterwards does not recall `abc`.
- `arg down down cancel`: highlight gone, cursor stays two lines down.
- `meta cancel`: meta cleared (ORACLE CMD-CANCEL-1).
- `cancel` during a held file load (`hold` directive): load aborted, the `setfile` returns false, editor stays on the old file.
- `cancel` during a long search: search stops, cursor unchanged, `psearch` returns false.
- `cancel` at a confirmation prompt (`refresh`): treated as "no".
