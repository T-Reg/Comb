# Step 2A.6 on Blender: one extracted stable minor per subdirectory of the raw dir. Runs the
# largest $Top files that exist under the same relative path in every release (usually the
# main binary or the largest shared library).
param(
    [string]$RawDir = 'D:\CombCorpus\blender\stable\raw',
    [int]$Top = 1,
    [int[]]$Windows,
    [string]$OutDir
)

. (Join-Path $PSScriptRoot 'common.ps1')
if (-not $Windows) { $Windows = $DefaultWindows }
if (-not $OutDir) { $OutDir = Join-Path $RepoRoot 'benches/autocorr' }

$releases = Sort-ByVersion (Get-ChildItem -LiteralPath $RawDir -Directory)
$i = 0
foreach ($rel in Get-CommonFiles $releases $Top) {
    $i++
    Write-Host "file $i of $Top`: $rel"
    $paths = $releases | ForEach-Object { Join-Path $_.FullName $rel }
    Invoke-Autocorr "blender-$i" $paths $Windows $OutDir
}
