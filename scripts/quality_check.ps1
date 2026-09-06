<#
.SYNOPSIS
    Quality Assurance Check Script for SubX (PowerShell port of quality_check.sh).

.DESCRIPTION
    PowerShell port of scripts/quality_check.sh for Windows CI runners.

    Performs comprehensive code quality checks: compilation, formatting,
    Clippy linting, documentation generation/examples/coverage, unit tests,
    and integration tests. Behaviour, CLI surface, environment, and exit
    codes mirror the Bash version so the two scripts are interchangeable
    from a CI workflow perspective.

    Copyright (C) 2025 陳鈞

    This program is free software: you can redistribute it and/or modify it
    under the terms of the GNU General Public License as published by the
    Free Software Foundation, either version 3 of the License, or (at your
    option) any later version.

    This program is distributed in the hope that it will be useful, but
    WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
    for more details.

    You should have received a copy of the GNU General Public License along
    with this program. If not, see <https://www.gnu.org/licenses/>.

.PARAMETER VerboseOutput
    Show verbose output (full command output) instead of capturing it and
    only printing on failure.

.PARAMETER Profile
    nextest profile to use. One of: default, ci, quick, full. Defaults to
    `default`.

.PARAMETER Full
    Run the full test suite, including slow tests gated by the `slow-tests`
    cargo feature. Forces `-Profile full` (matches the Bash version).

.PARAMETER CheckSpecGovernance
    Run only the spec-governance drift check (the three predicates over the
    two OpenSpec trees and the shared AGENTS.md conventions region) and skip
    all Cargo checks. Exits 2 on any governance failure, mirroring the Bash
    script's `--check-spec-governance` flag.

.EXAMPLE
    pwsh ./scripts/quality_check.ps1 -VerboseOutput -Profile ci -Full

.EXAMPLE
    pwsh ./scripts/quality_check.ps1 -CheckSpecGovernance

.NOTES
    Mirrors scripts/quality_check.sh (the canonical Bash version).
#>

