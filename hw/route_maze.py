"""A grid router: A* on two layers, eight directions, vias where a layer change earns its cost.

Octile movement is what gives the result its shape -- runs come out horizontal, vertical or at
45 degrees, the way a person draws them, rather than as whatever straight line happened to be
clear."""
import sys, heapq, math, collections, pcbnew
from array import array
from pcbnew import FromMM as MM, VECTOR2I

BRD   = sys.argv[1]
GRID  = 0.05                     # mm per cell
# Keep-out around foreign copper. 0.275 mm is what the rules actually need -- half a track plus
# the class clearance -- and the margin above that has to stay small: the keep-out is painted as
# a rectangle, so on a 0.5 mm pitch QFN too much of it closes the one way out of a pin, which is
# straight outward along the pin's own axis.
CLR   = 0.28
# 0.15 mm, not 0.25. On a 0.5 mm pitch QFN the neighbouring pads sit 0.35 mm from the pin's own
# axis, so a 0.25 mm track -- which needs 0.275 mm of room from its centre -- cannot leave the pin
# at all; the keep-outs meet over the escape corridor and close it. At 0.15 mm the corridor opens.
W     = MM(0.15)
VIA_D, DRILL = MM(0.6), MM(0.3)
VIA_COST = 24                    # in cells: a via is worth about 5 mm of track
SKIP  = {'RF_P','RF_N','RFA','RFB','RFC','RFD','RFE','ANT','RF_SHUNT','GND','VCC'}

board = pcbnew.LoadBoard(BRD)
bb = board.GetBoardEdgesBoundingBox()
X0, Y0 = bb.GetLeft(), bb.GetTop()
NX = int(pcbnew.ToMM(bb.GetWidth())  / GRID) + 1
NY = int(pcbnew.ToMM(bb.GetHeight()) / GRID) + 1
LAYERS = [pcbnew.F_Cu, pcbnew.B_Cu]
LI = {pcbnew.F_Cu: 0, pcbnew.B_Cu: 1}

def cell(pt):  return (int(round(pcbnew.ToMM(pt.x - X0)/GRID)), int(round(pcbnew.ToMM(pt.y - Y0)/GRID)))
def point(cx, cy): return VECTOR2I(int(X0 + MM(cx*GRID)), int(Y0 + MM(cy*GRID)))

# owner[layer][y*NX+x] -- 0 free, else the net code that occupies it
owner = [array('H', bytes(2*NX*NY)) for _ in LAYERS]

MULTI = 65534   # claimed by more than one net: nobody may pass

def paint(bbox, netcode, layers, grow=CLR, force=False):
    x0 = int((pcbnew.ToMM(bbox.GetLeft()  - X0) - grow)/GRID)
    x1 = int((pcbnew.ToMM(bbox.GetRight() - X0) + grow)/GRID)
    y0 = int((pcbnew.ToMM(bbox.GetTop()   - Y0) - grow)/GRID)
    y1 = int((pcbnew.ToMM(bbox.GetBottom()- Y0) + grow)/GRID)
    for li in layers:
        o = owner[li]
        for y in range(max(0,y0), min(NY-1,y1)+1):
            base = y*NX
            for x in range(max(0,x0), min(NX-1,x1)+1):
                if force:
                    o[base+x] = netcode or 65535
                elif o[base+x] == 0:
                    o[base+x] = netcode or 65535
                elif o[base+x] != (netcode or 65535):
                    # two nets want this cell; on a 0.5 mm pitch package their keep-outs
                    # overlap, and letting the first claimant own it is how a track ends up
                    # crossing its neighbour's pad
                    o[base+x] = MULTI

BOTH = [0,1]
for fp in board.Footprints():
    for p in fp.Pads():
        lay = BOTH if p.GetAttribute()==pcbnew.PAD_ATTRIB_PTH else [LI[p.GetLayer()]] if p.GetLayer() in LI else BOTH
        paint(p.GetBoundingBox(), p.GetNetCode(), lay)
