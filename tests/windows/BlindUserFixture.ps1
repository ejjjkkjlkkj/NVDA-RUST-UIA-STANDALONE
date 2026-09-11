param(
    [Parameter(Mandatory = $true)]
    [string]$EvidenceDir
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$eventLog = Join-Path $EvidenceDir 'fixture-actions.log'
$readyPath = Join-Path $EvidenceDir 'fixture-ready.json'
$bootstrapLog = Join-Path $EvidenceDir 'fixture-bootstrap.log'

function Write-Bootstrap {
    param([Parameter(Mandatory = $true)][string]$Stage)
    "$(Get-Date -Format o)|$Stage|pid=$PID" | Add-Content -Path $bootstrapLog -Encoding utf8
}

function Write-FixtureEvent {
    param(
        [Parameter(Mandatory = $true)][string]$Kind,
        [Parameter(Mandatory = $true)][string]$Name,
        [string]$Value = ''
    )

    $safeValue = $Value -replace "`r", '<CR>' -replace "`n", '<LF>'
    "$(Get-Date -Format o)|$Kind|$Name|$safeValue" |
        Add-Content -Path $eventLog -Encoding utf8
}

Write-Bootstrap -Stage 'START'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()
Write-Bootstrap -Stage 'WINFORMS_READY'

$form = New-Object System.Windows.Forms.Form
$form.Text = 'Blind User Keyboard E2E Fixture'
$form.Name = 'BlindUserKeyboardE2EFixture'
$form.AccessibleName = 'Blind User Keyboard E2E Fixture'
$form.StartPosition = 'CenterScreen'
$form.TopMost = $true
$form.Width = 640
$form.Height = 300
$form.KeyPreview = $false

$heading = New-Object System.Windows.Forms.Label
$heading.Text = 'Keyboard-only accessibility journey'
$heading.Name = 'JourneyHeading'
$heading.AccessibleName = 'Keyboard-only accessibility journey'
$heading.AutoSize = $true
$heading.SetBounds(20, 20, 560, 24)

$editor = New-Object System.Windows.Forms.TextBox
$editor.Name = 'BlindUserEditor'
$editor.AccessibleName = 'Blind user document'
$editor.AccessibleDescription = 'Editable text used by the keyboard-only screen reader test'
$editor.Text = 'initial document'
$editor.TabIndex = 0
$editor.SetBounds(20, 60, 560, 30)

$check = New-Object System.Windows.Forms.CheckBox
$check.Name = 'EnableFeature'
$check.AccessibleName = 'Enable feature'
$check.Text = 'Enable feature'
$check.TabIndex = 1
$check.SetBounds(20, 105, 220, 30)

$mode = New-Object System.Windows.Forms.ComboBox
$mode.Name = 'ReadingMode'
$mode.AccessibleName = 'Reading mode'
$mode.DropDownStyle = [System.Windows.Forms.ComboBoxStyle]::DropDownList
[void]$mode.Items.Add('Standard')
[void]$mode.Items.Add('Detailed')
$mode.SelectedIndex = 0
$mode.TabIndex = 2
$mode.SetBounds(260, 105, 180, 30)

$apply = New-Object System.Windows.Forms.Button
$apply.Name = 'ApplyChanges'
$apply.AccessibleName = 'Apply changes'
$apply.Text = 'Apply changes'
$apply.TabIndex = 3
$apply.SetBounds(20, 155, 180, 36)

$status = New-Object System.Windows.Forms.Label
$status.Name = 'JourneyStatus'
$status.AccessibleName = 'Journey status'
$status.Text = 'Not applied'
$status.AutoSize = $true
$status.SetBounds(220, 163, 360, 28)

$form.Controls.AddRange(@($heading, $editor, $check, $mode, $apply, $status))
Write-Bootstrap -Stage 'CONTROLS_READY'

$editor.Add_GotFocus({ Write-FixtureEvent -Kind 'FOCUS' -Name 'Editor' -Value $editor.Text })
$editor.Add_TextChanged({ Write-FixtureEvent -Kind 'TEXT' -Name 'Editor' -Value $editor.Text })
$editor.Add_KeyUp({
    if ($editor.SelectionLength -gt 0) {
        Write-FixtureEvent -Kind 'SELECTION' -Name 'Editor' -Value "$($editor.SelectionStart):$($editor.SelectionLength)"
    }
})

$check.Add_GotFocus({ Write-FixtureEvent -Kind 'FOCUS' -Name 'EnableFeature' -Value ([string]$check.Checked) })
$check.Add_CheckedChanged({ Write-FixtureEvent -Kind 'STATE' -Name 'EnableFeature' -Value ([string]$check.Checked) })

$mode.Add_GotFocus({ Write-FixtureEvent -Kind 'FOCUS' -Name 'ReadingMode' -Value ([string]$mode.SelectedItem) })
$mode.Add_SelectedIndexChanged({ Write-FixtureEvent -Kind 'STATE' -Name 'ReadingMode' -Value ([string]$mode.SelectedItem) })

$apply.Add_GotFocus({ Write-FixtureEvent -Kind 'FOCUS' -Name 'ApplyChanges' })
$apply.Add_Click({
    $status.Text = "Applied: $($editor.Text) | enabled=$($check.Checked) | mode=$($mode.SelectedItem)"
    Write-FixtureEvent -Kind 'ACTIVATE' -Name 'ApplyChanges' -Value $status.Text
})

$form.Add_FormClosed({
    Write-FixtureEvent -Kind 'CLOSE' -Name 'Window'
})

$failsafe = New-Object System.Windows.Forms.Timer
$failsafe.Interval = 35000
$failsafe.Add_Tick({
    $failsafe.Stop()
    Write-FixtureEvent -Kind 'FAILSAFE_CLOSE' -Name 'Window'
    $form.Close()
})

# Do not depend on the Shown event to publish readiness. Hosted Windows runners can
# create the process before the first GUI event is delivered. Create the HWND and
# readiness evidence synchronously, then enter the message loop.
Write-Bootstrap -Stage 'SHOW_BEGIN'
$form.Show()
[System.Windows.Forms.Application]::DoEvents()
$form.Activate()
$form.ActiveControl = $editor
$editor.SelectAll()
Write-Bootstrap -Stage "WINDOW_CREATED|hwnd=$($form.Handle.ToInt64())"

$ready = [ordered]@{
    schemaVersion = 2
    pid = $PID
    windowHandle = $form.Handle.ToInt64()
    windowTitle = $form.Text
    initialControl = 'Blind user document'
    controls = @(
        'Blind user document',
        'Enable feature',
        'Reading mode',
        'Apply changes'
    )
}
$ready | ConvertTo-Json -Depth 4 | Set-Content -Path $readyPath -Encoding utf8
Write-FixtureEvent -Kind 'READY' -Name 'Window' -Value "PID=$PID;HWND=$($form.Handle.ToInt64())"
Write-Bootstrap -Stage 'READY_WRITTEN'
$failsafe.Start()

[System.Windows.Forms.Application]::Run($form)
Write-Bootstrap -Stage 'EXIT'
