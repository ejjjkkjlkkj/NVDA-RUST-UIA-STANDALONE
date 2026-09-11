param(
    [Parameter(Mandatory = $true)]
    [string]$ScreenReaderExe,

    [Parameter(Mandatory = $true)]
    [string]$EvidenceDir
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class DiagnosticPrivacyNativeHarness
{
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
}
'@

if (-not (Test-Path $ScreenReaderExe)) {
    throw "Screen reader executable not found: $ScreenReaderExe"
}

$fixtureScript = Join-Path $PSScriptRoot 'BlindUserFixture.ps1'
if (-not (Test-Path $fixtureScript)) {
    throw "Fixture script missing: $fixtureScript"
}

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$stdout = Join-Path $EvidenceDir 'diagnostic-redaction.stdout.txt'
$stderr = Join-Path $EvidenceDir 'diagnostic-redaction.stderr.txt'
$readyPath = Join-Path $EvidenceDir 'fixture-ready.json'

$previous = $env:NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT
Remove-Item Env:NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT -ErrorAction SilentlyContinue

$reader = $null
$fixture = $null
try {
    $reader = Start-Process -FilePath $ScreenReaderExe -ArgumentList @('10') `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru

    Start-Sleep -Seconds 2
    $pwsh = (Get-Command pwsh.exe -ErrorAction Stop).Source
    $fixture = Start-Process -FilePath $pwsh -ArgumentList @(
        '-NoProfile', '-STA', '-ExecutionPolicy', 'Bypass',
        '-File', $fixtureScript, '-EvidenceDir', $EvidenceDir
    ) -PassThru

    $deadline = (Get-Date).AddSeconds(10)
    while ((Get-Date) -lt $deadline -and -not (Test-Path $readyPath)) {
        if ($fixture.HasExited) {
            throw "Fixture exited before readiness with code $($fixture.ExitCode)"
        }
        Start-Sleep -Milliseconds 200
    }
    if (-not (Test-Path $readyPath)) {
        throw 'Fixture readiness timeout'
    }

    $ready = Get-Content $readyPath -Raw | ConvertFrom-Json
    $handle = [IntPtr][int64]$ready.windowHandle
    [void][DiagnosticPrivacyNativeHarness]::ShowWindow($handle, 5)
    [void][DiagnosticPrivacyNativeHarness]::SetForegroundWindow($handle)
    Start-Sleep -Seconds 2

    if (-not $reader.WaitForExit(30000)) {
        Stop-Process -Id $reader.Id -Force -ErrorAction SilentlyContinue
        throw 'Diagnostic privacy runtime did not exit in time'
    }
    if ($reader.ExitCode -ne 0) {
        throw "Diagnostic privacy runtime exited with code $($reader.ExitCode)"
    }
}
finally {
    if ($null -ne $fixture -and -not $fixture.HasExited) {
        Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne $reader -and -not $reader.HasExited) {
        Stop-Process -Id $reader.Id -Force -ErrorAction SilentlyContinue
    }
    if ($null -eq $previous) {
        Remove-Item Env:NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT -ErrorAction SilentlyContinue
    } else {
        $env:NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT = $previous
    }
}

$log = Get-Content $stdout -Raw
foreach ($marker in @(
    'DIAGNOSTIC_TEXT_POLICY = REDACTED_DEFAULT',
    'PASSWORD_TEXT_POLICY = ALWAYS_REDACTED',
    'SPEECH_OUTPUT_INIT = PASS',
    '<redacted>'
)) {
    if (-not $log.Contains($marker)) {
        throw "Missing privacy marker: $marker"
    }
}

# Production diagnostics must reveal neither content nor content length.
if ($log -match '<redacted chars=\d+>') {
    throw 'DIAGNOSTIC_PRIVACY length disclosure detected'
}

# These known fixture strings are non-password data. Their absence proves that
# production/default diagnostics do not retain accessible names or spoken text.
foreach ($forbidden in @(
    'Blind user document',
    'Enable feature',
    'Reading mode',
    'Apply changes',
    'blind user typed text'
)) {
    if ($log.Contains($forbidden)) {
        throw "DIAGNOSTIC_PRIVACY plaintext leak detected: $forbidden"
    }
}

$redactedSpeech = @(
    ($log -split "`r?`n") |
        Where-Object { $_ -match '^SPEECH_(REQUEST|OUTPUT) #' -and $_ -match '<redacted>' }
)
if ($redactedSpeech.Count -lt 1) {
    throw 'No redacted speech evidence was captured'
}

"DIAGNOSTIC_REDACTION_E2E = PASS | plaintext_leaks=0 | length_leaks=0 | redacted_speech_lines=$($redactedSpeech.Count) | default_policy=true"