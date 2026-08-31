import sys, math, pcbnew
from pcbnew import FromMM as MM, VECTOR2I

board = pcbnew.LoadBoard(sys.argv[1])
NETS  = sys.argv[2].split(',')          # nets to stitch down to their plane

# every pad's bounding box, to test a candidate via against
obstacles = []
for fp in board.Footprints():
    for pad in fp.Pads():
        obstacles.append(pad.GetBoundingBox())

def clear(pt, r):
    box = pcbnew.BOX2I(VECTOR2I(pt.x - r, pt.y - r), pcbnew.VECTOR2L(2 * r, 2 * r))
    return not any(box.Intersects(o) for o in obstacles)

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
                if not clear(pt, VIA_D // 2 + MM(0.15)):
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
                obstacles.append(v.GetBoundingBox())
                added += 1; placed = True; break
            if placed: break
        if not placed: skipped += 1

pcbnew.ZONE_FILLER(board).Fill(board.Zones())
board.Save(sys.argv[1])
print(f"vias added {added}, no room for {skipped}")
