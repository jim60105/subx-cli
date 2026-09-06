<#
.SYNOPSIS
    Test coverage threshold checking script (PowerShell port of check_coverage.sh).

.DESCRIPTION
    PowerShell port of scripts/check_coverage.sh for Windows CI runners.

    Uses `cargo llvm-cov nextest` to generate coverage data and `cargo llvm-cov
    report` to extract a JSON summary, then verifies that line coverage meets
    a configurable threshold. Behaviour, CLI surface, environment variables,
    and exit codes mirror the Bash version so the two scripts are
    interchangeable from a CI workflow perspective.

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

.PARAMETER Threshold
    Minimum required line-coverage percentage. Defaults to 75.0 or to the
    `COVERAGE_THRESHOLD` environment variable when set.

.PARAMETER ThresholdCore
    Minimum required line-coverage percentage for the subx-core crate
    (per-crate attribution of the single workspace-attributed run). Defaults
    to 90.0 or to the `COVERAGE_THRESHOLD_CORE` environment variable when
    set.

.PARAMETER ThresholdCli
    Minimum required line-coverage percentage for the subx-cli crate.
    Defaults to 82.0 or to the `COVERAGE_THRESHOLD_CLI` environment variable
    when set.

.PARAMETER Profile
    nextest profile to use. One of: default, ci, quick, full. Defaults to
    `default` or to the `NEXTEST_PROFILE` environment variable when set.

.PARAMETER Full
    Run the full test suite, including slow tests gated by the `slow-tests`
    cargo feature. Implies `-Profile full` unless `-Profile` was supplied.

.PARAMETER Table
    Print a per-file coverage table after running the suite.

.PARAMETER File
    Show coverage details only for files whose path contains the given
    substring (case-insensitive).

.PARAMETER VerboseOutput
    Print verbose diagnostics, including the full test runner log when tests
    fail and a per-metric breakdown of overall coverage.

.PARAMETER Lcov
    Additionally emit an LCOV report at the given path. The JSON summary is
    always produced; this flag toggles a second report invocation.

.EXAMPLE
    pwsh ./scripts/check_coverage.ps1 -Table -Profile ci -Full -Lcov lcov.info

.NOTES
    Mirrors scripts/check_coverage.sh (the canonical Bash version).
#>

