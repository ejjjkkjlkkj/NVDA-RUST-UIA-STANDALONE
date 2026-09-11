param(
    [Parameter(Mandatory = $true)]
    [string]$EvidenceDir
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$fixtureScript = Join-Path $PSScriptRoot 'BlindUserFixture.ps1'
$readyPath = Join-Path $EvidenceDir 'fixture-ready.json'
$stdoutPath = Join-Path $EvidenceDir 'fixture.stdout.txt'
$stderrPath = Join-Path $EvidenceDir 'fixture.stderr.txt'

if (-not (Test-Path $fixtureScript)) {
    throw "Fixture script missing: $fixtureScript"
}

$pwsh = (Get-Command pwsh.exe -ErrorAction Stop).Source
$arguments = @(
    '-NoProfile',
    '-STA',
    '-ExecutionPolicy', 'Bypass',
    '-File', $fixtureScript,
    '-EvidenceDir', $EvidenceDir
)

$fixture = $null
try {
    $fixture = Start-Process -FilePath $pwsh -ArgumentList $arguments `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath `
        -PassThru

    $deadline = (Get-Date).AddSeconds(12)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path $readyPath) {
            break
        }
        if ($fixture.HasExited) {
            $stderr = if (Test-Path $stderrPath) { Get-Content $stderrPath -Raw } else { '' }
            throw "Fixture exited before readiness (exit=$($fixture.ExitCode)): $stderr"
        }
        Start-Sleep -Milliseconds 200
    }

    if (-not (Test-Path $readyPath)) {
        $bootstrapPath = Join-Path $EvidenceDir 'fixture-bootstrap.log'
        $bootstrap = if (Test-Path $bootstrapPath) { Get-Content $bootstrapPath -Raw } else { '<none>' }
        $stderr = if (Test-Path $stderrPath) { Get-Content $stderrPath -Raw } else { '<none>' }
        throw "Fixture readiness timeout. bootstrap=$bootstrap stderr=$stderr"
    }

    $ready = Get-Content $readyPath -Raw | ConvertFrom-Json
    if ([int]$ready.pid -ne $fixture.Id) {
        throw "Fixture smoke PID mismatch: process=$($fixture.Id) ready=$($ready.pid)"
    }
    if ([int64]$ready.windowHandle -eq 0) {
        throw 'Fixture smoke produced a zero HWND'
    }

    "FIXTURE_SMOKE = PASS | pid=$($ready.pid) | hwnd=$($ready.windowHandle)"
}
finally {
    if ($null -ne $fixture -and -not $fixture.HasExited) {
        Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue
    }
}
