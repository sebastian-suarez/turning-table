# Measurement aids for the Phase 2 hardware acceptance

## 3D-printed disc, pointer and fit coupon

Print **`fit-coupon.stl` first**, before the large disc. The source for all three parts is
[`../cad/protractor.scad`](../cad/protractor.scad); all dimensions and fits are named parameters at its top.
Use the Homebrew `openscad@snapshot` cask (2026.10.11), plus Make and Python 3. From the repository root:

```sh
cd hardware/cad
make                 # Binary STLs, Manifold backend, exported into ../measure/.
make check           # Bounds, triangle counts, closed topology, size under 20 MB, screw fit.
make preview         # ../measure/protractor-disc.png, pointer.png and pointer-tip.png.
```

The exports are `protractor-disc.stl`, `pointer.stl`, `fit-coupon.stl` and `screws.stl` (printed substitutes
for the two M4 screws, see below). The disc PNG is a top view; the pointer
PNG shows the **assembled** side view at a fixed scale, with the arm above the hub; `pointer-tip.png` closes in
on the last 6 mm so the slope and end face are legible. The pointer STL is already upside down for printing.
In the GUI the source shows all three parts side by side, each in print orientation. Select one part with
`part = "disc"`, `"pointer"` or `"coupon"` when exporting individually.

### Print settings and orientation

PLA, 0.2 mm layers, 0.4 mm nozzle, 3 walls, 4 top and bottom layers, 20% infill. Use light filament so the raised
ticks and numerals can be darkened with a marker or pencil. Slice at 100%, with no scaling.
The slicer must print thin walls (Arachne in PrusaSlicer or OrcaSlicer, "print thin walls" in Cura), or the
0.5 mm ticks and the 0.8 mm neck and tip vanish.

- **Coupon:** plate flat on the bed, five collars pointing up. No supports. Approximately 20 to 30 minutes; use it to
  validate this motor and filament before spending material on the disc.
- **Disc:** flat underside on the bed, ticks and numerals up; add a 5 mm brim against warping. Nominal Ø190 ×
  2.8 mm including the raised marks, approximately 60 g at 20% infill and 3 hours. The base alone is twelve layers.
- **Pointer:** its arm's broad **top** face and the hub's top face on the bed. The hub and tip wedge grow upward
  from that face; the tip is a 45° overhang with no cantilever and needs no supports. Turn it over after printing.
  It weighs a few grams.

### Coupon: locate the motor and choose a fit

Verified on the project's motor on 2026-10-26: the disc and coupon holes locate on the boss and line up with
the ears, and the pointer printed at the default `flat_fit = 3.0` and `round_bore_d = 5.3` passes the twist and
rock checks, so no parameter change or reprint was needed. Repeat the steps below for a different motor or
filament.

1. With both supplies disconnected, put the coupon **plate down** over the motor's shaft and boss. Its Ø9.4
   centre hole locates on the Ø9 boss with a designed 0.2 mm radial clearance; the plate must sit flat on the
   mounting plate and ears. Up to about 0.3 mm of slide is within the 0.2° budget, but there must be no visible
   wobble once the screws are tight. Both Ø4.6 screw holes must align with the ear holes at (±17.5, −8) relative
   to the shaft. If the coupon binds on the boss or the ears do not align, check the named locating parameters
   and print the coupon again. The screws only clamp: **do not enlarge the centre hole**. A 0.2 mm eccentricity
   costs about 0.12° at the tick ring; 0.75 mm costs about 0.46°.
2. Invert the coupon to try each **collar end first** on the shaft. The bores go through the plate, so the shaft
   cannot bottom in a blind hole. Each collar's free end (top as printed) has the same Ø5.3 round relief as the
   pointer, 1.8 mm deep by default. Because the double-D bore continues through the plate, the coupon engages
   the full **6 mm of shaft flats**, so it reads slightly tighter than the pointer hub's 4.1 mm engagement.
   Count the raised dots in each collar's shallow label pad:

   | Dots | Across flats, mm |
   |---|---|
   | 1 | 2.8 |
   | 2 | 2.9 |
   | 3 | 3.0 |
   | 4 | 3.1 |
   | 5 | 3.2 |

   All five Ø14 collars form one row on the +y side, clear of every locating hole by at least 2 mm; the holes
   never cut into their walls. The label pads keep all dots within the coupon's 70 × 34 × 8.3 mm envelope.
   All round bore portions are Ø5.3; the flats, rather than the arcs, control play.