[CmdletBinding()]
param(
    [switch]$VerboseOutput,

    [ValidateSet('default', 'ci', 'quick', 'full')]
    [string]$Profile = 'default',

    [switch]$Full,

    [switch]$CheckSpecGovernance
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

# Move to repository root, mirroring the `cd "$PROJECT_ROOT"` line in the
# Bash version. PSScriptRoot is the directory containing this .ps1 file.
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $ProjectRoot

if ($Full) {
    # Match the Bash version: --full unconditionally pins profile to "full".
    $Profile = 'full'
}

# -----------------------------------------------------------------------------
# Pretty-printing helpers
# -----------------------------------------------------------------------------

function Write-Info    { param([string]$Message) Write-Host $Message -ForegroundColor Cyan }
function Write-Success { param([string]$Message) Write-Host $Message -ForegroundColor Green }
function Write-WarnMsg { param([string]$Message) Write-Host $Message -ForegroundColor Yellow }
function Write-ErrMsg  { param([string]$Message) Write-Host $Message -ForegroundColor Red }

# -----------------------------------------------------------------------------
# Counters
# -----------------------------------------------------------------------------

$script:TotalChecks  = 0
$script:PassedChecks = 0
$script:FailedChecks = 0

# Governance failure state: any failed predicate makes the script exit 2
# (distinct from the generic failure exit 1) so a caller can tell a
# spec/document governance defect apart from a code-quality failure.
$script:GovernanceFailed = $false

function Extract-SharedRegion {
    <#
    Extracts the SHARED_CONVENTIONS marker-inclusive region from a Markdown
    file and writes it LF-normalised to $OutPath (mirrors extract_shared_region
    in quality_check.sh: the region lines are re-joined with `n so the
    comparison is content-based, not dependent on each checkout's line-ending
    policy). Requires exactly one start marker and exactly one end marker line,
    in order.
    #>
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$OutPath
    )
    # .NET file APIs resolve relative paths against the process CWD, not the
    # PowerShell location set by Set-Location — resolve explicitly.
    $resolved = (Resolve-Path -LiteralPath $Path).Path
    $lines = [System.IO.File]::ReadAllLines($resolved)
    $starts = @($lines | Where-Object { $_ -eq '<!-- SHARED_CONVENTIONS_START -->' }).Count
    $ends   = @($lines | Where-Object { $_ -eq '<!-- SHARED_CONVENTIONS_END -->' }).Count
    if ($starts -ne 1 -or $ends -ne 1) {
        Write-ErrMsg "   ${Path}: expected exactly one SHARED_CONVENTIONS_START and one SHARED_CONVENTIONS_END marker, found ${starts} start / ${ends} end"
        return $false
    }
    $startIdx = [Array]::IndexOf($lines, '<!-- SHARED_CONVENTIONS_START -->')
    $endIdx   = [Array]::IndexOf($lines, '<!-- SHARED_CONVENTIONS_END -->')
    if ($endIdx -le $startIdx) {
        Write-ErrMsg "   ${Path}: SHARED_CONVENTIONS_END appears before SHARED_CONVENTIONS_START"
        return $false
    }
    $region = $lines[$startIdx..$endIdx] -join "`n"
    [System.IO.File]::WriteAllText($OutPath, $region, (New-Object System.Text.UTF8Encoding($false)))
    return $true
}

function Test-SpecGovernance {
    <#
    Spec-governance drift check (spec-governance capability; design.md
    Decision 7 of refresh-docs-for-two-crate-architecture). Three predicates,
    mirroring check_spec_governance in quality_check.sh:
      1. No capability directory name appears in both openspec/specs/ and
         subx-core/openspec/specs/ unless recorded in
         openspec/split-capabilities.txt.
      2. Every recorded split name exists in BOTH trees.
      3. The SHARED_CONVENTIONS region of AGENTS.md and subx-core/AGENTS.md
         is identical (marker-inclusive; the tool-managed CODEGRAPH block
         lives outside the region in both files).
    #>
    $rootSpecs = 'openspec/specs'
    $coreSpecs = 'subx-core/openspec/specs'
    $record    = 'openspec/split-capabilities.txt'
    $ok = $true

    # A missing submodule already fails the build at manifest-parse time;
    # skip with a warning rather than adding a second, vaguer failure.
    if (-not (Test-Path -LiteralPath $coreSpecs -PathType Container)) {
        Write-WarnMsg "⚠️  Spec Governance Check: Skipped — ${coreSpecs}/ not found (submodule not initialised?)"
        return $true
    }
    if (-not (Test-Path -LiteralPath $record -PathType Leaf)) {
        Write-ErrMsg "❌ Spec Governance Check: ${record} not found"
        return $false
    }

    # Predicate 1: duplicated capability names must be recorded splits.
    $recorded = @(Get-Content -LiteralPath $record | Where-Object { $_ -ne '' })
    $coreDirs = @(Get-ChildItem -LiteralPath $coreSpecs -Directory | ForEach-Object { $_.Name })
    foreach ($name in $coreDirs) {
        if ((Test-Path -LiteralPath (Join-Path $rootSpecs $name) -PathType Container) -and
            ($recorded -notcontains $name)) {
            $ok = $false
            Write-ErrMsg "❌ Predicate 1: capability present in both OpenSpec trees but not recorded in ${record}:"
            Write-ErrMsg "   ${name}"
        }
    }

    # Predicate 2: every recorded split exists in both trees.
    foreach ($name in $recorded) {
        $missing = ''
        if (-not (Test-Path -LiteralPath (Join-Path $rootSpecs $name) -PathType Container)) { $missing += " ${rootSpecs}" }
        if (-not (Test-Path -LiteralPath (Join-Path $coreSpecs $name) -PathType Container)) { $missing += " ${coreSpecs}" }
        if ($missing -ne '') {
            $ok = $false
            Write-ErrMsg "❌ Predicate 2: recorded split '${name}' is missing from:${missing}"
        }
    }

    # Predicate 3: identical shared conventions region (raw-byte compare of
    # LF-normalised extractions; line diff only as diagnostics).
    $regionRoot = New-TemporaryFile
    $regionCore = New-TemporaryFile
    try {
        $exRoot = Extract-SharedRegion -Path 'AGENTS.md' -OutPath $regionRoot.FullName
        $exCore = Extract-SharedRegion -Path 'subx-core/AGENTS.md' -OutPath $regionCore.FullName
        if ($exRoot -and $exCore) {
            $bytesRoot = [System.IO.File]::ReadAllBytes($regionRoot.FullName)
            $bytesCore = [System.IO.File]::ReadAllBytes($regionCore.FullName)
            if (-not [System.Linq.Enumerable]::SequenceEqual($bytesRoot, $bytesCore)) {
                $ok = $false
                Write-ErrMsg '❌ Predicate 3: shared conventions region differs between:'
                Write-ErrMsg '   AGENTS.md and subx-core/AGENTS.md — diff of the extracted regions:'
                $lr = [System.IO.File]::ReadAllLines($regionRoot.FullName)
                $lc = [System.IO.File]::ReadAllLines($regionCore.FullName)
                $max = [Math]::Max($lr.Count, $lc.Count)
                for ($i = 0; $i -lt $max; $i++) {
                    $a = if ($i -lt $lr.Count) { $lr[$i] } else { $null }
                    $b = if ($i -lt $lc.Count) { $lc[$i] } else { $null }
                    if ($a -cne $b) {
                        Write-Host ("- {0}" -f $a)
                        Write-Host ("+ {0}" -f $b)
                    }
                }
            }
        } else {
            $ok = $false
            Write-ErrMsg '❌ Predicate 3: SHARED_CONVENTIONS markers malformed in AGENTS.md and/or subx-core/AGENTS.md'
        }
    } finally {
        Remove-Item -LiteralPath $regionRoot.FullName -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $regionCore.FullName -ErrorAction SilentlyContinue
    }

    return $ok
}

function Show-CheckResult {
    param(
        [Parameter(Mandatory)][int]$ExitCode,
        [Parameter(Mandatory)][string]$Name
    )
    if ($ExitCode -eq 0) {
        Write-Success "✅ $Name`: Passed"
        return $true
    } else {
        Write-ErrMsg "❌ $Name`: Failed"
        return $false
    }
}

function Invoke-Check {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Command
    )
    $script:TotalChecks++

    if ($VerboseOutput) {
        Write-Info "`n🔍 Running check: $Name"
    }

    & $Command
    $exit = $LASTEXITCODE
    if ($null -eq $exit) { $exit = 0 }

    if (Show-CheckResult -ExitCode $exit -Name $Name) {
        $script:PassedChecks++
        return $true
    } else {
        $script:FailedChecks++
        return $false
    }
}

function Invoke-TestCheck {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Command
    )
    $script:TotalChecks++

    if ($VerboseOutput) {
        Write-Info "`n🔍 Running check: $Name"
        & $Command
        $exit = $LASTEXITCODE
        if ($null -eq $exit) { $exit = 0 }
        if (Show-CheckResult -ExitCode $exit -Name $Name) {
            $script:PassedChecks++
            return $true
        } else {
            $script:FailedChecks++
            return $false
        }
    }

    $tmp = New-TemporaryFile
    try {
        & $Command *> $tmp.FullName
        $exit = $LASTEXITCODE
        if ($null -eq $exit) { $exit = 0 }

        if ($exit -eq 0) {
            Show-CheckResult -ExitCode 0 -Name $Name | Out-Null
            $script:PassedChecks++
            return $true
        } else {
            Show-CheckResult -ExitCode $exit -Name $Name | Out-Null
            Write-Host ''
            Write-Host '=== Test Output ==='
            Get-Content -LiteralPath $tmp.FullName | ForEach-Object { Write-Host $_ }
            Write-Host '==================='
            $script:FailedChecks++
            return $false
        }
    } finally {
        Remove-Item -LiteralPath $tmp.FullName -ErrorAction SilentlyContinue
    }
}

