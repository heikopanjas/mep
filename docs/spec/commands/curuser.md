# curuser

Default keys: none
Source: M guide p. 96 (Table A.3 "using the MAILNAME environment variable"), p. 30 (Ch. 4.2.3 "USER environment variable"), p. 88 (Table A.1); z-vs-mep.md T1, T4; IMPLEMENTATION_PLAN.md §4.2 (`Env::var`, `Env::os`), §6.7, §12

## Flags
NOARG, MODIFIES (single syntax line in Table A.3, no argument). No meta form; not KEEPMETA, so meta is ignored and cleared. `Arg Arg` does not apply.

## Behavior

### noarg
Project decision (IMPLEMENTATION_PLAN.md §6.7): inserts the value of `MAILNAME` if it is set and not empty; otherwise `USER` on macOS and Linux, `USERNAME` on Windows (read through `Env::var` and `Env::os`). The value is inserted as is (no trimming). Non-UTF-8 values: inserted byte-exact on Unix; on Windows the UTF-16 value is converted to UTF-8.

The text is always inserted at the cursor, also in overtype mode. If the cursor is beyond the end of the line, the line is padded with spaces first. The cursor moves to just after the inserted text. ORACLE CMD-CURTEXT-1. Read-only file: plan §5.6.

None of the variables set → nothing inserted, message (project wording, e.g. `user name not set`), false. Otherwise true.

## Rejected arguments
Any argument → `invalid argument`, false.

## Open points
- The manual contradicts itself: p. 30 says `USER`, p. 96 says `MAILNAME` (z-vs-mep.md T4). Resolved by the plan (above). `USER` here is the login name, not a directory list (T1).
- Insert vs overtype: ORACLE CMD-CURTEXT-1.

## Test checklist
- `MAILNAME=heiko` → `heiko` (also when `USER` is set).
- `MAILNAME` unset, `USER=bob`, macOS/Linux → `bob`.
- Windows, `USERNAME=alice` → `alice`; `USER` ignored on Windows.
- `MAILNAME` set but empty → fallback used.
- Nothing set → message, false, file unchanged.
- `arg curuser` → `invalid argument`.
- `meta curuser` → inserted, meta cleared.
