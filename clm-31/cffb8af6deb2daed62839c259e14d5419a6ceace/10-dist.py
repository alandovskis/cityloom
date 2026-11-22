"""Length along the road ways from a split to the junction ahead: pairs of <split> <junction OSM node>."""
import math
import sys

pairs = [(int(a), int(b)) for a, b in (p.split(":") for p in sys.argv[2:])]
ways = []
coords = {}
for line in open(sys.argv[1]):
    parts = line.rstrip("\n").split(" ")
    f = {p[0]: p[1:] for p in parts[1:] if p}
    if parts[0][0] == "n":
        if "x" in f and f["x"]:
            coords[int(parts[0][1:])] = (float(f["x"]), float(f["y"]))
    elif parts[0][0] == "w":
        t = f.get("T", "")
        if "highway=cycleway" in t or "highway=path" in t or "highway=footway" in t:
            continue
        ways.append([int(n[1:]) for n in f.get("N", "").split(",") if n])


def d(a, b):
    (x1, y1), (x2, y2) = coords[a], coords[b]
    k = math.cos(math.radians((y1 + y2) / 2))
    return math.hypot((x2 - x1) * k, y2 - y1) * 111_320


for split, junction in pairs:
    best = None
    for nds in ways:
        for seq in (nds, nds[::-1]):
            if seq[0] == split and junction in seq:
                j = seq.index(junction)
                run = sum(d(seq[i], seq[i + 1]) for i in range(j))
                best = run if best is None else min(best, run)
    print(f"split {split} -> junction node {junction}: {best if best is None else round(best, 1)} m along one way")
