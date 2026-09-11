param(
    [Parameter(Mandatory = $true)]
    [string]$ScreenReaderExe,

    [Parameter(Mandatory = $true)]
    [string]$EvidenceDir
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PasswordPrivacyNativeHarness
{
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
}
'@

if (-not (Test-Path $ScreenReaderExe)) {
    throw "Screen reader executable not found: $ScreenReaderExe"
}

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null

$secret = 'SR-PRIVACY-4e2c9e'
$stdout = Join-Path $EvidenceDir 'password-screen-reader.stdout.txt'
$stderr = Join-Path $EvidenceDir 'password-screen-reader.stderr.txt'
$fixtureStdout = Join-Path $EvidenceDir 'password-fixture.stdout.txt'
$fixtureStderr = Join-Path $EvidenceDir 'password-fixture.stderr.txt'
$controllerLog = Join-Path $EvidenceDir 'password-controller.log'
$summaryPath = Join-Path $EvidenceDir 'password-privacy-summary.json'
$readyPath = Join-Path $EvidenceDir 'password-ready.json'
$fixtureScript = Join-Path $PSScriptRoot 'PasswordPrivacyFixture.ps1'

function Write-Controller {
    param([Parameter(Mandatory = $true)][string]$Value)
    "$(Get-Date -Format o)|$Value" | Add-Content -Path $controllerLog -Encoding utf8
}

function Wait-Ready {
    param([System.Diagnostics.Process]$Process)
    $deadline = (Get-Date).AddSeconds(10)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path $readyPath) { return }
        if ($Process.HasExited) {
            $err = if (Test-Path $fixtureStderr) { Get-Content $fixtureStderr -Raw } else { '' }
            throw "Password fixture exited before readiness (exit=$($Process.ExitCode)): $err"
        }
        Start-Sleep -Milliseconds 200
    }
    throw "Password fixture readiness timeout: $readyPath"
}

function Focus-Window {
    param([Parameter(Mandatory = $true)][Int64]$Handle)
    $hWnd = [IntPtr]$Handle
    [void][PasswordPrivacyNativeHarness]::ShowWindow($hWnd, 5)
    for ($attempt = 1; $attempt -le 10; $attempt++) {
        [void][PasswordPrivacyNativeHarness]::SetForegroundWindow($hWnd)
        Start-Sleep -Milliseconds 150
        if ([PasswordPrivacyNativeHarness]::GetForegroundWindow() -eq $hWnd) {
            Write-Controller "FOREGROUND|attempt=$attempt|hwnd=$Handle"
            return
        }
    }
    throw "Unable to focus password fixture hwnd=$Handle"
}

