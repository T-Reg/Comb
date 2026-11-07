# Step 2A.6: runs every corpus script, then gathers the pooled rows of every table into
# benches\autocorr\SUMMARY.md. -Threshold is the ratio pinned in PLAN.md before running;
# it is required so the verdict column cannot be filled in after looking.
param(
    [Parameter(Mandatory = $true)][double]$Threshold,
    [string]$CorpusRoot = 'D:\CombCorpus',
    [int]$Top = 1,
    [int[]]$Windows,
    [switch]$SummaryOnly
)

. (Join-Path $PSScriptRoot 'common.ps1')
if (-not $Windows) { $Windows = $DefaultWindows }
$OutDir = Join-Path $RepoRoot 'benches/autocorr'

if (-not $SummaryOnly) {
    & (Join-Path $PSScriptRoot 'steamos.ps1') -RawDir "$CorpusRoot/steamos/raw" -Windows $Windows -OutDir $OutDir
    & (Join-Path $PSScriptRoot '0ad.ps1') -RawDir "$CorpusRoot/0ad/raw" -Top $Top -Windows $Windows -OutDir $OutDir
    & (Join-Path $PSScriptRoot 'blender.ps1') -RawDir "$CorpusRoot/blender/stable/raw" -Top $Top -Windows $Windows -OutDir $OutDir
}

# Pooled rows look like: | 1 | **pooled** | | 0.0412 | 0.2100 | **5.10** | 3 used, 0 excluded |
$pooled = '^\| (\d+) \| \*\*pooled\*\* \| \| ([^|]+) \| ([^|]+) \| \*\*([^|]+)\*\* \| ([^|]+) \|$'

$summary = @(
    '## Step 2A.6 change autocorrelation, pooled',
    '',
    "Threshold pinned in PLAN.md: $Threshold (lag-1 pooled ratio). Generated $(Get-Date -Format 'yyyy-MM-dd').",
    '',
    '| series | h | lag | base | conditional | ratio | anchors | lag 1 above threshold? |',
    '|---|---|---|---|---|---|---|---|'
)
$tables = Get-ChildItem -LiteralPath $OutDir -Filter '*-h*.md' | Sort-Object Name
foreach ($table in $tables) {
    $series, $h = $table.BaseName -split '-h(?=\d+$)'
    foreach ($line in Get-Content -LiteralPath $table.FullName) {
        $m = [regex]::Match($line, $pooled)
        if (-not $m.Success) { continue }
        $lag = [int]$m.Groups[1].Value
        $ratio = $m.Groups[4].Value.Trim()
        $verdict = ''
        if ($lag -eq 1) {
            if ($ratio -eq 'n/a') { $verdict = 'n/a' }
            elseif ([double]$ratio -ge $Threshold) { $verdict = 'yes' }
            else { $verdict = '**no**' }
        }
        $summary += "| $series | $h | $lag | $($m.Groups[2].Value.Trim()) | $($m.Groups[3].Value.Trim()) | $ratio | $($m.Groups[5].Value.Trim()) | $verdict |"
    }
}
$summary += @('', 'Per-anchor tables: `benches/autocorr/<series>-h<window>.md`.')

$summaryFile = Join-Path $OutDir 'SUMMARY.md'
$summary | Out-File -LiteralPath $summaryFile -Encoding utf8
$summary | Write-Host
Write-Host ''
Write-Host "wrote $summaryFile; paste it into benches\RESULTS.md and record the verdict on (B) in PLAN.md section 8"
