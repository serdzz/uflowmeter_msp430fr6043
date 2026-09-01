"""Cut every square corner with a 45 degree chamfer.

Tracks should run horizontal, vertical or at 45 degrees, and a right angle between two of them is
neither. This finds each place where exactly two tracks of one net meet on one layer at 90 degrees
with no via holding the junction, pulls both back, and joins them with the diagonal.

The chamfer is 0.4 mm or 40 % of the shorter leg, whichever is less, so a short segment is never
consumed entirely.
"""
import sys, math, collections, pcbnew
from pcbnew import VECTOR2I, FromMM as MM

CHAMFER = MM(0.2)

board = pcbnew.LoadBoard(sys.argv[1])

NEAR = MM(0.6)      # keep away from anything a track has to actually land on

anchors = [(v.GetPosition().x, v.GetPosition().y)
           for v in board.Tracks() if isinstance(v, pcbnew.PCB_VIA)]
for fp in board.Footprints():
    for pad in fp.Pads():
        anchors.append((pad.GetPosition().x, pad.GetPosition().y))

def near_anchor(x, y):
    return any(abs(ax-x) < NEAR and abs(ay-y) < NEAR for ax, ay in anchors)

def key(t, end):
    p = t.GetEnd() if end else t.GetStart()
    return (t.GetNetCode(), t.GetLayer(), p.x, p.y)

ends = collections.defaultdict(list)
segs = [t for t in board.Tracks() if not isinstance(t, pcbnew.PCB_VIA)]
for t in segs:
    ends[key(t, False)].append((t, False))
    ends[key(t, True)].append((t, True))

def direction(t, from_start):
    a = t.GetStart() if from_start else t.GetEnd()
    b = t.GetEnd() if from_start else t.GetStart()
    dx, dy = b.x - a.x, b.y - a.y
    n = math.hypot(dx, dy)
    return (dx / n, dy / n, n) if n else (0.0, 0.0, 0.0)

cut = 0
for (net, layer, x, y), joined in list(ends.items()):
    if len(joined) != 2: continue
    # Leave alone anything close to a via or a pad. Pulling a track back from one of those is how
    # a chamfer turns into a broken connection.
    if near_anchor(x, y): continue
    (t1, e1), (t2, e2) = joined
    if t1 is t2: continue
    d1 = direction(t1, not e1)                        # away from the corner
    d2 = direction(t2, not e2)
    if d1[2] == 0 or d2[2] == 0: continue
    dot = d1[0]*d2[0] + d1[1]*d2[1]
    if abs(dot) > 0.02: continue                      # only true right angles
    if d1[2] < MM(1.0) or d2[2] < MM(1.0): continue   # short legs are not worth the risk
    c = min(CHAMFER, 0.3*d1[2], 0.3*d2[2])
    if c < MM(0.1): continue
    p1 = VECTOR2I(int(x + d1[0]*c), int(y + d1[1]*c))
    p2 = VECTOR2I(int(x + d2[0]*c), int(y + d2[1]*c))
    if e1: t1.SetEnd(p1)
    else:  t1.SetStart(p1)
    if e2: t2.SetEnd(p2)
    else:  t2.SetStart(p2)
    d = pcbnew.PCB_TRACK(board)
    d.SetStart(p1); d.SetEnd(p2); d.SetWidth(t1.GetWidth())
    d.SetLayer(layer); d.SetNet(t1.GetNet())
    board.Add(d)
    cut += 1

pcbnew.ZONE_FILLER(board).Fill(board.Zones())
board.Save(sys.argv[1])
print(f"chamfered {cut} square corners")
