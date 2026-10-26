# Plan

Legend: [x] done, [~] in progress, [ ] todo. Written 2026-10-22; decisions recorded 2026-10-22.

## Decisions (2026-10-22)

- **D1 Drive: 28BYJ-48 stepper + ULN2003 driver board** (from the user's kit, identified by appearance; the
  motor prints "28BYJ-48" on its label and has a 5-wire plug). A second one, bought 2026-10-22, drives the camera
  arm (Phase 4). The SG90 servo is a spare: shutter presser or clamp tilt if ever needed.
- **D2 Capture: RealityScan on the phone.** Use its **Object Mode** (automatic background removal, made for
  objects that are rotated or moved between shots; the object must not change shape), not AR Guidance, which
  expects the phone to move around a static scene. Shutter by hand at each stop, or RealityScan's **Capture
  Interval Timer** (made for turntable and tripod use) with the table's pause set to the same interval. A hardware
  trigger is a later option. Source: Epic, RealityScan Mobile 1.8 documentation.
- **D3 Reconstruction: RealityScan** (processing from the app). Turntable caveat: photogrammetry assumes a static
  scene; Object Mode's background removal handles it, and a featureless background (plain cloth, lightbox) still
  helps. RealityScan caps a scan at 300 photos (72 stops × 3 heights = 216 fits). Capture trial: phase 1b.
- **Mechanical: custom design**, not a downloaded STL: the platter rotates and the camera is raised between rings
  of photos. Options in Phase 4.
- **Board confirmed:** chip marking ATMEGA328PB; FTDI USB-serial at `/dev/cu.usbserial-A5069RR4`; bootloader
  answers at 115200 baud with signature `1E 95 0F` (328P) and not at 57600, so the `nano-new` preset applies.
- **D4 Power: the 18650 power-bank shield already on hand** (decided 2026-10-22 from photos of the board: single
  cell, 5 V 3 A and 3 V 1 A outputs, USB-C and micro-USB charging, XB8608 cell protection, push button, HOLD/AUTO
  slide switch; three boards and three Bengala 2200 mAh cells). A red/black pair soldered to one 5V header pair
  feeds the ULN2003 boards, ground common with the Nano; slide switch on HOLD (knob toward the `ims` mark; verified
  2026-10-23: 5.17 V after two minutes with no load) so the output survives the idle time between moves (the
  firmware releases the coils); charged between sessions, never while the table runs. Replaces
  the 5 V 3 A mains adapter and the barrel-jack adapters that were in the BOM.

## Phases

0. **Research** [x] 2026-10-22. `docs/phase-0-research.md`. Toolchain installed; `firmware/` builds for the Nano
   (atmega328p, 1934 B flash) and a native atmega328pb build was verified.
1. **Bring-up** [x] 2026-10-22. Chip marking confirmed, port found, bootloader probed (see Decisions),
   `cargo run` wrote and verified 2088 B and the console showed `turntable-firmware boot` and `tick 0..19` at
   1 Hz with the D13 LED blinking. Step-by-step commands for a first-time Arduino user: `docs/bring-up.md`.
1b. **Capture trial** [x] 2026-10-22. Phone on a tripod, RealityScan Object Mode with the Capture Interval Timer,
   object turned by hand: the user reports a recognisable model. The interval-timer flow is the design basis for
   Phase 3.
2. **Stepper drive** [x] 2026-10-26. ULN2003 IN1..IN4 on D2..D5, motor powered from the shield's 5 V output (D4, common
   ground). Half-step
   sequencer (8-step table) on a TC2 CTC interrupt, non-blocking, coils released after each move; integer error
   accumulation so N stops add up to exactly one revolution. Commands `STEP <n>`, `ROT <deg>`. First measurement:
   the gearbox ratio. 28BYJ-48 units come as 63.68395:1 (4075.77 half-steps per revolution) or exactly 64:1
   (4096); command 10 revolutions with the provisional 4076 and read the error on the protractor disc (17.8° short
   over 10 turns means the 64:1 variant), then fix the constant. Exit, measured on a printed protractor disc on the
   shaft with a 150 mm pointer (or a phone photo of a rim marker with the pixels-per-degree stated): a commanded
   90° and 10°, always approached from the same direction to hide gearbox backlash, land within ±0.5° (about 6
   half-steps; one half-step is 0.088°, invisible on paper); 10 commanded revolutions produce exactly 10 observed
   passes of the index mark and return to start within ±0.5°. Result (`docs/phase-2-verification.md`, hardware
   acceptance log): all sections pass; the platter motor is the 64:1 gearbox, now the firmware constant; ten turns
   in 360 stops return within 0.5°; no timer ticks lost under serial traffic; the board's clock runs 0.70 % slow.
