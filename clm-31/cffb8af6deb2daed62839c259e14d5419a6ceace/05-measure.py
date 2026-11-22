"""How the importer's network tiles carry OpenStreetMap's traffic signals (CLM-31).

usage: python3 -I measure.py <roads.opl> <tiles before> <tiles after>

<roads.opl> is the importer's input as OPL text (`osmium cat roads.osm.pbf -f opl`); each tiles folder holds the
`<x>_<y>.json` network tiles `citymodel_builder tiles` writes. Every `highway=traffic_signals` node is followed the
way osm2streets places it: a signal at a node of two or more ways stays there; a signal inside a way, facing a
direction (`traffic_signals:direction`, or the way's one-way direction), goes to the next node ahead where another
road way meets or the way ends. Only the ways the importer hands osm2streets count as road ways: cycleways, paths
and footways do not (`is_off_road`). That node, the signal's "target", is looked up in the tiles.

The counts predict osm2streets' choice rather than instrument it, so they are close, not exact.
"""
import glob
import json
import re
import sys
from collections import Counter, defaultdict

OFF_ROAD = {"cycleway", "path", "footway"}


def dec(s):
    return re.sub(r"%([0-9a-f]+)%", lambda m: chr(int(m.group(1), 16)), s)


def tags_of(field):
    out = {}
    for kv in field.split(",") if field else []:
        if "=" in kv:
            k, v = kv.split("=", 1)
            out[dec(k)] = dec(v)
    return out


ISSUE_EXAMPLES = {
    31700054: "Avenue Vincent-D'Indy x Boulevard Edouard-Montpetit (divided road)",
    213649922: "Avenue Maplewood (signals before a cycleway crossing)",
    188871488: "unnamed tertiary_link (signals before a split of the way)",
}


def read_opl(path):
    signals, ways = {}, []
    for line in open(path):
        parts = line.rstrip("\n").split(" ")
        fields = {p[0]: p[1:] for p in parts[1:] if p}
        if parts[0][0] == "n":
            t = tags_of(fields.get("T", ""))
            if t.get("highway") == "traffic_signals":
                signals[int(parts[0][1:])] = t
        elif parts[0][0] == "w":
            t = tags_of(fields.get("T", ""))
            ways.append((t, [int(n[1:]) for n in fields.get("N", "").split(",") if n]))
    return signals, ways


def one_way(t):
    if t.get("oneway") in ("yes", "1", "true", "-1"):
        return True
    if t.get("oneway") in ("no", "false", "0", "reversible", "alternating"):
        return False
    return t.get("junction") in ("roundabout", "circular") or t.get("highway") == "motorway"


def way_of_signal(signals, ways):
    """The tags of the way each signal stands on (the first that holds it)."""
    out = {}
    for t, nds in ways:
        for n in nds:
            if n in signals and n not in out:
                out[n] = t
    return out


def links_of(ways):
    """The road ways that start, and that end, at each node."""
    starts, ends = defaultdict(list), defaultdict(list)
    for t, nds in ways:
        if t.get("highway") not in OFF_ROAD:
            starts[nds[0]].append(nds)
            ends[nds[-1]].append(nds)
    return starts, ends


def onward(n, links, signals, ways, by_osm):
    """For a signal: the first node of the tiles from it on in its direction, along its way and on along the road
    ways that go on from where its way ends (at most 20 of them)."""
    starts, ends = links
    for t, nds in ways:
        if n not in nds:
            continue
        forward = not (signals[n].get("traffic_signals:direction") == "backward" or t.get("oneway") == "-1")
        seq = nds if forward else nds[::-1]
        j = seq.index(n) + 1
        for _ in range(20):
            while j < len(seq):
                if seq[j] in by_osm:
                    return seq[j]
                j += 1
            nxt = [w for w in (starts if forward else ends)[seq[-1]] if w is not nds]
            if not nxt:
                return None
            nds = nxt[0]
            seq = nds if forward else nds[::-1]
            j = 1
        return None
    return None


