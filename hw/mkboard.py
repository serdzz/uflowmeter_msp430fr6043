import re, sys, os, pcbnew
from pcbnew import FromMM as MM, VECTOR2I

NET  = sys.argv[1]; OUT = sys.argv[2]
LIB  = "/Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints"
W, H = 70.0, 55.0                     # board, mm

# placement: ref -> (x, y, rotation degrees)
PLACE = {
 'U1':(38,26,0),
 # ultrasonic corner -- U1's top edge carries pins 53-63
 'J4':(20,5,0),'Y2':(29,16,0),'C8':(25.5,19.5,0),'C9':(32.5,19.5,0),
 'C20':(43,19,0),'C21':(46,19,0),
 # timekeeping and debug -- U1's left edge, pins 6-12
 'Y1':(25,24,90),'C6':(28.5,21,0),'C7':(28.5,27,0),
 'J1':(6,26,90),'R1':(17,26,0),'C10':(17,29,0),
 # the radio is a module on a header now; the balun, its crystal and the u.FL are gone
 'J6':(6,48,90),'C2':(6,41,0),
 # front panel: the display module hangs from J3, the button sits directly below it
 'J3':(48,6,270),'SW1':(51.8,41,0),'C19':(44,36,0),
 'Q1':(55,33,0),'R2':(51,33,0),'R3':(59,35,0),'R4':(59,38,0),'C18':(58,48,0),
 # calibration and power
 'J5':(62,45,0),'BT1':(62,38,0),'D1':(44,50,0),'C1':(50,50,0),
 'C3':(46,22,0),'C4':(46,25,0),
}

BACK = set()    # the display sits on the front, above the button

comps = re.findall(r'\(comp \(ref "([^"]+)"\) \(value "([^"]+)"\) \(footprint "([^"]+)"\)\)', open(NET).read())
nets  = re.findall(r'\(net \(code \d+\) \(name "([^"]+)"\)\n((?:   \(node[^\n]*\n)*)', open(NET).read())

board = pcbnew.BOARD()

# ---- design rules -----------------------------------------------------------
ds = board.GetDesignSettings()
ds.SetCopperLayerCount(4)
try:
    ds.m_TrackMinWidth = MM(0.127); ds.m_MinClearance = MM(0.127)
    ds.m_ViasMinSize = MM(0.6); ds.m_MinThroughDrill = MM(0.2)
except Exception as e: print("rules:", e)

# ---- nets -------------------------------------------------------------------
netmap = {}
for name, nodes in nets:
    ni = pcbnew.NETINFO_ITEM(board, name)
    board.Add(ni); netmap[name] = ni
pad_net = {}
for name, nodes in nets:
    for ref, pin in re.findall(r'\(node \(ref "([^"]+)"\) \(pin "([^"]+)"\)', nodes):
        pad_net[(ref, pin)] = name

# ---- footprints -------------------------------------------------------------
placed = missing = 0
for ref, val, fpid in comps:
    lib, name = fpid.split(':', 1)
    try:
        fp = pcbnew.FootprintLoad(os.path.join(LIB, lib + ".pretty"), name)
    except Exception:
        fp = None
    if fp is None:
        print("  could not load", fpid); missing += 1; continue
    fp.SetReference(ref); fp.SetValue(val)
    x, y, rot = PLACE.get(ref, (57, 48, 0))
    fp.SetPosition(VECTOR2I(MM(x), MM(y)))
    if rot: fp.SetOrientationDegrees(rot)
    for pad in fp.Pads():
        nm = pad_net.get((ref, pad.GetNumber()))
        if nm and nm in netmap:
            pad.SetNet(netmap[nm])
    board.Add(fp); placed += 1
    # Flip only once the footprint belongs to the board -- flipping a loose one segfaults.
    if ref in BACK:
        fp.Flip(fp.GetPosition(), False)

# ---- board outline ----------------------------------------------------------
for a, b_ in [((0,0),(W,0)), ((W,0),(W,H)), ((W,H),(0,H)), ((0,H),(0,0))]:
    seg = pcbnew.PCB_SHAPE(board)
    seg.SetShape(pcbnew.SHAPE_T_SEGMENT)
    seg.SetStart(VECTOR2I(MM(a[0]), MM(a[1])))
    seg.SetEnd(VECTOR2I(MM(b_[0]), MM(b_[1])))
    seg.SetLayer(pcbnew.Edge_Cuts); seg.SetWidth(MM(0.1))
    board.Add(seg)

# ---- copper pours -----------------------------------------------------------
def pour(layer, netname, inset=0.3):
    z = pcbnew.ZONE(board)
    z.SetLayer(layer)
    if netname in netmap: z.SetNet(netmap[netname])
    o = z.Outline()
    o.NewOutline()
    for px, py in [(inset,inset),(W-inset,inset),(W-inset,H-inset),(inset,H-inset)]:
        o.Append(MM(px), MM(py))
    # Solid, not thermal relief. A QFN's ground pads have no room for spokes -- DRC calls that a
    # starved thermal -- and thermal relief under an RF ground is bad practice anyway.
    z.SetPadConnection(pcbnew.ZONE_CONNECTION_FULL)
    z.SetLocalClearance(MM(0.2))
    z.SetMinThickness(MM(0.15))
    board.Add(z); return z

pour(pcbnew.In1_Cu, 'GND')     # layer 2: unbroken ground, per LAYOUT.md
pour(pcbnew.In2_Cu, 'VCC')     # layer 3: supply
pour(pcbnew.F_Cu,   'GND')
pour(pcbnew.B_Cu,   'GND')

pcbnew.ZONE_FILLER(board).Fill(board.Zones())
board.Save(OUT)
print(f"placed {placed} footprints ({missing} missing), {len(netmap)} nets, 4 zones -> {OUT}")
