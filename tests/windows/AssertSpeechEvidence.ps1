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
    'SCREEN_READER_PIPELINE = UIA_TO_SPEECH',
    'SPEECH_REPLACE_POLICY = INTERACTIVE_LATEST_WINS',
    'SPEECH_FLUSH = PASS'
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

$queueCounts = [regex]::Match(
    $log,
    'SPEECH_QUEUE_COUNTS \| enqueue_failures=(\d+) \| replaced=(\d+)'
)
if (-not $queueCounts.Success) {
    throw 'Unable to parse SPEECH_QUEUE_COUNTS including replacement count'
}

$requested = [int64]$counts.Groups[1].Value
$output = [int64]$counts.Groups[2].Value
$failed = [int64]$counts.Groups[3].Value
$enqueueFailures = [int64]$queueCounts.Groups[1].Value
$replaced = [int64]$queueCounts.Groups[2].Value
$handled = $output + $replaced + $failed

if ($requested -lt 4) {
    throw "Too few speech requests for the keyboard journey: $requested"
}
if ($output -lt 1) {
    throw 'No interactive speech request reached MediaPlayer.Play()'
}
if ($handled -ne $requested) {
    throw "Speech requests were lost: requested=$requested output=$output replaced=$replaced failed=$failed"
}
if ($failed -ne 0) {
    throw "Speech pipeline reported failures: $failed"
}
if ($enqueueFailures -ne 0) {
    throw "Speech queue reported enqueue/flush failures: $enqueueFailures"
}

$expectedAccessibleNames = @(
    'Blind user document',
    'Enable feature',
    'Reading mode',
    'Apply changes'
)

$requestProof = [ordered]@{}
foreach ($name in $expectedAccessibleNames) {
    $escaped = [regex]::Escape($name)
    $matches = @(
        ($log -split "`r?`n") |
            Where-Object {
                $_ -match '^SPEECH_REQUEST #\d+' -and
                $_ -match "\|\s+$escaped(?:,|$)"
            }
    )
    $requestProof[$name] = $matches.Count
    if ($matches.Count -lt 1) {
        throw "No speech request was generated for focused element '$name'"
    }
}

$summary = Get-Content $SummaryPath -Raw | ConvertFrom-Json
$summary | Add-Member -NotePropertyName speechPipelineAsserted -NotePropertyValue $true -Force
$summary | Add-Member -NotePropertyName speechRequests -NotePropertyValue $requested -Force
$summary | Add-Member -NotePropertyName speechOutputsStarted -NotePropertyValue $output -Force
$summary | Add-Member -NotePropertyName speechReplacedAsStale -NotePropertyValue $replaced -Force
$summary | Add-Member -NotePropertyName speechFailures -NotePropertyValue $failed -Force
$summary | Add-Member -NotePropertyName speechQueueFailures -NotePropertyValue $enqueueFailures -Force
$summary | Add-Member -NotePropertyName speechFocusRequestProof -NotePropertyValue ([pscustomobject]$requestProof) -Force
$summary | Add-Member -NotePropertyName physicalAudioDeviceAsserted -NotePropertyValue $false -Force

if ($null -ne $summary.currentLimitations) {
    $summary.currentLimitations = @(
        $summary.currentLimitations |
            Where-Object { $_ -ne 'No speech/TTS output assertion yet' }
    )
}

$summary | ConvertTo-Json -Depth 10 | Set-Content -Path $SummaryPath -Encoding utf8

"SPEECH_PIPELINE_E2E = PASS | requested=$requested | output=$output | replaced=$replaced | failed=$failed | queue_failures=$enqueueFailures | physical_audio_asserted=false"
