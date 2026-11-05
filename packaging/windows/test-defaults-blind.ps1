[CmdletBinding()]
param([string]$AglinkExe = '')

# Independent acceptance test: invokes entry points without reading their source.
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
if (-not $AglinkExe) { $AglinkExe = Join-Path $repo 'target/debug/aglink.exe' }
$AglinkExe = [IO.Path]::GetFullPath($AglinkExe)
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) ('aglink-defaults-blind-' + [guid]::NewGuid().ToString('N'))
$oldData = $env:AGLINK_DATA_DIR
$oldLocal = $env:LOCALAPPDATA
$version = '0.0.0-blind'
$locationPushed = $false

function Assert-True([bool]$Value, [string]$Message) { if (-not $Value) { throw $Message } }
function Invoke-Ag([string]$Exe, [string[]]$Arguments) {
    $result = & $Exe @Arguments 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw "aglink $Arguments failed ($LASTEXITCODE): $result" }
    return $result
}
function Assert-SkillFiles([string]$Root) {
    Assert-True (Test-Path -LiteralPath (Join-Path $Root 'skills/aglink-cli/SKILL.md') -PathType Leaf) "Missing complete SKILL.md in $Root"
    $references = @(Get-ChildItem -LiteralPath (Join-Path $Root 'skills/aglink-cli/references') -Recurse -File)
    Assert-True ($references.Count -gt 0) "Missing skill references in $Root"
    foreach ($file in $references) { Assert-True ($file.Length -gt 0) "Empty skill reference: $($file.FullName)" }
}
function Get-ItemId([string]$Show) {
    $id = [regex]::Match($Show, '\bitm_[0-9a-f]+\b').Value
    Assert-True (-not [string]::IsNullOrWhiteSpace($id)) "No stable item ID in CLI show output: $Show"
    return $id
}
function Invoke-OfflineInstall([string]$InstallDir) {
    & (Join-Path $repo 'packaging/windows/install.ps1') -Version $version -InstallDir $InstallDir -NoPath -NoBroker
}

