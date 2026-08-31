import sys, math, pcbnew
from pcbnew import FromMM as MM, VECTOR2I

board = pcbnew.LoadBoard(sys.argv[1])
NETS  = sys.argv[2].split(',')          # nets to stitch down to their plane

# Everything a via or its stub has to keep away from -- pads AND whatever copper is already on
# the board. Testing only pads is how stitching vias end up sitting on hand-laid tracks.
obstacles = []
for fp in board.Footprints():
    for pad in fp.Pads():
        obstacles.append((pad.GetNetname(), pad.GetBoundingBox()))
for t in board.Tracks():
    obstacles.append((t.GetNetname(), t.GetBoundingBox()))

def clear(pt, r, net=None):
    box = pcbnew.BOX2I(VECTOR2I(pt.x - r, pt.y - r), pcbnew.VECTOR2L(2 * r, 2 * r))
    return not any(box.Intersects(o) for n, o in obstacles if n != net)

def stub_clear(a, b, r, net=None):
    """the short run from pad to via has to be clear too, not just the via"""
    dx, dy = b.x - a.x, b.y - a.y
    n = max(2, int((abs(dx) + abs(dy)) / MM(0.1)))
    for i in range(n + 1):
        if not clear(VECTOR2I(int(a.x + dx*i/n), int(a.y + dy*i/n)), r, net):
            return False
    return True

VIA_D, VIA_DRILL = MM(0.6), MM(0.3)
added = skipped = 0
for fp in list(board.Footprints()):
    for pad in fp.Pads():
        if pad.GetNetname() not in NETS:
            continue
        if pad.GetAttribute() == pcbnew.PAD_ATTRIB_PTH:
            continue                     # a through pin already reaches every layer
        p = pad.GetPosition()
        placed = False
        for dist in (0.75, 1.0, 1.3):
            for ang in range(0, 360, 30):
                a = math.radians(ang)
                pt = VECTOR2I(int(p.x + MM(dist) * math.cos(a)), int(p.y + MM(dist) * math.sin(a)))
                nm = pad.GetNetname()
                if not clear(pt, VIA_D // 2 + MM(0.2), nm):
                    continue
                if not stub_clear(p, pt, MM(0.3), nm):
                    continue
                v = pcbnew.PCB_VIA(board)
                v.SetPosition(pt); v.SetWidth(VIA_D); v.SetDrill(VIA_DRILL)
                v.SetNet(pad.GetNet()); v.SetViaType(pcbnew.VIATYPE_THROUGH)
                v.SetLayerPair(pcbnew.F_Cu, pcbnew.B_Cu)
                board.Add(v)
                # a short trace from the pad to the via
                t = pcbnew.PCB_TRACK(board)
                t.SetStart(p); t.SetEnd(pt); t.SetWidth(MM(0.3))
                t.SetLayer(pcbnew.F_Cu); t.SetNet(pad.GetNet())
                board.Add(t)
                obstacles.append((nm, v.GetBoundingBox()))
                obstacles.append((nm, t.GetBoundingBox()))
                added += 1; placed = True; break
            if placed: break
        if not placed: skipped += 1

pcbnew.ZONE_FILLER(board).Fill(board.Zones())
board.Save(sys.argv[1])
print(f"vias added {added}, no room for {skipped}")
