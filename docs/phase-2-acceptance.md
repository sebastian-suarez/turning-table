# Phase 2 hardware acceptance, step by step

The procedure itself is in `docs/phase-2-spec.md` ("Hardware acceptance"); this is the walkthrough with the exact
commands and what to expect. Record the readings at the end in `docs/phase-2-verification.md`.

## 0. Before you start

- Stepper wired with the USB cable unplugged (`docs/bring-up.md`, section 9): ULN2003 IN1..IN4 to D2..D5, driver
  `+` to the red wire soldered to a `5V` pad of the 18650 power-bank shield, driver `-` to the black wire from the
  `-` pad of the same pair and to a Nano GND (common ground).
- Shield ready: cell charged (LEDs), slide switch on HOLD (knob toward the `ims` mark; verified 2026-10-23: 5.17 V
  on the `5V` pads after two minutes with nothing connected), output switched on with the button. In AUTO the
  shield switches off a few seconds after each move and the next command moves nothing.
- Print and test `hardware/measure/fit-coupon.stl` first, as described in `hardware/measure/README.md`: the plate
  must sit flat over the boss (up to about 0.3 mm of slide is within the 0.2° budget; no visible wobble once the
  screws are tight), both ear holes must align, and the chosen double-D fit must go on with a firm thumb push
  with no rotation under light finger pressure. The coupon takes 20 to 30 minutes and engages the full 6 mm
  of shaft flats, so it reads slightly tighter than the hub's 4.1 mm
  engagement. Set `flat_fit` and re-export `pointer.stl` before printing the pointer and `protractor-disc.stl`.
  If no fit holds, `hub_clamp = true` adds an M3 clamp and grows the hub to Ø18. For recessed ticks, export the
  pointer with the same `tick_style = "recessed"` as the disc; the pointer's hub height depends on that style.
- With both supplies still disconnected, hold the motor down (tape or a clamp) so it cannot creep. Centre the
  printed disc on the shaft boss and clamp it to the ears with **two M4 button-head screws from above, nuts
  underneath**, or the printed substitutes in `screws.stl` (flat side of each nut toward the motor can; see
  `hardware/measure/README.md`). Heads and any washers above the disc must total at most 3 mm; do not use 4 mm socket-cap heads.
  Align the pointer's double-D flats with the shaft and press on the hub, arm uppermost. The Ø5.3 round relief
  at the hub's plate-facing end is 1.8 mm deep by default (2.2 mm for recessed ticks), allowing it to pass the
  shaft's round base and reach the paper gap while retaining 4.1 mm of double-D engagement. Find the highest
  point of a full hand sweep, where the tip comes closest to the ticks. Set its height there with a sheet of
  paper (about 0.1 mm) just sliding between a tick and the tip's flat lower face: leave 0.1–0.3 mm of gap and
  never let it drag anywhere in the sweep. If the hub stops on its relief shoulder before reaching the paper
  gap, lower `motor_flat_length` and re-export the pointer; do not force it down.
- **Before motor power, turn the pointer a full 360° by hand:** it must clear every head and nut by at least
  1 mm, never touch the disc, and remain tight on the shaft. The reading datum is the **0.8 × 0.8 mm end face
  at r = 93 mm**, read against the **outer ends of the ticks from above**; nothing of the pointer hides the
  end face and the arm is never the datum. 0° is at the top, increasing clockwise. Take readings after `DONE` (coils off),
  approaching each target from the same direction. The SVG sheet and card pointer remain a paper alternative;
  see `hardware/measure/README.md` for their 100% print scale and setup.
- Power up in this order: shield on (button), then the USB cable. `ls /dev/cu.*` must show the board again.

## 1. Flash the Phase 2 firmware and talk to it

```sh
cd firmware
cargo run
```

After `turntable-firmware boot`, type commands and press Enter (what you type is not echoed; the replies are):

| You type | Board answers | Meaning |
|---|---|---|
| `STATUS` | `STATUS Idle 2 407577 36 0` | state, step period in ms, gearbox ratio × 100, stops per revolution, stop counter |
| `POS` | `POS 0 0` | position in half-steps since `ZERO`, stop counter |
| `STEP 512` | `OK` … `DONE 512` | motor turns 1/8 of a turn (45°) in about 1.2 s, then releases |
| `STEP -512` | `OK` … `DONE 0` | back the other way |
| `ROT 90` | `OK` … `DONE 1019` | a quarter turn |
| `SPEED 4` | `OK` | 4 ms per half-step from the next move on (1..50) |
| `ZERO` | `OK` | call the current position 0 |
| `STOPS 36` | `OK` | 36 stops per revolution; `NEXT` moves to the next one |
| `NEXT` | `OK` … `DONE 113` | first stop of 36 (10°) |
| `ERR BUSY` | | you typed a move while one was running; wait for `DONE` |
| `ERR LINE` | | a line was lost or corrupted on the wire; retype the command |