def targets(signals, ways):
    """Each signal's target node, with the kind of way the signal stands on ("road" or "off-road")."""
    road_ways = [(t, n) for t, n in ways if t.get("highway") not in OFF_ROAD]
    uses = Counter()  # how many road-way ends or middles a node is: two or more, or an end, makes a node osm2streets keeps
    for _, nds in road_ways:
        for i, n in enumerate(nds):
            uses[n] += 2 if i in (0, len(nds) - 1) else 1
    out = {}
    for t, nds in ways:
        kind = "off-road" if t.get("highway") in OFF_ROAD else "road"
        for i, n in enumerate(nds):
            if n not in signals or n in out:
                continue
            if uses[n] >= 2 or i in (0, len(nds) - 1):
                out[n] = (n, kind)
                continue
            d = signals[n].get("traffic_signals:direction")
            if d == "forward" or (d is None and one_way(t) and t.get("oneway") != "-1"):
                step = 1
            elif d == "backward" or (d is None and t.get("oneway") == "-1"):
                step = -1
            else:
                continue  # osm2streets ignores a signal it cannot give a direction
            j = i + step
            while 0 < j < len(nds) - 1 and uses[nds[j]] < 2:
                j += step
            out[n] = (nds[j], kind)
    return out


def read_tiles(folder):
    """Every node of the tiles, by its OSM nodes: control, junction flag, OSM nodes, roads meeting there."""
    nodes = {}
    for f in glob.glob(folder + "/*_*.json"):
        net = json.load(open(f))["network"]
        ends = Counter()
        for r in net["roads"]:
            ends[r["edge"]["from"]] += 1
            ends[r["edge"]["to"]] += 1
        for x in net["junctions"]:
            ids = tuple(sorted(p["node"]["id"] for p in x["provenance"] if "node" in p))
            control = x["control"] if isinstance(x["control"], str) else "priority"
            roads = max(ends[x["id"]], nodes.get(ids, (None, None, 0))[2])  # a tile can cut roads off; take the most
            nodes[ids] = (control, x["junction"], roads)
    by_osm = {n: ids for ids in nodes for n in ids}
    return nodes, by_osm


def reached(target_of, links, signals, ways, by_osm):
    """Each signal's node of the tiles: its target, or, where the tiles have no node there (osm2streets dropped a
    split of the way), the first node of the tiles ahead of the signal; with the kind of way the signal stands on."""
    out = {}
    for n, (t, kind) in target_of.items():
        node = t if t in by_osm else onward(n, links, signals, ways, by_osm)
        if node in by_osm:
            out[n] = (by_osm[node], kind)
    return out


def lost(reach, nodes):
    """The nodes of the tiles a signal reaches that are not signalled, with the kinds of way the signals stand on."""
    out = defaultdict(set)
    for _, (ids, kind) in reach.items():
        if nodes[ids][0] != "signals":
            out[ids].add(kind)
    return out


def summary(name, nodes, missing):
    real = {ids: k for ids, k in missing.items() if nodes[ids][1]}
    print(f"== {name} ==")
    print(f"  signalled nodes: {sum(1 for c, _, _ in nodes.values() if c == 'signals')}, of them junctions (junction: true): "
          f"{sum(1 for c, j, _ in nodes.values() if c == 'signals' and j)}")
    print(f"  nodes a signal reaches that are not signalled: {len(missing)} "
          f"({sum(1 for i in missing if len(i) > 1)} made of several OSM nodes)")
    print(f"  of them junctions (junction: true): {len(real)} ({sum(1 for i in real if len(i) > 1)} made of several OSM nodes)")
    return real