$reader = Start-Process -FilePath $ScreenReaderExe -ArgumentList @('15') `
    -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
Write-Controller "START_READER|pid=$($reader.Id)"

$fixture = $null
try {
    Start-Sleep -Seconds 2
    $pwsh = (Get-Command pwsh.exe -ErrorAction Stop).Source
    $fixture = Start-Process -FilePath $pwsh -ArgumentList @(
        '-NoProfile', '-STA', '-ExecutionPolicy', 'Bypass',
        '-File', $fixtureScript, '-EvidenceDir', $EvidenceDir
    ) -RedirectStandardOutput $fixtureStdout -RedirectStandardError $fixtureStderr -PassThru
    Write-Controller "START_FIXTURE|pid=$($fixture.Id)"

    Wait-Ready -Process $fixture
    $ready = Get-Content $readyPath -Raw | ConvertFrom-Json
    if ([int]$ready.pid -ne $fixture.Id) {
        throw "Password fixture PID mismatch: process=$($fixture.Id) ready=$($ready.pid)"
    }
    if (-not [bool]$ready.protected) {
        throw 'Password fixture did not declare protected=true'
    }

    Focus-Window -Handle ([int64]$ready.windowHandle)

    # Do not write the test secret to controller evidence.
    Write-Controller 'KEYBOARD|type-password|<redacted>'
    [System.Windows.Forms.SendKeys]::SendWait($secret)
    Start-Sleep -Milliseconds 500

    # Exercise a selection in the protected field. Providers that emit UIA text
    # selection events must still be redacted by the Rust runtime.
    Write-Controller 'KEYBOARD|password-select-left|+{LEFT}'
    [System.Windows.Forms.SendKeys]::SendWait('+{LEFT}')
    Start-Sleep -Milliseconds 500

    Write-Controller 'KEYBOARD|close-window|%{F4}'
    [System.Windows.Forms.SendKeys]::SendWait('%{F4}')

    if (-not $fixture.WaitForExit(8000)) {
        Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue
        throw 'Password fixture did not close after Alt+F4'
    }

    if (-not $reader.WaitForExit(22000)) {
        Stop-Process -Id $reader.Id -Force -ErrorAction SilentlyContinue
        throw 'Password privacy screen reader monitor did not exit in time'
    }
    if ($reader.ExitCode -ne 0) {
        throw "Password privacy screen reader exited with code $($reader.ExitCode)"
    }
}
finally {
    if ($null -ne $fixture -and -not $fixture.HasExited) {
        Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue
    }
    if (-not $reader.HasExited) {
        Stop-Process -Id $reader.Id -Force -ErrorAction SilentlyContinue
    }
}

$screenReaderLog = Get-Content $stdout -Raw
$screenReaderError = if (Test-Path $stderr) { Get-Content $stderr -Raw } else { '' }
$fixtureLogPath = Join-Path $EvidenceDir 'password-fixture.log'
$fixtureLog = if (Test-Path $fixtureLogPath) { Get-Content $fixtureLogPath -Raw } else { '' }
$controller = Get-Content $controllerLog -Raw

$evidenceText = @($screenReaderLog, $screenReaderError, $fixtureLog, $controller) -join "`n"
$leakCount = ([regex]::Matches($evidenceText, [regex]::Escape($secret))).Count
if ($leakCount -ne 0) {
    throw "PASSWORD_PRIVACY leak detected in runtime evidence: count=$leakCount"
}

$ready = Get-Content $readyPath -Raw | ConvertFrom-Json
$pidPattern = "PID=$([int]$ready.pid)\s*\|"
$fixtureUia = @(
    ($screenReaderLog -split "`r?`n") |
        Where-Object { $_ -match $pidPattern }
)

$passwordFocus = @(
    $fixtureUia |
        Where-Object { $_ -match '^FOCUS #' -and $_ -match 'Name=Account password(?:\s*\||$)' }
)
if ($passwordFocus.Count -lt 1) {
    throw 'Protected password field never produced a UIA focus event'
}

$passwordSpeech = @(
    ($screenReaderLog -split "`r?`n") |
        Where-Object { $_ -match '^SPEECH_OUTPUT #\d+ = PASS \| password field\s*$' }
)
if ($passwordSpeech.Count -lt 1) {
    throw 'Protected password focus was not converted to the generic speech phrase password field'
}

$protectedTextPattern = @(
    $fixtureUia |
        Where-Object { $_ -match '^TEXT_PATTERN2 #' -and $_ -match 'protected=password' }
)

$summary = [ordered]@{
    schemaVersion = 1
    platform = 'windows'
    scenario = 'password-privacy-e2e'
    pass = $true
    keyboardOnly = $true
    passwordSecretPersistedInEvidence = $false
    secretLeakCount = 0
    fixturePid = [int]$ready.pid
    passwordFocusEvents = $passwordFocus.Count
    genericPasswordSpeechOutputs = $passwordSpeech.Count
    protectedTextPatternEvents = $protectedTextPattern.Count
    physicalAudioDeviceAsserted = $false
}
$summary | ConvertTo-Json -Depth 5 | Set-Content -Path $summaryPath -Encoding utf8

"PASSWORD_PRIVACY_E2E = PASS | focus=$($passwordFocus.Count) | generic_speech=$($passwordSpeech.Count) | textpattern_protected=$($protectedTextPattern.Count) | secret_leaks=0 | physical_audio_asserted=false"
