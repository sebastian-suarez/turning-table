# Phase 2 protractor spec: 3D-printed disc and pointer (v2, 2026-10-23)

The user has a Creality Hi (260 × 260 × 300 mm) and no paper printer, so the paper protractor and card pointer in
`hardware/measure/` get printed counterparts. Same purpose: read the platter stepper's angle to ±0.5° during the
Phase 2 hardware acceptance (`docs/phase-2-acceptance.md`). This is also the first parametric OpenSCAD part of the
project, so it sets the conventions for Phase 4.

v1 → v2 (Astra attack, three findings, all accepted): the pointer arm now rides above the screw heads and carries a
blade that reaches down to the ticks (v1 swept a 1 mm-high arm through 2.2 mm-high button heads at r = 19.2 mm);
the hub fit is set by the across-flats dimension with an interference series on the coupon and a no-play check
(v1's uniform clearance left up to ±4° of rotational play on the double-D); the reading datum is the blade's lower
edge a few tenths of a millimetre above the ticks, read from above (v1's 2.4 mm-thick tip 1 mm above the ticks
gave 0.5° of parallax at 15° off-normal).

## Deliverables

- `hardware/cad/protractor.scad`: one file, all parts, parameters at the top with the defaults below, commented
  generously (why each dimension, which FDM constraint or finding it answers). `part = "disc" | "pointer" |
  "coupon"` selects what to render (`-D 'part="disc"'`); the default (no `-D`) shows all three side by side for
  the GUI, each already in its print orientation.
- `hardware/cad/Makefile`: `make` exports `hardware/measure/protractor-disc.stl`, `pointer.stl` and
  `fit-coupon.stl` (binary STL, manifold backend); `make preview` renders `hardware/measure/protractor-disc.png`
  (top view, for a visual check of ticks and numerals) and `pointer.png` (side view); `make check` prints each
  STL's bounding box and triangle count through `hardware/cad/stl_bounds.py` (binary STL parser, standard library
  only). `OPENSCAD ?= openscad`.
- `hardware/measure/README.md`: printed parts first (print settings and orientation, coupon first, how to mount,
  how to set the blade height, how to read), SVGs kept as the paper alternative. `docs/phase-2-acceptance.md`
  section 0: mounting the disc with two M4 button-head screws, pressing the pointer on, the 360° hand sweep, the
  reading rule. Nothing else in `docs/` changes.
- OpenSCAD is the Homebrew `openscad@snapshot` cask (2026.10.11; the `openscad` cask is disabled since
  2026-09-01). Use `--backend=manifold --export-format binstl`; the GUI preview must also work.

## Motor geometry (28BYJ-48, published nominal values; the coupon validates them before the big print)

Shaft axis at the origin, +z toward the shaft tip, 0° toward +y. Mounting plate (the face with the two ears) at
z = 0. Ear holes Ø4.2 at (±17.5, −8): the ear line is 8 mm from the shaft axis, on the cable side, so each screw
sits at r = 19.2 mm, inside the pointer's sweep. Shaft boss Ø9.0, 1.5 mm tall, coaxial with the shaft. Shaft Ø5.0,
10 mm above the plate, double-D 3.0 mm across the flats over its outer 6 mm (the flats are 4 mm wide chords).
Body Ø28 below the plate, so nothing else is in the way above z = 0.

## Stack-up (z from the mounting plate)

| Item | z | Why |
|---|---|---|
| Disc | 0 … 2.4 | 12 layers at 0.2 mm, sits on the plate and the ears |
| Ticks and numerals | 2.4 … 2.8 | raised 0.4 mm (two layers) |
| Screw heads on the disc | ≤ 2.4 + 3.0 = 5.4 | M4 button heads are 2.2 mm; any head up to 3 mm fits, socket caps (4 mm) do not |
| Pointer hub | 3.1 … 9.1 | 6 mm tall, bottom 0.3 mm above the ticks (set with a paper feeler, see below) |
| Pointer arm | 6.7 … 9.1 | 2.4 mm thick, flush with the hub's top; underside 1.3 mm above the tallest allowed head |
| Blade at the tip | 3.1 … 6.7 | hangs from the arm's end down to the plane of the hub's underside |
| Shaft tip | 10 | 0.9 mm above the hub; a 9 mm shaft would still engage 5.9 mm of hub |

## Disc

- Ø190 × 2.4 mm, ticks and numerals raised 0.4 mm on top, printed flat with the ticks up.
- Centred by the boss, not by the screws: centre hole Ø9.4 (boss 9.0 + 0.2 radial clearance; FDM holes print
  small, the coupon confirms the fit). Reason: eccentricity e between disc and shaft reads as an angle error of
  up to e / 93 mm, so 0.2 mm costs 0.12° and 0.75 mm would cost 0.46°, most of the ±0.5° budget. Two Ø4.6 holes at
  (±17.5, −8) for M4 button-head screws from the top with nuts under the ears; they only clamp. The top side has
  nothing taller than the ticks, so the keep-out for hardware on top of the disc is 3 mm.