[CmdletBinding()]
param(
    [double]$Threshold,

    [double]$ThresholdCore,

    [double]$ThresholdCli,

    [ValidateSet('default', 'ci', 'quick', 'full')]
    [string]$Profile,

    [switch]$Full,

    [switch]$Table,

    [string]$File,

    [switch]$VerboseOutput,

    [string]$Lcov
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

$DefaultThreshold = 75.0
$DefaultThresholdCore = 90.0
$DefaultThresholdCli = 82.0
$DefaultNextestProfile = 'default'

if (-not $PSBoundParameters.ContainsKey('Threshold')) {
    $envThreshold = $env:COVERAGE_THRESHOLD
    if ($envThreshold) {
        $Threshold = [double]$envThreshold
    } else {
        $Threshold = $DefaultThreshold
    }
}

if (-not $PSBoundParameters.ContainsKey('ThresholdCore')) {
    $envThresholdCore = $env:COVERAGE_THRESHOLD_CORE
    if ($envThresholdCore) {
        $ThresholdCore = [double]$envThresholdCore
    } else {
        $ThresholdCore = $DefaultThresholdCore
    }
}

if (-not $PSBoundParameters.ContainsKey('ThresholdCli')) {
    $envThresholdCli = $env:COVERAGE_THRESHOLD_CLI
    if ($envThresholdCli) {
        $ThresholdCli = [double]$envThresholdCli
    } else {
        $ThresholdCli = $DefaultThresholdCli
    }
}

# Reporting exclusion set — byte-identical to LLVM_COV_IGNORE in
# scripts/check_coverage.sh. Every `cargo llvm-cov report` invocation below
# MUST receive it via --ignore-filename-regex, and both scripts MUST pass the
# same value: the lcov file uploaded to Codecov and the JSON the thresholds
# are computed from MUST describe the same codebase.
$LlvmCovIgnore = '(^|/)(tests|benches)/|(^|/)src/main\.rs$|(^|/)src/test_support/'

$profileExplicitlySet = $PSBoundParameters.ContainsKey('Profile')
if (-not $profileExplicitlySet) {
    $envProfile = $env:NEXTEST_PROFILE
    if ($envProfile) {
        if ($envProfile -notin @('default', 'ci', 'quick', 'full')) {
            Write-Error "Invalid profile '$envProfile' from NEXTEST_PROFILE. Available profiles: default, ci, quick, full"
            exit 1
        }
        $Profile = $envProfile
        $profileExplicitlySet = $true
    } else {
        $Profile = $DefaultNextestProfile
    }
}

if ($Full -and -not $profileExplicitlySet) {
    $Profile = 'full'
}

# -----------------------------------------------------------------------------
# Pretty-printing helpers
# -----------------------------------------------------------------------------

function Write-Info    { param([string]$Message) Write-Host $Message -ForegroundColor Cyan }
function Write-Success { param([string]$Message) Write-Host $Message -ForegroundColor Green }
function Write-WarnMsg { param([string]$Message) Write-Host $Message -ForegroundColor Yellow }
function Write-ErrMsg  { param([string]$Message) Write-Host $Message -ForegroundColor Red }

function Format-Percent {
    param([Parameter(Mandatory)][double]$Value)
    # Match the jq filter ((x * 100 | round) / 100): keep at most 2 decimals.
    return [math]::Round($Value, 2)
}

# -----------------------------------------------------------------------------
# Dependency check
# -----------------------------------------------------------------------------

function Test-Dependencies {
    $missing = @()

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        $missing += 'cargo'
    } else {
        # cargo-llvm-cov and cargo-nextest are cargo subcommand binaries;
        # probe them through cargo itself so we don't depend on PATH layout.
        try {
            $llvmCov = & cargo llvm-cov --version 2>$null
            if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($llvmCov)) {
                $missing += 'cargo-llvm-cov'
            }
        } catch {
            $missing += 'cargo-llvm-cov'
        }

        try {
            $nextest = & cargo nextest --version 2>$null
            if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($nextest)) {
                $missing += 'cargo-nextest'
            }
        } catch {
            $missing += 'cargo-nextest'
        }
    }

    if ($missing.Count -gt 0) {
        Write-ErrMsg '❌ Missing required dependencies:'
        foreach ($dep in $missing) {
            Write-ErrMsg "   - $dep"
        }
        Write-WarnMsg ''
        Write-WarnMsg 'Installation commands:'
        Write-WarnMsg '   cargo install cargo-llvm-cov'
        Write-WarnMsg '   cargo install cargo-nextest --locked'
        exit 1
    }
}

# -----------------------------------------------------------------------------
# Coverage rendering
# -----------------------------------------------------------------------------

function Show-CoverageTable {
    param([Parameter(Mandatory)]$Coverage)

    Write-Info '📊 File Coverage Report'
    Write-Host ''

    $headerFmt = '{0,-60} {1,8} {2,8} {3,8} {4,8}'
    Write-Host ([string]::Format($headerFmt, 'File', 'Lines', 'Funcs', 'Regions', 'Instants'))
    Write-Host ([string]::Format($headerFmt, ('-' * 60), '--------', '--------', '--------', '--------'))

    foreach ($file in $Coverage.data[0].files) {
        $linesPct    = Format-Percent $file.summary.lines.percent
        $funcsPct    = Format-Percent $file.summary.functions.percent
        $regionsPct  = Format-Percent $file.summary.regions.percent
        $instantsPct = Format-Percent $file.summary.instantiations.percent

        $name = $file.filename
        if ($name.Length -gt 57) {
            $name = '...' + $name.Substring($name.Length - 54)
        }

        $color = if ($linesPct -ge 80) { 'Green' }
                 elseif ($linesPct -ge 60) { 'Yellow' }
                 else { 'Red' }

        $left  = ([string]::Format('{0,-60} ', $name))
        $right = ([string]::Format('{0,8} {1,8} {2,8}', "$funcsPct%", "$regionsPct%", "$instantsPct%"))

        Write-Host -NoNewline $left
        Write-Host -NoNewline ([string]::Format('{0,8}', "$linesPct%")) -ForegroundColor $color
        Write-Host (' ' + $right.Substring(0))
    }

    Write-Host ''
    Write-Host 'Legend: ' -NoNewline -ForegroundColor Cyan
    Write-Host '>=80% ' -NoNewline -ForegroundColor Green
    Write-Host '60-79% ' -NoNewline -ForegroundColor Yellow
    Write-Host '<60% ' -NoNewline -ForegroundColor Red
    Write-Host '(based on line coverage)'

    return Show-OverallCoverage -Coverage $Coverage -ShowHeader:$true
}

