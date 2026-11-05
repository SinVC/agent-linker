# Run as the installing user, after the selected database has been initialized.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string] $AglinkExe,
    [Parameter(Mandatory = $true)][string] $SkillsDir,
    [Parameter(Mandatory = $true)][string] $DataDir
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$destination = Join-Path $DataDir 'skills'
$skill = [IO.Path]::GetFullPath((Join-Path $destination 'aglink-cli'))
$marker = Join-Path $DataDir '.aglink-cli-configured'
$entries = & $AglinkExe skill list
if ($LASTEXITCODE -ne 0) { throw 'Unable to query installed skills' }
$existing = @($entries | Where-Object { ($_ -split "`t", 4)[0] -ieq 'aglink-cli' })
if ($existing.Count -gt 0) {
    $registeredSource = ($existing[0] -split "`t", 4)[3]
    if ($registeredSource -ine $skill) {
        throw "Skill aglink-cli is already registered from another source: $registeredSource"
    }
}

if ([IO.Path]::GetFullPath($SkillsDir) -ine [IO.Path]::GetFullPath($destination)) {
    # data is writable by the installing user; never copy through an existing link.
    $cursor = [IO.Path]::GetFullPath($destination)
    while ($cursor) {
        if ((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw "Refusing skill installation through reparse point: $cursor"
        }
        $cursor = Split-Path -Parent $cursor
    }
    if (Test-Path -LiteralPath $destination) {
        if (Get-ChildItem -LiteralPath $destination -Recurse -Force | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }) {
            throw "Refusing skill installation through reparse points in $destination"
        }
    }
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    Copy-Item -Path (Join-Path $SkillsDir '*') -Destination $destination -Recurse -Force
}

if ($existing.Count -eq 0) {
    & $AglinkExe skill add $skill --name aglink-cli
    if ($LASTEXITCODE -ne 0) { throw 'Bundled skill registration failed' }
} else {
    & $AglinkExe skill refresh aglink-cli
    if ($LASTEXITCODE -ne 0) { throw 'Bundled skill refresh failed' }
}
if (-not (Test-Path -LiteralPath $marker)) {
    & $AglinkExe init defaults add aglink-cli
    if ($LASTEXITCODE -ne 0) { throw 'Bundled skill default configuration failed' }
    Set-Content -LiteralPath $marker -Value 'Configured by the aglink installer.' -Encoding UTF8
}
