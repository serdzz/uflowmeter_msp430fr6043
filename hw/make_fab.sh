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
    for r in rows:
        w.writerow([r['Ref'], r['PosX'], r['PosY'],
                    'top' if r['Side'] == 'top' else 'bottom', r['Rot']])
print(f"  placement: {len(rows)} parts")

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
PY

# KiCad names the Gerbers by layer -- .gtl .gbl .g1 .g2 .gts .gbs .gto .gbo .gm1 .gtp .gbp --
# not .gbr, so a wildcard on .gbr silently ships an archive with only the drill files in it.
( cd "$OUT" && zip -q uflowmeter-jlcpcb.zip \
    *.gtl *.gbl *.g1 *.g2 *.gts *.gbs *.gto *.gbo *.gm1 *.gtp *.gbp *.drl )
# The schematic as a PDF, so the folder is a complete handover and not just machine files.
kicad-cli sch export pdf --output "$OUT/uflowmeter-schematic.pdf" uflowmeter.kicad_sch >/dev/null
kicad-cli sch erc --output "$OUT/erc.json" --format json --severity-error uflowmeter.kicad_sch \
  | grep -E "violations" | sed 's/^/  ERC: /'

echo "wrote $OUT/uflowmeter-jlcpcb.zip plus the BOM, placement and schematic"
