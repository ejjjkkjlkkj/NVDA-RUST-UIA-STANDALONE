param(
    [Parameter(Mandatory = $true)]
    [string]$ScreenReaderLog,

    [Parameter(Mandatory = $true)]
    [string]$NativeProbePath,

    [Parameter(Mandatory = $true)]
    [string]$SummaryPath
)

$ErrorActionPreference = 'Stop'

foreach ($path in @($ScreenReaderLog, $NativeProbePath, $SummaryPath)) {
    if (-not (Test-Path $path)) {
        throw "Required evidence file missing: $path"
    }
}

$log = Get-Content $ScreenReaderLog -Raw
if (-not $log.Contains('TEXT_PATTERN2_CARET_SELECTION = ENABLED')) {
    throw 'TextPattern2 runtime marker is missing'
}

$native = Get-Content $NativeProbePath -Raw | ConvertFrom-Json
if (-not $native.windowAvailable -or [int]$native.pid -le 0) {
    throw "Native Notepad probe unavailable: $($native.error)"
}

$pidPattern = "PID=$([int]$native.pid)\s*\|"
$nativePatternLines = @(
    ($log -split "`r?`n") |
        Where-Object { $_ -match '^TEXT_PATTERN2 #' -and $_ -match $pidPattern }
)

if ($nativePatternLines.Count -lt 1) {
    throw "No TextPattern2 evidence was captured for Notepad PID $($native.pid)"
}

$successful = @(
    $nativePatternLines |
        Where-Object { $_ -match '\| status=pass \|' }
)
if ($successful.Count -lt 1) {
    throw 'Notepad emitted selection events but no successful TextPattern2 caret retrieval'
}

$activeCaret = @(
    $successful |
        Where-Object { $_ -match '\| caret_active=true \|' }
)
if ($activeCaret.Count -lt 1) {
    throw 'TextPattern2 GetCaretRange never reported an active caret in focused Notepad'
}

$selectedText = @(
    $successful |
        Where-Object {
            $_ -match '\| selection_text=(.+)$' -and
            $Matches[1] -ne '<none>' -and
            $Matches[1] -ne '<unavailable>' -and
            $Matches[1] -ne '<redacted>'
        }
)
if ($selectedText.Count -lt 1) {
    throw 'TextPattern2 GetSelection never returned non-empty selected text after Shift+Left in Notepad'
}

$counts = [regex]::Match(
    $log,
    'TEXT_PATTERN2_COUNTS \| pass=(\d+) \| caret_active=(\d+) \| selection_text=(\d+) \| protected=(\d+)'
)
if (-not $counts.Success) {
    throw 'Unable to parse TEXT_PATTERN2_COUNTS'
}

$summary = Get-Content $SummaryPath -Raw | ConvertFrom-Json
$summary | Add-Member -NotePropertyName textPattern2Asserted -NotePropertyValue $true -Force
$summary | Add-Member -NotePropertyName nativeTextPattern2Events -NotePropertyValue $nativePatternLines.Count -Force
$summary | Add-Member -NotePropertyName nativeActiveCaretEvents -NotePropertyValue $activeCaret.Count -Force
$summary | Add-Member -NotePropertyName nativeSelectionTextEvents -NotePropertyValue $selectedText.Count -Force
$summary | Add-Member -NotePropertyName textPattern2GlobalPass -NotePropertyValue ([int64]$counts.Groups[1].Value) -Force
$summary | Add-Member -NotePropertyName caretActiveGlobal -NotePropertyValue ([int64]$counts.Groups[2].Value) -Force
$summary | Add-Member -NotePropertyName selectionTextGlobal -NotePropertyValue ([int64]$counts.Groups[3].Value) -Force
$summary | Add-Member -NotePropertyName protectedTextPatternEvents -NotePropertyValue ([int64]$counts.Groups[4].Value) -Force

if ($null -ne $summary.currentLimitations) {
    $summary.currentLimitations = @(
        $summary.currentLimitations |
            Where-Object { $_ -ne 'WinForms caret/selection UIA event support is not yet guaranteed' }
    )
}

$summary | ConvertTo-Json -Depth 10 | Set-Content -Path $SummaryPath -Encoding utf8
$nativePatternLines | Set-Content -Path (Join-Path (Split-Path $SummaryPath -Parent) 'native-textpattern2-events.txt') -Encoding utf8

"TEXT_PATTERN2_E2E = PASS | pid=$($native.pid) | pattern_events=$($nativePatternLines.Count) | active_caret=$($activeCaret.Count) | selected_text=$($selectedText.Count)"
