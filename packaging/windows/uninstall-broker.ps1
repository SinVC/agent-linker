[CmdletBinding()]
param(
    [string] $ServiceName = "WinSymlinksBroker"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Admin)) {
    throw "WinSymlinksBroker removal requires an elevated PowerShell session."
}

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $existing) {
    Write-Host "$ServiceName is not installed."
    exit 0
}

if ($existing.Status -ne "Stopped") {
    Stop-Service -Name $ServiceName -Force
}

$delete = Start-Process -FilePath sc.exe -ArgumentList @("delete", $ServiceName) -Wait -PassThru -WindowStyle Hidden
if ($delete.ExitCode -ne 0) {
    throw "sc.exe delete failed with exit code $($delete.ExitCode)"
}

Write-Host "$ServiceName removed."
