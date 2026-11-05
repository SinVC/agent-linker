[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$PackagePath, [Parameter(Mandatory=$true)][string]$Version)

# Black-box local-package acceptance; all installation/data/temp paths are fixtures.
$ErrorActionPreference = 'Stop'
$installer = Join-Path $PSScriptRoot 'install.ps1'
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('aglink-local-blind-' + [guid]::NewGuid().ToString('N'))
$saved = @{ TEMP=$env:TEMP; TMP=$env:TMP; LOCALAPPDATA=$env:LOCALAPPDATA; AGLINK_DATA_DIR=$env:AGLINK_DATA_DIR }
$eventId = 'aglink-local-created-' + [guid]::NewGuid().ToString('N')
$watcher = $null
$pushed = $false
function Assert([bool]$Value, [string]$Message) { if (-not $Value) { throw $Message } }
function Invoke-Cli([string]$Exe, [string[]]$Arguments) {
    $text = & $Exe @Arguments 2>&1 | Out-String
    Assert ($LASTEXITCODE -eq 0) "CLI $Arguments failed: $text"
    return $text
}
function Invoke-WebRequest { throw 'Unexpected network access in PackagePath mode' }
try {
    New-Item -ItemType Directory -Path $fixture | Out-Null
    Push-Location $fixture; $pushed=$true
    $scratch = Join-Path $fixture 'scratch'
    New-Item -ItemType Directory -Path $scratch | Out-Null
    $env:TEMP=$scratch; $env:TMP=$scratch; $env:LOCALAPPDATA=Join-Path $fixture 'appdata'; $env:AGLINK_DATA_DIR=$null
    $watcher = New-Object IO.FileSystemWatcher $scratch
    $watcher.IncludeSubdirectories=$true
    $watcher.EnableRaisingEvents=$true
    Register-ObjectEvent -InputObject $watcher -EventName Created -SourceIdentifier $eventId | Out-Null
    $install = Join-Path $fixture 'installed'
    $first = & $installer -PackagePath $PackagePath -Version $Version -InstallDir $install -NoPath -NoBroker -SkipDatabaseSnapshot 2>&1 | Out-String
    Assert ($first -match 'Schema:\s+0\s+->\s+4') "Local first install did not migrate: $first"
    Assert ($first -match 'Status:\s+healthy_writable') "Local first install did not check database: $first"
    $exe = Join-Path $install 'bin/aglink.exe'
    Invoke-Cli $exe @('group','create','local-package-fixture') | Out-Null
    $second = & $installer -PackagePath $PackagePath -Version $Version -InstallDir $install -NoPath -NoBroker -SkipDatabaseSnapshot 2>&1 | Out-String
    Assert ($second -match 'Schema:\s+4\s+->\s+4') "Local upgrade skipped migrate: $second"
    Assert ($second -match 'Status:\s+healthy_writable') "Local upgrade skipped check: $second"
    Assert (-not ($second -match 'Backup:')) "SkipDatabaseSnapshot emitted a database backup: $second"
    $created = @(Get-Event -SourceIdentifier $eventId -ErrorAction SilentlyContinue | ForEach-Object { $_.SourceEventArgs.FullPath })
    Assert (@($created | Where-Object { $_ -like '*target-validation.db*' }).Count -eq 0) 'SkipDatabaseSnapshot created a target validation snapshot'
    Assert ((Invoke-Cli $exe @('group','list')).Contains('local-package-fixture')) 'Local upgrade lost fixture registry data'
    Assert ((Invoke-Cli $exe @('init','defaults','list')).Contains('aglink-cli')) 'Local installation did not configure bundled skill'
    $latestFailed=$false
    try { & $installer -PackagePath $PackagePath -Version latest -InstallDir (Join-Path $fixture 'latest-rejected') -NoPath -NoBroker -SkipDatabaseSnapshot } catch { $latestFailed=$true }
    Assert $latestFailed 'Local package mode accepted latest instead of an explicit version'
    $bad = Join-Path $fixture 'bad-assets'
    New-Item -ItemType Directory -Path $bad | Out-Null
    $badPackage = Join-Path $bad ([IO.Path]::GetFileName($PackagePath))
    Copy-Item -LiteralPath $PackagePath -Destination $badPackage
    [IO.File]::WriteAllText(($badPackage+'.sha256'), (('0'*64)+'  '+[IO.Path]::GetFileName($badPackage)))
    $badFailed=$false
    try { & $installer -PackagePath $badPackage -Version $Version -InstallDir (Join-Path $fixture 'checksum-rejected') -NoPath -NoBroker -SkipDatabaseSnapshot } catch { $badFailed=$true }
    Assert $badFailed 'Local package mode ignored a mismatched checksum'
    Write-Output 'PASS: PackagePath offline install/checksum/version and SkipDatabaseSnapshot preserving migrate/check/data'
} finally {
    Unregister-Event -SourceIdentifier $eventId -ErrorAction SilentlyContinue
    Remove-Event -SourceIdentifier $eventId -ErrorAction SilentlyContinue
    if ($watcher) { $watcher.Dispose() }
    if ($pushed) { Pop-Location }
    foreach ($key in $saved.Keys) { [Environment]::SetEnvironmentVariable($key,$saved[$key],'Process') }
    $full = [IO.Path]::GetFullPath($fixture)
    $tempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
    if ($full.StartsWith($tempParent,[StringComparison]::OrdinalIgnoreCase) -and [IO.Path]::GetFileName($full).StartsWith('aglink-local-blind-')) { Remove-Item -LiteralPath $full -Recurse -Force -ErrorAction SilentlyContinue }
}
