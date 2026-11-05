# Black-box URL-contract fixture. All network calls are intercepted and the
# deliberately invalid archive stops execution before installation.
$ErrorActionPreference='Stop'
$installer=Join-Path $PSScriptRoot 'install.ps1'
$fixture=Join-Path ([IO.Path]::GetTempPath()) ('aglink-url-blind-'+[guid]::NewGuid().ToString('N'))
$savedLocal=$env:LOCALAPPDATA; $savedData=$env:AGLINK_DATA_DIR
$results=New-Object Collections.Generic.List[object]
$payload=[Text.Encoding]::UTF8.GetBytes('URL acceptance fixture; deliberately not an archive')
$hash=([BitConverter]::ToString(([Security.Cryptography.SHA256]::Create()).ComputeHash($payload))).Replace('-','').ToLowerInvariant()
function Assert([bool]$Value,[string]$Message) { if(-not $Value){throw $Message} }
function Invoke-RestMethod {
    param([string]$Uri,[object]$Headers)
    $global:UrlBlindRequests.Add($Uri)
    if($Uri -notmatch '/releases/latest$'){throw "Unexpected REST URL: $Uri"}
    return [pscustomobject]@{tag_name='v9.8.7'}
}
function Invoke-WebRequest {
    param([string]$Uri,[string]$OutFile,[switch]$UseBasicParsing,[object]$Headers)
    $global:UrlBlindRequests.Add($Uri)
    if($Uri -match '/releases/latest$'){return [pscustomobject]@{Content='{"tag_name":"v9.8.7"}';tag_name='v9.8.7'}}
    if($Uri.EndsWith('.zip')){[IO.File]::WriteAllBytes($OutFile,$global:UrlBlindPayload);return}
    if($Uri.EndsWith('.zip.sha256')){[IO.File]::WriteAllText($OutFile,($global:UrlBlindHash+'  '+[IO.Path]::GetFileName(([uri]$Uri).AbsolutePath).Replace('.sha256','')));return}
    throw "Unexpected download URL: $Uri"
}
try {
    New-Item -ItemType Directory -Path $fixture|Out-Null
    $global:UrlBlindPayload=$payload;$global:UrlBlindHash=$hash
    $env:LOCALAPPDATA=Join-Path $fixture 'appdata';$env:AGLINK_DATA_DIR=$null
    foreach($case in @(@{label='default';repo='SinVC/agent-linker';override=$false},@{label='explicit';repo='ExampleOwner/explicit-repo';override=$true})) {
        $global:UrlBlindRequests=New-Object Collections.Generic.List[string]
        $install=Join-Path $fixture $case.label
        $stopped=$false
        try {
            if($case.override){& $installer -Repo $case.repo -InstallDir $install -NoPath -NoBroker 2>&1|Out-Null}
            else {& $installer -InstallDir $install -NoPath -NoBroker 2>&1|Out-Null}
        }catch{$stopped=$true;$stopMessage=[string]$_}
        Assert $stopped 'Invalid fixture archive unexpectedly proceeded through installation'
        $base='https://github.com/'+$case.repo+'/releases/download/v9.8.7/aglink-v9.8.7-x86_64-pc-windows-msvc.zip'
        $expected=@(('https://api.github.com/repos/'+$case.repo+'/releases/latest'),$base,($base+'.sha256'))
        Assert (($global:UrlBlindRequests|ConvertTo-Json -Compress) -eq ($expected|ConvertTo-Json -Compress)) ('Wrong URL sequence: '+($global:UrlBlindRequests -join ' | ')+'; stop='+$stopMessage)
        Assert (-not(Test-Path -LiteralPath (Join-Path $install 'bin/aglink.exe'))) 'Fixture reached executable installation'
        $results.Add([pscustomobject]@{case=$case.label;requests=@($global:UrlBlindRequests);installation_stopped=$true})
    }
    $results|ConvertTo-Json -Depth 5
    Write-Output 'PASS: Windows default and explicit-Repo latest API/archive/checksum URL contracts; no network or installation'
}finally{
    $env:LOCALAPPDATA=$savedLocal;$env:AGLINK_DATA_DIR=$savedData
    Remove-Variable UrlBlindRequests -Scope Global -ErrorAction SilentlyContinue
    Remove-Variable UrlBlindPayload,UrlBlindHash -Scope Global -ErrorAction SilentlyContinue
    $full=[IO.Path]::GetFullPath($fixture);$temp=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
    if($full.StartsWith($temp,[StringComparison]::OrdinalIgnoreCase)-and[IO.Path]::GetFileName($full).StartsWith('aglink-url-blind-')){Remove-Item -LiteralPath $full -Recurse -Force -ErrorAction SilentlyContinue}
}
