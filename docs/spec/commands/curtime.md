# curtime

Default keys: none
Source: M guide p. 96 (Table A.3 "13:45:55"), p. 88 (Table A.1), p. 30 (Ch. 4.2.3); IMPLEMENTATION_PLAN.md §4.2, §6.8

## Flags
NOARG, MODIFIES (single syntax line in Table A.3, no argument). No meta form; not KEEPMETA, so meta is ignored and cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Inserts the current local time (from `Env::now`) as `HH:MM:SS`, 24-hour clock, each field two digits zero-padded (p. 96 example `13:45:55`).

The text is always inserted at the cursor, also in overtype mode. If the cursor is beyond the end of the line, the line is padded with spaces first. The cursor moves to just after the inserted text. ORACLE CMD-CURTEXT-1. Read-only file: plan §5.6.

Returns true (project choice).

## Rejected arguments
Any argument → `invalid argument`, false.

## Open points
- Zero padding of the hour (`09:05:03` vs `9:05:03`) is not shown by the example; we pad. Covered by ORACLE CMD-CURDATE-1 (formats).
- Insert vs overtype: ORACLE CMD-CURTEXT-1.

## Test checklist
- 13:45:55 → `13:45:55`.
- 09:05:03 → `09:05:03`; 00:00:00 → `00:00:00`.
- `arg curtime` → `invalid argument`.
- `meta curtime` → inserted, meta cleared.