If the motor buzzes without turning, two of IN1..IN4 are swapped; if nothing happens, check that the shield is on
(LEDs lit) and the common ground; if moves stop working after a pause while the console still answers, the shield
switched itself off: set the slide switch to HOLD and press the button. The sign of `STEP` is simply a direction:
note which way positive turns and keep that convention.

Leave the console with Ctrl-C before running the helper script below: the port is exclusive.

## 2. The helper for long runs

`hardware/measure/lockstep.py` sends commands one at a time and waits for each reply (and for `DONE` after a
move), which is exactly how the Phase 3 host will behave. Opening the port resets the board, so every run starts
from position 0 with 36 stops; put any `ZERO`, `SPEED`, `STOPS` you need at the front of the same command list.

```sh
python3 hardware/measure/lockstep.py /dev/cu.usbserial-A5069RR4 STATUS POS
```

## 3. Gearbox calibration (six runs, about 25 minutes of motor time)

Align the pointer onto the 0° tick **always arriving in the positive direction**: in the console, `STEP 50`
repeatedly until it sits on 0; if it passes 0, `STEP -300` and come up again. Leave the console, then:

```sh
python3 hardware/measure/lockstep.py /dev/cu.usbserial-A5069RR4 "SPEED 10" "STEP 40760"   # 6.8 min
```

Watch the pointer pass 0 ten times, then read where it stops. Realign, run at `SPEED 4` (2.7 min), realign, run at
`SPEED 2` (82 s); then a second round of each speed. Write down the six readings and the shield's 5 V reading at
the ULN2003 while the motor runs.

| Result | Meaning | Do |
|---|---|---|
| all six about +0.2° past 0 | 63.68:1 gearbox, the provisional constant is right | nothing |
| all six about 17.6° short of 0 | 64:1 gearbox | set `RATIO_X100` to `409_600` in `turntable-core/src/planner.rs`, `cargo run` again |
| slow runs agree, faster ones short | lost steps at that speed, not the gearbox | set `DEFAULT_PERIOD_MS` in `turntable-core/src/controller.rs` to the slowest good speed and repeat |
| readings scattered | pointer slipping on the shaft, sheet moving, or the index count was off | fix the mechanics, repeat |

## 4. Positioning and holding

In the console, pointer on 0, `ZERO`. Then, each reading taken after `DONE`:

- `ROT 90` → pointer on 90° ± 0.5°.
- `ROT -100` then `ROT 20` → pointer on 10° ± 0.5° (arrived positively, so backlash is excluded).
- Wait 10 s: the pointer must not move (the coils are off; friction holds).
- `STEP 8` ten times: the pointer advances 7.1° in total with no jump at any restart.

## 5. Closure over ten revolutions

Pointer on 0, then:

```sh
python3 hardware/measure/lockstep.py /dev/cu.usbserial-A5069RR4 "STOPS 36" --repeat 360 NEXT
python3 hardware/measure/lockstep.py /dev/cu.usbserial-A5069RR4 --no-boot-wait POS   # optional, resets the board
```

Expect the index to pass ten times and the pointer back on 0° ± 0.5° (the last `DONE` reads `40758`, the planner's
ten-turn total; the board forgets it on the next reset, so read the pointer first).

## 6. Serial robustness and timing under traffic

```sh
python3 hardware/measure/lockstep.py /dev/cu.usbserial-A5069RR4 --repeat 200 "STEP 10"      # 200 lockstep moves, ~40 s
python3 hardware/measure/lockstep.py /dev/cu.usbserial-A5069RR4 "ROT 3600"                   # 10 turns idle: note OK→DONE time
python3 hardware/measure/lockstep.py /dev/cu.usbserial-A5069RR4 --spam-status "ROT 3600"     # same with STATUS every 200 ms
```

Pass: 200 × `OK`/`DONE`, no `ERR LINE`; both ten-turn moves take 81.5 s ± 0.5 s from `OK` to `DONE` (the script
prints the time). A longer time under STATUS traffic means ticks were lost to masked interrupts.
