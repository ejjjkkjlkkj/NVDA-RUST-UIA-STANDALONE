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
public static class EdgeWebNativeHarness
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

$fixturePath = Join-Path $PSScriptRoot 'fixtures\edge-accessible-form.html'
if (-not (Test-Path $fixturePath)) {
    throw "Edge web fixture missing: $fixturePath"
}

$edgeCandidates = @()
if (${env:ProgramFiles(x86)}) {
    $edgeCandidates += Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'
}
if ($env:ProgramFiles) {
    $edgeCandidates += Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe'
}
$edgeCommand = Get-Command msedge.exe -ErrorAction SilentlyContinue
if ($null -ne $edgeCommand) {
    $edgeCandidates += $edgeCommand.Source
}
$edgeExe = $edgeCandidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $edgeExe) {
    throw 'Microsoft Edge executable was not found on the Windows runner'
}

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$stdout = Join-Path $EvidenceDir 'edge-screen-reader.stdout.txt'
$stderr = Join-Path $EvidenceDir 'edge-screen-reader.stderr.txt'
$controllerLog = Join-Path $EvidenceDir 'edge-keyboard-controller.log'
$summaryPath = Join-Path $EvidenceDir 'edge-web-summary.json'
$userDataDir = Join-Path $EvidenceDir 'edge-profile'
New-Item -ItemType Directory -Force -Path $userDataDir | Out-Null

function Write-Controller {
    param([Parameter(Mandatory = $true)][string]$Value)
    "$(Get-Date -Format o)|$Value" | Add-Content -Path $controllerLog -Encoding utf8
}

function Focus-Window {
    param(
        [Parameter(Mandatory = $true)][Int64]$Handle,
        [Parameter(Mandatory = $true)][string]$Label
    )
    $hWnd = [IntPtr]$Handle
    [void][EdgeWebNativeHarness]::ShowWindow($hWnd, 5)
    for ($attempt = 1; $attempt -le 15; $attempt++) {
        [void][EdgeWebNativeHarness]::SetForegroundWindow($hWnd)
        Start-Sleep -Milliseconds 150
        if ([EdgeWebNativeHarness]::GetForegroundWindow() -eq $hWnd) {
            Write-Controller "FOREGROUND|$Label|attempt=$attempt|hwnd=$Handle"
            return
        }
    }
    $actual = [EdgeWebNativeHarness]::GetForegroundWindow().ToInt64()
    throw "Unable to focus $Label. expected=$Handle actual=$actual"
}

function Send-UserKeys {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$Keys,
        [int]$DelayMs = 450
    )
    Write-Controller "KEYBOARD|$Label|$Keys"
    [System.Windows.Forms.SendKeys]::SendWait($Keys)
    Start-Sleep -Milliseconds $DelayMs
}

function Wait-ForEdgeWindow {
    param(
        [Parameter(Mandatory = $true)]
        [AllowEmptyCollection()]
        [int[]]$ExistingIds,
        [int]$Seconds = 15
    )
    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        $all = @(Get-Process msedge -ErrorAction SilentlyContinue)
        $candidate = $all |
            Where-Object { $_.MainWindowHandle -ne 0 -and $_.Id -notin $ExistingIds } |
            Select-Object -First 1
        if ($null -eq $candidate) {
            $candidate = $all | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
        }
        if ($null -ne $candidate) {
            return $candidate
        }
        Start-Sleep -Milliseconds 250
    }
    throw 'Microsoft Edge did not expose a foreground window in time'
}

$existingEdgeIds = @(
    Get-Process msedge -ErrorAction SilentlyContinue | ForEach-Object { $_.Id }
)
$fixtureUri = ([System.Uri]::new((Resolve-Path $fixturePath).Path)).AbsoluteUri

$reader = Start-Process -FilePath $ScreenReaderExe -ArgumentList @('25') `
    -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
Write-Controller "START_READER|pid=$($reader.Id)"

$edgeLauncher = $null
$edgeWindow = $null
try {
    Start-Sleep -Seconds 2

    $edgeArgs = @(
        '--kiosk',
        $fixtureUri,
        '--edge-kiosk-type=fullscreen',
        '--no-first-run',
        '--disable-extensions',
        '--force-renderer-accessibility',
        "--user-data-dir=`"$userDataDir`""
    )
    $edgeLauncher = Start-Process -FilePath $edgeExe -ArgumentList $edgeArgs -PassThru
    Write-Controller "START_EDGE|launcher_pid=$($edgeLauncher.Id)|uri=$fixtureUri"

    $edgeWindow = Wait-ForEdgeWindow -ExistingIds $existingEdgeIds -Seconds 15
    Write-Controller "EDGE_WINDOW|pid=$($edgeWindow.Id)|hwnd=$($edgeWindow.MainWindowHandle)"
    Focus-Window -Handle ([int64]$edgeWindow.MainWindowHandle) -Label 'Edge kiosk web fixture'
    Start-Sleep -Seconds 2

    # The HTML fixture autofocuses the first edit. Functional interaction below is keyboard-only.
    Send-UserKeys -Label 'web-type-account' -Keys 'web blind user'
    Send-UserKeys -Label 'web-tab-checkbox' -Keys '{TAB}'
    Send-UserKeys -Label 'web-toggle-checkbox' -Keys ' '
    Send-UserKeys -Label 'web-tab-mode' -Keys '{TAB}'
    Send-UserKeys -Label 'web-select-detailed' -Keys '{DOWN}'
    Send-UserKeys -Label 'web-tab-apply' -Keys '{TAB}'
    Send-UserKeys -Label 'web-activate-apply' -Keys '{ENTER}'
    Send-UserKeys -Label 'web-tab-help-link' -Keys '{TAB}'

    Start-Sleep -Seconds 2
    Send-UserKeys -Label 'web-close-edge' -Keys '%{F4}' -DelayMs 700

    if (-not $reader.WaitForExit(45000)) {
        Stop-Process -Id $reader.Id -Force -ErrorAction SilentlyContinue
        throw 'Screen reader did not exit after Edge E2E monitor and async speech flush'
    }
    if ($reader.ExitCode -ne 0) {
        throw "Screen reader exited with code $($reader.ExitCode) during Edge E2E"
    }
}
finally {
    $newEdgeProcesses = @(
        Get-Process msedge -ErrorAction SilentlyContinue |
            Where-Object { $_.Id -notin $existingEdgeIds }
    )
    foreach ($edgeProcess in $newEdgeProcesses) {
        Stop-Process -Id $edgeProcess.Id -Force -ErrorAction SilentlyContinue
    }
    if (-not $reader.HasExited) {
        Stop-Process -Id $reader.Id -Force -ErrorAction SilentlyContinue
    }
}

