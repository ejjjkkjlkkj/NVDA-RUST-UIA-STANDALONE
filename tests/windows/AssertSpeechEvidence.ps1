param(
    [Parameter(Mandatory = $true)]
    [string]$ScreenReaderLog,

    [Parameter(Mandatory = $true)]
    [string]$SummaryPath
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $ScreenReaderLog)) {
    throw "Screen reader stdout log not found: $ScreenReaderLog"
}
if (-not (Test-Path $SummaryPath)) {
    throw "Blind-user summary not found: $SummaryPath"
}

$log = Get-Content $ScreenReaderLog -Raw

foreach ($marker in @(
    'SPEECH_OUTPUT_INIT = PASS',
    'SCREEN_READER_PIPELINE = UIA_TO_SPEECH'
)) {
    if (-not $log.Contains($marker)) {
        throw "Missing speech runtime marker: $marker"
    }
}

$counts = [regex]::Match(
    $log,
    'SPEECH_COUNTS \| requested=(\d+) \| output=(\d+) \| failed=(\d+)'
)
if (-not $counts.Success) {
    throw 'Unable to parse SPEECH_COUNTS from screen reader output'
}

$requested = [int64]$counts.Groups[1].Value
$output = [int64]$counts.Groups[2].Value
$failed = [int64]$counts.Groups[3].Value

if ($requested -lt 4) {
    throw "Too few speech requests for the keyboard journey: $requested"
}
if ($output -ne $requested) {
    throw "Not every speech request reached MediaPlayer.Play(): requested=$requested output=$output"
}
if ($failed -ne 0) {
    throw "Speech pipeline reported failures: $failed"
}

$expectedAccessibleNames = @(
    'Blind user document',
    'Enable feature',
    'Reading mode',
    'Apply changes'
)

$speechProof = [ordered]@{}
foreach ($name in $expectedAccessibleNames) {
    $escaped = [regex]::Escape($name)
    $matches = @(
        ($log -split "`r?`n") |
            Where-Object {
                $_ -match '^SPEECH_OUTPUT #\d+ = PASS \|' -and
                $_ -match "\|\s+$escaped(?:,|$)"
            }
    )
    $speechProof[$name] = $matches.Count
    if ($matches.Count -lt 1) {
        throw "No successful speech output was logged for focused element '$name'"
    }
}

$summary = Get-Content $SummaryPath -Raw | ConvertFrom-Json
$summary | Add-Member -NotePropertyName speechPipelineAsserted -NotePropertyValue $true -Force
$summary | Add-Member -NotePropertyName speechRequests -NotePropertyValue $requested -Force
$summary | Add-Member -NotePropertyName speechOutputsStarted -NotePropertyValue $output -Force
$summary | Add-Member -NotePropertyName speechFailures -NotePropertyValue $failed -Force
$summary | Add-Member -NotePropertyName speechFocusProof -NotePropertyValue ([pscustomobject]$speechProof) -Force
$summary | Add-Member -NotePropertyName physicalAudioDeviceAsserted -NotePropertyValue $false -Force

if ($null -ne $summary.currentLimitations) {
    $summary.currentLimitations = @(
        $summary.currentLimitations |
            Where-Object { $_ -ne 'No speech/TTS output assertion yet' }
    )
}

$summary | ConvertTo-Json -Depth 10 | Set-Content -Path $SummaryPath -Encoding utf8

"SPEECH_PIPELINE_E2E = PASS | requested=$requested | output=$output | failed=$failed | physical_audio_asserted=false"
