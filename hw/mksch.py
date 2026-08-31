"""Generate uflowmeter.kicad_sch from the netlist.

Every symbol is drawn here rather than pulled from a library, so the file is self-contained and
opens the same on any machine. They are plain rectangles with the pins down the sides -- this is a
record of the design, not a drawing anyone would ink by hand -- and connectivity is carried by a
global label on every pin rather than by wires between symbols. That is a normal way to draw a
dense schematic and it is the only way to generate one without solving a routing problem twice.
"""
import re, sys, uuid, math

NET = sys.argv[1] if len(sys.argv) > 1 else 'uflowmeter.net'
OUT = sys.argv[2] if len(sys.argv) > 2 else 'uflowmeter.kicad_sch'
t = open(NET).read()

comps = re.findall(r'\(comp \(ref "([^"]+)"\) \(value "([^"]+)"\) \(footprint "([^"]+)"\)\)', t)
nets = {}
for m in re.finditer(r'\(net \(code \d+\) \(name "([^"]+)"\)\n((?:   \(node[^\n]*\n)*)', t):
    for r, p in re.findall(r'\(node \(ref "([^"]+)"\) \(pin "([^"]+)"\)', m.group(2)):
        nets.setdefault(r, {})[p] = m.group(1)

# Pin names for the parts where a number alone says nothing.
PINNAMES = {
 'U1': {'1':'AVCC1','3':'P1.0','5':'AVSS2','6':'LFXIN','7':'LFXOUT','8':'AVSS3','11':'TEST',
        '12':'RST','17':'DVSS1','18':'DVCC1','19':'P1.2','20':'P1.3','23':'P1.6','24':'P1.7',
        '25':'P1.4','31':'P4.3','32':'P4.4','33':'DVSS2','48':'DVSS3','49':'DVCC3','51':'P3.4',
        '52':'P3.5','53':'CH1_IN','54':'CH1_OUT','55':'PVSS','56':'PVCC','57':'PVCC','58':'PVSS',
        '59':'CH0_OUT','60':'CH0_IN','61':'AVSS4','62':'USSXTIN','63':'USSXTOUT','64':'AVSS1',
        '65':'EP'},
 'J6': {'1':'GND','2':'VCC','3':'GDO0','4':'CSN','5':'SCK','6':'MOSI','7':'GDO2','8':'MISO'},
 'J3': {'1':'GND','2':'VCC','3':'SCL','4':'SDA'},
 'J1': {'1':'VCC','2':'GND','3':'SBWTDIO','4':'SBWTCK'},
 'J4': {'1':'CH0','2':'GND','3':'CH1','4':'GND'},
 'J5': {'1':'GND','2':'TX','3':'RX'},
 'Q1': {'1':'D','2':'G','3':'S'},
 'BT1':{'1':'+','2':'-'},
 'D1': {'1':'K','2':'A'},
}

def uid(): return str(uuid.uuid4())

SHEET = uid()

GRID, PITCH = 2.54, 2.54
sym_defs, insts, wires, labels = [], [], [], []

def make_symbol(ref, value, pins):
    """a rectangle with the pins split down the two sides"""
    n = len(pins)
    left = pins[:math.ceil(n/2)]
    right = pins[math.ceil(n/2):]
    rows = max(len(left), len(right), 2)
    h = (rows + 1) * PITCH
    w = 40.64 if n > 8 else 25.4
    body = [f'    (symbol "{ref}_1_1"',
            f'      (rectangle (start {-w/2:.2f} {h/2:.2f}) (end {w/2:.2f} {-h/2:.2f})',
            '        (stroke (width 0.254) (type default)) (fill (type background)))']
    for i, (num, name) in enumerate(left):
        y = h/2 - PITCH*(i+1)
        body.append(f'      (pin passive line (at {-w/2-PITCH:.2f} {y:.2f} 0) (length {PITCH})'
                    f' (name "{name}" (effects (font (size 1.27 1.27))))'
                    f' (number "{num}" (effects (font (size 1.0 1.0)))))')
    for i, (num, name) in enumerate(right):
        y = h/2 - PITCH*(i+1)
        body.append(f'      (pin passive line (at {w/2+PITCH:.2f} {y:.2f} 180) (length {PITCH})'
                    f' (name "{name}" (effects (font (size 1.27 1.27))))'
                    f' (number "{num}" (effects (font (size 1.0 1.0)))))')
    body.append('    )')
    head = [f'  (symbol "local:{ref}" (pin_names (offset 0.762)) (in_bom yes) (on_board yes)',
            f'    (property "Reference" "{ref}" (at 0 {h/2+2.54:.2f} 0)'
            '      (effects (font (size 1.27 1.27))))',
            f'    (property "Value" "{value}" (at 0 {-h/2-2.54:.2f} 0)'
            '      (effects (font (size 1.27 1.27))))']
    return "\n".join(head + body + ['  )']), w, h, left, right