# -----------------------------------------------------------------------------
# Main flow
# -----------------------------------------------------------------------------

# Standalone governance mode: run only the drift predicates.
if ($CheckSpecGovernance) {
    Write-Host '🔍 SubX Spec-Governance Drift Check Starting...'
    Write-Host '========================================'
    if (Test-SpecGovernance) {
        Write-Success "`n🎉 Spec-governance drift check passed!"
        exit 0
    } else {
        Write-ErrMsg "`n⚠️  Spec-governance drift check FAILED (exit 2)"
        exit 2
    }
}

Write-Host '🔍 SubX Quality Assurance Check Starting...'
Write-Host '========================================'
Write-Host "🔧 Using nextest profile: $Profile"
if ($Full) {
    Write-Host '⚡ Full tests mode: Including slow tests (~143s vs ~90s)'
} else {
    Write-Host '🚀 Fast tests mode: Excluding slow tests (~90s vs ~143s)'
    Write-Host '   Use -Full to include slow tests'
}
Write-Host '========================================'

$cargoFeatureArgs = @()
if ($Full) {
    $cargoFeatureArgs = @('--features', 'slow-tests')
}

# 1. Code compilation check
if ($VerboseOutput) {
    Invoke-Check -Name 'Code Compilation Check' -Command {
        & cargo check --workspace --all-features @cargoFeatureArgs
    } | Out-Null
} else {
    Invoke-Check -Name 'Code Compilation Check' -Command {
        & cargo check --workspace --all-features @cargoFeatureArgs --quiet
    } | Out-Null
}

# 2. Code formatting check
Invoke-Check -Name 'Code Formatting Check' -Command {
    & cargo fmt -- --check
} | Out-Null

# 3. Clippy linting check
if ($VerboseOutput) {
    Invoke-Check -Name 'Clippy Code Quality Check' -Command {
        & cargo clippy --workspace --all-features @cargoFeatureArgs -- -D warnings
    } | Out-Null
} else {
    Invoke-Check -Name 'Clippy Code Quality Check' -Command {
        & cargo clippy --workspace --all-features @cargoFeatureArgs --quiet -- -D warnings
    } | Out-Null
}

# 4. Documentation generation check
if ($VerboseOutput) {
    Write-Info "`n🔍 Running check: Documentation Generation Check"
}
$script:TotalChecks++

