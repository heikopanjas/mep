# Runs fmt, clippy, tests and the 100% coverage gate (IMPLEMENTATION_PLAN.md §9.1).
#
# Usage: scripts/check.ps1 [--report]
#   --report  also write an HTML coverage report to target/llvm-cov/html
#
# Formatting uses nightly rustfmt because .rustfmt.toml relies on options that
# stable rustfmt ignores. Everything else runs on the stable toolchain.

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
$ignore = '(mep-term[/\\]src[/\\]sys\.rs|mep-clip[/\\]src[/\\]system\.rs|mep[/\\]src[/\\]main\.rs)$'

# '--' is quoted at call sites: PowerShell would otherwise swallow it as its own end-of-parameters marker.
function Invoke-Step
{
    & $args[0] $args[1..($args.Count - 1)]
    if ($LASTEXITCODE -ne 0)
    {
        exit $LASTEXITCODE
    }
}

Invoke-Step cargo +nightly fmt --all '--' --check
Invoke-Step cargo clippy --workspace --all-targets --locked '--' -D warnings
Invoke-Step cargo test --workspace --doc --locked
Invoke-Step cargo llvm-cov clean --workspace
Invoke-Step cargo llvm-cov --workspace --locked --no-report
Invoke-Step cargo llvm-cov report --ignore-filename-regex $ignore --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
if ($report)
{
    Invoke-Step cargo llvm-cov report --ignore-filename-regex $ignore --html
}
