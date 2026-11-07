# Shared helpers for the Step 2A.6 change-autocorrelation runs. Dot-sourced by the
# per-corpus scripts; not meant to be run on its own.

$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path

# Tier windows from the default ladder, coarsest first (512 KiB, 32 KiB, 2 KiB).
$DefaultWindows = @(524288, 32768, 2048)

# Sort key for a release name: the last dotted number run in it, as a [version], with a
# trailing letter as an extra component ("0.0.23b" -> 0.0.23.2, after 0.0.23.0).
# "steamdeck-repair-20231117.11-3.5.6.img" -> 3.5.6, "blender-4.2.23-stable+v42.d0cb..."
# -> 4.2.23, "0ad-0.0.26-alpha" -> 0.0.26. Plain string order would put 4.2.10 before 4.2.9.
function Get-VersionKey([string]$Name) {
    # Nothing alphanumeric may follow, so hex in "+v43.32f5fdce" is not read as 43.32f.
    $found = [regex]::Matches($Name, '(\d+(?:\.\d+)+)([a-z]?)(?![a-z0-9])')
    if ($found.Count -eq 0) {
        throw "no version number in '$Name'; rename it or move it out of the corpus dir"
    }
    $last = $found[$found.Count - 1]
    $parts = @($last.Groups[1].Value.Split('.') | Select-Object -First 3)
    $letter = $last.Groups[2].Value
    if ($letter) { $parts += [int][char]$letter - [int][char]'a' + 1 } else { $parts += 0 }
    return [version]($parts -join '.')
}

# Items sorted oldest first by $Key (a name -> [version] function, Get-VersionKey by
# default). Prints the order so it can be checked.
function Sort-ByVersion($Items, [scriptblock]$Key = { param($n) Get-VersionKey $n }) {
    $sorted = @($Items | Sort-Object { & $Key $_.Name })
    Write-Host "version order:"
    foreach ($item in $sorted) {
        Write-Host ("  {0,-12} {1}" -f (& $Key $item.Name), $item.Name)
    }
    return $sorted
}

# Relative paths of files present in every release directory, largest first (by size in
# the newest release). Release directories must already be sorted oldest first.
function Get-CommonFiles($ReleaseDirs, [int]$Top) {
    $common = $null
    foreach ($dir in $ReleaseDirs) {
        $root = $dir.FullName.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
        $rel = @{}
        Get-ChildItem -LiteralPath $dir.FullName -Recurse -File | ForEach-Object {
            $rel[$_.FullName.Substring($root.Length)] = $_.Length
        }
        if ($null -eq $common) {
            $common = $rel
        } else {
            $next = @{}
            foreach ($key in $common.Keys) {
                if ($rel.ContainsKey($key)) { $next[$key] = $rel[$key] }
            }
            $common = $next
        }
    }
    if ($null -eq $common -or $common.Count -eq 0) {
        throw "no file exists under the same relative path in every release"
    }
    return @($common.GetEnumerator() |
        Sort-Object Value -Descending |
        Select-Object -First $Top |
        ForEach-Object { $_.Key })
}

# Runs autocorr once per window over $Paths (oldest first) and writes each table to
# $OutDir\<Label>-h<window>.md.
function Invoke-Autocorr([string]$Label, [string[]]$Paths, [int[]]$Windows, [string]$OutDir) {
    if ($Paths.Count -lt 3) {
        throw "$Label`: need at least 3 versions, found $($Paths.Count)"
    }
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    $manifest = Join-Path $RepoRoot 'Cargo.toml'

    foreach ($h in $Windows) {
        $outFile = Join-Path $OutDir "$Label-h$h.md"
        Write-Host "== $Label, h = $h -> $outFile"
        $lines = & cargo run --release --quiet --manifest-path $manifest --example autocorr -- $h @Paths
        if ($LASTEXITCODE -ne 0) {
            throw "autocorr failed for $Label at h = $h (exit $LASTEXITCODE)"
        }
        $body = @("### $Label, h = $h", '') + $lines
        $body | Out-File -LiteralPath $outFile -Encoding utf8
        $lines | Write-Host
    }
}
