[CmdletBinding()]
param(
    [string] $BrokerExe = "",
    [string] $ServiceName = "WinSymlinksBroker",
    [string] $DisplayName = "WinSymlinksBroker"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Admin)) {
    throw "WinSymlinksBroker installation requires an elevated PowerShell session."
}

if ([string]::IsNullOrWhiteSpace($BrokerExe)) {
    $BrokerExe = Join-Path (Split-Path -Parent $PSScriptRoot) "broker\win-symlinks-broker.exe"
}

$brokerPath = [System.IO.Path]::GetFullPath($BrokerExe)
if (-not (Test-Path -LiteralPath $brokerPath -PathType Leaf)) {
    throw "Broker executable not found: $brokerPath"
}

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
    if ($existing.Status -ne "Running") {
        Start-Service -Name $ServiceName
    }
    Write-Host "$ServiceName is already installed."
    exit 0
}

$brokerDir = Split-Path -Parent $brokerPath
$upstreamInstaller = Join-Path $brokerDir "install-service.ps1"
if (Test-Path -LiteralPath $upstreamInstaller -PathType Leaf) {
    & $upstreamInstaller
} else {
    $create = Start-Process -FilePath sc.exe -ArgumentList @(
        "create",
        $ServiceName,
        "binPath=",
        "`"$brokerPath`"",
        "DisplayName=",
        "`"$DisplayName`"",
        "start=",
        "auto"
    ) -Wait -PassThru -WindowStyle Hidden
    if ($create.ExitCode -ne 0) {
        throw "sc.exe create failed with exit code $($create.ExitCode)"
    }
}

Start-Service -Name $ServiceName
Write-Host "$ServiceName installed and started."
