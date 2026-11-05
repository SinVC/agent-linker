# Local fixture only: no network, user PATH update, real legacy database, or Broker service changes.
[CmdletBinding()]
param(
    [string] $AglinkExe = (Join-Path $PSScriptRoot '..\..\target\debug\aglink.exe'),
    [switch] $ProgramFiles,
    [string] $CrossEntryGate = ''
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('aglink-install-test-' + [Guid]::NewGuid().ToString('N'))
$installRoot = Join-Path $fixture 'custom root'
if ($ProgramFiles) {
    $programRoot = if ($env:ProgramW6432) { $env:ProgramW6432 } else { $env:ProgramFiles }
    $installRoot = Join-Path $programRoot (Split-Path -Leaf $fixture)
}
$savedData = $env:AGLINK_DATA_DIR
$savedLocal = $env:LOCALAPPDATA
$savedAssets = $env:AGLINK_SMOKE_ASSETS
$savedFailureCommand = $env:AGLINK_SMOKE_FAIL_DB_COMMAND
$savedFailureMarker = $env:AGLINK_SMOKE_FAIL_MARKER
$fixtureLocationPushed = $false
function Assert([bool] $Condition, [string] $Message) { if (-not $Condition) { throw $Message } }
function Invoke-WebRequest {
    param([string] $Uri, [string] $OutFile)
    Copy-Item -LiteralPath (Join-Path $env:AGLINK_SMOKE_ASSETS ($Uri.Split('/')[-1])) -Destination $OutFile
}
function Invoke-Cli {
    param([string] $Exe, [string[]] $Arguments)
    $output = & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) { throw "CLI failed: $Arguments ($LASTEXITCODE)" }
    return ($output -join "`n")
}
try {
    New-Item -ItemType Directory -Path $fixture | Out-Null
    Push-Location $fixture
    $fixtureLocationPushed = $true
    $broker = Join-Path $fixture 'mock-broker'
    New-Item -ItemType Directory -Path $broker | Out-Null
    Set-Content -LiteralPath (Join-Path $broker 'win-symlinks-broker.exe') -Value 'fixture only; never executed'
    $assets = Join-Path $fixture 'assets'
    & (Join-Path $PSScriptRoot 'package-windows.ps1') -Version '0.0.0-smoke' -AglinkExe $AglinkExe -BrokerDir $broker -OutDir $assets
    & (Join-Path $PSScriptRoot 'package-windows.ps1') -Version '0.0.0-smoke' -AglinkExe $AglinkExe -BrokerDir $broker -OutDir $assets -Portable
    $env:AGLINK_DATA_DIR = $null
    $portable = Join-Path $assets 'stage-windows-portable\aglink.exe'
    Assert ((Invoke-Cli $portable @('harness','list')).Contains('claude')) 'Portable first query failed'
    Assert ((Invoke-Cli $portable @('db','check')) -match 'Schema:\s+4\s+/\s+latest\s+4') 'Portable package did not contain healthy schema v4'

    # A test-only executable forwards all normal commands to the real CLI, then
    # injects one selected native failure during installation. No product script
    # or installed executable outside this random temporary fixture is changed.
    $forwarderSource = Join-Path $fixture 'failure-forwarder.rs'
    $forwarderExe = Join-Path $fixture 'failure-forwarder.exe'
    $realExeLiteral = ConvertTo-Json ([IO.Path]::GetFullPath($AglinkExe)) -Compress
    @'
use std::{env, fs, process::{Command, exit}};
fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let fail = env::var("AGLINK_SMOKE_FAIL_DB_COMMAND").unwrap_or_default();
    if args.len() >= 2 && args[0] == "db" && args[1] == fail.as_str() && !fail.is_empty() {
        if let Some(path) = env::var_os("AGLINK_SMOKE_FAIL_MARKER") { fs::write(path, &fail).unwrap(); }
        eprintln!("fixture injected db {} failure", fail);
        exit(47);
    }
    match Command::new(__REAL_EXE__).args(args).status() {
        Ok(status) => exit(status.code().unwrap_or(1)),
        Err(error) => { eprintln!("fixture forward failed: {}", error); exit(48); }
    }
}
'@.Replace('__REAL_EXE__', $realExeLiteral) | Set-Content -LiteralPath $forwarderSource -Encoding UTF8
    & rustc --edition=2021 --crate-name aglink_failure_forwarder $forwarderSource -o $forwarderExe
    Assert ($LASTEXITCODE -eq 0) 'Failed to compile the test-only command forwarder'
    $failureAssets = Join-Path $fixture 'failure-assets'
    & (Join-Path $PSScriptRoot 'package-windows.ps1') -Version '0.0.0-failure' -AglinkExe $forwarderExe -BrokerDir $broker -OutDir $failureAssets
    $env:AGLINK_SMOKE_ASSETS = $failureAssets
    foreach ($phase in @('migrate', 'check')) {
        $env:AGLINK_SMOKE_FAIL_DB_COMMAND = $phase
        $env:AGLINK_SMOKE_FAIL_MARKER = Join-Path $fixture "injected-$phase.txt"
        $failed = $false
        try { & (Join-Path $PSScriptRoot 'install.ps1') -Version '0.0.0-failure' -InstallDir (Join-Path $fixture "failed-$phase-install") -NoPath -NoBroker } catch { $failed = $true }
        Assert $failed "Installer ignored injected db $phase failure"
        Assert ((Get-Content -LiteralPath $env:AGLINK_SMOKE_FAIL_MARKER -Raw) -eq $phase) "Installation did not reach injected db $phase command"
    }
    $env:AGLINK_SMOKE_FAIL_DB_COMMAND = $null
    $env:AGLINK_SMOKE_FAIL_MARKER = $null

    # Isolate service branches in a child process; never change the machine service.
    $helperSource = Join-Path $fixture 'helper-source'
    Copy-Item -LiteralPath (Join-Path $assets 'stage-windows') -Destination $helperSource -Recurse
    Set-Content -LiteralPath (Join-Path $helperSource 'scripts\install-broker.ps1') -Value 'exit 23'
    $helperRoot = Join-Path $fixture 'helper root'
    New-Item -ItemType Directory -Path (Join-Path $helperRoot 'broker') -Force | Out-Null
    $retainedBroker = Join-Path $helperRoot 'broker\win-symlinks-broker.exe'
    Set-Content -LiteralPath $retainedBroker -Value 'existing shared broker'
    $helperRequest = Join-Path $fixture 'helper-request.json'
    @{ operation = 'install'; install_dir = $helperRoot; source = $helperSource
       user_sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value; no_broker = $false; sha256 = 'fixture' } |
        ConvertTo-Json | Set-Content -LiteralPath $helperRequest -Encoding UTF8
    $wrapper = Join-Path $fixture 'mock-service.ps1'
    @'
