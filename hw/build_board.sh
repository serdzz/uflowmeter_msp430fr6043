#!/bin/sh
# Everything, from the netlist source to the files a fab house reads.
set -e
cd "$(dirname "$0")"
K=/Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3
B=uflowmeter.kicad_pcb

python3 mknet.py uflowmeter.net
python3 mksch.py uflowmeter.net uflowmeter.kicad_sch

$K mkboard.py     uflowmeter.net "$B"
# The six the router cannot reach go down first, on an empty board, so they get the corridors --
# and before the supply stitching, so that avoids them rather than the other way round.
$K route_by_hand.py "$B"
$K stitch_vcc.py  "$B" VCC
$K route_maze.py  "$B" fat
for i in 1 2 3 4; do $K route_cleanup.py "$B" || continue; break; done
$K route_escape.py  "$B"
for i in 1 2 3 4; do $K route_cleanup.py "$B" || continue; break; done
$K mitre_corners.py "$B"
for i in 1 2 3 4; do $K route_cleanup.py "$B" || continue; break; done
$K finish_board.py  "$B"
./make_fab.sh
kicad-cli pcb drc --output /tmp/build_drc.json --format json --severity-error "$B" \
  | grep -E "violations|unconnected"
