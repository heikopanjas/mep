# curday

Default keys: none
Source: M guide p. 96 (Table A.3 "Sun...Sat"), p. 88 (Table A.1), p. 30 (Ch. 4.2.3); IMPLEMENTATION_PLAN.md §4.2, §6.8

## Flags
NOARG, MODIFIES (single syntax line in Table A.3, no argument). No meta form; not KEEPMETA, so meta is ignored and cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Inserts the current local day of the week (from `Env::now`) as one of `Sun`, `Mon`, `Tue`, `Wed`, `Thu`, `Fri`, `Sat` (p. 96), English, independent of the locale.

The text is always inserted at the cursor, also in overtype mode. If the cursor is beyond the end of the line, the line is padded with spaces first. The cursor moves to just after the inserted text. ORACLE CMD-CURTEXT-1. Read-only file: plan §5.6.

Returns true (project choice; not in Table 6.1).

## Rejected arguments
Any argument → `invalid argument`, false.

## Open points
- Insert vs overtype: ORACLE CMD-CURTEXT-1.

## Test checklist
- 1987-06-27 (a Saturday) → `Sat`.
- Each weekday mapped correctly (seven fixed dates).
- Overtype mode → inserted (ORACLE CMD-CURTEXT-1).
- `arg curday` → `invalid argument`.
- `meta curday` → inserted, meta cleared.