param([string] $Helper, [string] $Request, [string] $Mode)
function Get-Service {
    param([string] $Name, [object] $ErrorAction)
    if ($Mode -eq 'running') { [pscustomobject]@{ Status = 'Running' } }
}
& $Helper -RequestPath $Request
exit $LASTEXITCODE
'@ | Set-Content -LiteralPath $wrapper -Encoding UTF8
    $helper = Join-Path $helperSource 'scripts\maintain-install.ps1'
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $wrapper -Helper $helper -Request $helperRequest -Mode running
    Assert ($LASTEXITCODE -eq 0) 'Helper failed to reuse running Broker'
    Assert ((Get-Content -LiteralPath $retainedBroker -Raw).Trim() -eq 'existing shared broker') 'Helper replaced running Broker files'
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $wrapper -Helper $helper -Request $helperRequest -Mode missing
    Assert ($LASTEXITCODE -ne 0) 'Helper ignored required Broker installation failure'

    $env:LOCALAPPDATA = Join-Path $fixture 'legacy-appdata'
    $env:AGLINK_DATA_DIR = Join-Path $env:LOCALAPPDATA 'agent-linker'
    Invoke-Cli $AglinkExe @('db','migrate') | Out-Null
    Invoke-Cli $AglinkExe @('group','create','legacy-fixture') | Out-Null
    $legacyHash = (Get-FileHash (Join-Path $env:AGLINK_DATA_DIR 'agent-linker.db')).Hash
    $env:AGLINK_DATA_DIR = Join-Path $fixture 'unused-override'
    $env:AGLINK_SMOKE_ASSETS = $assets
    & (Join-Path $PSScriptRoot 'install.ps1') -Version '0.0.0-smoke' -InstallDir $installRoot -NoPath -NoBroker
    Assert ($env:AGLINK_DATA_DIR -eq (Join-Path $fixture 'unused-override')) 'Installer failed to restore override'
    Assert (-not (Test-Path -LiteralPath $env:AGLINK_DATA_DIR)) 'Installer created the user override database'
    $env:AGLINK_DATA_DIR = $null
    $exe = Join-Path $installRoot 'bin\aglink.exe'
    $database = Join-Path $installRoot 'data\agent-linker.db'
    Assert ((Invoke-Cli $exe @('db','path')).Contains($database)) 'Receipt did not resolve installation data'
    Assert ((Invoke-Cli $exe @('db','check')) -match 'Schema:\s+4\s+/\s+latest\s+4') 'Installed database is not healthy schema v4'
    Assert (Test-Path -LiteralPath (Join-Path $installRoot 'data\.aglink-cli-configured')) 'First install did not save bundled-skill configuration marker'
    Assert ((Invoke-Cli $exe @('group','list')).Contains('legacy-fixture')) 'Legacy import lost records'
    Assert ((Get-FileHash (Join-Path $env:LOCALAPPDATA 'agent-linker\agent-linker.db')).Hash -eq $legacyHash) 'Legacy source changed'
    $env:LOCALAPPDATA = Join-Path $fixture 'different-appdata'
    Assert ((Invoke-Cli $exe @('db','path')).Contains($database)) 'Normal install depends on LOCALAPPDATA'
    Invoke-Cli $exe @('group','create','upgrade-fixture') | Out-Null
    Invoke-Cli $exe @('db','backup') | Out-Null
    Assert (@(Get-ChildItem -LiteralPath (Join-Path $installRoot 'backups') -Filter '*.db').Count -eq 1) 'Default backup location failed'
    if ($CrossEntryGate) {
        $project = Join-Path $fixture 'project'
        $skill = Join-Path $fixture 'skill'
        New-Item -ItemType Directory -Path $project, $skill | Out-Null
        Set-Content -LiteralPath (Join-Path $skill 'SKILL.md') -Value 'cross entry fixture'
        Invoke-Cli $exe @('skill','add',$skill,'--name','cross-entry-skill') | Out-Null
        Push-Location $project
        try { Invoke-Cli $exe @('init') | Out-Null } finally { Pop-Location }
        $terminalGroup = 'terminal-' + [Guid]::NewGuid().ToString('N')
        $agentGroup = 'agent-' + [Guid]::NewGuid().ToString('N')
        Invoke-Cli $exe @('group','create',$terminalGroup) | Out-Null
        @{ exe = $exe; database = $database; project = $project; terminal_group = $terminalGroup; agent_group = $agentGroup } |
            ConvertTo-Json | Set-Content -LiteralPath $CrossEntryGate -Encoding UTF8
        $deadline = (Get-Date).AddMinutes(3)
        while (-not (Test-Path -LiteralPath "$CrossEntryGate.done")) {
            if ((Get-Date) -gt $deadline) { throw 'Cross-entry verification timed out' }
            Start-Sleep -Seconds 1
        }
        Assert ((Invoke-Cli $exe @('group','list')).Contains($agentGroup)) 'Ordinary process cannot read Agent group'
        Invoke-Cli $exe @('group','delete',$agentGroup) | Out-Null
        Assert (-not (Invoke-Cli $exe @('group','list')).Contains($terminalGroup)) 'Agent did not delete ordinary group'
        Write-Host 'PASS: ordinary process read/deleted the Agent group without AGLINK_DATA_DIR'
    }
    if ($ProgramFiles) {
        $denied = $false
        try { [IO.File]::WriteAllText((Join-Path $installRoot 'bin\must-not-write'), '') } catch [UnauthorizedAccessException] { $denied = $true }
        Assert $denied 'Ordinary user can modify protected bin directory'
    }
    & (Join-Path $PSScriptRoot 'install.ps1') -Version '0.0.0-smoke' -InstallDir $installRoot -NoPath -NoBroker
    Assert ((Invoke-Cli $exe @('group','list')).Contains('upgrade-fixture')) 'Upgrade lost records'
    & (Join-Path $PSScriptRoot 'uninstall.ps1') -InstallDir $installRoot
    Assert (-not (Test-Path -LiteralPath $exe)) 'Uninstall kept CLI'
    Assert (Test-Path -LiteralPath $database) 'Uninstall deleted database'
    Assert (Test-Path -LiteralPath (Join-Path $installRoot 'data\.aglink-cli-configured')) 'Ordinary uninstall deleted bundled-skill configuration marker'
    Assert (Test-Path -LiteralPath (Join-Path $installRoot 'data\skills\aglink-cli\SKILL.md')) 'Ordinary uninstall deleted retained skills'
    Assert (Test-Path -LiteralPath (Join-Path $installRoot 'scripts\uninstall-broker.ps1')) 'Uninstall deleted retained Broker script'
    Assert (Test-Path -LiteralPath (Join-Path $installRoot 'broker')) 'Uninstall deleted retained Broker directory'
    Assert (@(Get-ChildItem -LiteralPath (Join-Path $installRoot 'backups') -Filter '*.db').Count -eq 1) 'Uninstall deleted backup'
    # Unknown user content must survive purge.
    Set-Content -LiteralPath (Join-Path $installRoot 'data\unknown.txt') -Value 'preserve'
    & (Join-Path $PSScriptRoot 'uninstall.ps1') -InstallDir $installRoot -Purge
    Assert (-not (Test-Path -LiteralPath $database)) 'Purge kept installation database'
    Assert (-not (Test-Path -LiteralPath (Join-Path $installRoot 'data\.aglink-cli-configured'))) 'Purge kept bundled-skill configuration marker'
    Assert (Test-Path -LiteralPath (Join-Path $installRoot 'data\skills\aglink-cli\SKILL.md')) 'Purge deleted retained skills'
    Assert (Test-Path -LiteralPath (Join-Path $installRoot 'data\unknown.txt')) 'Purge deleted unknown content'
    Assert (@(Get-ChildItem -LiteralPath (Join-Path $installRoot 'backups') -Filter '*.db').Count -eq 0) 'Purge kept generated backups'
    # Corrupt targets must cause a failing installer and remain byte-for-byte intact.
    Set-Content -LiteralPath $database -Value 'not a database'
    $badHash = (Get-FileHash -LiteralPath $database).Hash
    $failed = $false
    try { & (Join-Path $PSScriptRoot 'install.ps1') -Version '0.0.0-smoke' -InstallDir $installRoot -NoPath -NoBroker } catch { $failed = $true }
    Assert $failed 'Installer ignored corrupt database'
    Assert ((Get-FileHash -LiteralPath $database).Hash -eq $badHash) 'Installer overwrote corrupt target'
    Write-Host "PASS: portable schema v4 and ordinary first query, import, override restore, receipt path, backup, upgrade, uninstall, purge, initialization failure, injected migrate/check failures; fixture=$installRoot"
} finally {
    if ($fixtureLocationPushed) { Pop-Location }
    $env:AGLINK_DATA_DIR = $savedData
    $env:LOCALAPPDATA = $savedLocal
    $env:AGLINK_SMOKE_ASSETS = $savedAssets
    $env:AGLINK_SMOKE_FAIL_DB_COMMAND = $savedFailureCommand
    $env:AGLINK_SMOKE_FAIL_MARKER = $savedFailureMarker
    if ($ProgramFiles -and (Test-Path -LiteralPath $installRoot)) {
        # Only this randomly named fixture may be recursively removed, never the product root.
        $cleanup = Join-Path $fixture 'cleanup.ps1'
        @'
param([string] $FixtureRoot)
$ErrorActionPreference = 'Stop'
$full = [IO.Path]::GetFullPath($FixtureRoot)
$program = if ($env:ProgramW6432) { $env:ProgramW6432 } else { $env:ProgramFiles }
if ((Split-Path -Parent $full) -ne $program -or (Split-Path -Leaf $full) -notmatch '^aglink-install-test-[a-f0-9]{32}$') { throw 'Invalid fixture cleanup path' }
$reparse = Get-ChildItem -LiteralPath $full -Recurse -Force | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }
if ($reparse -or ((Get-Item -LiteralPath $full).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Refusing reparse fixture cleanup' }
Remove-Item -LiteralPath $full -Recurse -Force
'@ | Set-Content -LiteralPath $cleanup -Encoding UTF8
        $process = Start-Process powershell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$cleanup`" -FixtureRoot `"$installRoot`"" -Verb RunAs -Wait -PassThru -WindowStyle Hidden
        if ($process.ExitCode -ne 0) { Write-Warning "Fixture cleanup failed: $installRoot" }
    }
    $resolved = [IO.Path]::GetFullPath($fixture)
    $tempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolved.StartsWith($tempParent, [StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolved).StartsWith('aglink-install-test-')) {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
