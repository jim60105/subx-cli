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

.EXAMPLE
    pwsh ./scripts/quality_check.ps1 -VerboseOutput -Profile ci -Full

.NOTES
    Mirrors scripts/quality_check.sh (the canonical Bash version).
#>

[CmdletBinding()]
param(
    [switch]$VerboseOutput,

    [ValidateSet('default', 'ci', 'quick', 'full')]
    [string]$Profile = 'default',

    [switch]$Full
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
        & cargo check --all-features @cargoFeatureArgs
    } | Out-Null
} else {
    Invoke-Check -Name 'Code Compilation Check' -Command {
        & cargo check --all-features @cargoFeatureArgs --quiet
    } | Out-Null
}

# 2. Code formatting check
Invoke-Check -Name 'Code Formatting Check' -Command {
    & cargo fmt -- --check
} | Out-Null

# 3. Clippy linting check
if ($VerboseOutput) {
    Invoke-Check -Name 'Clippy Code Quality Check' -Command {
        & cargo clippy --all-features @cargoFeatureArgs -- -D warnings
    } | Out-Null
} else {
    Invoke-Check -Name 'Clippy Code Quality Check' -Command {
        & cargo clippy --all-features @cargoFeatureArgs --quiet -- -D warnings
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
        & cargo doc --all-features @cargoFeatureArgs --no-deps --document-private-items 2>&1 |
            Tee-Object -FilePath $docOut.FullName | ForEach-Object { Write-Host $_ }
    } else {
        & cargo doc --all-features @cargoFeatureArgs --no-deps --document-private-items *> $docOut.FullName
    }

    $docLines = Get-Content -LiteralPath $docOut.FullName

    # Match the Bash filter: lines containing "error", excluding the known
    # `warning[E0602]: unknown lint` noise.
    $criticalErrors = $docLines | Where-Object {
        ($_ -match 'error') -and ($_ -notmatch 'warning\[E0602\]: unknown lint')
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
    & cargo nextest run --profile $Profile @nextestFeatureArgs -E 'kind(lib)' --ignore-default-filter
} | Out-Null

# 8. Integration tests
Invoke-TestCheck -Name 'Integration Tests' -Command {
    & cargo nextest run --profile $Profile @nextestFeatureArgs --ignore-default-filter
} | Out-Null

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
} else {
    Write-ErrMsg "`n⚠️  Some checks failed, please review the error messages above"
    exit 1
}