$screenReaderLog = Get-Content $stdout -Raw
$screenReaderLines = @($screenReaderLog -split "`r?`n")

foreach ($marker in @(
    'UIA_NATIVE_EVENTS_INIT = PASS',
    'UIA_NATIVE_EVENTS_RUNTIME = PASS',
    'SPEECH_DISPATCH = ASYNC_WORKER',
    'SPEECH_FLUSH = PASS'
)) {
    if (-not $screenReaderLog.Contains($marker)) {
        throw "Missing Edge E2E runtime marker: $marker"
    }
}

$expectedControls = @(
    'Web account name',
    'Web feature enabled',
    'Web reading mode',
    'Web apply changes',
    'Web help link'
)
$focusProof = [ordered]@{}
$speechProof = [ordered]@{}
foreach ($name in $expectedControls) {
    $escaped = [regex]::Escape($name)
    $focus = @(
        $screenReaderLines |
            Where-Object { $_ -match '^FOCUS #' -and $_ -match "Name=$escaped(?:\s*\||$)" }
    )
    $speech = @(
        $screenReaderLines |
            Where-Object { $_ -match '^SPEECH_OUTPUT #\d+ = PASS \|' -and $_ -match [regex]::Escape($name) }
    )
    $focusProof[$name] = $focus.Count
    $speechProof[$name] = $speech.Count
    if ($focus.Count -lt 1) {
        throw "Edge keyboard journey did not expose UIA focus for '$name'"
    }
    if ($speech.Count -lt 1) {
        throw "Edge keyboard journey did not produce speech output for '$name'"
    }
}

$chromeFrameworkLines = @(
    $screenReaderLines |
        Where-Object { $_ -match '^FOCUS #' -and $_ -match 'Framework=Chrome(?:\s*\||$)' }
)
if ($chromeFrameworkLines.Count -lt 1) {
    throw 'Edge web controls were not observed through the Chromium UIA framework'
}

$keyboardActions = @(
    Get-Content $controllerLog | Where-Object { $_ -match '\|KEYBOARD\|' }
)
if ($keyboardActions.Count -lt 9) {
    throw "Expected at least 9 external keyboard actions in Edge E2E, observed $($keyboardActions.Count)"
}

$valueEvents = @(
    $screenReaderLines | Where-Object { $_ -match '^PROPERTY_CHANGED/UIA_30045\[ValueValue\]' -and $_ -match 'Framework=Chrome' }
)
$toggleEvents = @(
    $screenReaderLines | Where-Object { $_ -match '^PROPERTY_CHANGED/UIA_30086\[ToggleToggleState\]' -and $_ -match 'Framework=Chrome' }
)
$textEvents = @(
    $screenReaderLines | Where-Object { $_ -match '^TEXT (CHANGED|SELECTION CHANGED) / UIA 2001[45]' -and $_ -match 'Framework=Chrome' }
)

$summary = [ordered]@{
    schemaVersion = 1
    platform = 'windows'
    scenario = 'edge-web-keyboard-e2e'
    pass = $true
    keyboardOnlyFunctionalInteraction = $true
    synthesizedMouseInputCalls = 0
    externalKeyboardActions = $keyboardActions.Count
    edgeExecutable = $edgeExe
    edgeWindowPid = if ($null -ne $edgeWindow) { $edgeWindow.Id } else { 0 }
    fixtureUriScheme = 'file'
    chromiumFrameworkFocusEvents = $chromeFrameworkLines.Count
    focusEventsByAccessibleName = $focusProof
    speechOutputsByAccessibleName = $speechProof
    chromiumValueEvents = $valueEvents.Count
    chromiumToggleEvents = $toggleEvents.Count
    chromiumTextEvents = $textEvents.Count
    physicalAudioDeviceAsserted = $false
}
$summary | ConvertTo-Json -Depth 6 | Set-Content -Path $summaryPath -Encoding utf8

"EDGE_WEB_KEYBOARD_E2E = PASS | keyboard_actions=$($keyboardActions.Count) | chromium_focus=$($chromeFrameworkLines.Count) | value_events=$($valueEvents.Count) | toggle_events=$($toggleEvents.Count) | text_events=$($textEvents.Count) | mouse_calls=0"