$docOut = New-TemporaryFile
try {
    if ($VerboseOutput) {
        & cargo doc --workspace --all-features @cargoFeatureArgs --no-deps --document-private-items 2>&1 |
            Tee-Object -FilePath $docOut.FullName | ForEach-Object { Write-Host $_ }
    } else {
        & cargo doc --workspace --all-features @cargoFeatureArgs --no-deps --document-private-items *> $docOut.FullName
    }

    $docLines = Get-Content -LiteralPath $docOut.FullName

    # Match the Bash filter: only lines that start with a genuine rustc/cargo
    # error line (error[...] or error: ...), excluding the known
    # `warning[E0602]: unknown lint` noise. Substring matches such as the
    # `proc-macro-error2` future-incompat warning must not trigger a failure.
    $criticalErrors = $docLines | Where-Object {
        ($_ -match '^error(\[|:)') -and ($_ -notmatch 'warning\[E0602\]: unknown lint')
    }

    if ($criticalErrors) {
        Write-ErrMsg '❌ Documentation Generation Check: Critical errors found'
        $script:FailedChecks++
    } else {
        $warnings = $docLines | Where-Object {
            ($_ -match 'warning') -and ($_ -notmatch 'warning\[E0602\]: unknown lint')
        }
        $warningCount = if ($warnings) { @($warnings).Count } else { 0 }
        if ($warningCount -gt 0) {
            Write-WarnMsg "⚠️  Documentation Generation Check: Passed (with $warningCount warnings)"
        } else {
            Write-Success '✅ Documentation Generation Check: Passed'
        }
        $script:PassedChecks++
    }
} finally {
    Remove-Item -LiteralPath $docOut.FullName -ErrorAction SilentlyContinue
}

# 5. Documentation examples test
Invoke-TestCheck -Name 'Documentation Examples Test' -Command {
    & cargo test --doc --all-features @cargoFeatureArgs
} | Out-Null

# 6. Documentation coverage check
if ($VerboseOutput) {
    Write-Info "`n🔍 Running check: Documentation Coverage Check"
}
$script:TotalChecks++

if ($VerboseOutput) {
    $clippyOut = & cargo clippy --all-features @cargoFeatureArgs -- -W missing_docs 2>&1
} else {
    $clippyOut = & cargo clippy --all-features @cargoFeatureArgs --quiet -- -W missing_docs 2>&1
}
$missingDocs = $clippyOut | Where-Object {
    ($_ -match 'missing documentation') -and ($_ -notmatch 'warning\[E0602\]')
}

if ($missingDocs) {
    $missingCount = @($missingDocs).Count
    Write-WarnMsg "⚠️  Documentation Coverage Check: Found $missingCount items missing documentation"
    if ($VerboseOutput) {
        $missingDocs | Select-Object -First 5 | ForEach-Object { Write-Host $_ }
        if ($missingCount -gt 5) {
            Write-Host "... (showing first 5 of $missingCount items)"
        }
        Write-Info 'ℹ️  These are improvement suggestions and won''t affect build success'
    }
} else {
    Write-Success '✅ Documentation Coverage Check: All public APIs have documentation'
}
$script:PassedChecks++

# 7. Unit tests
$nextestFeatureArgs = @()
if ($Full) {
    $nextestFeatureArgs = @('--features', 'slow-tests')
}

Invoke-TestCheck -Name 'Unit Tests' -Command {
    & cargo nextest run --workspace --profile $Profile @nextestFeatureArgs -E 'kind(lib)' --ignore-default-filter
} | Out-Null

# 8. Integration tests
Invoke-TestCheck -Name 'Integration Tests' -Command {
    & cargo nextest run --workspace --profile $Profile @nextestFeatureArgs --ignore-default-filter
} | Out-Null

# 9. Spec-governance drift check (two OpenSpec trees + shared AGENTS.md region)
$script:TotalChecks++
if (Test-SpecGovernance) {
    Write-Success '✅ Spec Governance Check: Passed'
    $script:PassedChecks++
} else {
    $script:FailedChecks++
    $script:GovernanceFailed = $true
}

# Summary
Write-Host ''
Write-Host '========================================'
Write-Info '📊 Quality Assurance Check Summary'
Write-Host '========================================'
Write-Success "✅ Passed checks: $script:PassedChecks"
Write-ErrMsg "❌ Failed checks: $script:FailedChecks"
Write-Info "📋 Total checks: $script:TotalChecks"

if ($script:FailedChecks -eq 0) {
    Write-Success "`n🎉 All quality assurance checks passed!"
    exit 0
} elseif ($script:GovernanceFailed) {
    Write-ErrMsg "`n⚠️  Spec-governance predicates failed — see 'Spec Governance Check' output above (exit 2)"
    exit 2
} else {
    Write-ErrMsg "`n⚠️  Some checks failed, please review the error messages above"
    exit 1
}
