#!/bin/sh
# Everything, from the netlist source to the files a fab house reads.
set -e
cd "$(dirname "$0")"
K=/Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3
B=uflowmeter.kicad_pcb

python3 mknet.py uflowmeter.net
python3 mksch.py uflowmeter.net uflowmeter.kicad_sch

$K mkboard.py     uflowmeter.net "$B"
$K stitch_vcc.py  "$B" VCC
$K route_maze.py  "$B" fat
$K route_cleanup.py "$B"
$K route_escape.py  "$B"
$K route_cleanup.py "$B"
$K finish_board.py  "$B"
./make_fab.sh
kicad-cli pcb drc --output /tmp/build_drc.json --format json --severity-error "$B" \
  | grep -E "violations|unconnected"
