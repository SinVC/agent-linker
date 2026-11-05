[CmdletBinding()]
param(
    [string] $Version = "latest",
    [string] $Repo = "SinVC/agent-linker",
    [string] $InstallDir = "",
    [string] $PackagePath = "",
    [switch] $SkipDatabaseSnapshot,
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

$userSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if ($principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Warning 'Installer is already elevated; successful initialization does not verify ordinary-user data permissions.'
}
$legacyDb = Join-Path $env:LOCALAPPDATA "agent-linker\agent-linker.db"
if ($PackagePath) {
    $PackagePath = [IO.Path]::GetFullPath($PackagePath)
    if ($Version -eq 'latest') { throw 'Specify -Version when installing a local -PackagePath' }
}
$tag = Get-ReleaseTag -RepoName $Repo -RequestedVersion $Version
$plainVersion = if ($tag.StartsWith("v")) { $tag.Substring(1) } else { $tag }

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    $programRoot = if ($env:ProgramW6432) { $env:ProgramW6432 } else { $env:ProgramFiles }
    $InstallDir = Join-Path $programRoot "aglink"
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
    if ($PackagePath) {
        Copy-Item -LiteralPath $PackagePath -Destination $archivePath
        Copy-Item -LiteralPath "$PackagePath.sha256" -Destination $shaPath
    } else {
        Invoke-WebRequest -Uri "$baseUrl/$assetName" -OutFile $archivePath
        Invoke-WebRequest -Uri "$baseUrl/$assetName.sha256" -OutFile $shaPath
    }

    $expected = ((Get-Content -LiteralPath $shaPath -Raw) -split "\s+")[0].ToLowerInvariant()
    $actual = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($expected -ne $actual) {
        throw "SHA256 mismatch for $assetName"
    }

    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractPath -Force

    # The outer process captures the initiating identity and never changes global data overrides.
    $request = @{
        operation = 'install'; install_dir = $installRoot; source = $extractPath
        user_sid = $userSid; no_broker = [bool]$NoBroker; sha256 = $actual
    }
    $requestPath = Join-Path $tempRoot 'request.json'
    $request | ConvertTo-Json | Set-Content -LiteralPath $requestPath -Encoding UTF8
    $helper = Join-Path $extractPath 'scripts\maintain-install.ps1'
    $needsElevation = $false
    try {
        New-Item -ItemType Directory -Force -Path $installRoot | Out-Null
        $probe = Join-Path $installRoot ([Guid]::NewGuid().ToString('N') + '.probe')
        [IO.File]::WriteAllText($probe, '')
        Remove-Item -LiteralPath $probe
        foreach ($directory in @($binDir, $brokerDir, $scriptDir)) {
            if (Test-Path -LiteralPath $directory) {
                $probe = Join-Path $directory ([Guid]::NewGuid().ToString('N') + '.probe')
                [IO.File]::WriteAllText($probe, '')
                Remove-Item -LiteralPath $probe
            }
        }
    } catch [System.UnauthorizedAccessException] { $needsElevation = $true }
    $service = Get-Service -Name 'WinSymlinksBroker' -ErrorAction SilentlyContinue
    if (-not $NoBroker -and (-not $service -or $service.Status -ne 'Running')) { $needsElevation = $true }
    $arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$helper`" -RequestPath `"$requestPath`""
    if ($needsElevation) {
        $process = Start-Process powershell.exe -ArgumentList $arguments -Verb RunAs -Wait -PassThru -WindowStyle Hidden
    } else {
        $process = Start-Process powershell.exe -ArgumentList $arguments -Wait -PassThru -WindowStyle Hidden
    }
    if ($process.ExitCode -ne 0) { throw "Installation helper failed: $($process.ExitCode)" }

    $previousDataDir = $env:AGLINK_DATA_DIR
    $exe = Join-Path $binDir 'aglink.exe'
    $dataDir = Join-Path $installRoot 'data'
    $targetDb = Join-Path $dataDir 'agent-linker.db'
    try {
        # Existing targets stay in place; snapshot validation is optional before migration.
        if (Test-Path -LiteralPath $targetDb) {
            if (-not $SkipDatabaseSnapshot) {
                $env:AGLINK_DATA_DIR = $dataDir
                & $exe db backup (Join-Path $tempRoot 'target-validation.db')
                if ($LASTEXITCODE -ne 0) { throw 'Existing target database failed validation' }
            }
        } elseif (Test-Path -LiteralPath $legacyDb -PathType Leaf) {
            $env:AGLINK_DATA_DIR = Split-Path -Parent $legacyDb
            & $exe db backup $targetDb
            if ($LASTEXITCODE -ne 0) { throw 'Legacy database snapshot failed' }
        }
        $env:AGLINK_DATA_DIR = $dataDir
        & $exe db migrate
        if ($LASTEXITCODE -ne 0) { throw 'Database initialization failed' }
        & $exe db check
        if ($LASTEXITCODE -ne 0) { throw 'Database check failed' }
        & (Join-Path $scriptDir 'configure-skills.ps1') -AglinkExe $exe -SkillsDir (Join-Path $extractPath 'skills') -DataDir $dataDir
        & $exe --verbose doctor
        # doctor also diagnoses the current project and optional Broker; it is informational here.
        $global:LASTEXITCODE = 0
    } finally { $env:AGLINK_DATA_DIR = $previousDataDir }
    if (-not $NoPath) { Add-UserPath -PathToAdd $binDir }
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        $resolvedTemp = [IO.Path]::GetFullPath($tempRoot)
        $tempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
        if (-not $resolvedTemp.StartsWith($tempParent, [StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid installer temporary path' }
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }
}
