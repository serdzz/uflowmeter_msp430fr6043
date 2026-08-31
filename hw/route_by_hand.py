"""Lay named connections along explicit waypoints.

For the nets a greedy router cannot reach, where only rip-up would free the corridors. A person
draws these; this writes down what they drew, so the board stays rebuildable from source instead
of carrying edits nobody can reproduce.

Manhattan discipline: every vertical run on the top layer, every horizontal on the bottom, a via
at each corner, and a lane of its own for each net. Two verticals never meet because their x
differs, two horizontals never meet because their y differs, and a vertical and a horizontal are
on different layers. Crossings then cannot happen -- drawing these by eye produced seven of them
before the rule was applied.

The crystal pair is the exception: LFXIN and LFXOUT are short, both ends are surface mount, and
they stay on the top layer throughout, which is why SPI_SCK leaves that area alone.

Each entry is (net, [(x, y, layer), ...]). A change of layer between consecutive points puts a
via there. Coordinates are millimetres in board space.
"""
import sys, pcbnew
from pcbnew import FromMM as MM, VECTOR2I

F, B = 'F', 'B'
LAYER = {F: pcbnew.F_Cu, B: pcbnew.B_Cu}
W, VIA_D, DRILL = MM(0.15), MM(0.6), MM(0.3)

ROUTES = [
 ('SPI_SCK',  [(33.56,23.25,F),(32.60,23.25,F),(32.60,23.25,B),
               (14.50,23.25,B),(14.50,23.25,F),(14.50,45.00,F),(14.50,45.00,B),
               (16.16,45.00,B),(16.16,45.00,F),(16.16,48.00,F)]),
 ('SPI_MOSI', [(35.25,30.44,F),(35.25,44.50,F),(35.25,44.50,B),
               (18.70,44.50,B),(18.70,44.50,F),(18.70,48.00,F)]),
 ('RADIO_CS', [(40.25,21.56,F),(40.25,17.00,F),(40.25,17.00,B),
               (12.00,17.00,B),(12.00,17.00,F),(12.00,46.00,F),(12.00,46.00,B),
               (13.62,46.00,B),(13.62,46.00,F),(13.62,48.00,F)]),
 ('UART_TX',  [(41.25,30.44,F),(41.25,53.00,F),(41.25,53.00,B),
               (60.00,53.00,B),(60.00,53.00,F),(60.00,47.54,F),(60.00,47.54,B),
               (62.00,47.54,B)]),
 ('CH0',      [(36.25,21.56,F),(36.25,2.00,F),(36.25,2.00,B),
               (20.00,2.00,B),(20.00,2.00,F),(20.00,5.00,F)]),
 ('CH0',      [(36.25,21.56,F),(36.75,21.56,F)]),
 ('I2C_SCL',  [(37.75,30.44,F),(37.75,32.50,F),(37.75,32.50,B),
               (22.50,32.50,B),(22.50,32.50,F),(22.50,3.00,F),(22.50,3.00,B),
               (42.92,3.00,B),(42.92,3.00,F),(42.92,6.00,F)]),
 ('I2C_SCL',  [(37.75,32.50,B),(48.00,32.50,B),(48.00,32.50,F),(48.00,29.00,F),
               (48.00,29.00,B),(59.51,29.00,B),(59.51,29.00,F),(59.51,35.00,F)]),
 ('TEST',     [(33.56,27.25,F),(31.50,27.25,F),(31.50,27.25,B),
               (9.81,27.25,B),(9.81,27.25,F),(9.81,26.00,F)]),
 ('LFXIN',    [(33.56,24.75,F),(30.00,24.75,F),(30.00,22.75,F),(25.00,22.75,F)]),
 ('LFXIN',    [(28.02,22.75,F),(28.02,21.00,F)]),
 ('LFXOUT',   [(33.56,25.25,F),(31.00,25.25,F),(31.00,26.30,F),(25.00,26.30,F),(25.00,25.25,F)]),
 ('LFXOUT',   [(28.02,26.30,F),(28.02,27.00,F)]),
 # DISP_GATE and RADIO_CS leave adjacent pins, so their lanes are half a millimetre apart and a
 # via on either sits too close to the other's track. Fanning one out sideways is what gives both
 # of them room -- it is the standard escape for a fine-pitch package and it is why they jog.
 ('DISP_GATE',[(40.75,21.56,F),(40.75,18.50,F),(41.90,17.70,F),(41.90,14.00,F),
               (41.90,14.00,B),(50.49,14.00,B),(50.49,14.00,F),(50.49,33.00,F)]),
 ('DISP_GATE',[(50.49,33.00,F),(50.49,35.50,F),(50.49,35.50,B),(54.06,35.50,B),
               (54.06,35.50,F),(54.06,33.95,F)]),
 # BUTTON and I2C_SCL likewise: adjacent pins, so BUTTON fans out before it turns.
 ('BUTTON',   [(38.25,30.44,F),(38.25,31.60,F),(39.50,32.40,F),(39.50,34.50,F),
               (39.50,34.50,B),(43.52,34.50,B),(43.52,34.50,F),(43.52,36.00,F)]),
 ('BUTTON',   [(43.52,34.50,B),(48.90,34.50,B),(48.90,34.50,F),(48.90,39.00,F)]),
 # UART_RX leaves the pin next to UART_TX, so it fans out too, and comes at J5 from the left --
 # a run straight down x=62 would cross the pin above it.
 ('UART_RX',  [(41.75,30.44,F),(41.75,31.60,F),(42.50,32.40,F),(42.50,46.50,F),
               (42.50,46.50,B),(60.80,46.50,B),(60.80,46.50,F),(60.80,50.08,F),
               (60.80,50.08,B),(62.00,50.08,B)]),
 # RST is the pin next to TEST; same fan, then west along the bottom to the pull-up and its cap.
 ('RST',      [(33.56,27.75,F),(32.20,27.75,F),(32.20,28.40,F),(32.20,28.40,B),
               (16.50,28.40,B),(16.50,28.40,F),(16.50,29.00,F)]),
 ('RST',      [(16.50,28.40,F),(16.50,26.00,F)]),
 # The ultrasonic crystal pair, again adjacent pins. USSXTOUT fans west; USSXTIN carries on down
 # past it before turning, which keeps their vias a millimetre and a half apart.
 ('USSXTOUT', [(34.75,21.56,F),(34.75,20.90,F),(33.20,20.60,F),(33.20,20.60,B),
               (32.02,20.60,B),(32.02,20.60,F),(32.02,19.50,F)]),
 ('USSXTOUT', [(32.02,20.60,B),(30.85,20.60,B),(30.85,20.60,F),(30.85,16.00,F)]),
 ('USSXTIN',  [(35.25,21.56,F),(35.25,19.00,F),(35.25,19.00,B),
               (25.02,19.00,B),(25.02,19.00,F),(25.02,19.50,F)]),
 ('USSXTIN',  [(27.15,19.00,B),(27.15,19.00,F),(27.15,16.00,F)]),
 # Routing boxes a 12 mm2 patch of ground in above U1, between DISP_GATE, RADIO_CS and the PVCC
 # decoupling. It holds a ground pad, so island removal keeps it, and on the top layer alone it
 # touches nothing else -- one via ties it to the plane and the board is whole.
 ('GND',      [(43.00,20.80,F),(43.00,20.80,B)]),
 # DVCC3 sits between DISP_GATE's fan and the PVCC decoupling, and the stitcher's stub kept being
 # cleaned away. Its own escape and via, placed where nothing else wants to be.
 ('VCC',      [(41.75,21.56,F),(41.75,19.50,F),(41.75,19.50,B)]),
]

