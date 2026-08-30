"""Remove whatever DRC objects to.

The routers model clearance as inflated rectangles on a grid, which is close to KiCad's own
computation but not identical to it. Where the two disagree, KiCad is right: this drops the
offending tracks and leaves those connections for a person, rather than shipping a board that
fails its own rule check."""
import sys, json, subprocess, re, pcbnew

BRD = sys.argv[1]
for attempt in range(6):
    subprocess.run(['kicad-cli','pcb','drc','--output','/tmp/_cl.json','--format','json',
                    '--severity-error', BRD], capture_output=True)
    v = json.load(open('/tmp/_cl.json')).get('violations', [])
    bad = [x for x in v if x['type'] in ('clearance','shorting_items','track_width','hole_clearance')]
    if not bad:
        print(f"clean after {attempt} pass(es)"); break
    kill = set()
    for x in bad:
        for i in x.get('items', []):
            d = i.get('description','')
            m = re.match(r'Track \[([^\]]+)\] on (\S+), length ([\d.]+) mm', d)
            if m:
                # position too, or every track of the same net and length goes with it
                pos = i.get('pos') or {}
                kill.add((m.group(1), m.group(2), round(float(m.group(3)), 4),
                          round(pos.get('x', -1e9), 3), round(pos.get('y', -1e9), 3)))
    if not kill:
        print(f"{len(bad)} violations left that are not tracks"); break
    b = pcbnew.LoadBoard(BRD)
    removed = 0
    for t in list(b.Tracks()):
        if isinstance(t, pcbnew.PCB_VIA): continue
        L = round(pcbnew.ToMM(t.GetLength()), 4)
        mx = round((pcbnew.ToMM(t.GetStart().x)+pcbnew.ToMM(t.GetEnd().x))/2, 3)
        my = round((pcbnew.ToMM(t.GetStart().y)+pcbnew.ToMM(t.GetEnd().y))/2, 3)
        net, lay = t.GetNetname(), b.GetLayerName(t.GetLayer())
        if any(k[0]==net and k[1]==lay and abs(k[2]-L)<1e-3
               and (k[3] < -1e8 or (abs(k[3]-mx)<0.6 and abs(k[4]-my)<0.6)) for k in kill):
            b.Remove(t); removed += 1
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    b.Save(BRD)
    print(f"  pass {attempt+1}: removed {removed} tracks for {len(bad)} violations")
