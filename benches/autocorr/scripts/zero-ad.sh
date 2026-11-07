#!/bin/bash
# 0 A.D. 0.0.11-0.0.22 (the last installers 7-Zip can open; 0.0.23+ use an NSIS payload
# it cannot parse). Run as installed, then again with every mod .zip unpacked in place.
set -euo pipefail
cd ~/comb-autocorr
C=/mnt/fast/code-server/corpus/0ad
docker run --rm -e OWNER=$(id -u):$(id -g) -v $C:/c debian:trixie sh -c '
  apt-get update -qq >/dev/null 2>&1 && apt-get install -y -qq 7zip >/dev/null 2>&1 || exit 1
  cd /c && mkdir -p raw
  for v in 11 12 13 14 15 16 17 18 19 20 21 22; do
    d=raw/0.0.$v; [ -d $d ] && continue
    echo "extract 0.0.$v"; 7z x -y -o$d.part 0ad-0.0.$v-alpha-win32.exe >/dev/null && mv $d.part $d || echo "FAILED 0.0.$v"
  done
  chown -R "$OWNER" raw'
./series-by-type.sh 0ad $(ls -dv $C/raw/0.0.*)
docker run --rm -e OWNER=$(id -u):$(id -g) -v $C:/c debian:trixie sh -c '
  apt-get update -qq >/dev/null 2>&1 && apt-get install -y -qq 7zip >/dev/null 2>&1 || exit 1
  find /c/raw -name "*.zip" | while read -r z; do 7z x -y -o"$z.d" "$z" >/dev/null && rm "$z" || echo "FAILED $z"; done
  chown -R "$OWNER" /c/raw'
./series-by-type.sh 0ad-unzipped $(ls -dv $C/raw/0.0.*)
echo ZEROAD-DONE
