# Recent Updates & Decisions

This file is the append-only log of project decisions and notable changes, maintained by coding agents following the `recent-updates` skill. Everything below the marker line is user-owned history: slopctl never overwrites it during init or merge.

<!-- {changelog} -->

### 2026-10-31 (v0.1.0, branch coverage gate, cli parser in mep)

- the coverage gate also requires 100% branch coverage; the check scripts collect with --branch and fail when the json summary reports any uncovered branch
- test modules are excluded from coverage with coverage(off), enabled through a test-only coverage_attribute feature gate
- bundled short command-line flags are accepted
- the cli parser moves from mep-core to the mep binary crate, so clap stays out of the core
- rationale: owner decisions; cargo-llvm-cov has no branch threshold option, and assertions in tests add branches a passing test never takes
- version bump: none (build configuration and plan only)

### 2026-10-31 (v0.1.0, mit license, pinned nightly, dash-only cli)

- licensed the project under mit with a LICENSE file and license metadata in all crates
- pinned rust-toolchain.toml to nightly-2026-10-31
- command-line options use only - and -- on all systems; the original slash forms are dropped and a slash argument is always a file
- updated implementation_plan.md for the nightly pin, the license, anyhow and clap in the allowed crates, and the dash-only cli
- rationale: owner decisions; a dated nightly keeps ci from breaking on a new nightly without a code change
- version bump: none (license and build configuration only)

### 2026-10-31 (v0.1.0, nightly toolchain and anyhow clap)

- switched rust-toolchain.toml to nightly with clippy, llvm-tools-preview and rustfmt; check scripts and ci use the toolchain file only
- added anyhow and clap to the allowed crates
- check.ps1 calls cargo directly with a quoted double dash and checks the exit code after each step
- rationale: owner decision; nightly also makes .rustfmt.toml effective without a separate fmt toolchain
- rationale: switching toolchains left stale llvm-cov data that produced bogus reports; a full cargo llvm-cov clean fixes it
- version bump: none (build configuration only)

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
