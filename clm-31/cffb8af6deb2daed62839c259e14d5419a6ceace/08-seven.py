"""Where the road signals that reach an unsignalled junction of 3+ roads stand, before and after."""
import sys

HERE = "/private/tmp/claude-501/-Users-alex-src-cityloom-main/40f831c0-2185-4631-8cf3-8f332b786f47/scratchpad/measure.py"
src = open(HERE).read().replace("\nmain()\n", "\n")
g = {}
exec(compile(src, "measure", "exec"), g)
signals, ways = g["read_opl"](sys.argv[1])
target_of = g["targets"](signals, ways)
links = g["links_of"](ways)
before, bo = g["read_tiles"](sys.argv[2])
after, ao = g["read_tiles"](sys.argv[3])
br = g["reached"](target_of, links, signals, ways, bo)
ar = g["reached"](target_of, links, signals, ways, ao)
for s in [9994830603, 5589008015, 11649397253, 11649397254, 10998451695, 11030876896, 9767349276, 12053666641,
          12982440248]:
    t, k = target_of[s]
    print(f"signal {s}: target {t}; in before tiles: {t in bo}, in after tiles: {t in ao}")
    for name, r, nodes in (("before", br, before), ("after", ar, after)):
        ids = (r.get(s) or (None,))[0]
        print(f"   {name}: reaches {list(ids) if ids else None} -> {nodes[ids] if ids else None}")
    for t2, nds in ways:
        if s in nds:
            print("   signal way:", {k: v for k, v in t2.items() if k in ("highway", "name", "oneway", "lanes")},
                  "len", len(nds), "idx", nds.index(s), "target idx", nds.index(t) if t in nds else None,
                  "first", nds[0], "last", nds[-1])
