#!/bin/sh
# Runs inside ubuntu:24.04 with the corpus at /c. Extracts each release into
# /c/<source>/raw/<version> with version-only names so `ls -v` orders them.
set -u
apt-get update -qq && apt-get install -y -qq 7zip >/dev/null || exit 1
x() { [ -d "$2" ] && return; echo "extract $1 -> $2"; mkdir -p "$2.part" && 7z x -y -o"$2.part" "$1" >/dev/null && mv "$2.part" "$2" || echo "FAILED $1"; }
cd /c/cdda && for f in $(ls cdda-*.zip | tail -8); do v=${f#cdda-windows-with-graphics-x64-}; x "$f" raw/${v%.zip}; done
cd /c/mindustry && for f in Mindustry-v*.jar; do v=${f#Mindustry-v}; v=${v%.jar}; case $v in *.*) ;; *) v=$v.0;; esac; x "$f" raw/$v; done
cd /c/minetest && for f in *.zip; do v=${f#*-}; v=${v%-win64.zip}; x "$f" raw/$v; done
cd /c/wesnoth && for f in wesnoth-1.19.*-win64.exe; do v=${f#wesnoth-}; x "$f" raw/${v%-win64.exe}; done
cd /c/openmw && for f in OpenMW-*.exe; do v=${f#OpenMW-}; x "$f" raw/${v%%-*}; done
cd /c/0ad && for f in 0ad-0.0.2[3-6]*.exe 0ad-0.2[78]*.exe; do v=${f#0ad-}; v=${v%-win*.exe}; x "$f" raw/${v%-alpha}; done
chown -R "$OWNER" /c/cdda/raw /c/mindustry/raw /c/minetest/raw /c/wesnoth/raw /c/openmw/raw /c/0ad/raw
echo EXTRACT-DONE
