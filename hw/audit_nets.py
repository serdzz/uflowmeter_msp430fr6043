"""Look for copper that should not be there.

A net routed twice still passes DRC -- both paths are on the same net, so nothing shorts and
nothing is unconnected. The only sign is that it carries far more copper than its pads need. This
compares each net's track length against the minimum spanning tree over its pads, which is the
shortest any router could manage, and reports the ones that are wildly over."""
import sys, collections, math, pcbnew

board = pcbnew.LoadBoard(sys.argv[1])
mm = pcbnew.ToMM

pads = collections.defaultdict(list)
for fp in board.Footprints():
    for p in fp.Pads():
        if p.GetNetname(): pads[p.GetNetname()].append(p.GetPosition())

length = collections.Counter(); vias = collections.Counter()
for t in board.Tracks():
    if isinstance(t, pcbnew.PCB_VIA): vias[t.GetNetname()] += 1
    else: length[t.GetNetname()] += mm(t.GetLength())

def mst(points):
    """shortest tree over the pads -- the floor for any routing of this net"""
    if len(points) < 2: return 0.0
    inside = [points[0]]; outside = list(points[1:]); total = 0.0
    while outside:
        best = min(((math.dist((a.x, a.y), (b.x, b.y)), b) for a in inside for b in outside),
                   key=lambda kv: kv[0])
        total += mm(int(best[0])); inside.append(best[1]); outside.remove(best[1])
    return total

print(f"{'net':14}{'copper':>9}{'floor':>8}{'ratio':>7}{'vias':>6}")
suspect = []
for name, pts in sorted(pads.items()):
    if name in ('GND', 'VCC') or len(pts) < 2: continue
    floor = mst(pts)
    got = length[name]
    if floor < 0.5: continue
    r = got / floor
    print(f"{name:14}{got:8.1f}{floor:8.1f}{r:7.1f}{vias[name]:6}")
    if r > 2.5: suspect.append((name, got, floor, r))
print()
if suspect:
    print("carrying far more copper than their pads need -- look for a second path:")
    for name, got, floor, r in sorted(suspect, key=lambda x: -x[3]):
        print(f"   {name:12} {got:6.1f} mm against a floor of {floor:5.1f} mm  ({r:.1f}x)")
else:
    print("nothing carrying more than 2.5x its floor")