# Partition the JSON file list into the two crates and derive one line
# coverage percentage per crate. Mirrors show_per_crate_coverage in
# scripts/check_coverage.sh:
#   - filenames are normalised backslash -> slash before any path test, and
#     so is the workspace root prefix, so a Windows-resident workspace
#     (C:\...\subx-core\src\...) attributes files to the same crate as a
#     POSIX one;
#   - a file is `subx-core` iff its normalised path contains "/subx-core/src/";
#   - otherwise it is `subx-cli` iff its normalised path starts with
#     "<workspace root>/src/";
#   - anything else is unexpected: it is named and the run fails, because
#     silently crediting a file outside both source roots to one crate's
#     group would let that floor gate the other crate's lines.
# Percentages come from summed covered/count lines, NEVER a mean of per-file
# percentages. The group line counts must sum to the report total; a
# mis-filed or dropped file makes the sums disagree.
function Show-PerCrateCoverage {
    param([Parameter(Mandatory)]$Coverage)

    $rootPrefix = ($PWD.Path -replace '\\', '/') + '/src/'
    $files = @($Coverage.data[0].files)

    $core = @(); $cli = @(); $unexpected = @()
    foreach ($file in $files) {
        $norm = $file.filename -replace '\\', '/'
        if ($norm.Contains('/subx-core/src/')) {
            $core += $file
        } elseif ($norm.StartsWith($rootPrefix)) {
            $cli += $file
        } else {
            $unexpected += $file
        }
    }

    if ($unexpected.Count -gt 0) {
        Write-ErrMsg "❌ Coverage partition found $($unexpected.Count) file(s) outside both source roots:"
        foreach ($file in $unexpected | Select-Object -First 10) {
            Write-ErrMsg "   - $($file.filename)"
        }
        return 1
    }

    $coreCovered = [double](@($core | ForEach-Object { $_.summary.lines.covered } | Measure-Object -Sum).Sum)
    $coreCount   = [double](@($core | ForEach-Object { $_.summary.lines.count } | Measure-Object -Sum).Sum)
    $cliCovered  = [double](@($cli | ForEach-Object { $_.summary.lines.covered } | Measure-Object -Sum).Sum)
    $cliCount    = [double](@($cli | ForEach-Object { $_.summary.lines.count } | Measure-Object -Sum).Sum)

    $totalLines = [double]$Coverage.data[0].totals.lines.count
    if (($coreCount + $cliCount) -ne $totalLines) {
        Write-ErrMsg "❌ Coverage partition incomplete: core($coreCount) + cli($cliCount) != total($totalLines)"
        return 1
    }

    $corePct = if ($coreCount -gt 0) { [math]::Round(100.0 * $coreCovered / $coreCount, 2) } else { 0.0 }
    $cliPct  = if ($cliCount  -gt 0) { [math]::Round(100.0 * $cliCovered  / $cliCount,  2) } else { 0.0 }

    Write-Host ''
    Write-Info '📦 Per-crate line coverage (workspace-attributed):'
    Write-Host "  subx-core: $corePct% ($coreCovered/$coreCount) - floor: $ThresholdCore%"
    Write-Host "  subx-cli:  $cliPct% ($cliCovered/$cliCount) - floor: $ThresholdCli%"

    $rc = 0
    if ($corePct -ge $ThresholdCore) {
        Write-Success '  ✅ subx-core meets its floor'
    } else {
        $deficit = [math]::Round($ThresholdCore - $corePct, 2)
        Write-ErrMsg "  ❌ subx-core below its floor (deficit: $deficit%)"
        $rc = 1
    }
    if ($cliPct -ge $ThresholdCli) {
        Write-Success '  ✅ subx-cli meets its floor'
    } else {
        $deficit = [math]::Round($ThresholdCli - $cliPct, 2)
        Write-ErrMsg "  ❌ subx-cli below its floor (deficit: $deficit%)"
        $rc = 1
    }
    return $rc
}

