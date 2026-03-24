param(
    [string]$OutputDir = "",
    [string]$Interface = "",
    [int]$IdleSeconds = 90,
    [int]$ActionSeconds = 120,
    [switch]$NoPrompt
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
    return (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

function Resolve-TsharkPath {
    $candidate = "C:\Program Files\Wireshark\tshark.exe"
    if (Test-Path $candidate) {
        return $candidate
    }

    $fromPath = (Get-Command tshark -ErrorAction SilentlyContinue)
    if ($fromPath) {
        return $fromPath.Source
    }

    throw "tshark.exe not found. Install Wireshark with CLI tools first."
}

function Get-UsbPcapInterface {
    param(
        [string]$TsharkPath,
        [string]$Requested
    )

    if (-not [string]::IsNullOrWhiteSpace($Requested)) {
        return $Requested
    }

    $lines = & $TsharkPath -D
    $match = $lines | Where-Object { $_ -match "USBPcap" } | Select-Object -First 1
    if (-not $match) {
        throw "No USBPcap interface found. Ensure USBPcap is installed and enabled."
    }

    if ($match -match "^(\d+)\.") {
        return $matches[1]
    }

    throw "Could not parse tshark interface line: $match"
}

function Show-TargetDevices {
    $devices = Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue |
        Where-Object {
            $_.InstanceId -match "VID_0416&PID_8040" -or
            $_.InstanceId -match "VID_0416&PID_8041" -or
            $_.InstanceId -match "VID_345F&PID_9132"
        }

    if (-not $devices) {
        Write-Host "No matching target USB devices were detected right now (0416:8040/8041, 345F:9132)."
        return
    }

    Write-Host "Detected candidate target devices:"
    $devices | Select-Object Status, Class, FriendlyName, InstanceId | Format-Table -AutoSize | Out-Host
}

function Start-Capture {
    param(
        [string]$TsharkPath,
        [string]$InterfaceValue,
        [string]$OutputPath,
        [int]$DurationSeconds
    )

    Write-Host "Starting capture: $OutputPath"
    & $TsharkPath -i $InterfaceValue -w $OutputPath -a "duration:$DurationSeconds"
    if ($LASTEXITCODE -ne 0) {
        throw "tshark capture failed with exit code $LASTEXITCODE"
    }
}

$repoRoot = Resolve-RepoRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "artifacts\usb-captures"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$tsharkPath = Resolve-TsharkPath
$interfaceValue = Get-UsbPcapInterface -TsharkPath $tsharkPath -Requested $Interface

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$idlePath = Join-Path $OutputDir "${timestamp}-idle-baseline.pcapng"
$actionPath = Join-Path $OutputDir "${timestamp}-meteor-action.pcapng"
$timelinePath = Join-Path $OutputDir "${timestamp}-action-timeline.txt"

Write-Host "Using tshark: $tsharkPath"
Write-Host "Using interface: $interfaceValue"
Show-TargetDevices

if (-not $NoPrompt) {
    Write-Host ""
    Write-Host "Preparation: keep L-Connect 3 open, target fans already on Meteor (light blue)."
    Write-Host "Press ENTER to start idle baseline capture ($IdleSeconds seconds)."
    [void](Read-Host)
}

Start-Capture -TsharkPath $tsharkPath -InterfaceValue $interfaceValue -OutputPath $idlePath -DurationSeconds $IdleSeconds

if (-not $NoPrompt) {
    Write-Host ""
    Write-Host "Action capture plan ($ActionSeconds seconds total):"
    Write-Host "  T+10s  -> click Apply for Meteor in L-Connect 3"
    Write-Host "  T+40s  -> do nothing (quiet window)"
    Write-Host "  T+70s  -> click Apply again"
    Write-Host "Press ENTER to start action capture now."
    [void](Read-Host)
}

$startTime = Get-Date
"CaptureStart=$($startTime.ToString("o"))" | Set-Content -Path $timelinePath -Encoding UTF8
"T+10s: Click Apply for Meteor" | Add-Content -Path $timelinePath -Encoding UTF8
"T+40s: Quiet window" | Add-Content -Path $timelinePath -Encoding UTF8
"T+70s: Click Apply again" | Add-Content -Path $timelinePath -Encoding UTF8

Start-Job -ScriptBlock {
    param($Path, $InterfaceArg, $OutputArg, $DurationArg)
    & $Path -i $InterfaceArg -w $OutputArg -a "duration:$DurationArg" | Out-Null
    return $LASTEXITCODE
} -ArgumentList $tsharkPath, $interfaceValue, $actionPath, $ActionSeconds | Out-Null

Start-Sleep -Seconds 10
Write-Host "[Cue] T+10s: Click Apply for Meteor now."
Start-Sleep -Seconds 30
Write-Host "[Cue] T+40s: Quiet window (no clicks)."
Start-Sleep -Seconds 30
Write-Host "[Cue] T+70s: Click Apply for Meteor again."

$remaining = [Math]::Max(0, $ActionSeconds - 70)
if ($remaining -gt 0) {
    Start-Sleep -Seconds $remaining
}

$job = Get-Job | Sort-Object Id -Descending | Select-Object -First 1
if ($job) {
    $result = Receive-Job -Id $job.Id -Wait
    Remove-Job -Id $job.Id -Force | Out-Null
    if ($result -ne 0) {
        throw "Action capture job failed with exit code $result"
    }
}

Write-Host ""
Write-Host "Capture complete."
Write-Host "Idle capture:   $idlePath"
Write-Host "Action capture: $actionPath"
Write-Host "Timeline file:  $timelinePath"
