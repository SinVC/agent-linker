[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Version,

    [Parameter(Mandatory = $true)]
    [string] $AglinkExe,

    [Parameter(Mandatory = $true)]
    [string] $BrokerDir,

    [Parameter(Mandatory = $true)]
    [string] $OutDir,

    [string] $BrokerSourceUrl = "",

    [string] $BrokerSha256 = "",

    [switch] $Portable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-FullPath([string] $Path) {
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))
}

$versionName = $Version
if ($versionName.StartsWith("v")) {
    $versionName = $versionName.Substring(1)
}

$aglinkPath = Resolve-FullPath $AglinkExe
$brokerPath = Resolve-FullPath $BrokerDir
$outPath = Resolve-FullPath $OutDir

if (-not (Test-Path -LiteralPath $aglinkPath -PathType Leaf)) {
    throw "aglink executable not found: $aglinkPath"
}

$brokerExe = Join-Path $brokerPath "win-symlinks-broker.exe"
if (-not (Test-Path -LiteralPath $brokerExe -PathType Leaf)) {
    throw "win-symlinks-broker.exe not found under broker directory: $brokerPath"
}

New-Item -ItemType Directory -Force -Path $outPath | Out-Null

$suffix = if ($Portable) { "-portable" } else { "" }
$archiveName = "aglink-v$versionName-x86_64-pc-windows-msvc$suffix.zip"
$archivePath = Join-Path $outPath $archiveName
$stageRoot = Join-Path $outPath "stage-windows$suffix"

if (Test-Path -LiteralPath $stageRoot) {
    Remove-Item -LiteralPath $stageRoot -Recurse -Force
}
if (Test-Path -LiteralPath $archivePath) {
    Remove-Item -LiteralPath $archivePath -Force
}

New-Item -ItemType Directory -Force -Path $stageRoot | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageRoot "broker") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageRoot "scripts") | Out-Null

Copy-Item -LiteralPath $aglinkPath -Destination (Join-Path $stageRoot "aglink.exe")
Copy-Item -Path (Join-Path $brokerPath "*") -Destination (Join-Path $stageRoot "broker") -Recurse
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "install-broker.ps1") -Destination (Join-Path $stageRoot "scripts")
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "uninstall-broker.ps1") -Destination (Join-Path $stageRoot "scripts")

if ($Portable) {
    New-Item -ItemType File -Force -Path (Join-Path $stageRoot "agent-linker.db") | Out-Null
}

$receipt = [ordered]@{
    schema_version = 1
    product = "agent-linker"
    command = "aglink"
    version = "v$versionName"
    target = "x86_64-pc-windows-msvc"
    portable = [bool] $Portable
    installed_by = "release-archive"
    install_dir = $null
    installed_at = $null
    archive = $archiveName
    data_dir = if ($Portable) { "agent-linker.db next to aglink.exe" } else { "%LOCALAPPDATA%\agent-linker" }
    broker = [ordered]@{
        required = $true
        service_name = "WinSymlinksBroker"
        bundled = $true
        source_url = $BrokerSourceUrl
        sha256 = $BrokerSha256
    }
}

$receipt |
    ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath (Join-Path $stageRoot "install-receipt.json") -Encoding UTF8

@"
# Agent Linker Windows Package

This package contains aglink.exe and the required WinSymlinksBroker runtime.

For a normal install, prefer the release asset aglink-installer.ps1.
For portable mode, keep agent-linker.db next to aglink.exe.

Windows symlink operations require the machine-level WinSymlinksBroker service.
Use scripts\install-broker.ps1 from an elevated PowerShell session if needed.
"@ | Set-Content -LiteralPath (Join-Path $stageRoot "README-windows.md") -Encoding UTF8

Compress-Archive -Path (Join-Path $stageRoot "*") -DestinationPath $archivePath -CompressionLevel Optimal

$hash = Get-FileHash -LiteralPath $archivePath -Algorithm SHA256
"$($hash.Hash.ToLowerInvariant())  $archiveName" |
    Set-Content -LiteralPath "$archivePath.sha256" -Encoding ASCII

Write-Host $archivePath
