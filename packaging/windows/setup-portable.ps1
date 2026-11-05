[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root 'aglink.exe'
$previousDataDir = $env:AGLINK_DATA_DIR
try {
    $env:AGLINK_DATA_DIR = $root
    & $exe db migrate
    if ($LASTEXITCODE -ne 0) { throw 'Portable database initialization failed' }
    & $exe db check
    if ($LASTEXITCODE -ne 0) { throw 'Portable database check failed' }
    & (Join-Path $PSScriptRoot 'configure-skills.ps1') -AglinkExe $exe -SkillsDir (Join-Path $root 'skills') -DataDir $root
} finally { $env:AGLINK_DATA_DIR = $previousDataDir }
