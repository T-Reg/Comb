"""Collects results2/*.md into one table per series: per type, share of the newest
release's bytes, and at each window the lag-1 median and pooled ratios, the median
ceiling (1/base), the median new %, and how many anchors were usable.

usage: python3 gather.py <results2 dir>  > SUMMARY-by-type.md
"""
import re
import statistics
import sys
from pathlib import Path

root = Path(sys.argv[1])
WINDOWS = [524288, 32768, 2048]
WNAME = {524288: "512K", 32768: "32K", 2048: "2K"}


def num(s):
    try:
        return float(s)
    except ValueError:
        return None


def lag1(path):
    """Lag-1 stats from one table: (median ratio, pooled ratio, median max, median new %, used, total)."""
    ratios, maxes, news, used, total = [], [], [], 0, 0
    pooled = None
    for line in path.read_text().splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 8 or cells[0] != "1":
            continue
        anchor, new, _base, _cond, ratio, mx, note = cells[1:]
        if anchor == "**pooled**":
            pooled = num(ratio.strip("*"))
        elif anchor.startswith("v"):
            total += 1
            if note.startswith("excluded"):
                continue
            used += 1
            news.append(num(new))
            if num(ratio) is not None:
                ratios.append(num(ratio))
            if num(mx) is not None:
                maxes.append(num(mx))
    med = lambda xs: statistics.median(xs) if xs else None
    return med(ratios), pooled, med(maxes), med([n for n in news if n is not None]), used, total


def fmt(x, d=2):
    return "n/a" if x is None else f"{x:.{d}f}"


def shares(series):
    """type -> % of bytes, from <series>-breakdown.md (or steamos-breakdown.md)."""
    out = {}
    f = root / f"{series}-breakdown.md"
    if not f.exists():
        return out
    for line in f.read_text().splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) in (3, 4) and cells[0] not in ("type", "partition") and not cells[0].startswith("-"):
            out[cells[0]] = num(cells[-1])
    return out


series = sorted({m.group(1) for f in root.glob("*-h*.md")
                 if (m := re.match(r"(.+?)-(.+)-h\d+\.md$", f.name)) and not m.group(1).endswith("order")})
# Series names can contain '-' (0ad-unzipped); classes are taken as whatever follows them.
series = sorted({f.name.rsplit("-h", 1)[0] for f in root.glob("*-h*.md")})
by_series = {}
for stem in series:
    for s in ("0ad-unzipped", "steamos_dedup", "steamos", "blender", "cdda", "mindustry", "minetest", "wesnoth", "openmw", "0ad"):
        if stem.startswith(s + "-"):
            by_series.setdefault(s, set()).add(stem[len(s) + 1:])
            break

print("# Step 2A.6 by file type: lag-1 summary\n")
print("Per window: **median** lag-1 ratio over usable anchors / pooled ratio, then the median")
print("ceiling `max` = 1/base (a ratio near its max is capped by churn), median `new %`, and")
print("usable anchors / all anchors. Rows sorted by share of bytes.\n")
for s in sorted(by_series):
    sh = shares(s)
    print(f"## {s}\n")
    order = (s + "-order.md")
    if (root / order).exists():
        print((root / order).read_text())
    hdr = "| type | % bytes | " + " | ".join(f"{WNAME[h]} median / pooled | {WNAME[h]} max | {WNAME[h]} new % | {WNAME[h]} used" for h in WINDOWS) + " |"
    print(hdr)
    print("|" + "---|" * (2 + 4 * len(WINDOWS)))
    classes = sorted(by_series[s], key=lambda c: (c != "all", -(sh.get(c.replace("_", "-"), sh.get(c)) or 0)))
    for c in classes:
        share = 100.0 if c == "all" else sh.get(c, sh.get(c.replace("_", "-")))
        if c == "other":
            known = sum(v for k, v in sh.items() if (root / f"{s}-{k}-h2048.md").exists() or (root / f"{s}-{k.replace('-', '_')}-h2048.md").exists())
            share = max(0.0, 100.0 - known)
        cells = [f"**{c}**" if c == "all" else c, fmt(share, 1)]
        for h in WINDOWS:
            f = root / f"{s}-{c}-h{h}.md"
            if not f.exists():
                cells += ["", "", "", ""]
                continue
            med, pooled, mx, new, used, total = lag1(f)
            cells += [f"{fmt(med)} / {fmt(pooled)}", fmt(mx), fmt(new, 1), f"{used}/{total}"]
        print("| " + " | ".join(cells) + " |")
    print()
