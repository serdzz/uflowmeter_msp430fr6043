"""Route the signal nets the way a person would: escape each pad to a via, then run on the
bottom layer where there is almost nothing in the way."""
import sys, math, collections, pcbnew
from pcbnew import FromMM as MM, VECTOR2I


def nets_with_work(board_path):
    """Which nets still have something unconnected, asked of DRC rather than of the API.

    pcbnew's GetRatsnestForNet hands back an opaque object with no length, so a naive check on it
    silently reports every net as unfinished -- which is how two nets ended up routed twice."""
    import subprocess, json, re, tempfile, os
    out = os.path.join(tempfile.gettempdir(), "_unconn.json")
    subprocess.run(["kicad-cli", "pcb", "drc", "--output", out, "--format", "json",
                    "--severity-error", board_path], capture_output=True)
    try:    items = json.load(open(out)).get("unconnected_items", [])
    except Exception: return None            # cannot tell -- route everything
    names = set()
    for it in items:
        for i in it.get("items", []):
            m = re.search(r"\[([^\]]+)\]", i.get("description", ""))
            if m: names.add(m.group(1))
    return names

board = pcbnew.LoadBoard(sys.argv[1])
SKIP = {'RF_P','RF_N','RFA','RFB','RFC','RFD','RFE','ANT','RF_SHUNT','GND','VCC'}

VIA_D, DRILL, W = MM(0.6), MM(0.3), MM(0.25)

# (net, bounding box, layer) -- and the layer matters: a track on the bottom runs happily under
# a surface-mount pad on the top, which is the whole reason to escape downward at all.
BOTH = None
obst = []
for fp in board.Footprints():
    for p in fp.Pads():
        lay = BOTH if p.GetAttribute() == pcbnew.PAD_ATTRIB_PTH else p.GetLayer()
        obst.append((p.GetNetname(), p.GetBoundingBox(), lay))
for t in board.Tracks():
    lay = BOTH if isinstance(t, pcbnew.PCB_VIA) else t.GetLayer()
    obst.append((t.GetNetname(), t.GetBoundingBox(), lay))

def free(pt, r, net=None, layer=BOTH):
    bb = pcbnew.BOX2I(VECTOR2I(pt.x-r, pt.y-r), pcbnew.VECTOR2L(2*r, 2*r))
    for n, o, l in obst:
        if n == net: continue
        if layer is not BOTH and l is not BOTH and l != layer: continue
        if bb.Intersects(o): return False
    return True

def line_free(a, b, r, net=None, layer=BOTH):
    dx, dy = b.x-a.x, b.y-a.y
    n = max(2, int((abs(dx)+abs(dy)) / MM(0.25)))
    for i in range(n+1):
        pt = VECTOR2I(int(a.x+dx*i/n), int(a.y+dy*i/n))
        if not free(pt, r, net, layer): return False
    return True

# Only nets that still have something unconnected. Without this the pass re-routes finished nets
# from scratch and lays a second complete path beside the first -- which is how CH1 ended up with
# 59 mm of copper and four vias where one path does, and USSXTIN with two.
TODO_NETS = nets_with_work(sys.argv[1])

pads = collections.defaultdict(list)
skipped_done = 0
for fp in board.Footprints():
    for p in fp.Pads():
        if p.GetNetname(): pads[p.GetNetname()].append(p)
for name in list(pads):
    if TODO_NETS is not None and name not in TODO_NETS:
        del pads[name]; skipped_done += 1
if skipped_done:
    print(f"  {skipped_done} nets already connected, left alone")

