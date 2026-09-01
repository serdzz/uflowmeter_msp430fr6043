#!/bin/sh
# Everything JLCPCB asks for, from the board file.
#
#   ./make_fab.sh            writes fab/ and fab/uflowmeter-jlcpcb.zip
#
# The zip holds the Gerbers and the drill files. The BOM and the placement file sit beside it and
# are uploaded separately, on the assembly step.
set -e
cd "$(dirname "$0")"
BRD=uflowmeter.kicad_pcb
OUT=fab
rm -rf "$OUT"; mkdir -p "$OUT"

# JLCPCB reads plain RS-274X with a separate Excellon drill; no X2 attributes, no job file.
kicad-cli pcb export gerbers \
  --output "$OUT" \
  --layers F.Cu,In1.Cu,In2.Cu,B.Cu,F.Paste,B.Paste,F.Silkscreen,B.Silkscreen,F.Mask,B.Mask,Edge.Cuts \
  --no-x2 --no-netlist --subtract-soldermask \
  "$BRD"

kicad-cli pcb export drill \
  --output "$OUT" \
  --format excellon --drill-origin absolute --excellon-units mm \
  --excellon-separate-th --generate-map --map-format gerberx2 \
  "$BRD"

# Placement for the assembly step. JLCPCB wants millimetres and the columns named this way.
kicad-cli pcb export pos \
  --output "$OUT/uflowmeter-cpl.csv" \
  --format csv --units mm --side both --bottom-negate-x \
  "$BRD"

python3 - <<'PY'
import csv, io, os
src = 'fab/uflowmeter-cpl.csv'
rows = list(csv.DictReader(open(src)))
with open(src, 'w', newline='') as f:
    w = csv.writer(f)
    w.writerow(['Designator', 'Mid X', 'Mid Y', 'Layer', 'Rotation'])
    placed = [r for r in rows if not r['Ref'].startswith('FID')]
    for r in placed:
        w.writerow([r['Ref'], r['PosX'], r['PosY'],
                    'top' if r['Side'] == 'top' else 'bottom', r['Rot']])
print(f"  placement: {len(placed)} parts ({len(rows) - len(placed)} fiducials left out)")

# The BOM JLCPCB reads: one line per value, designators joined, LCSC number in its own column.
import collections
bom = collections.OrderedDict()
for r in csv.DictReader(open('bom.csv')):
    lcsc = (r.get('LCSC') or '').strip()
    key = (r['Comment'], r['Footprint'], lcsc)
    bom.setdefault(key, []).extend(d.strip() for d in r['Designator'].split(','))
with open('fab/uflowmeter-bom.csv', 'w', newline='') as f:
    w = csv.writer(f)
    w.writerow(['Comment', 'Designator', 'Footprint', 'LCSC Part #'])
    for (c, fp, lcsc), des in bom.items():
        w.writerow([c, ','.join(des), fp, lcsc])
print(f"  bom: {len(bom)} lines")

# The two files have to name the same parts. A designator in the placement file with no BOM line
# is what makes an assembly house stop and write to you.
cpl_refs = {r['Ref'] for r in placed}
bom_refs = set()
for _, des in bom.items():
    bom_refs |= {d.strip() for d in des if d.strip()}
missing = sorted(cpl_refs - bom_refs)
extra   = sorted(bom_refs - cpl_refs)
if missing or extra:
    if missing: print(f"  ERROR: placed but not in the BOM: {missing}")
    if extra:   print(f"  ERROR: in the BOM but not placed: {extra}")
    raise SystemExit(1)
print("  placement and BOM name the same parts")
PY

# Drop any paste layer with nothing on it. Every part is on the top, so the bottom paste gerber
# comes out as a bare header -- and a fab house asked for a stencil of nothing quite reasonably
# asks what you meant.
for f in "$OUT"/*.gtp "$OUT"/*.gbp; do
  [ -f "$f" ] || continue
  if ! grep -qE 'D0[13]\*' "$f"; then
    echo "  no apertures on $(basename "$f") -- not shipping it"
    rm -f "$f"
  fi
done

# KiCad names the Gerbers by layer -- .gtl .gbl .g1 .g2 .gts .gbs .gto .gbo .gm1 .gtp .gbp --
# not .gbr, so a wildcard on .gbr silently ships an archive with only the drill files in it.
( cd "$OUT" && zip -q uflowmeter-jlcpcb.zip \
    *.gtl *.gbl *.g1 *.g2 *.gts *.gbs *.gto *.gbo *.gm1 *.gtp *.drl 2>/dev/null \
  || zip -q uflowmeter-jlcpcb.zip *.gtl *.gbl *.g1 *.g2 *.gts *.gbs *.gto *.gbo *.gm1 *.drl )
# The schematic as a PDF, so the folder is a complete handover and not just machine files.
kicad-cli sch export pdf --output "$OUT/uflowmeter-schematic.pdf" uflowmeter.kicad_sch >/dev/null
kicad-cli sch erc --output "$OUT/erc.json" --format json --severity-error uflowmeter.kicad_sch \
  | grep -E "violations" | sed 's/^/  ERC: /'

echo "wrote $OUT/uflowmeter-jlcpcb.zip plus the BOM, placement and schematic"
