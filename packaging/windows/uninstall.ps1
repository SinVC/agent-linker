[CmdletBinding()]
param(
    [string] $InstallDir = "",
    [switch] $Purge,
    [switch] $RemoveBroker
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Remove-UserPath {
    param([string] $PathToRemove)

    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($current)) {
        return
    }

    $parts = $current -split ";" |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Where-Object { $_.TrimEnd("\") -ne $PathToRemove.TrimEnd("\") }

    try {
        [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
    } catch {
        Write-Warning "Could not update the user PATH: $($_.Exception.Message)"
        Write-Warning "Remove this directory from PATH manually if it is present: $PathToRemove"
    }
}

function Invoke-ElevatedBrokerRemoval {
    param([string] $ScriptPath)

    $arguments = @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        "`"$ScriptPath`""
    ) -join " "

    $process = Start-Process -FilePath "powershell.exe" -ArgumentList $arguments -Verb RunAs -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "Broker removal failed with exit code $($process.ExitCode)"
    }
}

function Remove-DataDirIfManaged {
    param([string] $DataDir)

    if ([string]::IsNullOrWhiteSpace($DataDir)) {
        return
    }

    $full = [System.IO.Path]::GetFullPath($DataDir)
    $dangerous = @(
        [System.IO.Path]::GetPathRoot($full),
        $env:USERPROFILE,
        $env:LOCALAPPDATA,
        [Environment]::GetFolderPath("Desktop"),
        [Environment]::GetFolderPath("MyDocuments")
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }

    foreach ($path in $dangerous) {
        if ($full.TrimEnd("\") -eq ([System.IO.Path]::GetFullPath($path)).TrimEnd("\")) {
            throw "Refusing to purge dangerous data directory: $full"
        }
    }

    $marker = Join-Path $full ".aglink-data-dir"
    $receipt = Join-Path $full "install-receipt.json"
    if ((Test-Path -LiteralPath $marker -PathType Leaf) -and (Test-Path -LiteralPath $receipt -PathType Leaf)) {
        Remove-Item -LiteralPath $full -Recurse -Force
        return
    }

    $knownFiles = @(
        "agent-linker.db",
        "agent-linker.db-shm",
        "agent-linker.db-wal",
        "config.toml"
    )

    foreach ($fileName in $knownFiles) {
        $path = Join-Path $full $fileName
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            Remove-Item -LiteralPath $path -Force
        }
    }
}

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\aglink"
}

$installRoot = [System.IO.Path]::GetFullPath($InstallDir)
$binDir = Join-Path $installRoot "bin"
if ($RemoveBroker) {
    $brokerUninstall = Join-Path $installRoot "scripts\uninstall-broker.ps1"
    if (Test-Path -LiteralPath $brokerUninstall -PathType Leaf) {
        Invoke-ElevatedBrokerRemoval -ScriptPath $brokerUninstall
    } else {
        Write-Warning "Broker uninstall script was not found under $installRoot"
    }
}

Remove-UserPath -PathToRemove $binDir

if (Test-Path -LiteralPath $installRoot) {
    Remove-Item -LiteralPath $installRoot -Recurse -Force
}

if ($Purge) {
    $defaultDataDir = Join-Path $env:LOCALAPPDATA "agent-linker"
    Remove-DataDirIfManaged -DataDir $defaultDataDir

    if (-not [string]::IsNullOrWhiteSpace($env:AGLINK_DATA_DIR)) {
        Remove-DataDirIfManaged -DataDir $env:AGLINK_DATA_DIR
    }
}

Write-Host "Agent Linker uninstalled."