function Show-OverallCoverage {
    param(
        [Parameter(Mandatory)]$Coverage,
        [bool]$ShowHeader = $true
    )

    if ($ShowHeader) {
        Write-Host ''
        Write-Info '📈 Overall Coverage Summary'
        Write-Host ''
    }

    $totals = $Coverage.data[0].totals
    if ($null -eq $totals) {
        Write-ErrMsg '❌ Unable to parse overall coverage data'
        return 1
    }

    $current = Format-Percent $totals.lines.percent

    Write-Host "Current coverage: $current%"
    Write-Host "Required threshold: $Threshold%"

    if ($VerboseOutput -or $Table) {
        Write-Host ''
        Write-WarnMsg 'Detailed coverage information:'
        $func = Format-Percent $totals.functions.percent
        $line = Format-Percent $totals.lines.percent
        $reg  = Format-Percent $totals.regions.percent
        Write-Host ("  Function coverage: {0}% ({1}/{2})" -f $func, $totals.functions.covered, $totals.functions.count)
        Write-Host ("  Line coverage:     {0}% ({1}/{2})" -f $line, $totals.lines.covered, $totals.lines.count)
        Write-Host ("  Region coverage:   {0}% ({1}/{2})" -f $reg, $totals.regions.covered, $totals.regions.count)
    }

    if ($current -ge $Threshold) {
        Write-Host ''
        Write-Host ''
        Write-Success '✅ Combined coverage meets requirements'
        $rc = 0
    } else {
        $deficit = [math]::Round($Threshold - $current, 2)
        Write-Host ''
        Write-ErrMsg "❌ Combined coverage below threshold (deficit: $deficit%)"
        $rc = 1
    }

    # Gate the two per-crate floors. All three numbers are printed regardless
    # of outcome; any one below its floor fails the run.
    $perCrateRc = Show-PerCrateCoverage -Coverage $Coverage
    if ($perCrateRc -ne 0) { $rc = 1 }
    return $rc
}

function Show-FileCoverage {
    param(
        [Parameter(Mandatory)]$Coverage,
        [Parameter(Mandatory)][string]$Pattern
    )

    Write-Info "🔍 Searching for files matching '$Pattern'..."
    Write-Host ''

    $needle = $Pattern.ToLowerInvariant()
    $matches = $Coverage.data[0].files | Where-Object {
        $_.filename.ToLowerInvariant().Contains($needle)
    }

    if (-not $matches) {
        Write-ErrMsg "❌ No files found matching '$Pattern'"
        return 1
    }

    foreach ($file in $matches) {
        $linesPct    = Format-Percent $file.summary.lines.percent
        $funcsPct    = Format-Percent $file.summary.functions.percent
        $regionsPct  = Format-Percent $file.summary.regions.percent
        $instantsPct = Format-Percent $file.summary.instantiations.percent

        Write-Success ("📄 File: " + $file.filename)
        Write-Host ("   Lines:         {0}% ({1}/{2})" -f $linesPct, $file.summary.lines.covered, $file.summary.lines.count)
        Write-Host ("   Functions:     {0}% ({1}/{2})" -f $funcsPct, $file.summary.functions.covered, $file.summary.functions.count)
        Write-Host ("   Regions:       {0}% ({1}/{2})" -f $regionsPct, $file.summary.regions.covered, $file.summary.regions.count)
        Write-Host ("   Instantiations:{0}% ({1}/{2})" -f $instantsPct, $file.summary.instantiations.covered, $file.summary.instantiations.count)
        Write-Host ''
    }

    return 0
}

