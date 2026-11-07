#!/bin/bash
# Step 2A.6 on hive: SteamOS, then extract the rest and run every series.
cd ~/comb-autocorr
C=/mnt/fast/code-server/corpus
bin=./target/release/examples/autocorr
mkdir -p results
echo "== steamos BUILD_IDs (check order)"
for f in $C/steamos/raw/*.img; do echo "$(basename $f) $(grep -a -m1 -o 'BUILD_ID=[0-9.]*' $f)"; done
for h in 524288 32768 2048; do
  { echo "### steamos whole image, h = $h"; echo; $bin $h $(ls -v $C/steamos/raw/*.img) 2>/dev/null; } > results/steamos-1-h$h.md
  echo "done results/steamos-1-h$h.md"
done
docker run --rm -e OWNER=$(id -u):$(id -g) -v $C:/c -v ~/comb-autocorr/extract.sh:/extract.sh ubuntu:24.04 sh /extract.sh
for s in cdda mindustry minetest wesnoth openmw 0ad; do
  ./run-series.sh $s 3 $(ls -dv $C/$s/raw/* | grep -v '\.part$')
done
echo ALL-DONE