def main():
    signals, ways = read_opl(sys.argv[1])
    target_of = targets(signals, ways)
    print(f"{len(signals)} highway=traffic_signals nodes; {len(target_of)} with a target "
          f"({sum(1 for _, k in target_of.values() if k == 'off-road')} of them on cycleways, paths or footways)\n")
    links = links_of(ways)
    before, before_osm = read_tiles(sys.argv[2])
    after, after_osm = read_tiles(sys.argv[3])
    print("A signal reaches its target node, or, where the tiles have no node there, the first node of the tiles "
          "ahead of it.\n")
    summary("BEFORE", before, lost(reached(target_of, links, signals, ways, before_osm), before))
    after_reach = reached(target_of, links, signals, ways, after_osm)
    missing = lost(after_reach, after)
    real = summary("AFTER", after, missing)

    print("\n== The issue's examples: each signal's target node, before and after ==")
    for n, what in ISSUE_EXAMPLES.items():
        t, _ = target_of[n]
        for name, nodes, by_osm in (("before", before, before_osm), ("after", after, after_osm)):
            node = t if t in by_osm else onward(n, links, signals, ways, by_osm)
            ids = by_osm.get(node)
            state = "no node of the tiles ahead" if ids is None else f"{nodes[ids][0]} (junction: {str(nodes[ids][1]).lower()}, OSM nodes {list(ids)})"
            print(f"  {what}: signal {n} -> {name}: {state}")

    print("\n== AFTER: each junction (junction: true) a signal reaches that is not signalled, with the reason ==")
    sig_way = way_of_signal(signals, ways)
    pointing = defaultdict(list)
    for n, (ids, _) in after_reach.items():
        pointing[ids].append(n)
    reasons = Counter()
    for ids, kinds in missing.items():
        control, junction, roads = after[ids]
        if not junction:
            reasons["not a junction (junction: false): the editor uses no control there"] += 1
            continue
        if "road" not in kinds:
            reason = "reached only by signals on cycleways, paths or footways, ways the importer keeps out of osm2streets"
        elif roads < 3:
            reason = f"{roads} road(s) of the network meet there: the editor opens a junction only where 3 to 5 do"
        else:
            reason = "reached by a road's signals that osm2streets did not place there (see their tags)"
        reasons["a junction (junction: true)"] += 1
        print(f"  OSM nodes {list(ids)}: now {control}, {roads} road(s); {reason}")
        for s in pointing[ids]:
            w = sig_way[s]
            way = {k: w[k] for k in ("highway", "name", "oneway", "access", "bus", "psv") if k in w}
            print(f"      signal {s} {signals[s]} on way {way}")
    print("  totals:")
    for reason, n in reasons.most_common():
        print(f"  {n:5d}  {reason}")

    print("\n== Nodes signalled BEFORE that are not signalled AFTER ==")
    changed = Counter()
    real_changed = []
    for ids, (control, junction, _) in before.items():
        if control != "signals":
            continue
        now = {after_osm[n] for n in ids if n in after_osm}
        if not now:
            kind = "a junction (junction: true)" if junction else "not a junction (junction: false)"
            changed[f"gone: no node of the network now; before, {kind}"] += 1
        elif any(after[i][0] == "signals" for i in now):
            changed["still signalled"] += 1
        elif not junction:
            changed["not a junction (junction: false) before, not signalled now"] += 1
        else:
            real_changed.append((ids, now))
    for k, n in changed.most_common():
        print(f"  {n:5d}  {k}")
    print(f"  {len(real_changed):5d}  junctions (junction: true) not signalled now")

    print("\n== The nodes signalled BEFORE that are no node of the network now: where their signals are now ==")
    print("  (osm2streets calls two two-way roads meeting a junction too, so junction: true includes splits of a way)")
    targeted_by = defaultdict(list)
    for n, (t, _) in target_of.items():
        if t in before_osm:
            targeted_by[before_osm[t]].append(n)
    gone_roads = Counter()
    landing = Counter()
    for ids, (control, junction, roads) in before.items():
        if control != "signals" or any(n in after_osm for n in ids):
            continue
        gone_roads[f"{roads} road(s) of the network met there before"] += 1
        for s in targeted_by[ids]:
            node = onward(s, links, signals, ways, after_osm)
            at = after_osm.get(node)
            landing["no node of the tiles ahead" if at is None else
                    f"{after[at][0]} (junction: {str(after[at][1]).lower()}, {after[at][2]} roads)"] += 1
    for k, n in gone_roads.most_common():
        print(f"  {n:5d}  {k}")
    print("  the signals that targeted them, by the node they reach now:")
    for k, n in landing.most_common():
        print(f"  {n:5d}  {k}")
    for ids, now in real_changed:
        for i in now:
            c, j, roads = after[i]
            print(f"         OSM nodes {list(ids)}: now {c}, junction: {str(j).lower()}, {roads} road(s) meet there")


main()
