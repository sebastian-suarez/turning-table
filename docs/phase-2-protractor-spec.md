# Phase 2 protractor spec: 3D-printed disc and pointer (v2.3, 2026-10-23)

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

v2 → v2.1 (implementer's finding): the shaft is round below z = 4 (its flats cover only the outer 6 mm), so a
double-D bore running the full hub height could never descend to z = 3.1. The lowest 1.1 mm of every double-D
bore (hub and coupon rings) is now a round relief.

v2.1 → v2.2 (round-1 reviews of the implementation: Astra two findings, Fable nine, all accepted; the two they
share come first): the v2 blade hung
straight down from the arm with its reading edge chamfered to a line under the blade body, so from above the
user could only see the blade's top, 6.3 mm above the ticks, and the parallax was 1° at 15°, worse than the
paper pointer; the tip is now a low wedge that reaches beyond the arm so its 0.8 mm-tall end is the visible
datum. And the arm height was derived from the tick height, so `tick_style = "recessed"` lowered the arm and
failed the 1 mm screw-head clearance; the arm height is now derived from the head keep-out alone and the hub
height follows from it. Fable's other findings: the M3 clamp's nut pocket left a 0.5 mm wall against the bore
flat (the pocket now starts outside the flat and the hub grows to Ø18 when `hub_clamp` is on); the 50 × 26 mm
coupon envelope put three test collars across the Ø9.4 boss hole, thinning their walls to 1.1 mm so they fit
softer than the real hub (the plate is now 70 × 34 mm with the five collars in one row clear of every hole);
the 0.2 mm relief margin was a hard stop for a clone whose flats start higher (margin now 0.7 mm, and the
README says what to do if the hub stops above the paper gap); README wording on collar engagement, the gap set
at the highest point of the sweep, the coupon print time (20 to 30 minutes), and a tautological echo.

v2.2 → v2.3 (round-2 reviews; the maintainer's own fix in between: the wedge was fused to the arm along shared
faces and Manifold exported zero-area triangles, so the tip piece now owns everything beyond the arm's taper
and its root sits one layer inside the arm). Astra one finding and Fable seven, all minor; accepted: the bar's
underside was a 0.8 mm cantilever in print, found by both (the slope now runs at 45° all the way to the end
face); the head-clearance assert compared doubles exactly and failed at a true 1.0 mm for an 11-layer disc
(epsilon); the pointer preview was too small to show the tip (fixed scale plus a tip close-up); "no
perceptible side play" contradicted the designed 0.2 mm boss clearance (reworded as a tolerance); the collar
test checked twist but not rock along the flats (added, with `round_bore_d` as the documented knob); thin-wall
slicing needs saying; the disc mass and the recessed coupon height were wrong in the README, and
`check-recessed` now covers the coupon. Rejected: Ø5.1 for the bore's round part as the default, since a
shrunken FDM hole would then bind on the arcs instead of the flats.

## Deliverables

- `hardware/cad/protractor.scad`: one file, all parts, parameters at the top with the defaults below, commented
  generously (why each dimension, which FDM constraint or finding it answers). `part = "disc" | "pointer" |
  "coupon"` selects what to render (`-D 'part="disc"'`); the default (no `-D`) shows all three side by side for
  the GUI, each already in its print orientation.
- `hardware/cad/Makefile`: `make` exports `hardware/measure/protractor-disc.stl`, `pointer.stl` and
  `fit-coupon.stl` (binary STL, manifold backend); `make preview` renders `hardware/measure/protractor-disc.png`
  (top view, for a visual check of ticks and numerals), `pointer.png` (side view at a fixed scale, no
  `--viewall`) and `pointer-tip.png` (close-up of the last 6 mm, where the end face must be legible); `make check` prints each
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
| Ticks and numerals | 2.4 … 2.8 | raised 0.4 mm (two layers); with `tick_style = "recessed"` the surface is 2.4 |
| Screw heads on the disc | ≤ 2.4 + 3.0 = 5.4 | M4 button heads are 2.2 mm; any head up to 3 mm fits, socket caps (4 mm) do not |
| Pointer arm | 6.4 … 8.8 | underside = disc + 3 mm keep-out + 1 mm clearance, independent of the tick style; 2.4 mm thick |
| Pointer hub | 2.9 … 8.8 | top flush with the arm; bottom = tick surface + 0.1 mm, the tightest paper gap, so `hub_h` is derived: 5.9 mm raised, 6.3 mm recessed |
| Tip wedge | 2.9 … 6.4 at r = 90 … 90.3, sloping to 2.9 … 3.7 at r = 93 | hangs from the arm's end; its lower face is the datum plane, level with the hub's bottom |
| Shaft tip | 10 | 1.2 mm above the hub; the bore's 1.8 mm round relief leaves 4.1 mm of flats engaged |

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

- Hub Ø14, height derived (`hub_h` = arm top − hub bottom = 5.9 mm with raised ticks), through double-D bore:
  round part Ø5.3 (never binds), across the flats `flat_fit`, default 3.0 mm, set from the coupon result (see
  below). The lowest `bore_relief_h` of the bore (the end nearest the mounting plate in use, the top in print
  orientation) is round at Ø5.3: `bore_relief_h` = (shaft length − flat length) − hub bottom z + 0.2 mm margin =
  10 − 6 − 2.9 + `relief_margin` (0.7) = 1.8 mm, derived from the motor parameters, so the hub can pass the round
  part of the shaft even on a clone whose flats start up to 0.7 mm higher; the flats still engage 4.1 mm. Assert
  that the engagement stays ≥ 3 mm. `hub_clamp = false` by default; `true` adds a radial slot through the hub
  wall with a pocket for an M3 nut and an M3 screw hole, for a batch where no press fit holds; with the clamp on,
  the hub is Ø18 so that the nut pocket starts outside the bore flat (`clamp_nut_inner_y` = max(slot/2,
  `flat_fit`/2) + 1.6 mm jaw wall) and holds a full 2.4 mm nut inside the wall; assert both walls ≥ 1.6 mm.
- Arm 2.4 mm thick, its top flush with the hub's top, its underside at disc top + `max_head_h` (3) +
  `required_head_gap` (1) = 6.4 mm from the plate, whatever the tick style; 8 mm wide at the hub tapering to 2 mm
  at r = 88, then 0.8 mm wide to r = 90 where the arm ends.
- Tip wedge, 0.8 mm wide (tangential), from r = 90 to 93 (the outer end of the ticks), hanging from the arm's end.
  Its lower face is flat in the datum plane (level with the hub's bottom). Its upper face is a 45° slope (never
  shallower) that ends exactly at the end face, which is 0.8 mm tall and 0.8 mm wide with nothing above it:
  the slope starts at the arm's underside at r = 93 − (arm underside − datum − 0.8) = 90.3 with raised ticks
  (a 0.3 mm ledge at the arm's underside between r = 90 and 90.3), and at r = max(90, that) in general. In print
  orientation the slope is a 45° overhang with no cantilever anywhere, printable without support.
- Printed upside down: the arm's top face and the hub's top face on the bed, hub and wedge rising from them. No
  supports. Mass a few grams; the stepper's 300 gf·cm does not notice it.
- Setting the height: push the hub down the shaft until a sheet of paper (0.1 mm) just slides between the tip's
  lower face and a tick at the highest point of a full hand sweep (shaft tilt or disc warp can move the tip by
  0.5 mm around the turn); the tip then hovers 0.1 to 0.3 mm above the ticks there and never drags. If the hub
  stops on its relief shoulder before the paper gap is reached, the motor's flats start higher than 4 mm: lower
  `motor_flat_length` and re-export. Over that whole range the arm keeps
  ≥ 1 mm above a 3 mm head (the model's clearance is computed at the 0.1 mm end).
- Reading rule (the datum): the end of the tip, the 0.8 × 0.8 mm end face at r = 93, read against the outer
  ends of the ticks from above. Its top is at most 1.1 mm above the ticks, so a 15° viewing error moves the
  apparent position by 0.3 mm, 0.18°; nothing of the pointer sits above the tip to hide it. The arm's broad body
  is never the datum.
- Assembly check before any motor power: turn the pointer a full revolution by hand; it must clear the screw
  heads and nuts by at least 1 mm everywhere and must not touch the disc.

## Fit coupon (print first, 20 to 30 minutes)

One part: a 70 × 34 × 2.4 mm plate with the disc's three holes (Ø9.4 centre, two Ø4.6 at the ear spacing) to
check that it sits flat over the boss with no perceptible side play and that the screws line up, plus five
Ø14 hub rings of height `hub_h` standing on the plate in one row on the +y side, every ring footprint clear of
the three holes by ≥ 2 mm so no ring wall is thinner than the real hub's, with double-D bores at `flat_fit` = 2.8, 2.9, 3.0, 3.1 and 3.2 mm
(round part Ø5.3 in all, the same round relief at each ring's free end as on the hub), marked with 1 to 5 raised
dots. Criterion: the user picks the tightest ring that goes on
with a firm thumb push, and on it the ring must show no rotation when twisted with light finger pressure. That
value becomes `flat_fit`. Also lever the chosen ring up and down along the flats: it must not rock; if it does,
the round part of the bore is the knob (`round_bore_d`, default 5.3; try 5.2 on a reprinted coupon). If every
ring is loose or every ring is too tight, the user re-exports the coupon with
`flat_series` shifted by 0.2 mm and prints it again; if none holds, `hub_clamp = true`.

## Print settings (README)

PLA, 0.2 mm layers, 0.4 mm nozzle, 3 walls, 4 top/bottom layers, 20 % infill (the disc is only 12 layers; any
infill is fine), 5 mm brim on the disc against warping, light filament so the raised ticks can be darkened with a
marker or pencil. The slicer must print thin walls (Arachne in PrusaSlicer or OrcaSlicer, "print thin walls" in
Cura): the 0.5 mm ticks and the 0.8 mm neck and tip vanish under classic two-perimeter slicing. Disc ≈ 60 g at
20 % infill, about 3 h; pointer and coupon a few grams each.

## Acceptance (the maintainer runs these)

1. `make` with the snapshot OpenSCAD exports the three STLs without warnings; `make check` reports the disc as
   190.0 × 190.0 × 2.8 mm (±0.05), the pointer as 100 (x from −7 to 93) × 14 × 5.9 mm (`hub_h`, in print
   orientation), the coupon within 70 × 34 × 8.3 mm, every file manifold and under 20 MB. A `check-recessed`
   target does the same with `tick_style="recessed"` (disc 2.4 mm, pointer 6.3 mm, coupon 8.7 mm) and both
   styles export without tripping the clearance assert, as does `-D disc_h=2.2` (the assert compares with an
   epsilon).
2. `make preview` renders a top view in which all 36 numerals read correctly, 0 is at the top, 10 is clockwise
   from it, and every 10° sector shows 9 short ticks between the long ones; the pointer side view shows the arm
   on top of the hub and the tip wedge's lower face level with the hub's bottom, and the close-up shows the 45°
   slope ending in the 0.8 mm end face.
3. In the model, the pointer at hub bottom z = 2.9 clears a 3 mm-tall head at r = 19.2 by ≥ 1 mm, and the tip's
   lower face is at the hub's bottom plane (checked with `make check` on the pointer's bounding box plus a
   documented `echo()` of the two clearances in the .scad).
4. Changing `flat_fit`, `disc_d` or `flat_series` and re-exporting produces the expected change (parametric, no
   magic numbers in the bodies).
5. The implementer runs no reviews and no review scripts, and does not commit; the maintainer runs the Review
   protocol.