for t in board.Tracks():
    lay = BOTH if isinstance(t, pcbnew.PCB_VIA) else ([LI[t.GetLayer()]] if t.GetLayer() in LI else BOTH)
    paint(t.GetBoundingBox(), t.GetNetCode(), lay)

# Pad bodies go on last and unconditionally, so a pad stays reachable by its own net even
# though its neighbours' keep-outs have painted MULTI over it.
for fp in board.Footprints():
    for p in fp.Pads():
        lay = BOTH if p.GetAttribute()==pcbnew.PAD_ATTRIB_PTH else [LI[p.GetLayer()]] if p.GetLayer() in LI else BOTH
        paint(p.GetBoundingBox(), p.GetNetCode(), lay, grow=0.0, force=True)

pads = collections.defaultdict(list)
for fp in board.Footprints():
    for p in fp.Pads():
        if p.GetNetname(): pads[p.GetNetname()].append(p)

DIRS = [(1,0,10),(-1,0,10),(0,1,10),(0,-1,10),(1,1,14),(1,-1,14),(-1,1,14),(-1,-1,14)]

def cells_of(pad):
    """Only cells whose centre actually lands on the pad's copper.

    A QFN pad is 0.25 mm wide -- narrower than a grid cell -- so taking the 3x3 neighbourhood
    puts the track's endpoint beside the pad rather than on it, and nothing connects."""
    c = cell(pad.GetPosition()); out=[]
    for dy in (-2,-1,0,1,2):
        for dx in (-2,-1,0,1,2):
            x,y = c[0]+dx, c[1]+dy
            if not (0<=x<NX and 0<=y<NY): continue
            if pad.HitTest(point(x,y)): out.append((x,y))
    if not out: out=[c]
    lays = (0,1) if pad.GetAttribute()==pcbnew.PAD_ATTRIB_PTH else \
           ((LI[pad.GetLayer()],) if pad.GetLayer() in LI else (0,1))
    return [(x,y,l) for (x,y) in out for l in lays]

VIA_CELLS = int((0.3 + 0.15)/GRID) + 1

def via_ok(x, y, netcode):
    for li in (0,1):
        for dy in range(-VIA_CELLS, VIA_CELLS+1):
            yy = y+dy
            if not (0 <= yy < NY): return False
            base = yy*NX
            for dx in range(-VIA_CELLS, VIA_CELLS+1):
                xx = x+dx
                if not (0 <= xx < NX): return False
                if owner[li][base+xx] not in (0, netcode): return False
    return True

def route(sources, targets, netcode):
    """multi-source A* to the nearest target cell"""
    tset = set(targets)
    if not tset: return None
    tx = sum(t[0] for t in targets)/len(targets); ty = sum(t[1] for t in targets)/len(targets)
    # Weighted A*: paths come out a few per cent longer and the search finishes in a fraction of
    # the nodes, which is the only way a 0.05 mm grid is affordable in Python.
    def h(x,y):
        dx,dy = abs(x-tx), abs(y-ty)
        return int(12*max(dx,dy) + 5*min(dx,dy))

    # and it never needs to look far outside the box the net lives in
    MARG = int(12.0/GRID)
    axs=[c[0] for c in sources]+[t[0] for t in targets]
    ays=[c[1] for c in sources]+[t[1] for t in targets]
    lox, hix = min(axs)-MARG, max(axs)+MARG
    loy, hiy = min(ays)-MARG, max(ays)+MARG
    pq=[]; best={}
    for (x,y,li) in sources:
        if owner[li][y*NX+x] in (0, netcode):
            st=(x,y,li); best[st]=0; heapq.heappush(pq,(h(x,y),0,st,None))
    seen={}
    while pq:
        f,g,st,par = heapq.heappop(pq)
        if st in seen: continue
        seen[st]=par
        x,y,li = st
        if (x,y,li) in tset and owner[li][y*NX+x] in (0,netcode):
            path=[]; cur=st
            while cur is not None: path.append(cur); cur=seen[cur]
            return path[::-1]
        for dx,dy,c in DIRS:
            nx,ny = x+dx, y+dy
            if not (lox<=nx<=hix and loy<=ny<=hiy): continue
            if not (0<=nx<NX and 0<=ny<NY): continue
            o = owner[li][ny*NX+nx]
            if o not in (0, netcode): continue
            # no corner cutting: a 45 degree step sweeps the two cells beside it, and letting it
            # clip one is how a track ends up a hair too close to the pin next door
            if dx and dy:
                if owner[li][y*NX+nx] not in (0, netcode): continue
                if owner[li][ny*NX+x] not in (0, netcode): continue
            ns=(nx,ny,li); ng=g+c
            if best.get(ns, 1<<30) <= ng: continue
            best[ns]=ng; heapq.heappush(pq,(ng+h(nx,ny),ng,ns,st))
        # a via is 0.6 mm of copper, so the cells around it have to be free too -- checking
        # only the centre is how vias end up sitting on their neighbours
        ol = 1-li
        if via_ok(x, y, netcode):
            ns=(x,y,ol); ng=g+VIA_COST*10
            if best.get(ns,1<<30) > ng:
                best[ns]=ng; heapq.heappush(pq,(ng+h(x,y),ng,ns,st))
    return None