def strip(board, names):
    """clear whatever these nets already have, so a hand route is the only copper on them"""
    gone = 0
    for t in list(board.Tracks()):
        if t.GetNetname() in names:
            board.Remove(t); gone += 1
    return gone

def run(path, routes):
    board = pcbnew.LoadBoard(path)
    n = strip(board, {name for name, _ in routes})
    if n: print(f"  cleared {n} existing items on those nets")
    laid = 0
    for name, pts in routes:
        net = board.FindNet(name)
        if net is None:
            print(f"  no such net: {name}"); continue
        for i in range(len(pts) - 1):
            (x1, y1, l1), (x2, y2, l2) = pts[i], pts[i+1]
            if l1 != l2:
                v = pcbnew.PCB_VIA(board)
                v.SetPosition(VECTOR2I(MM(x1), MM(y1)))
                v.SetWidth(VIA_D); v.SetDrill(DRILL); v.SetNet(net)
                v.SetViaType(pcbnew.VIATYPE_THROUGH)
                v.SetLayerPair(pcbnew.F_Cu, pcbnew.B_Cu)
                board.Add(v)
                continue
            t = pcbnew.PCB_TRACK(board)
            t.SetStart(VECTOR2I(MM(x1), MM(y1))); t.SetEnd(VECTOR2I(MM(x2), MM(y2)))
            t.SetWidth(W); t.SetLayer(LAYER[l1]); t.SetNet(net)
            board.Add(t); laid += 1
    pcbnew.ZONE_FILLER(board).Fill(board.Zones())
    board.Save(path)
    print(f"laid {laid} segments over {len(routes)} routes")

if __name__ == '__main__':
    run(sys.argv[1], ROUTES)