try {
    New-Item -ItemType Directory -Path $fixtureRoot | Out-Null
    Push-Location $fixtureRoot
    $locationPushed = $true
    $env:LOCALAPPDATA = Join-Path $fixtureRoot 'local-app-data'
    Remove-Item Env:AGLINK_DATA_DIR -ErrorAction SilentlyContinue
    $broker = Join-Path $fixtureRoot 'broker'
    New-Item -ItemType Directory -Path $broker | Out-Null
    [IO.File]::WriteAllText((Join-Path $broker 'win-symlinks-broker.exe'), 'non-executable packaging fixture')
    $normalOut = Join-Path $fixtureRoot 'normal-output'
    & (Join-Path $repo 'packaging/windows/package-windows.ps1') -Version $version -AglinkExe $AglinkExe -BrokerDir $broker -OutDir $normalOut
    $archive = @(Get-ChildItem -LiteralPath $normalOut -Filter '*.zip' -File)
    Assert-True ($archive.Count -eq 1) 'Expected exactly one normal Windows archive'
    $normalExtract = Join-Path $fixtureRoot 'normal-extract'
    Expand-Archive -LiteralPath $archive[0].FullName -DestinationPath $normalExtract
    $normalSkill = @(Get-ChildItem -LiteralPath $normalExtract -Filter 'SKILL.md' -Recurse -File | Where-Object { $_.Directory.Name -eq 'aglink-cli' })
    Assert-True ($normalSkill.Count -eq 1) 'Normal archive lacks a unique aglink-cli skill'
    Assert-SkillFiles $normalSkill[0].Directory.Parent.Parent.FullName

    $global:BlindAssetDir = $normalOut
    function global:Invoke-WebRequest {
        param([string]$Uri, [string]$OutFile, [switch]$UseBasicParsing, [object]$Headers)
        $leaf = [IO.Path]::GetFileName(([uri]$Uri).AbsolutePath)
        $asset = Join-Path $global:BlindAssetDir $leaf
        if (-not $OutFile -or -not (Test-Path -LiteralPath $asset -PathType Leaf)) { throw "Unexpected offline request: $Uri -> $OutFile" }
        Copy-Item -LiteralPath $asset -Destination $OutFile
    }
    $install = Join-Path $fixtureRoot 'normal-install'
    Invoke-OfflineInstall $install
    $installedExe = Join-Path $install 'bin/aglink.exe'
    if (-not (Test-Path -LiteralPath $installedExe)) { $installedExe = Join-Path $install 'aglink.exe' }
    Assert-SkillFiles (Join-Path $install 'data')
    Assert-True (Test-Path -LiteralPath (Join-Path $install 'data/.aglink-cli-configured')) 'First normal install did not persist its configuration marker'
    Invoke-Ag $installedExe @('db', 'check') | Write-Output
    $firstShow = Invoke-Ag $installedExe @('skill', 'show', 'aglink-cli')
    $firstId = Get-ItemId $firstShow
    Assert-True ($firstShow.Contains((Join-Path $install 'data/skills/aglink-cli'))) "Registered source is outside actual installation data: $firstShow"
    Assert-True ((Invoke-Ag $installedExe @('init', 'defaults', 'list')).Contains('aglink-cli')) 'First normal install did not select bundled skill'
    Invoke-Ag $installedExe @('init', 'defaults', 'remove', 'aglink-cli') | Write-Output
    Invoke-OfflineInstall $install
    Assert-True ((Get-ItemId (Invoke-Ag $installedExe @('skill', 'show', 'aglink-cli'))) -eq $firstId) 'Same-source upgrade changed stable registry ID'
    Assert-True (-not (Invoke-Ag $installedExe @('init', 'defaults', 'list')).Contains('aglink-cli')) 'Upgrade restored a user-removed default'

    Invoke-Ag $installedExe @('skill', 'remove', 'aglink-cli') | Write-Output
    $differentSource = Join-Path $fixtureRoot 'user-aglink-cli'
    New-Item -ItemType Directory -Path $differentSource | Out-Null
    [IO.File]::WriteAllText((Join-Path $differentSource 'SKILL.md'), '# User-owned same-name skill')
    Invoke-Ag $installedExe @('skill', 'add', $differentSource, '--name', 'aglink-cli') | Write-Output
    $conflictId = Get-ItemId (Invoke-Ag $installedExe @('skill', 'show', 'aglink-cli'))
    $conflictFailed = $false
    try { Invoke-OfflineInstall $install } catch { $conflictFailed = $true; Write-Output "Expected conflict: $_" }
    Assert-True $conflictFailed 'Installer silently accepted a same-name different-source registration'
    $afterConflict = Invoke-Ag $installedExe @('skill', 'show', 'aglink-cli')
    Assert-True ((Get-ItemId $afterConflict) -eq $conflictId -and $afterConflict.Contains($differentSource)) "Installer overwrote user registration: $afterConflict"

    $portableOut = Join-Path $fixtureRoot 'portable-output'
    & (Join-Path $repo 'packaging/windows/package-windows.ps1') -Version $version -AglinkExe $AglinkExe -BrokerDir $broker -OutDir $portableOut -Portable
    $portableArchive = @(Get-ChildItem -LiteralPath $portableOut -Filter '*.zip' -File)
    Assert-True ($portableArchive.Count -eq 1) 'Expected one portable Windows archive'
    $portableExtract = Join-Path $fixtureRoot 'relocated-portable'
    Expand-Archive -LiteralPath $portableArchive[0].FullName -DestinationPath $portableExtract
    $setup = @(Get-ChildItem -LiteralPath $portableExtract -Filter 'setup-portable.ps1' -Recurse -File)
    Assert-True ($setup.Count -eq 1) 'Portable setup entry point is missing'
    $portable = $setup[0].Directory.Parent.FullName
    Assert-SkillFiles $portable
    $portableExe = Join-Path $portable 'aglink.exe'
    if (-not (Test-Path -LiteralPath $portableExe)) { $portableExe = Join-Path $portable 'bin/aglink.exe' }
    Invoke-Ag $portableExe @('db', 'check') | Write-Output
    Assert-True (-not (Invoke-Ag $portableExe @('skill', 'list')).Contains('aglink-cli')) 'Portable staging DB contains a registered skill'
    Assert-True (-not (Invoke-Ag $portableExe @('init', 'defaults', 'list')).Contains('aglink-cli')) 'Portable staging DB contains defaults'
    & $setup[0].FullName
    $portableShow = Invoke-Ag $portableExe @('skill', 'show', 'aglink-cli')
    Assert-True ($portableShow.Contains((Join-Path $portable 'skills/aglink-cli'))) "Portable source was not registered at relocated extraction path: $portableShow"
    Assert-True ((Invoke-Ag $portableExe @('init', 'defaults', 'list')).Contains('aglink-cli')) 'Portable setup failed to select default'
    Invoke-Ag $portableExe @('init', 'defaults', 'remove', 'aglink-cli') | Write-Output
    $portableId = Get-ItemId $portableShow
    & $setup[0].FullName
    Assert-True ((Get-ItemId (Invoke-Ag $portableExe @('skill', 'show', 'aglink-cli'))) -eq $portableId) 'Portable setup changed same-source ID'
    Assert-True (-not (Invoke-Ag $portableExe @('init', 'defaults', 'list')).Contains('aglink-cli')) 'Portable setup restored a user-removed default'
    Write-Output 'PASS: Windows defaults packaging, normal install, upgrade, conflict and relocated portable acceptance'
} finally {
    if ($locationPushed) { Pop-Location }
    $env:AGLINK_DATA_DIR = $oldData
    $env:LOCALAPPDATA = $oldLocal
    Remove-Item Function:Invoke-WebRequest -ErrorAction SilentlyContinue
    Remove-Variable BlindAssetDir -Scope Global -ErrorAction SilentlyContinue
    $resolved = [IO.Path]::GetFullPath($fixtureRoot)
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolved.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase) -and [IO.Path]::GetFileName($resolved).StartsWith('aglink-defaults-blind-')) {
        Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction SilentlyContinue
    }
}