def escape(pad):
    """A via clear of the package, reached by a stub from the pad.

    On a 0.5 mm pitch QFN the neighbouring pins are half a millimetre away, so a via anywhere
    beside a pad lands on one. The escape has to run outward, past the pad row, before it can
    turn into a hole -- which is exactly what a person does by hand."""
    p = pad.GetPosition()
    c = pad.GetParent().GetPosition()
    out = math.atan2(p.y - c.y, p.x - c.x) if (p.x, p.y) != (c.x, c.y) else 0.0
    angles = sorted(range(0, 360, 10), key=lambda a: abs(((a - math.degrees(out) + 180) % 360) - 180))
    for d in (1.0, 1.4, 1.8, 2.2, 2.6, 3.0, 3.6):
        for ang in angles:
            a = math.radians(ang)
            pt = VECTOR2I(int(p.x+MM(d)*math.cos(a)), int(p.y+MM(d)*math.sin(a)))
            net = pad.GetNetname()
            if free(pt, VIA_D//2 + MM(0.3), net) and line_free(pt, p, MM(0.3), net, pcbnew.F_Cu):
                return pt
    return None

def add_via(pt, net):
    v = pcbnew.PCB_VIA(board); v.SetPosition(pt); v.SetWidth(VIA_D); v.SetDrill(DRILL)
    v.SetNet(net); v.SetViaType(pcbnew.VIATYPE_THROUGH)
    v.SetLayerPair(pcbnew.F_Cu, pcbnew.B_Cu); board.Add(v)
    obst.append((v.GetNetname(), v.GetBoundingBox(), BOTH)); return v

def add_track(a, b, layer, net):
    t = pcbnew.PCB_TRACK(board); t.SetStart(a); t.SetEnd(b); t.SetWidth(W)
    t.SetLayer(layer); t.SetNet(net); board.Add(t)
    obst.append((t.GetNetname(), t.GetBoundingBox(), layer)); return t

done = failed = 0
for net, plist in sorted(pads.items(), key=lambda kv: -len(kv[1])):
    if net in SKIP or len(plist) < 2: continue
    vias = []
    for pad in plist:
        pt = escape(pad)
        if pt is None: vias.append(None); continue
        vias.append(pt)
    good = [(pad, pt) for pad, pt in zip(plist, vias) if pt]
    if len(good) < 2: failed += 1; continue
    # nearest-neighbour chain over the escape points
    order = [good[0]]; rest = good[1:]
    while rest:
        last = order[-1][1]
        rest.sort(key=lambda g: (g[1].x-last.x)**2 + (g[1].y-last.y)**2)
        order.append(rest.pop(0))
    # A straight run is tried first, then an L, then a Z with the corner walked sideways.
    # Real routing turns corners; only allowing straight lines rejects almost everything.
    def path(a, b):
        """bottom first -- it is nearly empty -- then the top, which the pour yields to"""
        for lay in (pcbnew.B_Cu, pcbnew.F_Cu):
            if line_free(a, b, MM(0.3), net, lay): return lay, [a, b]
            for c in (VECTOR2I(a.x, b.y), VECTOR2I(b.x, a.y)):
                if line_free(a, c, MM(0.3), net, lay) and line_free(c, b, MM(0.3), net, lay):
                    return lay, [a, c, b]
            for off in (2, -2, 4, -4, 6, -6, 9, -9, 13, -13):
                for m in (VECTOR2I((a.x+b.x)//2 + MM(off), (a.y+b.y)//2),
                          VECTOR2I((a.x+b.x)//2, (a.y+b.y)//2 + MM(off))):
                    if line_free(a, m, MM(0.3), net, lay) and line_free(m, b, MM(0.3), net, lay):
                        return lay, [a, m, b]
        return None

    paths = [path(order[i][1], order[i+1][1]) for i in range(len(order)-1)]
    if any(p is None for p in paths): failed += 1; continue
    net_obj = plist[0].GetNet()
    for pad, pt in order:
        add_via(pt, net_obj); add_track(pad.GetPosition(), pt, pcbnew.F_Cu, net_obj)
    for lay, pts in paths:
        for i in range(len(pts)-1):
            add_track(pts[i], pts[i+1], lay, net_obj)
    done += 1

pcbnew.ZONE_FILLER(board).Fill(board.Zones())
board.Save(sys.argv[1])
print(f"routed {done} nets, {failed} could not be escaped or crossed")
