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

    if (($parts -join ';') -eq $current) { return }

    try {
        [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
    } catch {
        Write-Warning "Could not update the user PATH: $($_.Exception.Message)"
        Write-Warning "Remove this directory from PATH manually if it is present: $PathToRemove"
    }
}

# Only known SQLite files are removed from an explicit override. Unknown content is retained.
function Remove-OverrideDatabase([string] $Directory) {
    $full = [IO.Path]::GetFullPath($Directory)
    foreach ($dangerous in @([IO.Path]::GetPathRoot($full), $env:USERPROFILE, $env:LOCALAPPDATA)) {
        if ($dangerous -and $full.TrimEnd('\') -eq ([IO.Path]::GetFullPath($dangerous)).TrimEnd('\')) { throw "Refusing to purge $full" }
    }
    $cursor = $full
    while ($cursor) {
        if ((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Refusing purge through reparse point: $cursor" }
        $cursor = Split-Path -Parent $cursor
    }
    foreach ($name in @('agent-linker.db', 'agent-linker.db-journal', 'agent-linker.db-wal', 'agent-linker.db-shm')) {
        $file = Join-Path $full $name
        if (Test-Path -LiteralPath $file -PathType Leaf) { Remove-Item -LiteralPath $file -Force }
    }
}
if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    $programRoot = if ($env:ProgramW6432) { $env:ProgramW6432 } else { $env:ProgramFiles }
    $InstallDir = Join-Path $programRoot 'aglink'
}
$installRoot = [IO.Path]::GetFullPath($InstallDir)
$helper = Join-Path $installRoot 'scripts\maintain-install.ps1'
if (-not (Test-Path -LiteralPath $helper -PathType Leaf)) { throw "Installation helper missing: $helper" }
$requestPath = Join-Path ([IO.Path]::GetTempPath()) ('aglink-uninstall-' + [Guid]::NewGuid().ToString('N') + '.json')
try {
    @{ operation = 'uninstall'; install_dir = $installRoot; purge = [bool]$Purge; remove_broker = [bool]$RemoveBroker } |
        ConvertTo-Json | Set-Content -LiteralPath $requestPath -Encoding UTF8
    $needsElevation = [bool]$RemoveBroker
    try {
        $probe = Join-Path $installRoot ([Guid]::NewGuid().ToString('N') + '.probe')
        [IO.File]::WriteAllText($probe, '')
        Remove-Item -LiteralPath $probe
        $bin = Join-Path $installRoot 'bin'
        if (Test-Path -LiteralPath $bin) {
            $probe = Join-Path $bin ([Guid]::NewGuid().ToString('N') + '.probe')
            [IO.File]::WriteAllText($probe, '')
            Remove-Item -LiteralPath $probe
        }
    } catch [System.UnauthorizedAccessException] { $needsElevation = $true }
    $arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$helper`" -RequestPath `"$requestPath`""
    if ($needsElevation) {
        $process = Start-Process powershell.exe -ArgumentList $arguments -Verb RunAs -Wait -PassThru -WindowStyle Hidden
    } else {
        $process = Start-Process powershell.exe -ArgumentList $arguments -Wait -PassThru -WindowStyle Hidden
    }
    if ($process.ExitCode -ne 0) { throw "Uninstall helper failed: $($process.ExitCode)" }
    Remove-UserPath -PathToRemove (Join-Path $installRoot 'bin')
    if ($Purge -and -not [string]::IsNullOrWhiteSpace($env:AGLINK_DATA_DIR)) {
        Remove-OverrideDatabase $env:AGLINK_DATA_DIR
    }
} finally { if (Test-Path -LiteralPath $requestPath) { Remove-Item -LiteralPath $requestPath -Force } }
Write-Host 'Agent Linker uninstalled. Data and backups are retained unless -Purge was specified.'
