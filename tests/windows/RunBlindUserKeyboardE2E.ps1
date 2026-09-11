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

public static class BlindUserNativeHarness
{
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
}
'@

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null

$stdout = Join-Path $EvidenceDir 'screen-reader.stdout.txt'
$stderr = Join-Path $EvidenceDir 'screen-reader.stderr.txt'
$controllerLog = Join-Path $EvidenceDir 'keyboard-controller.log'
$summaryPath = Join-Path $EvidenceDir 'blind-user-summary.json'
$nativeProbePath = Join-Path $EvidenceDir 'native-app-probe.json'

function Write-ControllerEvent {
    param(
        [Parameter(Mandatory = $true)][string]$Kind,
        [Parameter(Mandatory = $true)][string]$Value
    )

    "$(Get-Date -Format o)|$Kind|$Value" |
        Add-Content -Path $controllerLog -Encoding utf8
}

function Wait-ForFile {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [int]$Seconds = 10,
        [System.Diagnostics.Process]$Process
    )

    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path $Path) {
            return
        }
        if ($null -ne $Process -and $Process.HasExited) {
            throw "Process exited before creating $Path (exit=$($Process.ExitCode))"
        }
        Start-Sleep -Milliseconds 200
    }
    throw "Timed out waiting for $Path"
}

function Focus-Window {
    param(
        [Parameter(Mandatory = $true)][Int64]$Handle,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $hWnd = [IntPtr]$Handle
    [void][BlindUserNativeHarness]::ShowWindow($hWnd, 5)

    for ($attempt = 1; $attempt -le 10; $attempt++) {
        [void][BlindUserNativeHarness]::SetForegroundWindow($hWnd)
        Start-Sleep -Milliseconds 150
        if ([BlindUserNativeHarness]::GetForegroundWindow() -eq $hWnd) {
            Write-ControllerEvent -Kind 'FOREGROUND' -Value "$Label|attempt=$attempt|hwnd=$Handle"
            return
        }
    }

    $actual = [BlindUserNativeHarness]::GetForegroundWindow().ToInt64()
    throw "Unable to place $Label in foreground. expected=$Handle actual=$actual"
}

function Send-UserKeys {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$Keys,
        [int]$DelayMs = 350
    )

    Write-ControllerEvent -Kind 'KEYBOARD' -Value "$Label|$Keys"
    [System.Windows.Forms.SendKeys]::SendWait($Keys)
    Start-Sleep -Milliseconds $DelayMs
}

function Invoke-NotepadProbe {
    param(
        [Parameter(Mandatory = $true)][string]$OutputPath
    )

    $result = [ordered]@{
        attempted = $false
        windowAvailable = $false
        keyboardInputSent = $false
        selectionInputSent = $false
        pid = 0
        windowHandle = 0
        error = $null
    }

    $notepadCommand = Get-Command notepad.exe -ErrorAction SilentlyContinue
    if ($null -eq $notepadCommand) {
        $result.error = 'notepad.exe unavailable on runner'
        $result | ConvertTo-Json -Depth 4 | Set-Content -Path $OutputPath -Encoding utf8
        return $result
    }

    $result.attempted = $true
    $notepad = $null
    try {
        $notepad = Start-Process -FilePath $notepadCommand.Source -PassThru
        $result.pid = $notepad.Id

        $deadline = (Get-Date).AddSeconds(8)
        $handle = 0L
        while ((Get-Date) -lt $deadline) {
            $notepad.Refresh()
            $handle = $notepad.MainWindowHandle.ToInt64()
            if ($handle -ne 0) {
                break
            }
            if ($notepad.HasExited) {
                break
            }
            Start-Sleep -Milliseconds 250
        }

        if ($handle -eq 0) {
            $result.error = 'notepad window handle unavailable'
            return $result
        }

        $result.windowAvailable = $true
        $result.windowHandle = $handle
        Focus-Window -Handle $handle -Label 'Notepad native application'
        Send-UserKeys -Label 'notepad-type' -Keys 'native blind user keyboard probe'
        $result.keyboardInputSent = $true
        Send-UserKeys -Label 'notepad-select-left-1' -Keys '+{LEFT}'
        Send-UserKeys -Label 'notepad-select-left-2' -Keys '+{LEFT}'
        Send-UserKeys -Label 'notepad-select-left-3' -Keys '+{LEFT}'
        Send-UserKeys -Label 'notepad-select-left-4' -Keys '+{LEFT}'
        $result.selectionInputSent = $true
        Start-Sleep -Milliseconds 800
    }
    catch {
        $result.error = $_.Exception.Message
    }
    finally {
        if ($null -ne $notepad -and -not $notepad.HasExited) {
            Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
        }
        $result | ConvertTo-Json -Depth 4 | Set-Content -Path $OutputPath -Encoding utf8
    }

    return $result
}

