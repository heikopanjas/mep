# Recent Updates & Decisions

This file is the append-only log of project decisions and notable changes, maintained by coding agents following the `recent-updates` skill. Everything below the marker line is user-owned history: slopctl never overwrites it during init or merge.

<!-- {changelog} -->

### 2026-10-31 (v0.1.0, m1 task 1 workspace setup)

- created the cargo workspace with mep-core, mep-term, mep-clip and mep crates, a stable rust-toolchain.toml with clippy and llvm-tools-preview, and a committed Cargo.lock
- added scripts/check and scripts/check.ps1 running fmt, clippy, doc tests and the llvm-cov gate at 100% lines, functions and regions, plus a --report option for html output
- added a github actions workflow running the check script on ubuntu, macos and windows
- added docs/spec/switches.md, docs/oracle.md and tests/scenarios skeletons
- rationale: fmt runs on nightly rustfmt because stable rustfmt ignores brace_style, trailing_comma, imports_granularity and other .rustfmt.toml options
- rationale: the coverage gate fails when no function is instrumented, so mep-core ships a tested version function that the -V option will use
- rationale: anyhow and clap from the coding skill are not used because the plan limits crates to its allowed list
- version bump: none (0.1.0 is the first manifest version)

### 2026-10-31 (v0.1.0, fill agents.md placeholders)

- filled mission statement, technology stack, code review, development guidelines, security, testing and documentation sections of agents.md
- content taken from implementation_plan.md: clean-room rule, manual as spec, spec then tests then code, oracle items, core isolation behind traits, allowed crates, 100% coverage gate
- license left undecided; the plan calls the project private and names no license
- rationale: agents.md still held template placeholders while the plan had already settled these points
- version bump: none (documentation only, no crate exists yet)

### 2025-10-05 (v0.1.0, initial setup)

- initial AGENTS.md setup
- established core coding standards and conventions
- defined repository structure and governance principles
