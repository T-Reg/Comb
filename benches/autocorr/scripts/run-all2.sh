#!/bin/bash
# Step 2A.6 per file type (results2/). SteamOS per partition, then each game per extension.
cd ~/comb-autocorr
C=/mnt/fast/code-server/corpus
./steamos-by-partition.sh
./series-by-type.sh blender $(ls -dv $C/blender/stable/raw/*)
for s in cdda mindustry minetest wesnoth openmw; do
  ./series-by-type.sh $s $(ls -dv $C/$s/raw/*)
done
echo ALL-DONE
