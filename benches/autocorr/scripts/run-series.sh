#!/bin/bash
# usage: run-series.sh <label> <top> <release dir>...   (release dirs oldest first)
# Runs autocorr on the <top> largest files present under the same relative path in
# every release, at the three ladder windows. Writes ~/comb-autocorr/results/<label>-<i>-h<h>.md
set -euo pipefail
label=$1; top=$2; shift 2
[ $# -ge 3 ] || { echo "$label: need at least 3 release dirs, got $#" >&2; exit 1; }
# Descend through wrapper folders (a release dir holding exactly one dir and nothing else).
dirs=(); for d in "$@"; do while [ "$(ls -A "$d" | wc -l)" = 1 ] && [ -d "$d/$(ls -A "$d")" ]; do d="$d/$(ls -A "$d")"; done; dirs+=("$d"); done
set -- "${dirs[@]}"
bin=~/comb-autocorr/target/release/examples/autocorr
out=~/comb-autocorr/results; mkdir -p "$out"
common=$(for d in "$@"; do (cd "$d" && find . -type f -printf '%P\n' | sort -u); done \
  | sort | uniq -c | awk -v n=$# '$1==n {sub(/^ *[0-9]+ /,""); print}')
last=${@: -1}
mapfile -t files < <(while IFS= read -r f; do printf '%s\t%s\n' "$(stat -c %s "$last/$f")" "$f"; done <<<"$common" \
  | sort -rn | head -n "$top" | cut -f2)
i=0
for f in "${files[@]}"; do
  i=$((i+1)); paths=(); for d in "$@"; do paths+=("$d/$f"); done
  for h in 524288 32768 2048; do
    o="$out/$label-$i-h$h.md"
    { echo "### $label #$i \`$f\`, h = $h"; echo; "$bin" "$h" "${paths[@]}" 2>/dev/null; } > "$o"
    echo "done $o"
  done
done
