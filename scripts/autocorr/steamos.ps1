# Step 2A.6 on SteamOS: every decompressed image (*.img) under the raw dir, in release order.
# Never point this at the .bz2 files: compressed input looks random.
#
# The corpus mixes two naming schemes. steamdeck-recovery-<n>.img carry only a sequence
# number and predate every steamdeck-repair-<build>-<version>.img, so recovery images sort
# first by <n>, then repair images by version.
param(
    [string]$RawDir = 'D:\CombCorpus\steamos\raw',
    [int[]]$Windows,
    [string]$OutDir
)

. (Join-Path $PSScriptRoot 'common.ps1')
if (-not $Windows) { $Windows = $DefaultWindows }
if (-not $OutDir) { $OutDir = Join-Path $RepoRoot 'benches/autocorr' }

function Get-SteamKey([string]$Name) {
    $m = [regex]::Match($Name, '^steamdeck-recovery-(\d+)\.img$')
    if ($m.Success) { return [version]"0.0.$($m.Groups[1].Value)" }
    return Get-VersionKey $Name
}

$images = Sort-ByVersion (Get-ChildItem -LiteralPath $RawDir -File -Filter '*.img') ${function:Get-SteamKey}
Invoke-Autocorr 'steamos' ($images | ForEach-Object { $_.FullName }) $Windows $OutDir