- Tick ring, outer end at r = 93 (2 mm rim left): 1° ticks 3 mm long, 0.5 mm wide; 5° ticks 5 mm, 0.8 mm wide;
  10° ticks 8 mm, 0.8 mm wide; the 0° tick 10 mm, 1.2 mm wide, plus a raised triangle on the rim pointing at it.
  Widths are FDM line widths for a 0.4 mm nozzle (one or two extrusions).
- Numerals every 10° (0 … 350), 5 mm tall, `Liberation Sans:style=Bold` (bundled with OpenSCAD), centred at
  r = 79, rotated with the angle so that the top of the digits points outward, as on the paper version.
- Numbering clockwise seen from the shaft side (0 at top, 10 to the right), same as the paper disc, so readings
  from either aid agree. `tick_style = "raised"` default; `"recessed"` makes 0.6 mm wide, 0.4 mm deep grooves
  instead, for filling with a marker.
- `$fa = 0.5; $fs = 0.3`: the 190 mm edge is a true circle to better than 0.01 mm.

## Pointer

- Hub Ø14 × 6 mm, through double-D bore: round part Ø5.3 (never binds), across the flats `flat_fit`, default
  3.0 mm, set from the coupon result (see below). `hub_clamp = false` by default; `true` adds a radial slot
  through the hub wall with a pocket for an M3 nut and an M3 screw hole, for a batch where no press fit holds.
- Arm 2.4 mm thick, its top flush with the hub's top, 8 mm wide at the hub tapering to 2 mm at r = 88, then
  carrying the blade: 0.8 mm wide (tangential), spanning r = 90 … 93 (the outer end of the ticks), hanging from the
  arm's underside down to the plane of the hub's underside, so the blade's lower edge and the hub's bottom are
  level. Chamfer the blade's lower edge to a line along the radius so the reading edge is sharp.
- Printed upside down: the arm's top face and the hub's top face on the bed, hub and blade rising from them. No
  supports, no overhangs. Mass a few grams; the stepper's 300 gf·cm does not notice it.
- Setting the height: push the hub down the shaft until a sheet of paper (0.1 mm) just slides between the blade
  and a tick; the blade then hovers 0.1 to 0.3 mm above the ticks and the hub bottom is 3.1 mm above the plate.
- Reading rule (the datum): the blade's lower edge against the ticks, viewed from directly above the tip. At 0.3 mm
  above the ticks a 15° viewing error moves the apparent position by 0.08 mm, 0.05°.
- Assembly check before any motor power: turn the pointer a full revolution by hand; it must clear the screw
  heads and nuts by at least 1 mm everywhere and must not touch the disc.

## Fit coupon (print first, about 10 minutes)

One part: a 50 × 26 × 2.4 mm plate with the disc's three holes (Ø9.4 centre, two Ø4.6 at the ear spacing) to
check that it sits flat over the boss with no perceptible side play and that the screws line up, plus five
Ø14 × 6 mm hub rings standing on the plate with double-D bores at `flat_fit` = 2.8, 2.9, 3.0, 3.1 and 3.2 mm
(round part Ø5.3 in all), marked with 1 to 5 raised dots. Criterion: the user picks the tightest ring that goes on
with a firm thumb push, and on it the ring must show no rotation when twisted with light finger pressure. That
value becomes `flat_fit`. If every ring is loose or every ring is too tight, the user re-exports the coupon with
`flat_series` shifted by 0.2 mm and prints it again; if none holds, `hub_clamp = true`.

## Print settings (README)

PLA, 0.2 mm layers, 0.4 mm nozzle, 3 walls, 4 top/bottom layers, 20 % infill (the disc is only 12 layers; any
infill is fine), 5 mm brim on the disc against warping, light filament so the raised ticks can be darkened with a
marker or pencil. Disc ≈ 85 g, about 3 h; pointer and coupon a few grams each.

## Acceptance (the maintainer runs these)

1. `make` with the snapshot OpenSCAD exports the three STLs without warnings; `make check` reports the disc as
   190.0 × 190.0 × 2.8 mm (±0.05), the pointer as 100 (x from −7 to 93) × 14 × 6 mm (hub height, in print
   orientation), the coupon within 50 × 26 × 8.4 mm, every file manifold and under 20 MB.
2. `make preview` renders a top view in which all 36 numerals read correctly, 0 is at the top, 10 is clockwise
   from it, and every 10° sector shows 9 short ticks between the long ones; the pointer side view shows the arm
   on top of the hub and the blade level with the hub's bottom.
3. In the model, the pointer at hub bottom z = 3.1 clears a 3 mm-tall head at r = 19.2 by ≥ 1 mm, and the blade's
   lower edge is at the hub's bottom plane (checked with `make check` on the pointer's bounding box plus a
   documented `echo()` of the two clearances in the .scad).
4. Changing `flat_fit`, `disc_d` or `flat_series` and re-exporting produces the expected change (parametric, no
   magic numbers in the bodies).
5. The implementer runs no reviews and no review scripts, and does not commit; the maintainer runs the Review
   protocol.