# -----------------------------------------------------------------------------
# Main coverage flow
# -----------------------------------------------------------------------------

function Invoke-CoverageCheck {
    Write-Info '🔍 Checking test coverage...'
    Write-Info "🔧 Using nextest profile: $Profile"
    if ($Full) {
        Write-Info '⚡ Full tests mode: Including slow tests (~143s vs ~90s)'
    } else {
        Write-Info '🚀 Fast tests mode: Excluding slow tests (~90s vs ~143s)'
        Write-Info '   Use -Full to include slow tests'
    }

    $featureArgs = @()
    if ($Full) {
        $featureArgs = @('--features', 'slow-tests')
    }

    Write-Info '📄 Running tests to generate coverage data...'

    # Coverage profraw files are collected even when some tests fail, so we
    # continue to generate the report regardless of test exit code.
    $testLog = New-TemporaryFile
    try {
        & cargo llvm-cov nextest --profile $Profile --workspace @featureArgs --no-report `
            *> $testLog.FullName
        $testExit = $LASTEXITCODE

        if ($testExit -ne 0) {
            Write-WarnMsg "⚠️  Test runner exited with code $testExit"
            if ($VerboseOutput) {
                Get-Content -LiteralPath $testLog.FullName | ForEach-Object { Write-Host $_ }
            } else {
                $tail = Get-Content -LiteralPath $testLog.FullName -Tail 3
                if ($tail -and $tail.Count -gt 0) {
                    Write-WarnMsg ("   " + $tail[0])
                }
                Write-WarnMsg '   Use -VerboseOutput for full test output'
            }
        } else {
            Write-Success '✅ Tests completed successfully'
        }
    } finally {
        Remove-Item -LiteralPath $testLog.FullName -ErrorAction SilentlyContinue
    }

    if ($Lcov) {
        Write-Info "📄 Generating LCOV output at: $Lcov"
        & cargo llvm-cov report --lcov --output-path $Lcov --ignore-filename-regex $LlvmCovIgnore *> $null
        if ($LASTEXITCODE -ne 0) {
            Write-ErrMsg '❌ Unable to generate LCOV coverage report'
            exit 1
        }
        Write-Success '✅ LCOV coverage report generated successfully'
    }

    $jsonOutput = & cargo llvm-cov report --json --summary-only --ignore-filename-regex $LlvmCovIgnore 2>&1
    $jsonExit = $LASTEXITCODE
    if ($jsonExit -ne 0) {
        Write-ErrMsg '❌ Unable to extract JSON coverage data'
        Write-WarnMsg "Error message: $jsonOutput"
        exit 1
    }

    Write-Success '✅ JSON coverage data extracted successfully'

    # cargo llvm-cov emits the JSON summary as a single line; pull the last
    # JSON-shaped line in case progress messages leak into stdout.
    $jsonLine = ($jsonOutput | Where-Object { $_ -match '^\{.*\}$' } | Select-Object -Last 1)
    if (-not $jsonLine) {
        Write-ErrMsg '❌ Unable to extract JSON data from output'
        Write-WarnMsg 'Raw output:'
        $jsonOutput | ForEach-Object { Write-Host $_ }
        exit 1
    }

    try {
        $coverage = $jsonLine | ConvertFrom-Json
    } catch {
        Write-ErrMsg '❌ Unable to parse JSON coverage data'
        Write-WarnMsg $_.Exception.Message
        exit 1
    }

    if ($Table) {
        return (Show-CoverageTable -Coverage $coverage)
    }

    if ($File) {
        return (Show-FileCoverage -Coverage $coverage -Pattern $File)
    }

    return (Show-OverallCoverage -Coverage $coverage -ShowHeader:$false)
}

# -----------------------------------------------------------------------------
# Entry point
# -----------------------------------------------------------------------------

Test-Dependencies
$exitCode = Invoke-CoverageCheck
if ($null -eq $exitCode) { $exitCode = 0 }
exit [int]$exitCode
