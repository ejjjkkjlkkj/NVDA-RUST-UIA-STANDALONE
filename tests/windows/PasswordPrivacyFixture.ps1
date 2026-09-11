param(
    [Parameter(Mandatory = $true)]
    [string]$EvidenceDir
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$readyPath = Join-Path $EvidenceDir 'password-ready.json'
$eventLog = Join-Path $EvidenceDir 'password-fixture.log'

function Write-PrivacyEvent {
    param(
        [Parameter(Mandatory = $true)][string]$Kind,
        [Parameter(Mandatory = $true)][string]$Value
    )
    "$(Get-Date -Format o)|$Kind|$Value" | Add-Content -Path $eventLog -Encoding utf8
}

Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'Password Privacy E2E Fixture'
$form.Name = 'PasswordPrivacyE2EFixture'
$form.AccessibleName = 'Password Privacy E2E Fixture'
$form.StartPosition = 'CenterScreen'
$form.TopMost = $true
$form.Width = 520
$form.Height = 180

$label = New-Object System.Windows.Forms.Label
$label.Text = 'Password:'
$label.AutoSize = $true
$label.SetBounds(20, 24, 100, 24)

$password = New-Object System.Windows.Forms.TextBox
$password.Name = 'AccountPassword'
$password.AccessibleName = 'Account password'
$password.AccessibleDescription = 'Protected password field used to verify screen reader privacy'
$password.UseSystemPasswordChar = $true
$password.TabIndex = 0
$password.SetBounds(120, 20, 320, 30)

$close = New-Object System.Windows.Forms.Button
$close.Name = 'ClosePrivacyFixture'
$close.AccessibleName = 'Close privacy fixture'
$close.Text = 'Close'
$close.TabIndex = 1
$close.SetBounds(20, 75, 120, 34)

$form.Controls.AddRange(@($label, $password, $close))

$password.Add_GotFocus({
    Write-PrivacyEvent -Kind 'FOCUS' -Value 'AccountPassword|protected=true'
})
$password.Add_TextChanged({
    Write-PrivacyEvent -Kind 'PASSWORD_LENGTH' -Value ([string]$password.TextLength)
})
$password.Add_KeyUp({
    if ($password.SelectionLength -gt 0) {
        Write-PrivacyEvent -Kind 'SELECTION_LENGTH' -Value ([string]$password.SelectionLength)
    }
})
$close.Add_Click({ $form.Close() })
$form.Add_FormClosed({ Write-PrivacyEvent -Kind 'CLOSE' -Value 'Window' })

$failsafe = New-Object System.Windows.Forms.Timer
$failsafe.Interval = 25000
$failsafe.Add_Tick({
    $failsafe.Stop()
    Write-PrivacyEvent -Kind 'FAILSAFE_CLOSE' -Value 'Window'
    $form.Close()
})

$form.Show()
[System.Windows.Forms.Application]::DoEvents()
$form.Activate()
$form.ActiveControl = $password

$ready = [ordered]@{
    schemaVersion = 1
    pid = $PID
    windowHandle = $form.Handle.ToInt64()
    accessibleName = 'Account password'
    protected = $true
}
$ready | ConvertTo-Json -Depth 3 | Set-Content -Path $readyPath -Encoding utf8
Write-PrivacyEvent -Kind 'READY' -Value "pid=$PID;hwnd=$($form.Handle.ToInt64())"
$failsafe.Start()

[System.Windows.Forms.Application]::Run($form)