if (-not (Test-Path $ScreenReaderExe)) {
    throw "Screen reader executable not found: $ScreenReaderExe"
}

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$baselinePath = Join-Path $repoRoot 'reference\nvda-baseline.json'
if (-not (Test-Path $baselinePath)) {
    throw "Official NVDA baseline metadata missing: $baselinePath"
}
$nvdaBaseline = Get-Content $baselinePath -Raw | ConvertFrom-Json

$fixtureScript = Join-Path $PSScriptRoot 'BlindUserFixture.ps1'
if (-not (Test-Path $fixtureScript)) {
    throw "Fixture script missing: $fixtureScript"
}

$readerArguments = @{
    FilePath = $ScreenReaderExe
    ArgumentList = @('30')
    RedirectStandardOutput = $stdout
    RedirectStandardError = $stderr
    PassThru = $true
}
$reader = Start-Process @readerArguments
Write-ControllerEvent -Kind 'START' -Value "screen-reader|pid=$($reader.Id)"

$fixture = $null
$fixturePid = 0
$nativeProbe = $null
try {
    Start-Sleep -Seconds 2

    $fixtureCommandLine = "-NoProfile -STA -ExecutionPolicy Bypass -File `"$fixtureScript`" -EvidenceDir `"$EvidenceDir`""
    $fixture = Start-Process -FilePath 'powershell.exe' -ArgumentList $fixtureCommandLine -PassThru
    Write-ControllerEvent -Kind 'START' -Value "fixture|pid=$($fixture.Id)"

    $readyPath = Join-Path $EvidenceDir 'fixture-ready.json'
    Wait-ForFile -Path $readyPath -Seconds 12 -Process $fixture
    $ready = Get-Content $readyPath -Raw | ConvertFrom-Json
    $fixturePid = [int]$ready.pid

    if ($fixturePid -ne $fixture.Id) {
        throw "Fixture PID mismatch: process=$($fixture.Id) ready=$fixturePid"
    }

    Focus-Window -Handle ([int64]$ready.windowHandle) -Label 'blind-user fixture'

    # Functional interaction is deliberately keyboard-only. No mouse input is synthesized.
    Send-UserKeys -Label 'select-all-document' -Keys '^a'
    Send-UserKeys -Label 'type-document' -Keys 'blind user typed text'
    Send-UserKeys -Label 'tab-to-checkbox' -Keys '{TAB}'
    Send-UserKeys -Label 'toggle-checkbox' -Keys ' '
    Send-UserKeys -Label 'tab-to-mode' -Keys '{TAB}'
    Send-UserKeys -Label 'choose-detailed-mode' -Keys '{DOWN}'
    Send-UserKeys -Label 'tab-to-apply' -Keys '{TAB}'
    Send-UserKeys -Label 'activate-apply' -Keys '{ENTER}'

    # Reverse navigation matters for daily keyboard use, not only forward Tab traversal.
    Send-UserKeys -Label 'reverse-to-mode' -Keys '+{TAB}'
    Send-UserKeys -Label 'reverse-to-checkbox' -Keys '+{TAB}'
    Send-UserKeys -Label 'reverse-to-editor' -Keys '+{TAB}'
    Send-UserKeys -Label 'move-to-end' -Keys '{END}'
    Send-UserKeys -Label 'select-left-1' -Keys '+{LEFT}'
    Send-UserKeys -Label 'select-left-2' -Keys '+{LEFT}'
    Send-UserKeys -Label 'select-left-3' -Keys '+{LEFT}'
    Send-UserKeys -Label 'select-left-4' -Keys '+{LEFT}'

    Start-Sleep -Milliseconds 800

    # Probe a real inbox application surface as well as our deterministic fixture.
    $nativeProbe = Invoke-NotepadProbe -OutputPath $nativeProbePath

    Focus-Window -Handle ([int64]$ready.windowHandle) -Label 'blind-user fixture before close'
    Send-UserKeys -Label 'close-window' -Keys '%{F4}'

    if (-not $fixture.WaitForExit(10000)) {
        Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue
        throw 'Fixture did not close after keyboard Alt+F4'
    }
    if ($fixture.ExitCode -ne 0) {
        throw "Fixture exited with code $($fixture.ExitCode)"
    }

    if (-not $reader.WaitForExit(35000)) {
        Stop-Process -Id $reader.Id -Force -ErrorAction SilentlyContinue
        throw 'Screen reader monitor did not exit in time'
    }
    if ($reader.ExitCode -ne 0) {
        throw "Screen reader monitor exited with code $($reader.ExitCode)"
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
$fixtureLogPath = Join-Path $EvidenceDir 'fixture-actions.log'
if (-not (Test-Path $fixtureLogPath)) {
    throw 'Fixture event log was not produced'
}
$fixtureLog = Get-Content $fixtureLogPath -Raw

foreach ($marker in @(
    'UIA_NATIVE_EVENTS_INIT = PASS',
    'UIA_NATIVE_EVENTS_RUNTIME = PASS',
    'UIA_EVENT_PROPERTY_CACHE = ENABLED'
)) {
    if (-not $screenReaderLog.Contains($marker)) {
        throw "Missing screen reader runtime marker: $marker"
    }
}

$fixturePidPattern = "PID=$fixturePid\s*\|"
$fixtureUiaLines = @(
    ($screenReaderLog -split "`r?`n") |
        Where-Object { $_ -match $fixturePidPattern }
)
$fixtureUiaLines |
    Set-Content -Path (Join-Path $EvidenceDir 'fixture-uia-events.txt') -Encoding utf8

if ($fixtureUiaLines.Count -lt 1) {
    throw "No UIA event was captured from keyboard-driven fixture PID $fixturePid"
}

$expectedFocusNames = @(
    'Blind user document',
    'Enable feature',
    'Reading mode',
    'Apply changes'
)

$focusProof = [ordered]@{}
foreach ($name in $expectedFocusNames) {
    $escaped = [regex]::Escape($name)
    $matching = @(
        $fixtureUiaLines |
            Where-Object { $_ -match '^FOCUS #' -and $_ -match "Name=$escaped(?:\s*\||$)" }
    )
    $focusProof[$name] = $matching.Count
    if ($matching.Count -lt 1) {
        throw "Keyboard navigation never produced a UIA focus event for '$name'"
    }
}

$fixtureValueEvents = @(
    $fixtureUiaLines |
        Where-Object {
            $_ -match '^PROPERTY_CHANGED/UIA_30045\[ValueValue\]' -and
            $_ -match 'Name=Blind user document(?:\s*\||$)'
        }
)
if ($fixtureValueEvents.Count -lt 1) {
    throw 'WinForms editing changed the document but UIA ValueValue property fallback was not captured'
}

$fixtureToggleEvents = @(
    $fixtureUiaLines |
        Where-Object {
            $_ -match '^PROPERTY_CHANGED/UIA_30086\[ToggleToggleState\]' -and
            $_ -match 'Name=Enable feature(?:\s*\||$)'
        }
)
if ($fixtureToggleEvents.Count -lt 1) {
    throw 'Checkbox state changed but UIA ToggleToggleState property event was not captured'
}

$fixtureSelectionItemEvents = @(
    $fixtureUiaLines |
        Where-Object { $_ -match '^PROPERTY_CHANGED/UIA_30079\[SelectionItemIsSelected\]' }
)

$fixtureAssertions = @(
    '\|TEXT\|Editor\|blind user typed text',
    '\|FOCUS\|EnableFeature\|',
    '\|STATE\|EnableFeature\|True',
    '\|FOCUS\|ReadingMode\|',
    '\|STATE\|ReadingMode\|Detailed',
    '\|FOCUS\|ApplyChanges\|',
    '\|ACTIVATE\|ApplyChanges\|',
    '\|SELECTION\|Editor\|'
)
foreach ($pattern in $fixtureAssertions) {
    if ($fixtureLog -notmatch $pattern) {
        throw "Fixture did not prove expected keyboard user action: $pattern"
    }
}

$keyboardActions = @(
    Get-Content $controllerLog |
        Where-Object { $_ -match '\|KEYBOARD\|' }
)
if ($keyboardActions.Count -lt 20) {
    throw "Expected at least 20 external keyboard actions, observed $($keyboardActions.Count)"
}

$counts = [regex]::Match(
    $screenReaderLog,
    'COUNTS \| focus=(\d+) \| text_changed=(\d+) \| text_selection=(\d+) \| property_changed=(\d+)'
)
if (-not $counts.Success) {
    throw 'Unable to parse global UIA event counts including property changes'
}

if ($null -eq $nativeProbe -or -not $nativeProbe.windowAvailable -or -not $nativeProbe.keyboardInputSent) {
    throw "Real Notepad keyboard probe was not available: $($nativeProbe.error)"
}

$nativePidPattern = "PID=$($nativeProbe.pid)\s*\|"
$nativeUiaLines = @(
    ($screenReaderLog -split "`r?`n") |
        Where-Object { $_ -match $nativePidPattern }
)
$nativeUiaLines |
    Set-Content -Path (Join-Path $EvidenceDir 'native-app-uia-events.txt') -Encoding utf8

$nativeTextEvents = @(
    $nativeUiaLines |
        Where-Object { $_ -match '^TEXT_CHANGED/UIA_20015 #' }
)
if ($nativeTextEvents.Count -lt 1) {
    throw 'Real Notepad typing did not produce a UIA TextChanged event'
}

$nativeSelectionEvents = @(
    $nativeUiaLines |
        Where-Object { $_ -match '^TEXT_SELECTION_CHANGED/UIA_20014 #' }
)

$summary = [ordered]@{
    schemaVersion = 2
    platform = 'windows'
    scenario = 'blind-user-keyboard-e2e'
    pass = $true
    keyboardOnlyFunctionalInteraction = $true
    synthesizedMouseInputCalls = 0
    externalKeyboardActions = $keyboardActions.Count
    fixturePid = $fixturePid
    fixtureUiaEvents = $fixtureUiaLines.Count
    fixtureValuePropertyEvents = $fixtureValueEvents.Count
    fixtureTogglePropertyEvents = $fixtureToggleEvents.Count
    fixtureSelectionItemPropertyEvents = $fixtureSelectionItemEvents.Count
    expectedFocusNames = $expectedFocusNames
    focusEventsByAccessibleName = $focusProof
    nativeAppPid = [int]$nativeProbe.pid
    nativeAppUiaEvents = $nativeUiaLines.Count
    nativeAppTextChangedEvents = $nativeTextEvents.Count
    nativeAppTextSelectionEvents = $nativeSelectionEvents.Count
    globalFocusEvents = [int64]$counts.Groups[1].Value
    globalTextChangedEvents = [int64]$counts.Groups[2].Value
    globalTextSelectionEvents = [int64]$counts.Groups[3].Value
    globalPropertyChangedEvents = [int64]$counts.Groups[4].Value
    nvdaOfficialReference = [ordered]@{
        repository = $nvdaBaseline.repository
        branch = $nvdaBaseline.branch
        sha = $nvdaBaseline.sha
        mode = $nvdaBaseline.mode
        runtimeExecutedInThisScenario = $false
    }
    currentLimitations = @(
        'No speech/TTS output assertion yet',
        'No braille output assertion yet',
        'No NVDA-vs-Rust runtime speech comparison yet',
        'WinForms caret/selection UIA event support is not yet guaranteed',
        'Combo-box provider behavior still needs NVDA-equivalent fallback coverage'
    )
}

$summary | ConvertTo-Json -Depth 8 | Set-Content -Path $summaryPath -Encoding utf8

"BLIND_USER_KEYBOARD_E2E = PASS | keyboard_actions=$($keyboardActions.Count) | fixture_uia_events=$($fixtureUiaLines.Count) | value_events=$($fixtureValueEvents.Count) | toggle_events=$($fixtureToggleEvents.Count) | native_text_changed=$($nativeTextEvents.Count) | native_text_selection=$($nativeSelectionEvents.Count) | nvda_baseline=$($nvdaBaseline.sha)" |
    Tee-Object -FilePath (Join-Path $EvidenceDir 'result.txt')
