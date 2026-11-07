#!/bin/bash
# usage: series-by-type.sh <label> <release dir>...   (release dirs oldest first)
#
# Step 2A.6 per file type. For each release, every file is listed with its lowercase
# extension and a sort key (its path with version numbers replaced, so `5.1/scripts/x.py`
# and `5.2/scripts/x.py` line up). Per class, each release's files of that class are
# concatenated in key order and autocorr runs on the concatenations. Classes: every
# extension holding >= 1% of the newest release's bytes, "other" for the rest, and "all".
#
# Writes ~/comb-autocorr/results2/<label>-<class>-h<h>.md and <label>-breakdown.md.
set -euo pipefail
export LC_ALL=C
label=$1; shift
[ $# -ge 3 ] || { echo "$label: need at least 3 release dirs, got $#" >&2; exit 1; }

# Descend through wrapper folders (a release dir holding exactly one dir and nothing else).
dirs=(); for d in "$@"; do
  while [ "$(ls -A "$d" | wc -l)" = 1 ] && [ -d "$d/$(ls -A "$d")" ]; do d="$d/$(ls -A "$d")"; done
  dirs+=("$d")
done

bin=~/comb-autocorr/target/release/examples/autocorr
out=~/comb-autocorr/results2
tmp=/mnt/fast/code-server/comb-scratch/$label
mkdir -p "$out"; rm -rf "$tmp"; mkdir -p "$tmp"
n=${#dirs[@]}

# list.<i>: ext \t key \t size \t relpath, sorted by key.
for i in $(seq 1 $n); do
  (cd "${dirs[$((i-1))]}" && find . -type f -printf '%s\t%P\n') | awk -F'\t' '{
      p = $2; k = split(p, a, "/"); b = a[k]; e = "(none)"
      if (match(b, /.\.[^.]+$/)) e = tolower(substr(b, RSTART + 2))
      key = p; gsub(/[0-9]+(\.[0-9]+)+/, "V", key)
      print e "\t" key "\t" $1 "\t" p }' | sort -t$'\t' -k2,2 > "$tmp/list.$i"
done

# Breakdown of the newest release by extension.
total=$(awk -F'\t' '{s += $3} END {print s}' "$tmp/list.$n")
{
  echo "### $label: newest release by file type"
  echo
  echo "Release: \`${dirs[$((n-1))]}\` ($((total / 1000000)) MB, $(wc -l < "$tmp/list.$n") files)."
  echo "Classes >= 1% of bytes are measured on their own; the rest are \"other\"."
  echo
  echo "| type | files | MB | % of bytes |"
  echo "|---|---|---|---|"
  awk -F'\t' -v t="$total" '{s[$1] += $3; c[$1]++} END {
      for (e in s) printf "| %s | %d | %.1f | %.1f |\n", e, c[e], s[e] / 1e6, 100 * s[e] / t }' \
    "$tmp/list.$n" | sort -t'|' -k5 -rn
} > "$out/$label-breakdown.md"

classes=$(awk -F'\t' -v t="$total" '{s[$1] += $3} END {
    for (e in s) if (s[e] >= 0.01 * t) print e }' "$tmp/list.$n" | sort)
big=$(echo "$classes" | paste -sd'|')

for class in all $classes other; do
  mkdir -p "$tmp/c"
  for i in $(seq 1 $n); do
    d=${dirs[$((i-1))]}
    case $class in
      all)   cut -f4 "$tmp/list.$i" ;;
      other) awk -F'\t' -v big="^($big)\$" '$1 !~ big' "$tmp/list.$i" | cut -f4 ;;
      *)     awk -F'\t' -v e="$class" '$1 == e' "$tmp/list.$i" | cut -f4 ;;
    esac | (cd "$d" && xargs -r -d '\n' cat) > "$tmp/c/v$i.bin"
  done
  safe=${class//[^a-zA-Z0-9]/_}
  for h in 524288 32768 2048; do
    { echo "### $label, type $class, h = $h"; echo; "$bin" "$h" $(ls -v "$tmp"/c/v*.bin) 2>/dev/null; } \
      > "$out/$label-$safe-h$h.md" &
  done
  wait
  echo "done $label $class"
  rm -rf "$tmp/c"
done
rm -rf "$tmp"
