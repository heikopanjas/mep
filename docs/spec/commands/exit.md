# exit

Default keys: F8
Source: M guide p. 97 (Table A.3), p. 12 (Ch. 2.3.7), p. 13 (several files on the command line), p. 51 (Table 6.1 "No return condition"), p. 61 (`askexit`); MSJ Fig. 5 (meta exit); z-vs-mep.md S2; IMPLEMENTATION_PLAN.md §6.1, §6.10

## Flags
From p. 97:
- `Exit` → NOARG.
- `Meta Exit` → meta form.
- `Arg Exit` → NULLARG.
- `Arg Meta Exit` → NULLARG with meta.
- No text or selection forms. Not KEEPMETA. `Arg Arg` behaves like `Arg`.

## Behavior
MEP semantics, divergence S2 (Z swaps the meaning of `exit` and `arg exit`). "The file list" is the list of files from the command line (plan §6.1); "advance" means loading the next file of that list after the current one (I/O request, plan §3.5); a missing next file gets the create prompt of p. 13.

Saving the current file: only if it is modified; a pseudo-file is never saved. An untitled modified buffer cannot be saved: message (use `arg arg "name" setfile` or `meta exit`), nothing else happens, false. A save failure: message, no advance, no quit, false.

Quitting: with `askexit` on, the dialog line asks for confirmation first; anything but `y` cancels (false). If other files in memory are modified (and would be lost), the dialog line asks for confirmation as well. ORACLE CMD-EXIT-1. Then the editor quits (`Outcome::Quit`).

Return value: Table 6.1 says "No return condition". Macros must not branch on it. When the editor quits nothing runs afterwards; when it advances it returns true; when it stops (save failed, prompt declined) it returns false.

### noarg
Saves the current file (if modified). If there is a next file in the list, advances to it; otherwise quits (p. 97, p. 13).

### meta (`Meta Exit`)
Like `exit` but the current file is not saved (p. 97; MSJ "immediately exits the editor without saving any changes"). Advances or quits.

### nullarg (`Arg Exit`)
Saves the current file and quits without advancing, even if files remain in the list (p. 97).

### `Arg Meta Exit`
Quits without advancing and without saving the current file (p. 97).

## Rejected arguments
textarg, numarg, streamarg, linearg, boxarg → `invalid argument`, false.

## Open points
- Other modified files at quit time: the M guide does not say; Z asks for confirmation. We ask. ORACLE CMD-EXIT-1.
- Exit with an untitled modified buffer: message and no quit (project decision).

## Test checklist
- One file, modified, `exit` → saved, quit.
- Unmodified → quit without writing.
- `mep a b c`: `exit` in a → a saved, b loaded, true; in c → quit.
- `arg exit` in a → a saved, quit (b, c never loaded).
- `meta exit` in a (modified) → a not saved, b loaded.
- `arg meta exit` → quit, nothing saved.
- `askexit:` → prompt; `n` → false, still running; `y` → quit.
- Save failure (fake `Fs`) → message, no advance, false.
- Next file missing → create prompt.
- Untitled modified buffer → message, false, still running; `meta exit` quits.
- Another modified file in memory (autosave off) → confirmation (ORACLE CMD-EXIT-1).
- `<clipboard>` current and modified → not saved, quit.
- `arg "x" exit` → `invalid argument`, false.
