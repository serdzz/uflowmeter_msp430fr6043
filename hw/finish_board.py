"""The last things a board needs before anyone will build it.

Three fiducials, so the assembly machine can find the board: a QFN-64 on 0.5 mm pitch is placed
from the fiducials, not from the board outline, and without them the house adds its own or asks.
Asymmetric on purpose -- three in an L tell the machine which way round the board is.

And a legend, so that the bare board in your hand says what it is."""
import sys, os, pcbnew
from pcbnew import FromMM as MM, VECTOR2I

LIB = "/Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints"
NAME, REV = "uflowmeter", "A"

board = pcbnew.LoadBoard(sys.argv[1])
bb = board.GetBoardEdgesBoundingBox()
L, T = pcbnew.ToMM(bb.GetLeft()), pcbnew.ToMM(bb.GetTop())
R, B = pcbnew.ToMM(bb.GetRight()), pcbnew.ToMM(bb.GetBottom())

existing = {f.GetReference() for f in board.Footprints()}
spots = [(L+3.5, T+3.5), (R-3.5, T+3.5), (L+3.5, B-3.5)]
added = 0
for i, (x, y) in enumerate(spots, 1):
    ref = f"FID{i}"
    if ref in existing: continue
    fp = pcbnew.FootprintLoad(os.path.join(LIB, "Fiducial.pretty"), "Fiducial_1mm_Mask2mm")
    if fp is None: break
    fp.SetReference(ref); fp.SetValue("")
    board.Add(fp)
    fp.SetPosition(VECTOR2I(MM(x), MM(y)))
    fp.Reference().SetVisible(False)
    added += 1

def text(s, x, y, layer, size=1.0, mirror=False):
    t = pcbnew.PCB_TEXT(board)
    t.SetText(s); t.SetLayer(layer)
    t.SetTextSize(pcbnew.VECTOR2I(MM(size), MM(size)))
    t.SetTextThickness(MM(size/6))
    t.SetMirrored(mirror)
    board.Add(t)
    t.SetPosition(VECTOR2I(MM(x), MM(y)))

legend = f"{NAME} rev {REV}"
text(legend, (L+R)/2, B-2.0, pcbnew.F_SilkS)
text(legend, (L+R)/2, B-2.0, pcbnew.B_SilkS, mirror=True)

pcbnew.ZONE_FILLER(board).Fill(board.Zones())
board.Save(sys.argv[1])
print(f"added {added} fiducials and the legend '{legend}'")
