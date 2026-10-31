# curdate

Default keys: none
Source: M guide p. 95 (Table A.3 "Jun-27-1987"), p. 88 (Table A.1 "27-Jun-1987"), p. 30 (Ch. 4.2.3); IMPLEMENTATION_PLAN.md §4.2 (`Env::now`), §6.8

## Flags
NOARG, MODIFIES (single syntax line in Table A.3, no argument). No meta form; not KEEPMETA, so meta is ignored and cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Inserts the current local date (from `Env::now`, never the system clock directly, plan §4.2) at the cursor in the format of Table A.3: `Mon-DD-YYYY`, for example `Jun-27-1987`. Month: English three-letter abbreviation `Jan` … `Dec`, independent of the locale. Day: two digits, zero-padded. Year: four digits. ORACLE CMD-CURDATE-1.

The text is always inserted at the cursor, also in overtype mode. If the cursor is beyond the end of the line, the line is padded with spaces first. The cursor moves to just after the inserted text. ORACLE CMD-CURTEXT-1. Read-only file: plan §5.6.

Returns true. Not in Table 6.1; project choice.

## Rejected arguments
Any argument (nullarg, text, selection) → `invalid argument`, false.

## Open points
- Format: Table A.3 (p. 95) says `Jun-27-1987`; Table A.1 (p. 88) says `27-Jun-1987`. We implement Table A.3, the detailed listing. Zero padding of the day is not shown by either example. ORACLE CMD-CURDATE-1.
- Insert vs overtype and cursor position after the insert: ORACLE CMD-CURTEXT-1.

## Test checklist
- Fixed clock 1987-06-27 → `Jun-27-1987` inserted at the cursor, cursor after it, true.
- 2026-01-05 → `Jan-05-2026` (ORACLE CMD-CURDATE-1).
- Overtype mode, cursor inside `xxxxxxxxxxxx` → text inserted, not overwritten (ORACLE CMD-CURTEXT-1).
- Cursor beyond the end of the line → padded.
- `arg curdate` → `invalid argument`.
- `meta curdate` → inserted, meta cleared.
- `undo` removes it.
