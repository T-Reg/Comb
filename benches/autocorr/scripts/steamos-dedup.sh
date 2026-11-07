#!/bin/bash
# Step 2A.6 on SteamOS, per GPT partition. First reads each image's os-release from its
# btrfs rootfs (btrfs restore in a container, no mount) to put the images in true release
# order, then runs autocorr per partition and on the whole image.
#
# Writes ~/comb-autocorr/results2/steamos-<partition>-h<h>.md, steamos-order.md and
# steamos-breakdown.md.
set -euo pipefail
export LC_ALL=C
raw=/mnt/fast/code-server/corpus/steamos/raw
bin=~/comb-autocorr/target/release/examples/autocorr
out=~/comb-autocorr/results2
tmp=/mnt/fast/code-server/comb-scratch/steamos_dedup
mkdir -p "$out"; rm -rf "$tmp"; mkdir -p "$tmp"

# start and size (512-byte sectors) of partition <name> in image <img>
part() { sfdisk -d "$1" | grep "name=\"$2\"" | sed -E 's/.*start= *([0-9]+), size= *([0-9]+).*/\1 \2/'; }

# 1. Release order from os-release.
: > "$tmp/order"
for img in "$raw"/*.img; do
  b=$(basename "$img" .img)
  read -r s z < <(part "$img" rootfs-A)
  dd if="$img" of="$tmp/rootfs.img" bs=512 skip="$s" count="$z" status=none
  docker run --rm -v "$tmp":/w ubuntu:24.04 sh -c '
      apt-get update -qq >/dev/null && apt-get install -y -qq btrfs-progs >/dev/null &&
      mkdir -p /w/r && timeout 600 btrfs restore -i -S --path-regex "^/(|etc(|/os-release)|usr(|/lib(|/os-release)))$" /w/rootfs.img /w/r >/dev/null 2>&1;
      cat /w/r/usr/lib/os-release /w/r/etc/os-release 2>/dev/null; rm -rf /w/r' > "$tmp/osr" || true
  ver=$(grep -m1 '^VERSION_ID=' "$tmp/osr" | cut -d= -f2 | tr -d '"')
  build=$(grep -m1 '^BUILD_ID=' "$tmp/osr" | cut -d= -f2 | tr -d '"')
  echo "${build:-unknown} ${ver:-unknown} $b" >> "$tmp/order"
  rm -f "$tmp/rootfs.img"
done
# Keep one image per BUILD_ID: recovery-3 and recovery-4 are the same build re-issued, and
# the bytes that differ between them are re-issue noise (UUIDs, timestamps), not a release.
sort -V "$tmp/order" | awk '$1 == "unknown" || !seen[$1]++' > "$tmp/order.sorted"
{
  echo "### SteamOS release order (from os-release in rootfs-A)"
  echo
  echo "| v | BUILD_ID | VERSION_ID | image |"
  echo "|---|---|---|---|"
  awk '{printf "| v%d | %s | %s | %s |\n", NR, $1, $2, $3}' "$tmp/order.sorted"
} > "$out/steamos_dedup-order.md"
cat "$out/steamos_dedup-order.md"
mapfile -t imgs < <(awk -v r="$raw" '{print r "/" $3 ".img"}' "$tmp/order.sorted")

# 2. Breakdown by partition (layout is the same in every image).
first=${imgs[0]}
imgsize=$(stat -c %s "$first")
{
  echo "### SteamOS image by partition"
  echo
  echo "| partition | MB | % of image |"
  echo "|---|---|---|"
  for p in esp efi-A rootfs-A var-A home; do
    read -r s z < <(part "$first" $p)
    awk -v p=$p -v z="$z" -v t="$imgsize" 'BEGIN {printf "| %s | %.1f | %.1f |\n", p, z * 512 / 1e6, 100 * z * 512 / t}'
  done
} > "$out/steamos_dedup-breakdown.md"

# 3. Whole image and each partition.
run() {  # <class> <files...>
  local class=$1; shift
  for h in 524288 32768 2048; do
    { echo "### steamos, $class, h = $h"; echo; "$bin" "$h" "$@" 2>/dev/null; } > "$out/steamos_dedup-$class-h$h.md" &
  done
  wait
  echo "done steamos $class"
}
run all "${imgs[@]}"
for p in esp efi-A rootfs-A var-A home; do
  files=()
  for i in "${!imgs[@]}"; do
    read -r s z < <(part "${imgs[$i]}" $p)
    dd if="${imgs[$i]}" of="$tmp/v$((i+1)).bin" bs=512 skip="$s" count="$z" status=none
    files+=("$tmp/v$((i+1)).bin")
  done
  run "$p" "${files[@]}"
  rm -f "$tmp"/v*.bin
done
rm -rf "$tmp"