# page layout: widest-first into columns
order = sorted(comps, key=lambda c: -len(nets.get(c[0], {})))
x, y, colw, PAGEH = 40.0, 30.0, 0.0, 380.0
for ref, value, fp in order:
    pins = sorted(nets.get(ref, {}).items(), key=lambda kv: int(kv[0]) if kv[0].isdigit() else 999)
    pins = [(num, PINNAMES.get(ref, {}).get(num, f'~{num}')) for num, _ in pins]
    if not pins: continue
    d, w, h, left, right = make_symbol(ref, value, pins)
    if y + h > PAGEH:
        x += colw + 60.0; y = 30.0; colw = 0.0
    sym_defs.append(d)
    colw = max(colw, w + 2*PITCH)
    insts.append(
        f'  (symbol (lib_id "local:{ref}") (at {x:.2f} {y+h/2:.2f} 0) (unit 1)\n'
        f'    (uuid {uid()})\n'
        f'    (property "Reference" "{ref}" (at {x:.2f} {y-1.0:.2f} 0)'
        '      (effects (font (size 1.27 1.27))))\n'
        f'    (property "Value" "{value}" (at {x:.2f} {y+h+2.0:.2f} 0)'
        '      (effects (font (size 1.27 1.27))))\n'
        f'    (property "Footprint" "{fp}" (at {x:.2f} {y:.2f} 0)'
        '      (effects (font (size 1.27 1.27)) hide))\n'
        + "".join(f'    (pin "{num}" (uuid {uid()}))\n' for num, _ in pins)
        + f'    (instances (project "uflowmeter" (path "/{SHEET}" (reference "{ref}") (unit 1))))\n'
          '  )')
    for side, plist in (('L', left), ('R', right)):
        for i, (num, _) in enumerate(plist):
            py = y + h/2 - (h/2 - PITCH*(i+1))
            # The pin's connection point, not its far end. In symbol space the pin sits at
            # -w/2-PITCH and is drawn inward; placed at (x, y+h/2) with Y flipped, that lands
            # here. Being one pitch out is why nothing connected the first time.
            px = x - w/2 - PITCH if side == 'L' else x + w/2 + PITCH
            ex = px - 2*PITCH if side == 'L' else px + 2*PITCH
            wires.append(f'  (wire (pts (xy {px:.2f} {py:.2f}) (xy {ex:.2f} {py:.2f}))\n'
                         f'    (stroke (width 0) (type default)) (uuid {uid()}))')
            rot = 180 if side == 'L' else 0
            labels.append(f'  (global_label "{nets[ref][num]}" (shape passive) (at {ex:.2f} {py:.2f} {rot})\n'
                          f'    (effects (font (size 1.27 1.27)) (justify {"right" if side=="L" else "left"}))'
                          f' (uuid {uid()}))')
    y += h + 12.0

doc = ['(kicad_sch (version 20231120) (generator "uflowmeter") (generator_version "8.0")',
       f'  (uuid {SHEET})', '  (paper "A1")', '  (lib_symbols']
doc += sym_defs
doc += ['  )'] + insts + wires + labels
doc += [f'  (sheet_instances (path "/" (page "1")))', ')']
open(OUT, 'w').write("\n".join(doc).replace('{SHEET}', SHEET) + "\n")
print(f"{len(insts)} symbols, {len(labels)} pins labelled -> {OUT}")
