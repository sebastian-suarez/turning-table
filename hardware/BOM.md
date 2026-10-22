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

## Amazon links (amazon.com, found 2026-10-22)

Listings found by web search; prices, stock and "ships to Colombia" eligibility were not verified, so check each
page (Amazon Global filter) before ordering. Hardware items (bolts, nuts, washers) come in bulk packs on Amazon; a
local ferretería is cheaper for two bolts. Mercado Libre carries the steppers and bearings locally.

| # | Item | Listing | Alternate | Search |
|---|---|---|---|---|
| 1 | 28BYJ-48 + ULN2003, 5 sets | [CenryKay 5 sets](https://www.amazon.com/dp/B086D5SXPV) | [Wrenmor 5-pack](https://www.amazon.com/dp/B0H7X5F9NJ) | [search](https://www.amazon.com/s?k=28BYJ-48+ULN2003+5+pack) |
| 2 | 5 V 3 A adapter, 5.5 × 2.1 mm | [Security-01 2-pack, UL listed](https://www.amazon.com/dp/B09JW38H3X) | [OMNIHIL 5 V 3 A](https://www.amazon.com/dp/B006QYSRDQ) | [search](https://www.amazon.com/s?k=5V+3A+power+supply+5.5x2.1mm) |
| 2b | Barrel jack to screw terminals | [Cctv4Less female jack adapters](https://www.amazon.com/dp/B077QD4G3Q) | [DC-022 panel-mount sockets](https://www.amazon.com/dp/B0BC7JXZG2) | [search](https://www.amazon.com/s?k=5.5x2.1mm+female+dc+jack+screw+terminal) |
| 3 | 608ZZ bearings, 10-pack | [608ZZ 10 pcs](https://www.amazon.com/dp/B07H83VV6B) | [608ZZ 10 pcs](https://www.amazon.com/dp/B07DZDLB3N) | [search](https://www.amazon.com/s?k=608ZZ+bearing+10+pack) |
| 4 | M8 × 100 DIN 931, 8.8, zinc | [Jet Fitting, partially threaded, 100-pack](https://www.amazon.com/dp/B01MTAMVS9) | [Newport Fasteners 200-pack](https://www.amazon.com/dp/B07CQC2D72) (bulk; buy locally if you can) | [search](https://www.amazon.com/s?k=M8-1.25+x+100mm+hex+bolt+partially+threaded+8.8) |
| 5 | M8 nyloc nuts, DIN 985 | [MonsterBolts 10-pack](https://www.amazon.com/dp/B07MWC4V1R) | [Prime-Line 25-pack](https://www.amazon.com/dp/B07DSBTD8M) | [search](https://www.amazon.com/s?k=M8-1.25+nylon+insert+lock+nut+DIN+985) |
| 6 | M8 fender washers, 24 mm OD | [ASMC DIN 9021B, 50-pack](https://www.amazon.com/dp/B07CW3DCPL) | [100-pack](https://www.amazon.com/dp/B01LYUP7SM) | [search](https://www.amazon.com/s?k=M8+fender+washer+24mm+DIN+9021) |
| 7 | M3 heat-set inserts, 5.7 × 4.6 mm | [cSeao 120 pcs](https://www.amazon.com/dp/B07XXR2TFJ) | [MroMax 140 pcs](https://www.amazon.com/dp/B0DDWS7BTS) | [search](https://www.amazon.com/s?k=M3+heat+set+inserts+5.7+4.6+brass) |
| 8 | M3 (and M4) socket screws with nuts | [Glarks 510-piece M3/M4/M5 kit](https://www.amazon.com/dp/B08HHZHXTP) covers items 8 and 9 | [HH 120-piece M3 set](https://www.amazon.com/dp/B01BWN0VES) | [search](https://www.amazon.com/s?k=M3+socket+head+cap+screw+assortment+stainless) |
| 9 | M4 × 8 button head + nuts | [M4 × 8 button head, 100 pcs](https://www.amazon.com/dp/B07QDWH5C2) (nuts from item 8) | [170-piece M4 kit with nuts](https://www.amazon.com/dp/B0DSFW8LLY) | [search](https://www.amazon.com/s?k=M4+x+8mm+button+head+screws+nuts) |
| 11 | 22 AWG solid hook-up wire | [TUOFENG 6 colours, 30 ft each](https://www.amazon.com/dp/B07TX6BX47) | [TMH 6 colours, 25 ft each](https://www.amazon.com/dp/B0G43VFRM4) | [search](https://www.amazon.com/s?k=22+AWG+solid+core+hook+up+wire+kit) |
| 11b | Dupont jumper wires | [REXQualis 240 pcs, M-F/M-M/F-F](https://www.amazon.com/dp/B0F8VDMHRT) | [ELEGOO 40 × M-F 20 cm](https://www.amazon.com/dp/B0H6F7ZZFX) | [search](https://www.amazon.com/s?k=dupont+jumper+wires+kit+20cm) |

Item 10 (counterweight) is M8 nuts or coins; no purchase needed.
