#!/bin/sh
# Runs fmt, clippy, tests and the 100% coverage gate (lines, functions, regions, branches) (IMPLEMENTATION_PLAN.md §9.1).
#
# Usage: scripts/check.sh [--report]
#   --report  also write an HTML coverage report to target/llvm-cov/html

set -eu

report=false
for arg in "$@"; do
    case "$arg" in
        --report) report=true ;;
        *)
            echo "usage: scripts/check.sh [--report]" >&2
            exit 2
            ;;
    esac
done

cd "$(dirname "$0")/.."

command -v jq >/dev/null || { echo "error: jq is required" >&2; exit 1; }

# The only files excluded from coverage: OS glue without logic (§7).
ignore='(mep-term[/\\]src[/\\]sys\.rs|mep-clip[/\\]src[/\\]system\.rs|mep[/\\]src[/\\]main\.rs|mep[/\\]src[/\\]sys\.rs)$'

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --doc --locked --no-fail-fast
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --branch --locked --no-report --no-fail-fast
cargo llvm-cov report --branch --ignore-filename-regex "$ignore" \
    --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
# cargo-llvm-cov has no branch threshold; read the missed branches from the summary.
missed=$(cargo llvm-cov report --branch --json --summary-only --ignore-filename-regex "$ignore" |
    jq '.data[0].totals.branches.notcovered')
if [ "$missed" != 0 ]; then
    echo "error: $missed branches not covered" >&2
    exit 1
fi
if [ "$report" = true ]; then
    cargo llvm-cov report --branch --ignore-filename-regex "$ignore" --html
fi