3. Choose the **tightest** candidate that goes on with a firm thumb push. Hold the shaft and twist the coupon
   with light finger pressure: there must be **no rotation between bore and shaft**. Gearbox backlash is a
   different effect, so do not mistake the whole shaft turning for a loose bore. Set `flat_fit` to the chosen
   value. Also lever the ring up and down along the flats: it must **not rock**. If it does, adjust
   `round_bore_d` (default 5.3 mm; try 5.2 mm on a reprinted coupon), then repeat both the twist and rock checks.
   Re-export the pointer with the chosen `flat_fit` and `round_bore_d`. For example, from `hardware/cad/`:

   ```sh
   make -B ../measure/pointer.stl SCAD_ARGS='-D flat_fit=2.9'
   ```

4. If all five are loose, shift `flat_series` down by 0.2 mm; if all are too tight, shift it up by 0.2 mm. Print
   the coupon again and repeat the no-play test. For example, a tighter series is:

   ```sh
   make -B ../measure/fit-coupon.stl SCAD_ARGS='-D flat_series=[2.6,2.7,2.8,2.9,3.0]'
   ```

   If none holds, use `hub_clamp = true` and re-export the pointer; its hub grows to **Ø18** to accommodate the
   nut and jaw walls. An M3 nut sits in the rear hex pocket and an M3 screw crosses the radial split tangentially;
   tighten gently until the hub cannot rock. Recheck the fit and the full sweep. Do not force a tight test ring
   down the shaft.

`-B` forces regeneration after a command-line parameter change; Make otherwise tracks source timestamps. Edit
the defaults in the source to preserve a calibration for later builds. `make check` uses the default v2.3 raised
bounds; for another `disc_d` or the Ø18 clamp hub, use `stl_bounds.py` directly with the intended `--expect-size`
and `--expect-min`.

### Mount, set the tip height and read

Keep power disconnected while fitting the printed parts. Hold the motor down with tape or a clamp so it cannot
creep. Put the disc flat on the motor's plate and ears, centred on the boss. Clamp it with **two M4 button-head
screws from above and nuts below the ears**, or with the printed substitutes below. Keep every head or washer
stack above the disc at **3 mm or less**; ordinary 2.2 mm button heads fit. Do not use 4 mm tall socket-cap
heads. Avoid distorting the flat disc by overtightening the nuts.

#### Printed substitute screws (`screws.stl`)

Two bolts and four nuts, printed with the same thread so they only fit each other; they are not M4. The bolt is
Ø3.9 so it passes the motor's Ø4.2 steel ear hole, its knurled head is 2.4 mm tall, inside the 3 mm keep-out under
the pointer arm, and it is turned by hand. Each nut has one flat side: the ear hole is only 3.5 mm from the motor
can, so a round nut would hit the can. Turn the flat toward the can; it then also stops the nut from spinning, so
you hold nothing and turn only the head, clockwise to tighten.

- **Print:** bolts standing on their heads (as exported), nuts flat. No supports. 100% infill for this plate, and
  0.12 or 0.16 mm layers if your profile offers them (0.2 mm works too: the coarse 1.5 mm pitch is chosen for it).
  Print slowly with full cooling; the bolts are thin.
- **Nuts:** two plain nuts are the snug fit; two spares with a small notch on the rim opposite the flat are looser.
  Try a plain nut on a bolt first. If it binds after a turn, use the notched ones. Never force a printed thread.
- **Fit:** bolt down through the disc and the ear, nut under the ear with its flat against the can, then turn the
  head until the disc is held. Finger-tight is enough: the screws only stop the disc from turning; the boss centres
  it. They are PLA, so replace them with metal M4 button-head screws when you have some.
- `make check` verifies the thread in the model: a plain nut screwed on at a whole number of turns must not touch
  the bolt, and the same nut pushed a quarter turn without turning must (that is what makes it hold).

Turn the printed pointer right way up, with its arm on top, and align both bore flats with the shaft. Press on
the hub; support the hub rather than pushing on the long arm. The hub's plate-facing end (top as printed) has a
Ø5.3 round relief, 1.8 mm deep by default, to pass the shaft's round base and reach the paper gap while leaving
4.1 mm of double-D engagement. Turn the pointer a full revolution by hand to find the **highest point of the
sweep**, where the tip comes closest to the ticks; shaft tilt or disc warp can change the gap around the turn.
Set the hub height there with a sheet of ordinary paper (about 0.1 mm) just sliding between a tick and the tip's
**flat lower face**. Leave about 0.1–0.3 mm there and never let the tip drag anywhere in the sweep. If the hub
stops on its relief shoulder before reaching that paper gap, the motor's flats start higher than nominal:
lower `motor_flat_length` in the source and re-export the pointer rather than forcing the hub down.

