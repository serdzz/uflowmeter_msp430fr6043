"""The netlist, as source. Run it to regenerate uflowmeter.net.

The radio is a module on a header, not a chip on this board. That decision removes the CC1101
itself, its 26 MHz crystal, its bias resistor, its supply bead and decoupling, the whole filter
balun and the antenna connector -- about twenty parts and, more to the point, the requirement to
transplant TI's reference layout and prove EN 300 220 by conducted measurement. What is left of
the radio here is four signals and a supply."""
import sys

FP = {
 'mcu':   'Package_DFN_QFN:QFN-64-1EP_9x9mm_P0.5mm_EP5.45x5.45mm',
 'sot23': 'Package_TO_SOT_SMD:SOT-23',
 'r':     'Resistor_SMD:R_0402_1005Metric',
 'c':     'Capacitor_SMD:C_0402_1005Metric',
 'c0603': 'Capacitor_SMD:C_0603_1608Metric',
 'c0805': 'Capacitor_SMD:C_0805_2012Metric',
 'y32k':  'Crystal:Crystal_SMD_3215-2Pin_3.2x1.5mm',
 'y8m':   'Crystal:Crystal_SMD_5032-2Pin_5.0x3.2mm',
 'h8':    'Connector_PinHeader_2.54mm:PinHeader_1x08_P2.54mm_Vertical',
 'h4':    'Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical',
 'h3':    'Connector_PinHeader_2.54mm:PinHeader_1x03_P2.54mm_Vertical',
 'h2':    'Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical',
 'h4s':   'Connector_PinHeader_1.27mm:PinHeader_1x04_P1.27mm_Vertical',
 'sw':    'Button_Switch_SMD:SW_Push_1P1T_NO_CK_KSC7xxJ',
 'd':     'Diode_SMD:D_SOD-123',
}

C = []
def comp(r, v, f): C.append((r, v, FP[f]))

comp('U1','MSP430FR5043IRGC','mcu'); comp('Q1','IRLML6401','sot23')
comp('D1','Schottky','d');  comp('SW1','Button','sw')
comp('Y1','32.768kHz','y32k'); comp('Y2','8MHz','y8m')
comp('J1','SBW','h4s'); comp('J3','Display','h4'); comp('J4','Transducers','h4')
comp('J5','Cal UART','h3'); comp('J6','Radio module','h8'); comp('BT1','Cell','h2')
comp('C1','100uF','c0805'); comp('C2','10uF','c0603'); comp('C18','1uF','c0603')
for r in ['C3','C4','C10','C19','C20','C21']: comp(r,'100nF','c')
for r,v in [('C6','18pF'),('C7','18pF'),('C8','18pF'),('C9','18pF')]: comp(r,v,'c')
for r,v in [('R1','47k'),('R2','1M'),('R3','4.7k'),('R4','4.7k')]: comp(r,v,'r')

N = {}
def net(name, *nodes): N.setdefault(name, []).extend(nodes)
def P(ref, *pins): return [(ref, str(p)) for p in pins]

net('GND', *P('U1',5,8,17,33,48,55,58,61,64,65), *P('BT1',2), *P('J1',2), *P('J3',1),
          *P('J5',1), *P('J6',1), *P('J4',2), *P('J4',4), *P('SW1',2),
          *P('C1',2),*P('C2',2),*P('C3',2),*P('C4',2),*P('C10',2),*P('C18',2),*P('C19',2),
          *P('C20',2),*P('C21',2),*P('C6',2),*P('C7',2),*P('C8',2),*P('C9',2))
net('BAT+', *P('BT1',1), *P('D1',2))
net('VCC',  *P('D1',1), *P('U1',1,18,49,56,57), *P('J1',1), *P('J6',2), *P('Q1',3),
            *P('R1',2), *P('R2',2),
            *P('C1',1),*P('C2',1),*P('C3',1),*P('C4',1),*P('C20',1),*P('C21',1))
net('VCC_DISP',  *P('Q1',1), *P('J3',2), *P('R3',1), *P('R4',1), *P('C18',1))
net('DISP_GATE', *P('U1',51), *P('Q1',2), *P('R2',1))
net('I2C_SDA',   *P('U1',23), *P('J3',4), *P('R4',2))
net('I2C_SCL',   *P('U1',24), *P('J3',3), *P('R3',2))
net('BUTTON',    *P('U1',25), *P('SW1',1), *P('C19',1))
# The module's header, in the order these boards are silkscreened:
#   1 GND  2 VCC  3 GDO0  4 CSN  5 SCK  6 MOSI  7 GDO2  8 MISO
# GDO0 and GDO2 are left unconnected: the firmware polls MARCSTATE over SPI rather than watching
# a pin, because a frame is four milliseconds and the CPU has nothing else to do in them.
net('RADIO_CS',  *P('U1',52), *P('J6',4))
net('SPI_SCK',   *P('U1',3),  *P('J6',5))
net('SPI_MOSI',  *P('U1',19), *P('J6',6))
net('SPI_MISO',  *P('U1',20), *P('J6',8))
net('UART_TX',   *P('U1',31), *P('J5',2))
net('UART_RX',   *P('U1',32), *P('J5',3))
net('RST',       *P('U1',12), *P('R1',1), *P('C10',1), *P('J1',3))
net('TEST',      *P('U1',11), *P('J1',4))
net('LFXIN',     *P('U1',6),  *P('Y1',1), *P('C6',1))
net('LFXOUT',    *P('U1',7),  *P('Y1',2), *P('C7',1))
net('USSXTIN',   *P('U1',62), *P('Y2',1), *P('C8',1))
net('USSXTOUT',  *P('U1',63), *P('Y2',2), *P('C9',1))
net('CH0',       *P('U1',59), *P('U1',60), *P('J4',1))
net('CH1',       *P('U1',54), *P('U1',53), *P('J4',3))

seen_pads = {}
for name, nodes in N.items():
    for node in nodes:
        if node in seen_pads and seen_pads[node] != name:
            raise SystemExit(f"pad {node[0]}.{node[1]} is on both "
                             f"{seen_pads[node]} and {name} -- a pad has one net")
        seen_pads[node] = name

out = ['(export (version "E")',
       ' (design (source "hw/mknet.py") (date "generated") (tool "uflowmeter"))',
       ' (components']
for r, v, f in C:
    out.append(f'  (comp (ref "{r}") (value "{v}") (footprint "{f}"))')
out += [' )', ' (nets']
for i, (name, nodes) in enumerate(sorted(N.items()), 1):
    seen = []
    for x in nodes:
        if x not in seen: seen.append(x)
    out.append(f'  (net (code {i}) (name "{name}")')
    out += [f'   (node (ref "{r}") (pin "{p}"))' for r, p in seen]
    out.append('  )')
out += [' )', ')']

path = sys.argv[1] if len(sys.argv) > 1 else 'uflowmeter.net'
open(path, 'w').write('\n'.join(out) + '\n')
print(f"{len(C)} components, {len(N)} nets, {sum(len(v) for v in N.values())} nodes -> {path}")