3. **Scan mode, standalone** [ ]. Second axis first: the arm stepper on D6, D7, D8, D10 as a second controller
   instance with `ARM <deg>` (through the 8:1 reduction) and its own calibration. Then `SCAN <rings> <stops>
   <pause_ms>` runs K ring angles × N platter stops with a pause matched to RealityScan's interval timer (or a
   manual tap) and an LED cue; a pushbutton starts or advances; settings kept in EEPROM; the serial console is
   the UI. `turntable-core/` holds the command parser and motion math with host
   `cargo test`. A `host/` CLI is optional later (not needed for the RealityScan flow). Host rule from the Phase 2
   review: `ERR LINE` means resend the last unacknowledged command, and so does a reply timeout right after an
   unsolicited `ERR LINE`. Two measurements from the Phase 2 acceptance feed in: the board's clock runs 0.70 %
   slow, so scan pauses need a calibrated clock correction (or a margin) to stay in step with the phone's interval
   timer; and the arm motor gets its own gearbox calibration, since 28BYJ-48 units come in two ratios. Flash budget for Phase 3: 16 KiB (Phase 2 used 8134 B of its 8 KiB guideline; the
   hardware limit is 30720 B). Exit: a 36-stop scan runs unattended from one command.
4. **Mechanical, custom** [ ]. Printer: Creality Hi, 260 × 260 × 300 mm. Parametric CAD in `hardware/cad/`
   (OpenSCAD, STL exported from the command line). Inputs (2026-10-22): the kit has exactly one 28BYJ-48 + ULN2003
   and one SG90; phones: iPhone Air (165 g, 156.2 × 74.7 × 5.6 mm) or iPhone 18 Pro Max (249 g, 163.4 × 78 ×
   8.75 mm; reseller listings, not Apple's page), so the clamp takes 74-80 mm width, 5-10 mm thickness and 250 g
   plus a case; object: a car side-mirror cap, roughly 20-25 cm long.
   **Camera elevation: a second 28BYJ-48 on an arc arm** (decided 2026-10-22, replacing the SG90 arm chosen earlier
   the same day after comparing torque, dead band and code reuse; the SG90 becomes a spare for a shutter presser or a
   clamp tilt). The arm is balanced by an adjustable counterweight behind the pivot (with the motor unplugged it
   must stay wherever it is put) and driven through a **printed ~8:1 reduction** (spur pair, or a worm if
   self-locking matters): about 2.8 kg·cm at the arm, 60° in roughly 5 s, 0.011° per half-step, same driver code as
   the platter. Geometry: pivot on a horizontal axis at the object's centre height, arm radius ≈ 35 cm (the iPhone
   main camera frames a 25 cm object at about half the frame width from there), elevation 5°..65° for three rings
   (about 10°, 35°, 60°), arm printed in two bolted segments, pivot on an M8 partially threaded bolt in two 608ZZ
   bearings with printed spacers, hard stops so the arm cannot swing the phone into the platter, an index marker or
   switch for homing the arm if open-loop drift shows. Phone clamp: screw clamp for the range above, lens on the
   arm's radial line, USB-C cutout. Platter: 250 mm single piece (a two-segment 300 mm variant if the cap needs it),
   hub on two stacked 608ZZ so the motor shaft carries no platter weight, marker ring on the rim. Electronics bay
   in the base for the Nano, both ULN2003 boards and the shield (D4), with the shield's button, slide switch, LEDs
   and charging ports reachable from outside and the cell removable without tools. Parts:
   `hardware/BOM.md`. Firmware impact: second ULN2003 on D6, D7, D8, D10 (D9 stays reserved for an optional servo),
   a second controller instance in the same 1 kHz tick, `ARM <deg>` command (Phase 3 scan mode runs K rings × N
   stops). Exit: assembled rig on the shield's 5 V, strain-relieved cables, arm balanced, Phase 2 acceptance repeated
   with the platter loaded, and the three ring angles repeatable within ±0.5°.
5. **Scan pipeline** [ ]. RealityScan procedure: Object Mode, Capture Interval Timer matched to the table's pause,
   featureless background, diffuse fixed lighting, locked phone exposure, 36 stops × 3 heights; compare 24/36/72
   stops. Target object risk: a painted mirror cap is glossy and uniform, the worst case for photogrammetry
   (reflections move with the camera, no texture to match). Mitigations, in order: diffuse light (softbox or
   white tent) to kill reflections; a matte vanishing scanning spray or a dusting of dry shampoo; a few pieces of
   patterned painter's tape as temporary texture (removed in the model afterwards). Exit: a clean model of the
   mirror cap and a written procedure.
6. **Hardening** [ ], optional. Index sensor for homing, calibration in EEPROM, enclosure, CI building the Nano
   and UNO variants.

## Open questions for the user

- Does the kit contain a second 28BYJ-48 + ULN2003 (for the lift), or should the lift use the SG90 arm?
- Phone model (weight, dimensions) and the largest object to scan.
- Printer bed size.
