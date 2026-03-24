param(
    [Parameter(Mandatory = $true)]
    [string]$ActionCapture,
    [string]$IdleCapture = "",
    [string]$OutputDir = "",
    [string]$PrimaryVid = "0416",
    [string[]]$PrimaryPids = @("8040", "8041"),
    [string]$FallbackVid = "345f",
    [string[]]$FallbackPids = @("9132")
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
    return (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

function Resolve-ToolPath {
    param(
        [string]$DefaultPath,
        [string]$CommandName
    )

    if (Test-Path $DefaultPath) {
        return $DefaultPath
    }

    $fromPath = Get-Command $CommandName -ErrorAction SilentlyContinue
    if ($fromPath) {
        return $fromPath.Source
    }

    throw "$CommandName not found. Install Wireshark CLI tools."
}

function Normalize-Hex {
    param([string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return ""
    }

    $clean = $Value.Trim().ToLower()
    if ($clean.StartsWith("0x")) {
        return $clean
    }

    return "0x$clean"
}

function Build-VidPidFilter {
    param(
        [string]$Vid,
        [string[]]$Pids
    )

    $vidHex = Normalize-Hex $Vid
    $pidParts = @()
    foreach ($pidValue in $Pids) {
        $pidParts += "usb.idProduct == $(Normalize-Hex $pidValue)"
    }

    if ($pidParts.Count -eq 0) {
        return "usb.idVendor == $vidHex"
    }

    return "usb.idVendor == $vidHex && (" + ($pidParts -join " || ") + ")"
}

function Get-CaptureMeta {
    param(
        [string]$CapinfosPath,
        [string]$CapturePath
    )

    return (& $CapinfosPath $CapturePath | Out-String)
}

function Get-TargetAddresses {
    param(
        [string]$TsharkPath,
        [string]$CapturePath,
        [string]$Filter
    )

    $lines = & $TsharkPath -r $CapturePath -Y $Filter -T fields -e usb.device_address 2>$null
    return $lines |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Sort-Object -Unique
}

function Get-FrameRows {
    param(
        [string]$TsharkPath,
        [string]$CapturePath,
        [string]$Filter
    )

    $lines = & $TsharkPath -r $CapturePath -Y $Filter -T fields -E separator='|' -E quote=n -E occurrence=f -e frame.number -e frame.time_relative -e usb.device_address -e usb.endpoint_address -e usb.transfer_type -e usb.data_len -e usb.capdata -e usb.urb_status 2>$null

    $rows = @()
    foreach ($line in $lines) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }

        $parts = $line.Split("|")
        if ($parts.Count -lt 8) {
            continue
        }

        $rows += [pscustomobject]@{
            FrameNumber     = $parts[0]
            TimeRelative    = $parts[1]
            DeviceAddress   = $parts[2]
            Endpoint        = $parts[3]
            TransferType    = $parts[4]
            DataLen         = $parts[5]
            CapData         = $parts[6]
            UrbStatus       = $parts[7]
        }
    }

    return $rows
}

function Get-RequestStats {
    param(
        [string]$TsharkPath,
        [string]$CapturePath,
        [string]$Filter
    )

    $requests = & $TsharkPath -r $CapturePath -Y "$Filter && usb.setup.bRequest" -T fields -e usb.setup.bRequest 2>$null
    return $requests |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Group-Object |
        Sort-Object Count -Descending
}

function Get-Prefix {
    param([string]$Hex, [int]$NibbleCount = 16)

    if ([string]::IsNullOrWhiteSpace($Hex)) {
        return ""
    }

    if ($Hex.Length -le $NibbleCount) {
        return $Hex
    }

    return $Hex.Substring(0, $NibbleCount)
}

function To-Decimal {
    param([string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return 0
    }

    $trimmed = $Value.Trim()
    if ($trimmed.StartsWith("0x", [System.StringComparison]::OrdinalIgnoreCase)) {
        try {
            return [Convert]::ToInt32($trimmed.Substring(2), 16)
        }
        catch {
            return 0
        }
    }

    $parsed = 0
    if ([int]::TryParse($trimmed, [ref]$parsed)) {
        return $parsed
    }

    return 0
}

$repoRoot = Resolve-RepoRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "artifacts\usb-analysis"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if (-not (Test-Path $ActionCapture)) {
    throw "Action capture not found: $ActionCapture"
}

if (-not [string]::IsNullOrWhiteSpace($IdleCapture) -and -not (Test-Path $IdleCapture)) {
    throw "Idle capture not found: $IdleCapture"
}

$tsharkPath = Resolve-ToolPath -DefaultPath "C:\Program Files\Wireshark\tshark.exe" -CommandName "tshark"
$capinfosPath = Resolve-ToolPath -DefaultPath "C:\Program Files\Wireshark\capinfos.exe" -CommandName "capinfos"

$primaryFilter = Build-VidPidFilter -Vid $PrimaryVid -Pids $PrimaryPids
$fallbackFilter = Build-VidPidFilter -Vid $FallbackVid -Pids $FallbackPids

$targetAddresses = Get-TargetAddresses -TsharkPath $tsharkPath -CapturePath $ActionCapture -Filter $primaryFilter
$selectedFilter = $primaryFilter
$selectedVidLabel = "$(Normalize-Hex $PrimaryVid):" + (($PrimaryPids | ForEach-Object { Normalize-Hex $_ }) -join ",")

if (-not $targetAddresses -or $targetAddresses.Count -eq 0) {
    $targetAddresses = Get-TargetAddresses -TsharkPath $tsharkPath -CapturePath $ActionCapture -Filter $fallbackFilter
    if ($targetAddresses -and $targetAddresses.Count -gt 0) {
        $selectedFilter = $fallbackFilter
        $selectedVidLabel = "$(Normalize-Hex $FallbackVid):" + (($FallbackPids | ForEach-Object { Normalize-Hex $_ }) -join ",")
    }
}

if (-not $targetAddresses -or $targetAddresses.Count -eq 0) {
    throw "No target device addresses found in action capture for filters '$primaryFilter' or '$fallbackFilter'."
}

$addressFilterParts = @()
foreach ($addr in $targetAddresses) {
    $addressFilterParts += "usb.device_address == $addr"
}
$addressFilter = "(" + ($addressFilterParts -join " || ") + ")"

$targetDataFilter = "$addressFilter && usb.data_len > 0"
$actionRows = Get-FrameRows -TsharkPath $tsharkPath -CapturePath $ActionCapture -Filter $targetDataFilter

$idleRows = @()
if (-not [string]::IsNullOrWhiteSpace($IdleCapture)) {
    $idleRows = Get-FrameRows -TsharkPath $tsharkPath -CapturePath $IdleCapture -Filter $targetDataFilter
}

$actionRequestStats = Get-RequestStats -TsharkPath $tsharkPath -CapturePath $ActionCapture -Filter $addressFilter

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$rowsCsv = Join-Path $OutputDir "${timestamp}-meteor-target-rows.csv"
$reportPath = Join-Path $OutputDir "${timestamp}-meteor-report.md"

$actionRows | Export-Csv -Path $rowsCsv -NoTypeInformation -Encoding UTF8

$actionFrameCount = $actionRows.Count
$actionUniquePayloads = ($actionRows | Where-Object { -not [string]::IsNullOrWhiteSpace($_.CapData) } | Select-Object -ExpandProperty CapData -Unique).Count
$actionErrorFrames = ($actionRows | Where-Object { $_.UrbStatus -ne "" -and $_.UrbStatus -ne "0" }).Count

$endpointStats = $actionRows |
    Group-Object Endpoint |
    Sort-Object Count -Descending

$prefixStats = $actionRows |
    Where-Object { -not [string]::IsNullOrWhiteSpace($_.CapData) } |
    ForEach-Object {
        [pscustomobject]@{
            Prefix = Get-Prefix -Hex $_.CapData
        }
    } |
    Group-Object Prefix |
    Sort-Object Count -Descending |
    Select-Object -First 15

$actionPayloadMap = @{}
foreach ($row in $actionRows) {
    if ([string]::IsNullOrWhiteSpace($row.CapData)) {
        continue
    }

    if (-not $actionPayloadMap.ContainsKey($row.CapData)) {
        $actionPayloadMap[$row.CapData] = 0
    }
    $actionPayloadMap[$row.CapData]++
}

$idlePayloadMap = @{}
foreach ($row in $idleRows) {
    if ([string]::IsNullOrWhiteSpace($row.CapData)) {
        continue
    }

    if (-not $idlePayloadMap.ContainsKey($row.CapData)) {
        $idlePayloadMap[$row.CapData] = 0
    }
    $idlePayloadMap[$row.CapData]++
}

$newInAction = @()
foreach ($key in $actionPayloadMap.Keys) {
    if (-not $idlePayloadMap.ContainsKey($key)) {
        $newInAction += [pscustomobject]@{
            CapData = $key
            Count   = $actionPayloadMap[$key]
            Prefix  = Get-Prefix -Hex $key
        }
    }
}
$newInAction = $newInAction | Sort-Object Count -Descending | Select-Object -First 20

$outEndpointFrames = $actionRows | Where-Object {
    $epValue = To-Decimal $_.Endpoint
    ($epValue -band 0x80) -eq 0
}
$inEndpointFrames = $actionRows | Where-Object {
    $epValue = To-Decimal $_.Endpoint
    ($epValue -band 0x80) -ne 0
}

$report = New-Object System.Collections.Generic.List[string]

$report.Add("# USB Meteor Traffic Analysis")
$report.Add("")
$report.Add("Generated: $(Get-Date -Format o)")
$report.Add("")
$report.Add("## Inputs")
$report.Add("- Action capture: $ActionCapture")
if (-not [string]::IsNullOrWhiteSpace($IdleCapture)) {
    $report.Add("- Idle capture: $IdleCapture")
}
$report.Add("- Selected target filter: $selectedFilter")
$report.Add("- Selected VID/PID group: $selectedVidLabel")
$report.Add("- Target device addresses: $($targetAddresses -join ', ')")
$report.Add("")

$report.Add("## Capture Metadata (Action)")
$report.Add('```')
$report.Add((Get-CaptureMeta -CapinfosPath $capinfosPath -CapturePath $ActionCapture).TrimEnd())
$report.Add('```')
$report.Add("")

if (-not [string]::IsNullOrWhiteSpace($IdleCapture)) {
    $report.Add("## Capture Metadata (Idle)")
    $report.Add('```')
    $report.Add((Get-CaptureMeta -CapinfosPath $capinfosPath -CapturePath $IdleCapture).TrimEnd())
    $report.Add('```')
    $report.Add("")
}

$report.Add("## Target Traffic Summary")
$report.Add("- Target frames with payload (usb.data_len > 0): $actionFrameCount")
$report.Add("- Unique payload blobs: $actionUniquePayloads")
$report.Add("- Target URB status != 0 frames: $actionErrorFrames")
$report.Add("- OUT endpoint payload frames (host -> device): $($outEndpointFrames.Count)")
$report.Add("- IN endpoint payload frames (device -> host): $($inEndpointFrames.Count)")
$report.Add("")

$report.Add("## Endpoint Distribution")
foreach ($entry in $endpointStats) {
    $report.Add("- Endpoint $($entry.Name): $($entry.Count) frames")
}
$report.Add("")

$report.Add("## Setup Request Distribution (target addresses)")
if ($actionRequestStats -and $actionRequestStats.Count -gt 0) {
    foreach ($entry in $actionRequestStats) {
        $report.Add("- bRequest=$($entry.Name): $($entry.Count)")
    }
}
else {
    $report.Add("- No setup requests seen for target addresses.")
}
$report.Add("")

$report.Add("## Dominant Payload Prefixes (first 8 bytes)")
foreach ($entry in $prefixStats) {
    $report.Add("- Prefix $($entry.Name): $($entry.Count) frames")
}
$report.Add("")

if (-not [string]::IsNullOrWhiteSpace($IdleCapture)) {
    $report.Add("## New Payloads in Action vs Idle")
    if ($newInAction.Count -eq 0) {
        $report.Add("- No action-only payloads found (exact payload comparison).")
    }
    else {
        foreach ($entry in $newInAction) {
            $report.Add("- Prefix $($entry.Prefix) count=$($entry.Count) payload=$($entry.CapData)")
        }
    }
    $report.Add("")
}

$report.Add("## Interpretation (Best-Effort)")
$report.Add("- Endpoint 0x00 / 0x80 traffic with setup requests (e.g. bRequest 6/9) usually indicates USB enumeration/control, not effect streaming.")
$report.Add("- Repeating OUT payloads on non-control endpoints are likely effect/frame transport from host to dongle.")
$report.Add("- If action-only payloads appear right after Apply, those bytes are the strongest Meteor candidates.")
$report.Add("- Prefix-level repetition suggests packet families (header/chunk/control). Field-level meaning still needs controlled parameter sweeps (speed/color/direction).")
$report.Add("")

$report.Add("## Artifacts")
$report.Add("- Target frame export CSV: $rowsCsv")
$report.Add("- This report: $reportPath")

$report | Set-Content -Path $reportPath -Encoding UTF8

Write-Host "Analysis complete."
Write-Host "Report: $reportPath"
Write-Host "CSV:    $rowsCsv"
