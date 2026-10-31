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
