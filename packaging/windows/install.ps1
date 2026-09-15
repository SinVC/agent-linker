[CmdletBinding()]
param(
    [string] $Version = "latest",
    [string] $Repo = "winddxr/agent-linker",
    [string] $InstallDir = "",
    [switch] $NoPath,
    [switch] $NoBroker
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-ReleaseTag {
    param([string] $RepoName, [string] $RequestedVersion)

    if ($RequestedVersion -ne "latest") {
        if ($RequestedVersion.StartsWith("v")) {
            return $RequestedVersion
        }
        return "v$RequestedVersion"
    }

    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$RepoName/releases/latest" -Headers @{
        "User-Agent" = "aglink-installer"
    }
    return $release.tag_name
}

function Add-UserPath {
    param([string] $PathToAdd)

    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if (-not [string]::IsNullOrWhiteSpace($current)) {
        $parts = $current -split ";" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }

    if ($parts -notcontains $PathToAdd) {
        $updated = (@($parts) + $PathToAdd) -join ";"
        try {
            [Environment]::SetEnvironmentVariable("Path", $updated, "User")
            $env:Path = "$env:Path;$PathToAdd"
        } catch {
            Write-Warning "Installed aglink, but could not update the user PATH: $($_.Exception.Message)"
            Write-Warning "Add this directory to PATH manually: $PathToAdd"
        }
    }
}

function Invoke-ElevatedBrokerInstall {
    param([string] $ScriptPath, [string] $BrokerExe)

    $arguments = @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        "`"$ScriptPath`"",
        "-BrokerExe",
        "`"$BrokerExe`""
    ) -join " "

    $process = Start-Process -FilePath "powershell.exe" -ArgumentList $arguments -Verb RunAs -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "Broker installation failed with exit code $($process.ExitCode)"
    }
}

$tag = Get-ReleaseTag -RepoName $Repo -RequestedVersion $Version
$plainVersion = if ($tag.StartsWith("v")) { $tag.Substring(1) } else { $tag }

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\aglink"
}

$installRoot = [System.IO.Path]::GetFullPath($InstallDir)
$binDir = Join-Path $installRoot "bin"
$brokerDir = Join-Path $installRoot "broker"
$scriptDir = Join-Path $installRoot "scripts"
$assetName = "aglink-v$plainVersion-x86_64-pc-windows-msvc.zip"
$baseUrl = "https://github.com/$Repo/releases/download/$tag"

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "aglink-install-$plainVersion-$([Guid]::NewGuid().ToString('N'))"
$archivePath = Join-Path $tempRoot $assetName
$shaPath = "$archivePath.sha256"
$extractPath = Join-Path $tempRoot "extract"

New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
New-Item -ItemType Directory -Force -Path $extractPath | Out-Null

try {
    Invoke-WebRequest -Uri "$baseUrl/$assetName" -OutFile $archivePath
    Invoke-WebRequest -Uri "$baseUrl/$assetName.sha256" -OutFile $shaPath

    $expected = ((Get-Content -LiteralPath $shaPath -Raw) -split "\s+")[0].ToLowerInvariant()
    $actual = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($expected -ne $actual) {
        throw "SHA256 mismatch for $assetName"
    }

    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractPath -Force

    New-Item -ItemType Directory -Force -Path $binDir, $brokerDir, $scriptDir | Out-Null
    Copy-Item -LiteralPath (Join-Path $extractPath "aglink.exe") -Destination (Join-Path $binDir "aglink.exe") -Force
    Copy-Item -Path (Join-Path $extractPath "broker\*") -Destination $brokerDir -Recurse -Force
    Copy-Item -Path (Join-Path $extractPath "scripts\*") -Destination $scriptDir -Recurse -Force

    if (-not $NoPath) {
        Add-UserPath -PathToAdd $binDir
    }

    $receiptPath = Join-Path $installRoot "install-receipt.json"
    $sourceReceipt = Get-Content -LiteralPath (Join-Path $extractPath "install-receipt.json") -Raw | ConvertFrom-Json
    $sourceReceipt.installed_by = "aglink-installer.ps1"
    $sourceReceipt.install_dir = $installRoot
    $sourceReceipt.installed_at = (Get-Date).ToUniversalTime().ToString("o")
    $sourceReceipt.archive_sha256 = $actual
    $sourceReceipt |
        ConvertTo-Json -Depth 8 |
        Set-Content -LiteralPath $receiptPath -Encoding UTF8

    if (-not $NoBroker) {
        $service = Get-Service -Name "WinSymlinksBroker" -ErrorAction SilentlyContinue
        if (-not $service -or $service.Status -ne "Running") {
            Invoke-ElevatedBrokerInstall `
                -ScriptPath (Join-Path $scriptDir "install-broker.ps1") `
                -BrokerExe (Join-Path $brokerDir "win-symlinks-broker.exe")
        }
    }

    & (Join-Path $binDir "aglink.exe") --verbose doctor
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
