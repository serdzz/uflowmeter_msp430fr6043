"""Route the 868 MHz chain.

Top layer only and no vias: a via in an RF path is a discontinuity, and the whole point of the
filter balun is that the impedance stays what TI characterised. The parts are already placed in
the reference topology's order, so every hop is between neighbours and wants nothing cleverer
than a straight line.

This does NOT make the section correct. The values are TI's; the geometry is not, and the
reference layout still wants transplanting before anyone trusts the radio's range or its
harmonics."""
import sys, collections, pcbnew
from pcbnew import FromMM as MM, VECTOR2I

RF = ['RF_P','RF_N','RFA','RFB','RFC','RFD','RFE','ANT','RF_SHUNT','RFNOTCH']
W  = MM(0.3)
CLR = MM(0.32)

board = pcbnew.LoadBoard(sys.argv[1])
obst = []
for fp in board.Footprints():
    for p in fp.Pads():
        lay = None if p.GetAttribute()==pcbnew.PAD_ATTRIB_PTH else p.GetLayer()
        obst.append((p.GetNetname(), p.GetBoundingBox(), lay))
for t in board.Tracks():
    lay = None if isinstance(t, pcbnew.PCB_VIA) else t.GetLayer()
    obst.append((t.GetNetname(), t.GetBoundingBox(), lay))

def clear_line(a, b, net):
    dx, dy = b.x-a.x, b.y-a.y
    n = max(2, int((abs(dx)+abs(dy))/MM(0.05)))
    for i in range(n+1):
        pt = VECTOR2I(int(a.x+dx*i/n), int(a.y+dy*i/n))
        bb = pcbnew.BOX2I(VECTOR2I(pt.x-CLR, pt.y-CLR), pcbnew.VECTOR2L(2*CLR, 2*CLR))
        for nm, o, lay in obst:
            if nm == net: continue
            if lay is not None and lay != pcbnew.F_Cu: continue
            if bb.Intersects(o): return False
    return True

pads = collections.defaultdict(list)
for fp in board.Footprints():
    for p in fp.Pads():
        if p.GetNetname() in RF: pads[p.GetNetname()].append(p)

done = skipped = 0
for net in RF:
    plist = pads.get(net, [])
    if len(plist) < 2: continue
    order = [plist[0]]; rest = plist[1:]
    while rest:
        last = order[-1].GetPosition()
        rest.sort(key=lambda p: (p.GetPosition().x-last.x)**2 + (p.GetPosition().y-last.y)**2)
        order.append(rest.pop(0))
    for i in range(len(order)-1):
        a, b = order[i].GetPosition(), order[i+1].GetPosition()
        if not clear_line(a, b, net): skipped += 1; continue
        t = pcbnew.PCB_TRACK(board)
        t.SetStart(a); t.SetEnd(b); t.SetWidth(W)
        t.SetLayer(pcbnew.F_Cu); t.SetNet(order[i].GetNet()); board.Add(t)
        obst.append((net, t.GetBoundingBox(), pcbnew.F_Cu))
        done += 1

pcbnew.ZONE_FILLER(board).Fill(board.Zones())
board.Save(sys.argv[1])
print(f"RF hops routed {done}, blocked {skipped}")
