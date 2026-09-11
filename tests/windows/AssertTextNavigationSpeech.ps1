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
        throw "Required text-navigation speech evidence is missing: $path"
    }
}

$log = Get-Content $ScreenReaderLog -Raw
if (-not $log.Contains('TEXT_NAVIGATION_SPEECH = ENABLED')) {
    throw 'Text-navigation speech runtime marker is missing'
}

$native = Get-Content $NativeProbePath -Raw | ConvertFrom-Json
if (-not $native.windowAvailable -or [int]$native.pid -le 0) {
    throw "Notepad probe unavailable for text-navigation speech: $($native.error)"
}

$pidPattern = "PID=$([int]$native.pid)\s*\|"
$selectionSpeechEvents = @(
    ($log -split "`r?`n") |
        Where-Object {
            $_ -match '^TEXT_PATTERN2 #' -and
            $_ -match $pidPattern -and
            $_ -match '\| speech_kind=selection \|' -and
            $_ -match '\| speech_text=robe\s*$'
        }
)
if ($selectionSpeechEvents.Count -lt 1) {
    throw 'Notepad selection of the final four characters was not converted into selection speech'
}

$speechOutputs = @(
    ($log -split "`r?`n") |
        Where-Object { $_ -match '^SPEECH_OUTPUT #\d+ = PASS \| robe\s*$' }
)
if ($speechOutputs.Count -lt 1) {
    throw 'Selection speech was requested but no successful speech output for "robe" was logged'
}

$counts = [regex]::Match(
    $log,
    'TEXT_NAV_SPEECH_COUNTS \| caret_character=(\d+) \| selection=(\d+)'
)
if (-not $counts.Success) {
    throw 'Unable to parse TEXT_NAV_SPEECH_COUNTS'
}
$caretSpeech = [int64]$counts.Groups[1].Value
$selectionSpeech = [int64]$counts.Groups[2].Value
if ($selectionSpeech -lt 1) {
    throw 'Global selection speech count is zero'
}

$summary = Get-Content $SummaryPath -Raw | ConvertFrom-Json
$summary | Add-Member -NotePropertyName textNavigationSpeechAsserted -NotePropertyValue $true -Force
$summary | Add-Member -NotePropertyName textNavigationSpeechScope -NotePropertyValue 'native-notepad-selection' -Force
$summary | Add-Member -NotePropertyName caretCharacterSpeechCount -NotePropertyValue $caretSpeech -Force
$summary | Add-Member -NotePropertyName selectionSpeechCount -NotePropertyValue $selectionSpeech -Force
$summary | Add-Member -NotePropertyName nativeSelectionSpeechProof -NotePropertyValue $selectionSpeechEvents.Count -Force
$summary | ConvertTo-Json -Depth 10 | Set-Content -Path $SummaryPath -Encoding utf8

"TEXT_NAVIGATION_SPEECH_E2E = PASS | scope=native-notepad-selection | selection_outputs=$($speechOutputs.Count) | selection_count=$selectionSpeech | caret_character_count=$caretSpeech"
