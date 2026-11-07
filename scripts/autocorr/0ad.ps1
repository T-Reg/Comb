# Step 2A.6 on 0 A.D.: one extracted release per subdirectory of the raw dir. Runs the
# largest $Top files that exist under the same relative path in every release. Expect mod
# archives (.zip inside the release) to be excluded as compressed; that is the content.
param(
    [string]$RawDir = 'D:\CombCorpus\0ad\raw',
    [int]$Top = 1,
    [int[]]$Windows,
    [string]$OutDir
)

. (Join-Path $PSScriptRoot 'common.ps1')
if (-not $Windows) { $Windows = $DefaultWindows }
if (-not $OutDir) { $OutDir = Join-Path $RepoRoot 'benches/autocorr' }

# Pre-alpha releases are named by SVN revision (0ad-r07419-pre-alpha) and predate 0.0.1.
function Get-0adKey([string]$Name) {
    $m = [regex]::Match($Name, '^0ad-r(\d+)-')
    if ($m.Success) { return [version]"0.0.0.$([int]$m.Groups[1].Value)" }
    return Get-VersionKey $Name
}

$releases = Sort-ByVersion (Get-ChildItem -LiteralPath $RawDir -Directory) ${function:Get-0adKey}
$i = 0
foreach ($rel in Get-CommonFiles $releases $Top) {
    $i++
    Write-Host "file $i of $Top`: $rel"
    $paths = $releases | ForEach-Object { Join-Path $_.FullName $rel }
    Invoke-Autocorr "0ad-$i" $paths $Windows $OutDir
}
