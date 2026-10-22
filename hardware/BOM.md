# Bill of materials (decided 2026-10-22; quantities for one rig plus spares)

| # | Part | Spec | Qty | Why this one |
|---|---|---|---|---|
| 1 | Stepper + driver | 28BYJ-48 **5 V** + ULN2003 board (usually sold as a 5-pack) | 1 more needed (have 1) | Second axis for the camera arm; same driver code as the platter. Check the label says 5 V, 12 V variants exist and look identical |
| 2 | Power supply | Regulated **5 V, 3 A** DC adapter, 5.5 × 2.1 mm barrel plug, plus a barrel-to-screw-terminal adapter or panel socket | 1 | Two steppers (~0.25 A each), an optional servo (0.65 A peaks) and LEDs; 3 A leaves margin. A USB-A 5 V / 2.4 A charger with a USB breakout cable also works. The kit's breadboard module (AMS1117, ~0.7 A) does not |
| 3 | Bearings | **608ZZ** (8 × 22 × 7 mm, metal shields), 10-pack | 4 used | Two stacked on the platter hub, two in the arm pivot. ZZ has less drag than 2RS rubber seals; dust is not an issue indoors |
| 4 | Pivot bolt | **M8 × 100 mm, partially threaded (DIN 931), grade 8.8, zinc plated** | 2 | The bearings must ride on the smooth shank, not on thread; 12.9 adds nothing here and its black oxide rusts. 100 mm leaves room for two bearings, the hub, washers and the nut; CAD trims the spacers |
| 5 | Nuts | **M8 nyloc (DIN 985)** | 4 | Clamps the bearing inner races through printed spacers without creeping; serrated flange nuts bite into plastic and loosen as it relaxes |
| 6 | Washers | M8 fender washers (DIN 9021, 24 mm OD) | 6 | Spread load on printed faces |
| 7 | Inserts | **M3 heat-set brass inserts, 5.7 mm long, 4.6 mm OD** (hole 4.0 mm in the print), 50-pack | ~20 used | Phone clamp, gear covers, cable clips, electronics tray. Set with a soldering iron (an insert tip set is optional) |
| 8 | Screws | M3 × 8, × 12, × 16 socket head, zinc or stainless | ~40 | Everything that goes into an insert |
| 9 | Motor screws | **M4 × 8** button head + M4 nuts | 8 | 28BYJ-48 ears take M4 at 35 mm spacing; nuts sit in printed pockets, no inserts needed |
| 10 | Counterweight | ~300 g of M8 nuts or coins in a printed box, or a 300 g diving weight | 1 | Balances the arm (phone 165 to 250 g at 35 cm) |
| 11 | Wire | 22 AWG hook-up wire, Dupont jumpers (F-F and M-F) | assorted | ULN2003 inputs, 5 V distribution; motors come with their plugs |

Not needed: a second power supply for the Nano (USB from the Mac during development), threaded inserts larger
than M3 (M4 and M8 joints use nuts), 12.9 hardware, 2RS bearings.

Power rules (also in `docs/bring-up.md`): motors, drivers and servo from the 5 V adapter; the adapter's GND to a
Nano GND pin; never the Nano's `5V` pin. Do not connect the adapter's 5 V to `VIN` (wants 7-12 V) and do not feed
the `5V` pin while USB is connected.
