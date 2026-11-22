"""The road ways at each given node: tags, and where the node is in the way."""
import sys

HERE = "/private/tmp/claude-501/-Users-alex-src-cityloom-main/40f831c0-2185-4631-8cf3-8f332b786f47/scratchpad/measure.py"
src = open(HERE).read().replace("\nmain()\n", "\n")
g = {}
exec(compile(src, "measure", "exec"), g)
signals, ways = g["read_opl"](sys.argv[1])
want = [int(a) for a in sys.argv[2:]]
for n in want:
    print(f"node {n}: signal tags {signals.get(n)}")
    for t, nds in ways:
        if n in nds:
            keep = ("highway", "name", "oneway", "lanes", "layer", "turn:lanes")
            print(f"   {[k + '=' + t[k] for k in keep if k in t]} pos {nds.index(n)}/{len(nds) - 1} ends {nds[0]}..{nds[-1]}")