def commit(path, net):
    """turn the cell path into segments and vias, merging every straight run"""
    added=[]
    i=0
    while i < len(path)-1:
        x,y,li = path[i]; nx,ny,nl = path[i+1]
        if nl != li:
            v=pcbnew.PCB_VIA(board); v.SetPosition(point(x,y)); v.SetWidth(VIA_D); v.SetDrill(DRILL)
            v.SetNet(net); v.SetViaType(pcbnew.VIATYPE_THROUGH); v.SetLayerPair(pcbnew.F_Cu,pcbnew.B_Cu)
            board.Add(v); added.append(v); i+=1; continue
        d=(nx-x, ny-y); j=i+1
        while j < len(path)-1:
            ax,ay,al = path[j]; bx,by,bl = path[j+1]
            if bl!=al or (bx-ax,by-ay)!=d: break
            j+=1
        t=pcbnew.PCB_TRACK(board); t.SetStart(point(*path[i][:2])); t.SetEnd(point(*path[j][:2]))
        t.SetWidth(W); t.SetLayer(LAYERS[li]); t.SetNet(net); board.Add(t); added.append(t)
        i=j
    for a in added:
        lay = BOTH if isinstance(a, pcbnew.PCB_VIA) else [LI[a.GetLayer()]]
        paint(a.GetBoundingBox(), net.GetNetCode(), lay)

# A greedy router spends whatever space it is given, so the order it takes the nets in changes
# the result more than any of its costs do. The driver tries several and keeps the best.
ORDER = sys.argv[2] if len(sys.argv) > 2 else 'fat'

def span(kv):
    xs=[p.GetPosition().x for p in kv[1]]; ys=[p.GetPosition().y for p in kv[1]]
    return (max(xs)-min(xs)) + (max(ys)-min(ys))

KEYS = {
 'fat':   lambda kv: -len(kv[1]),          # most pads first
 'thin':  lambda kv:  len(kv[1]),
 'short': lambda kv:  span(kv),            # tightest nets first
 'long':  lambda kv: -span(kv),
 'name':  lambda kv:  kv[0],
}

done=fail=links=0
for netname, plist in sorted(pads.items(), key=KEYS[ORDER]):
    if netname in SKIP or len(plist) < 2: continue
    net = plist[0].GetNet(); nc = net.GetNetCode()
    connected = set(cells_of(plist[0]))
    ok=True
    for pad in plist[1:]:
        tgt = cells_of(pad)
        if connected & set(tgt): connected |= set(tgt); continue
        p = route(connected, tgt, nc)
        if p is None:
            # one unreachable pad must not abandon the other eleven
            ok=False; continue
        commit(p, net)
        links += 1
        connected |= set(p) | set(tgt)
    done += ok; fail += (not ok)

pcbnew.ZONE_FILLER(board).Fill(board.Zones())
board.Save(BRD)
print(f"order={ORDER} nets fully routed {done}, partly {fail}, connections made {links}")
print(f"SCORE {links}")