At the model's tightest 0.1 mm gap, the raised-tick hub bottom and tip's lower face are at z = 2.9 mm; the arm
underside is at 6.4 mm and its top at 8.8 mm. The `echo()` values report 1 mm above a
3 mm head, 0.1 mm above the tick surface, and the relief and engagement lengths. A larger paper gap raises the
whole pointer and increases head clearance.

**Before motor power, turn the pointer through a full 360° by hand.** It must clear every screw head and nut by
at least 1 mm, clear the disc throughout, and stay tight on the shaft. Adjust anything that rubs or rocks before
continuing with the acceptance procedure.

The reading datum is the **0.8 × 0.8 mm end face at r = 93 mm**, read against the **outer ends of the ticks
from above**. Nothing of the pointer sits above the end face to hide it; the arm is never the datum. The end
face's top is at most 1.1 mm above the ticks at the specified gap, limiting a 15° viewing error to
about 0.3 mm (0.18°). Zero is at the top and numbers increase clockwise from the shaft side. Read after `DONE`
(coils released), approaching each target from the same direction.

## Paper alternative

The original SVGs are kept for use with a paper printer. Print both at 100% (no "fit to page"), then measure the
100 mm check line on the protractor with a ruler.

- `protractor-190mm-a4.svg`: 190 mm disc, 1° ticks, labels every 10°, 0° at the top, clockwise. 0.5° is 0.83 mm
  at the rim, readable by eye. The centre hole clears the 28BYJ-48 shaft; the dotted circles mark its two M4
  mounting ears. Locate the sheet on the shaft and tape it flat over the motor lying face-up on the desk; the
  printed disc's locating boss and offset ear holes are validated by the coupon instead of those dotted guides.
- `pointer-95mm.svg`: pointer template for card stock with a D-shaped hole for the 5 mm shaft. Press it onto the
  shaft and add a drop of glue; the tip then sweeps the tick ring.

Procedure: [`../../docs/phase-2-acceptance.md`](../../docs/phase-2-acceptance.md); the acceptance requirements are
in `docs/phase-2-spec.md`, section "Hardware acceptance".

## Phase 3 test rig (W9/W10)

A stand that holds the **platter** motor shaft-up and a Ø150 mm test platter pressed onto its shaft, so the scan
criteria W9 and W10 can run before the Phase 4 rig exists. The phone stays on its tripod. Source:
[`../cad/test-rig.scad`](../cad/test-rig.scad); `make` in `hardware/cad` exports `rig-base.stl` and
`rig-platter.stl` here, `make check` checks their bounds and topology, and `make preview` draws
`rig-assembly.png`.

**Not the Phase 4 platter.** There the platter rides on two 608ZZ bearings; here the motor shaft carries
everything. Keep the object **light (under about 200 g) and centred** (use the rings), and expect a little wobble
at the rim.

### Print

PLA, 0.2 mm layers, 0.4 mm nozzle, 3 walls, 4 top and bottom layers, 15-20% infill, no supports. Slice at 100%.

- **Base** (`rig-base.stl`, Ø100 × 24 mm): flat bottom on the bed, as exported.
- **Platter** (`rig-platter.stl`, Ø150 × 10 mm): its **top face on the bed** and the hub pointing up, as exported:
  the same orientation as the pointer, whose shaft fit passed. Add a 5 mm brim against warping. The centring
  rings and cross are cut 0.6 mm into the bed-side face; the notch in the rim marks whole revolutions.

### Assemble

1. With the motor unplugged, drop the platter motor into the base: the can goes through the round pocket, the two
   ears settle into the shaped recess on top (that stops the motor turning), and the blue wire cover and the cable
   go out through the slot.
2. Hold the base and press the platter's hub straight down onto the shaft, flats lined up, until it stops: the
   shaft tip bottoms in the blind bore, which sets the height and leaves 1.5 mm between the hub and the motor's
   fixed boss. The same 3.0 mm flat fit and Ø5.3 bore as the pointer; it must not twist on the shaft.
3. Turn the platter once by hand: it must not touch the base. Plug the motor back into the **platter** driver
   (D2-D5).

To take it apart, hold the base and pull the platter straight up; push the motor out from underneath through the
open pocket.
