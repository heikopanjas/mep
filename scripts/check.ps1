# Runs fmt, clippy, tests and the 100% coverage gate (lines, functions, regions, branches) (IMPLEMENTATION_PLAN.md §9.1).
#
# Usage: scripts/check.ps1 [--report]
#   --report  also write an HTML coverage report to target/llvm-cov/html

$ErrorActionPreference = 'Stop'

$report = $false
foreach ($arg in $args)
{
    if ($arg -eq '--report')
    {
        $report = $true
    }
    else
    {
        [Console]::Error.WriteLine('usage: scripts/check.ps1 [--report]')
        exit 2
    }
}

Set-Location (Join-Path $PSScriptRoot '..')

# The only files excluded from coverage: OS glue without logic (§7).
$ignore = '(mep-term[/\\]src[/\\]sys\.rs|mep-clip[/\\]src[/\\]system\.rs|mep[/\\]src[/\\]main\.rs|mep[/\\]src[/\\]sys\.rs)$'

# Native commands do not throw on failure; stop at the first non-zero exit code.
function Assert-Success
{
    if ($LASTEXITCODE -ne 0)
    {
        exit $LASTEXITCODE
    }
}

# '--' is quoted so PowerShell passes it to cargo instead of treating it as its own end-of-parameters marker.
cargo fmt --all '--' --check
Assert-Success
cargo clippy --workspace --all-targets --locked '--' -D warnings
Assert-Success
cargo test --workspace --doc --locked --no-fail-fast
Assert-Success
cargo llvm-cov clean --workspace
Assert-Success
cargo llvm-cov --workspace --branch --locked --no-report --no-fail-fast
Assert-Success
cargo llvm-cov report --branch --ignore-filename-regex $ignore --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
Assert-Success
# cargo-llvm-cov has no branch threshold; read the missed branches from the summary.
$summary = cargo llvm-cov report --branch --json --summary-only --ignore-filename-regex $ignore
Assert-Success
$missed = (($summary -join "`n") | ConvertFrom-Json).data[0].totals.branches.notcovered
if ($missed -ne 0)
{
    [Console]::Error.WriteLine("error: $missed branches not covered")
    exit 1
}
if ($report)
{
    cargo llvm-cov report --branch --ignore-filename-regex $ignore --html
    Assert-Success
}
