# Disk-backed privileged helper. User PATH and database migration belong to the caller.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string] $RequestPath)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-PlainDirectory([string] $Path) {
    $cursor = [IO.Path]::GetFullPath($Path)
    while ($cursor) {
        if (Test-Path -LiteralPath $cursor) {
            if ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "Refusing installation operation through reparse point: $cursor"
            }
        }
        $cursor = Split-Path -Parent $cursor
    }
}

function Remove-KnownDatabase([string] $Directory) {
    Assert-PlainDirectory $Directory
    foreach ($name in @('agent-linker.db', 'agent-linker.db-journal', 'agent-linker.db-wal', 'agent-linker.db-shm', '.aglink-cli-configured')) {
        $file = Join-Path $Directory $name
        if (Test-Path -LiteralPath $file -PathType Leaf) { Remove-Item -LiteralPath $file -Force }
    }
}

try {
    $request = Get-Content -LiteralPath $RequestPath -Raw | ConvertFrom-Json
    $root = [IO.Path]::GetFullPath($request.install_dir)
    if ($root.TrimEnd('\') -eq [IO.Path]::GetPathRoot($root).TrimEnd('\')) { throw 'Installation root cannot be a drive root' }
    Assert-PlainDirectory $root
    $receiptPath = Join-Path $root 'install-receipt.json'
    $data = Join-Path $root 'data'
    $backups = Join-Path $root 'backups'
    if ($request.operation -eq 'install') {
        $source = $request.source
        $receipt = Get-Content -LiteralPath (Join-Path $source 'install-receipt.json') -Raw | ConvertFrom-Json
        if ($receipt.product -ne 'agent-linker' -or $receipt.portable -ne $false) { throw 'Invalid normal release receipt' }
        foreach ($name in @('bin', 'data', 'backups', 'broker', 'scripts')) {
            $directory = Join-Path $root $name
            Assert-PlainDirectory $directory
            New-Item -ItemType Directory -Force -Path $directory | Out-Null
        }
        foreach ($directory in @($data, $backups)) {
            $reparse = Get-ChildItem -LiteralPath $directory -Recurse -Force |
                Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }
            if ($reparse) { throw "Refusing recursive ACL changes through reparse points in $directory" }
            & icacls.exe $directory /grant "*$($request.user_sid):(OI)(CI)M" /T /Q | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "Unable to grant data access: $directory" }
        }
        Copy-Item -LiteralPath (Join-Path $source 'aglink.exe') -Destination (Join-Path $root 'bin\aglink.exe') -Force
        # A shared service may run from this directory. Reuse it and never overwrite its files.
        $service = Get-Service -Name 'WinSymlinksBroker' -ErrorAction SilentlyContinue
        if (-not $service) {
            Copy-Item -Path (Join-Path $source 'broker\*') -Destination (Join-Path $root 'broker') -Recurse -Force
        }
        Copy-Item -Path (Join-Path $source 'scripts\*') -Destination (Join-Path $root 'scripts') -Recurse -Force
        foreach ($entry in @{
            installed_by = 'aglink-installer.ps1'; install_dir = $root; data_dir = $data
            installed_at = (Get-Date).ToUniversalTime().ToString('o'); archive_sha256 = $request.sha256
        }.GetEnumerator()) { $receipt | Add-Member -NotePropertyName $entry.Key -NotePropertyValue $entry.Value -Force }
        $receipt | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $receiptPath -Encoding UTF8
        if (-not $request.no_broker -and (-not $service -or $service.Status -ne 'Running')) {
            & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'scripts\install-broker.ps1') -BrokerExe (Join-Path $root 'broker\win-symlinks-broker.exe')
            if ($LASTEXITCODE -ne 0) { throw "Broker installation failed: $LASTEXITCODE" }
        }
    } elseif ($request.operation -eq 'uninstall') {
        Assert-PlainDirectory (Join-Path $root 'bin')
        $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
        if ($receipt.product -ne 'agent-linker' -or $receipt.installed_by -ne 'aglink-installer.ps1' -or $receipt.portable -ne $false) { throw 'Invalid installation receipt' }
        if ($request.remove_broker) {
            & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'scripts\uninstall-broker.ps1')
            if ($LASTEXITCODE -ne 0) { throw "Broker removal failed: $LASTEXITCODE" }
        }
        $exe = Join-Path $root 'bin\aglink.exe'
        if (Test-Path -LiteralPath $exe -PathType Leaf) { Remove-Item -LiteralPath $exe -Force }
        if ($request.purge) {
            Remove-KnownDatabase $data
            Assert-PlainDirectory $backups
            if (Test-Path -LiteralPath $backups) {
                Get-ChildItem -LiteralPath $backups -File -Filter 'agent-linker-backup-*.db' | ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force }
            }
        }
        # Keep receipt and maintenance scripts for a later purge or shared Broker removal.
        # Never recursively remove the root, shared Broker, or unknown user files.
    } else { throw 'Unknown maintenance operation' }
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
}
