# Spec: phase-3

Version: 3
Status: suite-ready

## Problem

Phase 2 drives the platter from the console, one move per command. A photogrammetry scan needs the object seen
from K camera heights (rings) all the way round, with the camera arm raised between rings, and it should run
without the Mac. The phone captures on its own (RealityScan's interval timer or its auto-capture), so the table
does not try to keep time with it: the platter turns at a steady, slow pace, and any photo the phone takes is
usable. This is Phase 3 of `docs/PLAN.md` (board card B3). Version 1 synchronised stop-and-go moves to the phone's
timer; its attack showed that a phone timer a few milliseconds off puts late photos into moves, and the user chose
the steady turn instead (2026-10-26).

## Scope

In:
- The arm stepper on its own ULN2003 as a second `Controller` instance in the same 1 kHz TC2 tick, with `ARM <deg>`.
- `HOME`: a push button used as a home switch sets the arm's reference angle.
- `SCAN <rings> <photos> <interval>`: for each ring the arm moves to its height, then the platter turns exactly
  one revolution in about `photos × interval` ms; `STOP` halts it.
- A start/stop push button for use without the Mac.
- Settings in EEPROM (two copies, never a mix after a power cut): calibration (gear ratios, home angle) and scan
  settings (ring heights, last scan), with defaults built into the code and console commands that override them.
- The new logic in `turntable-core/`, tested on the host; the firmware glue in `firmware/`.

Out: the platter's Phase 2 motion behaviour (half-step table, move lifecycle and timings, cumulative stop grid, RX
loss handling), which stays as it is; the CAD; the display.

Non-goals: the LCD1602 menu (next phase; its I2C pins A4/A5 stay free); a host CLI; synchronising with the phone;
correcting the board's 0.70 % slow clock (a turn takes 0.7 % longer than asked); the home switch as a limit
during ordinary moves; acceleration ramps; a spiral path; a `DEFAULTS` command (type the values listed below
instead); homing the platter; the SG90.

## Definitions

### Pins

| Signal | Nano pin | AVR | Notes |
|---|---|---|---|
| Platter ULN2003 IN1..IN4 | D2..D5 | PD2..PD5 | unchanged |
| Arm ULN2003 IN1..IN4 | D6, D7, D8, D10 | PD6, PD7, PB0, PB2 | D9 stays reserved for the optional servo |
| Start/stop button | A0 | PC0 | to GND, internal pull-up, closed = low |
| Home switch (a push button) | A1 | PC1 | to GND, internal pull-up, closed = low |
| Activity LED | D13 | PB5 | on while either motor is not `Idle` (moving or settling), as in Phase 2 |
| Console | D0/D1 | USART0 | 57600 8N1, unchanged |

Free for later: A2, A3, D11, D12 (menu buttons), A4/A5 (I2C display), A6/A7 (analog inputs only). Both ULN2003
boards take their `+` from the shield's 5 V with ground common to the Nano.

### Constants

| Name | Value | Meaning |
|---|---|---|
| `ARM_MAX_TENTHS` | 650 | highest arm elevation, 65.0° (the plan's 5°..65° travel) |
| `MAX_RINGS` | 4 | at most four ring heights |
| `MAX_PHOTOS` | 300 | RealityScan's photo limit per scan (`docs/PLAN.md`, D3) |
| `CLOCK_MARGIN_PERCENT` | 1 | the photo estimate assumes a board clock up to 1 % slow (measured: 0.70 %) |
| `START_ALLOWANCE_MS` | 10000 | the photo estimate allows the phone to start shooting 10 s before the table |
| `MOVE_OVERHEAD_TICKS` | 171 | 1 start tick + 20 energise + 50 hold + 100 settle, from the Phase 2 lifecycle |
| `HOME_PERIOD_MS` | 4 | arm speed while homing, ms per half-step |
| `HOME_TRAVEL_TENTHS` | 800 | longest homing search, 80.0° of arm travel |
| `HOME_BACKOFF_TENTHS` | 50 | lift before the search when the switch is already closed, 5.0° |
| `SWITCH_DEBOUNCE_TICKS` | 2 | home switch: closed for 2 consecutive 1 ms samples |
| `BUTTON_DEBOUNCE_TICKS` | 20 | start/stop button: 20 consecutive 1 ms samples |

### Settings and their defaults

| Setting | Command | Unit and range | Built-in default |
|---|---|---|---|
| Platter ratio | `PRATIO <n>` | half-steps per platter revolution × 100, 100000..1000000 | 409600 (64:1, measured 2026-10-26) |
| Arm ratio | `ARATIO <n>` | half-steps per arm revolution × 100 (motor ratio × reduction), 100000..20000000 | 3276800 (409600 × 8, provisional: arm motor unmeasured, reduction designed in Phase 4) |
| Home angle | `AHOME <deg>` | arm elevation at which the home switch closes, one decimal, 0.0..30.0 | 5.0 (provisional, from the Phase 4 CAD) |
| Ring heights | `RINGS <h1> [h2] [h3] [h4]` | 1..4 whole degrees, each 0..90 | 10 35 60 |
| Saved scan | `SCAN <r> <p> <i>` | rings 1..4, photos per ring 1..300, interval 100..60000 ms | 3 36 3000 |

Arm angles are elevations: 0° is horizontal, positive is up. The arm position is an `i32` count of arm half-steps
from 0°, and an angle converts as `round_half_away(tenths × aratio_x100 / 360000)` (the Phase 2 rounding). With the
defaults the home angle 5.0° is position 455, and 10°, 35°, 60° are 910, 3186, 5461. Positive arm half-steps are
"up"; on the bench, by convention, up is clockwise seen from the shaft side (as the platter's `STEP +`). If the
assembled rig moves the wrong way, swapping the jumpers IN1↔IN4 and IN2↔IN3 reverses the motor with no code change.

### EEPROM record

Two slots of 24 bytes, slot 0 at EEPROM address 0 and slot 1 at address 32, each little-endian:

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | magic `0x54` |
| 1 | 1 | layout version `1` |
| 2 | 4 | platter ratio `u32` |
| 6 | 4 | arm ratio `u32` |
| 10 | 2 | home angle, tenths, `i16` |
| 12 | 1 | ring count 1..4 |
| 13 | 4 | ring heights `u8`, unused entries 0 |
| 17 | 1 | saved scan rings |
| 18 | 2 | saved scan photos per ring `u16` |
| 20 | 2 | saved scan interval ms `u16` |
| 22 | 1 | CRC-8 (polynomial 0x07, initial value 0, no reflection, no final XOR) over bytes 0..21 and 23 |
| 23 | 1 | generation `u8` |

- A slot is valid when magic, version and CRC match and every field is in its range from the settings table
  (unused ring entries must be 0). Cross-field rules (a ring below the home angle, saved rings above the ring
  count) are not decode errors; `SCAN` checks them when it runs.
- Loading: if both slots are valid, the one whose generation is one ahead of the other's (modulo 256) is current;
  if the generations are equal or differ by anything else, slot 0. If one slot is valid, it is current. If none is,
  the built-in defaults apply, with no current slot. Nothing is written at boot.
- Saving writes the new record to the slot that is not current (slot 0 when there is none), with generation =
  current generation + 1 (or 1), in this order:
  1. if the destination's magic byte reads `0x54`, it is set to `0x00` first;
  2. offsets 1..23 in address order (a byte that already holds its value may be skipped);
  3. the magic byte `0x54`, last.
- Every byte is read back after its write. The first mismatch stops the save with `ERR SAVE`, and nothing after
  it is written.
- A power cut at any point therefore leaves the old settings current. The destination's magic is not `0x54` from
  step 1 until step 3 completes. The step 1 write only ever turns an older valid record (one generation behind)
  into an invalid one. The new settings become current with the last byte.
- A record whose payload was complete but whose magic write was cut is never revived: the next save sees a magic
  other than `0x54`, skips step 1 and overwrites it. This holds across any number of interrupted saves.

### Console protocol additions

Lines, case, the 24-byte limit, `ERR LINE` handling and every Phase 2 command keep their Phase 2 behaviour, with
three exceptions:
- `SPEED` sets the half-step period of both motors' console moves and is refused while a scan or homing runs.
- `STATUS` gains the arm and scan fields at the end of its line.
- `STOP` halts any move.

| Command | Reply | Then |
|---|---|---|
| `ARM <deg>` (one decimal) | `OK` | `ADONE <arm position>` |
| `HOME` | `OK` | `ADONE <home position>` on success, `ERR HOME` on failure |
| `SCAN <rings> <photos> <interval>` or `SCAN` | `OK` | `ADONE`/`DONE` per move, then `END <turns completed>` |
| `STOP` | `OK` | halts whatever moves now (see Halt); ends a scan or homing |
| `RINGS`, `PRATIO`, `ARATIO`, `AHOME` | `OK` after the EEPROM write | |
| `CFG` | `CFG <pratio> <aratio> <ahome tenths> <scan rings> <scan photos> <scan interval> <h1> [h2 h3 h4]` | |
| `STATUS` | `STATUS <state> <period> <pratio> <stops> <k> <arm state> <arm position> <homed 0/1> <ring>` | |

- `<ring>` is `-` when no scan runs, otherwise the 1-based ring being scanned.
- New errors: `ERR HOME` (arm not homed, or homing failed) and `ERR SAVE` (EEPROM read-back mismatch).
- Boot prints the Phase 2 banner, then `DEFAULTS` if no EEPROM slot was valid, then a `CFG` line.
- `RINGS` checks each height only against 0..90. `SCAN` checks the heights it uses against the arm's travel, so a
  later `AHOME` cannot slip past.

Host rule from the Phase 2 review, unchanged: any `ERR LINE`, solicited or not, means "resend the last
unacknowledged command", and so does a reply timeout right after an unsolicited `ERR LINE`.

### Admission (which command runs when)

- `POS`, `STATUS`, `CFG`, `STOP`: always accepted.
- `SPEED`: `ERR BUSY` while a scan or homing runs, otherwise accepted.
- Every other command: `ERR BUSY` unless both motors are `Idle` and no scan or homing runs.
- Then `ARM` and `SCAN`: `ERR HOME` unless the arm is homed. `BUSY` takes precedence over `HOME`.
- The arm is not homed at boot, after a failed or stopped `HOME`, and after an accepted `ARATIO` or `AHOME`.

### Arm moves

`ARM <deg>` moves the arm to that absolute elevation. It is `ERR RANGE` below the home angle or above
`ARM_MAX_TENTHS`. The arm uses the platter's move lifecycle, so `ADONE` means released and settled.

### Homing

1. If the home switch reads closed when homing starts, the arm first lifts `HOME_BACKOFF_TENTHS` at
   `HOME_PERIOD_MS`. If the switch still reads closed after that move, homing fails.
2. Search: the arm moves down `HOME_TRAVEL_TENTHS` at `HOME_PERIOD_MS`. In the TC2 tick, the first tick on which
   the switch has read closed for `SWITCH_DEBOUNCE_TICKS` consecutive samples halts the arm.
3. When the search move is `Idle`: if a switch closure halted it, the arm position becomes the home angle's
   position, the arm is homed and `ADONE <position>` is printed. If the travel ran out, homing fails.
4. A failure, or a `STOP` during step 1 or 2, prints `ERR HOME` once the arm is `Idle` and leaves the arm not
   homed. The lift in step 1 prints nothing.

### Halt and moves at a given period

- `Controller::halt()` ends a move early: no half-step is emitted after it. The coils stay on the last pattern and
  go all-low on the 50th tick after the halt; `Done`, carrying the position reached, fires on the 150th.
- A halt during `Energise` emits no half-step at all. A halt in `Idle`, `Hold`, `Release` or `Settle` changes
  nothing. Every accepted move still produces exactly one `Done`.
- `Controller::start_with_period(steps, period_ms)` starts a move like `start` but at `period_ms` (1..255) for that
  move only; the `SPEED` period is unchanged. Homing and scan turns use it.
- A halted `NEXT` leaves the platter off its stop grid; the next `NEXT` returns to the grid (Phase 2's absolute
  targeting). `STOP` is the console's way to halt; the button does the same.

### Scan sequence

`SCAN` with arguments saves them as the saved scan (after the EEPROM write), then runs. `SCAN` alone runs the
saved scan. In both cases:

1. Checks, all before anything moves:
   - the arm must be homed (`ERR HOME`);
   - `rings` must not exceed the ring count, and each of the first `rings` heights must lie within the home angle
     and 65.0° (`ERR RANGE`);
   - the turn period `p = round_half_away(photos × interval × 100 / pratio_x100)` ms per half-step must lie in
     1..255 (`ERR RANGE`);
   - the photo estimate must be at most `MAX_PHOTOS` (`ERR RANGE`).
2. The platter planner is rebased like `STOPS 1` (origin = platter position, k = 0), so each turn is a `NEXT`
   closing one exact revolution on the cumulative grid.
3. For ring r = 1..rings: if the arm is not at ring r's position, an arm move there at the `SPEED` period with the
   platter still; then the turn, one `NEXT` at period `p` with the arm still. Each move starts once the previous
   one is `Idle` (its `ADONE`/`DONE` has fired).
4. `END <turns completed>` when the last turn is `Idle`. On `STOP`, every motor halts at once and
   `END <turns completed>` is printed once both are `Idle` (a halted turn does not count).

The photo estimate assumes the phone shoots every `interval` ms for the whole scan, starts up to
`START_ALLOWANCE_MS` before the table, and that the board's ticks may last up to `CLOCK_MARGIN_PERCENT` longer
than a millisecond:
`floor((floor(T × (100 + CLOCK_MARGIN_PERCENT) / 100) + START_ALLOWANCE_MS) / interval) + 1`. `T` is in ticks:
- one `MOVE_OVERHEAD_TICKS + |Δ| × speed` per arm move;
- plus `rings × MOVE_OVERHEAD_TICKS`;
- plus `round_half_away(rings × pratio_x100 / 100) × p`.

Worked example (defaults, `SPEED 2`, arm homed at 455, `SCAN 3 36 3000`):
- `p = round(108000 × 100 / 409600) = 26` ms, so a turn takes 106.5 s by the board's ticks.
- The moves are arm +455, turn, arm +2276, turn, arm +2275, turn.
- `T = 330526` ticks, so the estimate is 115 photos (the phone really takes about 111 when started with the
  table).
- `SCAN 1 36 3000` from home: arm +455, turn, estimate 40.
- `SCAN 3 96 3000` is accepted (`p = 70`, estimate 297; about 296 real photos with the phone 10 s early) and
  `SCAN 3 97 3000` is `ERR RANGE` (`p = 71`, estimate 301).
- With `RINGS 5 65 5 65`, `SCAN 4 70 3000` is accepted (estimate 297) and `SCAN 4 71 3000` is `ERR RANGE`
  (estimate 302).
- `SCAN 1 1 100` (`p = 0`) and `SCAN 1 300 60000` (`p = 4395`) are `ERR RANGE`.

Procedure this serves: set RealityScan's interval timer to `interval` (or use auto-capture), start it, start the
scan, and stop the phone after `END`.

### Start/stop button

The press is registered after `BUTTON_DEBOUNCE_TICKS` closed samples that follow at least as many open ones;
holding the button produces no repeats. The debouncer starts as released.

| State | Press does |
|---|---|
| any motor moving, or a scan or homing running | `STOP` |
| idle, arm not homed | `HOME` |
| idle, arm homed | `SCAN` with the saved settings; a refusal prints its `ERR` line |

### Core interface (what the host suite tests)

New public items in `turntable-core`, re-exported from the crate root. The implementer may add items but not
change these without a spec change.

- **Commands and errors.** `Command` gains `Arm(i16)` (tenths), `Home`, `Rings(Rings)`, `Scan(Option<ScanSettings>)`,
  `Stop`, `PRatio(u32)`, `ARatio(u32)`, `AHome(i16)` (tenths) and `Cfg`. `Error` gains `Home` and `Save` (`as_str`
  gives `HOME`, `SAVE`).
- **Settings.**
  - `Rings { heights: [u8; 4], len: u8 }`, `ScanSettings { rings: u8, photos: u16, interval_ms: u16 }`.
  - `Settings { pratio_x100: u32, aratio_x100: u32, ahome_tenths: i16, rings: Rings, scan: ScanSettings }` with
    public fields, and `Settings::DEFAULT`.
  - `SETTINGS_BYTES = 24`, `Settings::encode(&self, generation: u8) -> [u8; 24]` and
    `Settings::decode(&[u8; 24]) -> Option<(Settings, u8)>` (settings and generation).
  - `load(slot0: &[u8; 24], slot1: &[u8; 24]) -> Option<Loaded>` with
    `Loaded { settings: Settings, slot: u8, generation: u8 }`.
  - `save_plan(slot0: &[u8; 24], slot1: &[u8; 24], settings: &Settings) -> SavePlan`, where `SavePlan` is an
    `Iterator<Item = (u16, u8)>` of EEPROM address and byte, in the order of the saving rule (at most 25 writes).
    The firmware executes it with read-back; the host suite simulates power cuts with it.
- **Controller.** `Controller::halt(&mut self)` and
  `Controller::start_with_period(&mut self, steps: i32, period_ms: u8) -> Result<Output, Error>`.
- **Inputs.** `Debouncer::new(ticks: u8)`, `Debouncer::sample(&mut self, closed: bool) -> bool` (true on the sample
  that registers a press), `Debouncer::closed(&self) -> bool` (the debounced level).
- **Admission.** `Activity { platter_idle: bool, arm_idle: bool, scanning: bool, homing: bool, homed: bool }`,
  `admit(&Command, Activity) -> Result<(), Error>`, `button_action(Activity) -> ButtonAction` with
  `ButtonAction::{Stop, Home, Scan}`.
- **Homing.** `Homing::start(switch_closed: bool, settings: &Settings) -> (Homing, HomingStep)`,
  `Homing::done(&mut self, switch_closed: bool, halted_by_switch: bool) -> HomingStep` and `Homing::stop(&mut self)`,
  with `HomingStep::{Move { steps: i32, period_ms: u8, halt_on_switch: bool }, Homed(i32), Failed}`.
- **Scan.**
  - `Scan::new(settings: &Settings, run: ScanSettings, speed_ms: u8, arm_position: i32) -> Result<Scan, Error>`.
  - `Scan::next(&mut self) -> Option<ScanMove>` with `ScanMove::{Arm(i32), Turn}`.
  - `Scan::turn_period_ms(&self) -> u8` and `Scan::photos(&self) -> u32` (the estimate).

### Firmware rules

- The Phase 2 concurrency rules hold for both motors:
  - PORTB and PORTD (coils and LED) are written only by the TC2 ISR, or before interrupts are enabled;
  - shared state crosses the ISR boundary only through `interrupt::Mutex`;
  - no `i64` arithmetic happens in an ISR or a critical section;
  - each motor has its own `DONE` mailbox.
- The TC2 ISR ticks both controllers, samples the home switch and the button, and halts the arm on a debounced
  closure while a homing search is armed.
- EEPROM writes run in main, write only bytes that changed and are read back.
  - Before each byte, wait with interrupts enabled until `EEPE` clears.
  - Then mask interrupts only around the `EEMPE`→`EEPE` strobe. The HAL's `write_byte` does not do this itself:
    an interrupt between the two lets the 4-cycle window lapse, and the write is silently dropped.
  - A read-back mismatch replies `ERR SAVE`. The new value stays in RAM, and the slot that was current stays
    current.
  - Settings commands are refused during moves and scans.
- Strings stay terse (they live in SRAM); `ufmt` only.

## Acceptance criteria

### P: parser (host)

- P1 `ARM 10` → `Arm(100)`, `arm 47.5` → `Arm(475)`, `ARM 0` and `ARM 90` accepted; `ARM 90.1`, `ARM -0.1` →
  `RANGE`; `ARM`, `ARM 1.25`, `ARM 1 2` → `SYNTAX`.
- P2 `HOME`, `STOP`, `CFG`, `SCAN` parse with no argument; `HOME 1`, `STOP 1`, `CFG 1` → `SYNTAX`.
- P3 `RINGS 10` → heights `[10]`; `RINGS 10 35 60` and `RINGS 60 35 10 5` keep their order; `RINGS` and five
  heights → `SYNTAX`; `RINGS 10.5` → `SYNTAX`; `RINGS 91` → `RANGE`.
- P4 `SCAN 3 36 3000` → `Scan(Some(..))`; `SCAN 3 36` and `SCAN 1 2 3 4` → `SYNTAX`; `SCAN 0 36 3000`,
  `SCAN 5 36 3000`, `SCAN 1 0 3000`, `SCAN 1 301 3000`, `SCAN 1 36 99`, `SCAN 1 36 60001` → `RANGE`;
  `SCAN 4 300 60000` and `SCAN 1 1 100` parse (their limits are `Scan::new`'s).
- P5 `PRATIO 100000` and `1000000` accepted, `99999` and `1000001` → `RANGE`; `ARATIO 100000` and `20000000`
  accepted, `99999` and `20000001` → `RANGE`; `AHOME 0`, `AHOME 5.5`, `AHOME 30` accepted, `AHOME 30.1` and
  `AHOME -0.1` → `RANGE`; `TRIM 0` → `SYNTAX` (no such command).
- P6 The Phase 2 suite (`turntable-core/src/tests.rs`) passes. Its single change is that `ARM 10` leaves the
  list of rejected lines, since it is now a valid command.

### S: settings and EEPROM (host)

- S1 `Settings::DEFAULT` holds exactly the defaults in the settings table.
- S2 `decode(encode(s, g)) == Some((s, g))` for the defaults and for settings with every field at its minimum and
  at its maximum, for generations 0, 1 and 255.
- S3 `decode` of 24 bytes of `0xFF` (erased EEPROM) and of 24 bytes of `0x00` → `None`.
- S4 For every byte position of a valid encoding, replacing that byte with `byte ^ 0x01` and with `byte ^ 0xFF`
  makes `decode` return `None`.
- S5 Bytes laid out as in the record table with a correct CRC but one field out of range → `None`. Each field is
  tried once, including a nonzero unused ring entry and a wrong version.
- S6 `load` picks per the loading rule. Cases:
  - both slots valid with generations (4, 5), (5, 4), (255, 0) and (0, 255);
  - equal generations;
  - one slot valid (either side);
  - none valid → `None`.
- S7 `save_plan` follows the saving rule:
  - the destination is the slot that is not current;
  - the first write sets the magic byte to `0x00` when it reads `0x54`, and is absent otherwise;
  - the last write sets the magic byte to `0x54`;
  - applying every write makes `load` return the new settings at generation + 1.
- S8 Interrupted saves. Start states:
  - both slots valid at adjacent generations, including (255, 0) and (0, 255);
  - a blank EEPROM (all `0xFF`).

  Run up to three consecutive saves of different settings, each made with `save_plan` on the slots as left by the
  previous one. Each save is cut after any number of its writes, and the write being cut leaves `0x00`, `0xFF`,
  its intended value, its intended value XOR `0x5A`, or `0x54`. After every cut, `load` returns exactly what it
  returned before that save began, unless every write of the save completed except, at most, the last one, and
  that last one left `0x54`: then it returns the new settings. On a blank start, "before" is `None`.
  Every combination of start state, cut position and torn value is checked.

### C: controller (host, production sequencer, simulated 1 kHz tick)

- C1 Halt during `Stepping` at periods 1, 2 and 50, both directions: no half-step after the halt; outputs keep
  the last pattern until the 50th tick after the halt and are all-low from then; `Done` fires on the 150th tick
  with the position at the halt; the next move resumes from the retained phase.
- C2 Halt during `Energise`: no half-step is ever emitted; exactly one `Done`; position unchanged.
- C3 Halt in `Idle`, `Hold`, `Release` and `Settle` leaves the tick-by-tick outputs and the `Done` tick identical
  to a run without the halt.
- C4 `start_with_period(n, p)` for p = 1, 26, 53, 255 emits the half-steps `p` ticks apart (same timing rule as
  Phase 2 with `p` as the period). A later `start` uses the unchanged `SPEED` period. `start_with_period` with
  p = 0 → `RANGE`. It is `BUSY` while the controller is not `Idle`.

### D: debouncing, admission and button (host)

- D1 `Debouncer::new(20)`:
  - 19 closed samples register nothing;
  - the 20th consecutive closed sample after 20 open ones returns true exactly once;
  - holding closed for 1000 more samples returns nothing;
  - samples alternating closed/open for 1000 ticks register nothing;
  - after a release, a new press needs 20 open then 20 closed samples;
  - a fresh debouncer registers a press after 20 closed samples.
- D2 `Debouncer::new(2)`: `closed()` becomes true on the 2nd consecutive closed sample and false on the 2nd
  consecutive open one.
- D3 `button_action` follows the button table for every combination of the `Activity` flags.
- D4 `admit` follows the admission rules for every command and every combination of the `Activity` flags,
  including `BUSY` before `HOME`.

### H: homing (host)

- H1 `Homing::start(false, &DEFAULT)` → `Move { steps: -7282, period_ms: 4, halt_on_switch: true }` (80.0° at the
  default arm ratio).
- H2 `Homing::start(true, &DEFAULT)` → `Move { steps: 455, period_ms: 4, halt_on_switch: false }`; then
  `done(false, false)` → the search move of H1; `done(true, false)` after the lift → `Failed`.
- H3 The search's `done(_, true)` → `Homed(455)` with the defaults, and `Homed(round(ahome × aratio / 360000))`
  for other settings.
- H4 The search's `done(_, false)` → `Failed`.
- H5 After `stop()` in either phase, any `done(..)` → `Failed`.

### N: scan sequence (host)

- N1 The worked example: `Scan::new(&DEFAULT, {3, 36, 3000}, 2, 455)` yields `Arm(455), Turn, Arm(2276), Turn,
  Arm(2275), Turn`, then `None`; `turn_period_ms` 26; `photos` 115. `{1, 36, 3000}` yields `Arm(455), Turn`
  with `photos` 40.
- N2 `turn_period_ms` equals `round_half_away(photos × interval × 100, pratio_x100)` across ratios, photo counts and
  intervals. The result 0 and results above 255 → `Err(Range)` (`{1, 1, 100}`, `{1, 300, 60000}`).
- N3 Arm moves:
  - none when the arm is already at the ring's position, or for two equal consecutive heights;
  - a descending list gives negative arm moves;
  - each arm move equals the target position minus the previous position exactly.
- N4 Photo estimate per the formula, across speeds, ratios and arm start positions.
  - With the defaults, `{3, 96, 3000}` is accepted with `photos` 297 and `{3, 97, 3000}` → `Err(Range)`.
  - With `RINGS 5 65 5 65`, `{4, 70, 3000}` is accepted with `photos` 297 and `{4, 71, 3000}` → `Err(Range)`.
- N5 `Err(Range)` when rings exceed the ring count, and when one of the first `rings` heights is below the home
  angle or above 65°.

### F: firmware build and budgets (hub-verified)

- F1 In `firmware/`: `cargo build --release`, `cargo build --release --no-default-features --features uno`,
  `cargo clippy -- -D warnings` and `cargo fmt --check` succeed. In `turntable-core/`: `cargo test`,
  `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` succeed.
- F2 `avr-size -A` on the release ELF: `.text + .data ≤ 16384` and `.data + .bss ≤ 640`.
- F3 Cycle counts from the disassembly, written in `docs/specs/phase-3/log.md`:
  - the TC2 ISR's worst case is at most 800 cycles (50 µs);
  - every critical section in main and in the USART ISR stays at most 320 cycles (20 µs), as in Phase 2.
  - A reviewer checks the code for the EEPROM rule above.
- F4 Reply formats match the protocol tables (a reviewer checks the code; W-criteria check them on the board).

### W: hardware acceptance (on the bench, after the host suite passes; results in the log)

- W1 Phase 2 smoke test after flashing: `STATUS`, `STEP 512`, `STEP -512`, `NEXT` → `DONE 512`, `DONE 0`,
  `DONE 114`. Boot prints `DEFAULTS` on a fresh EEPROM, then `CFG 409600 3276800 50 3 36 3000 10 35 60`.
- W2 Arm gearbox: plug the arm motor into the platter's driver and run the Phase 2 calibration procedure (section
  3 of `docs/phase-2-acceptance.md`). Set `ARATIO` to the measured half-steps × 100 × 8, check it with `CFG`, and
  reconnect the arm motor to its own driver.
- W3 Arm channel: protractor disc on the arm motor.
  - `HOME`, then press the home switch by hand during the search → `ADONE` with the home position (455 at
    `ARATIO 3276800`).
  - `ARM 50` is 45° of arm, so with `ARATIO` = the W2 ratio × 8 the shaft turns exactly one revolution clockwise
    (pointer back at its start ±0.5°).
  - `ARM 5` returns it to its start ±0.5°.
- W4 Homing failures: `HOME` with the switch held closed throughout → `ERR HOME` after the lift; `HOME` with
  nobody pressing → `ERR HOME` after about 29 s; `STATUS` shows homed 0 after each.
- W5 EEPROM: `AHOME 6` and `RINGS 10 40` → `OK`; after a power cycle and after a `cargo run` reflash, `CFG`
  shows both; then `AHOME 5` and `RINGS 10 35 60` restore them (and `HOME` again).
- W6 Turn: after homing, `SCAN 1 36 3000` turns the platter exactly one revolution, back on the protractor's
  start mark ±0.5°. The Mac times the turn from its start (the arm's `ADONE`) to its `DONE`; repeated with
  `STATUS` sent every 200 ms, the two times agree within 0.10 s (no ticks lost with both motors in the ISR).
- W7 `STOP` during a turn: the platter stops within one half-step, `DONE` and `END 0` follow; a following `NEXT`
  lands back on the grid.
- W8 Button: a press with the arm not homed starts `HOME`; a press on the home switch completes it; a press
  starts the saved scan; a press mid-turn halts it and `END` follows.
- W9 Exit: phone on its holder with RealityScan's interval timer at 3 s (or auto-capture). Run `SCAN 1 36 3000`
  and leave the table alone. The scan runs unattended to `END 1`, the photos are sharp, and RealityScan builds a
  model from them.
- W10 The full default scan (`SCAN 3 36 3000`, estimate 115, about 111 real photos) also builds a model.

## Decisions

- 2026-10-26: the platter turns at a steady pace and the phone shoots on its own (interval timer or
  auto-capture); the table does not synchronise with the phone. This replaces version 1's fixed-period
  stop-and-go after its attack (a phone timer a few ms off put late photos into moves).
- 2026-10-26: `SCAN <rings> <photos per ring> <interval ms>`; `SCAN` alone repeats the saved scan.
- 2026-10-26: between rings the platter stops and the arm climbs at the `SPEED` period (fast, about 5.5°/s at
  `SPEED 2`); a photo or two per change may blur.
- 2026-10-26: no clock correction: a turn takes 0.7 % longer than asked.
- 2026-10-26: a push button mounted as a home switch on A1 gives the arm its reference (`HOME`). A tilt ball
  switch was rejected as too imprecise.
- 2026-10-26: a start/stop button on A0:
  - while anything moves, it stops;
  - when idle, it homes if needed and otherwise starts the saved scan.
- 2026-10-26: calibration values are built-in defaults that console commands override and the board keeps in
  EEPROM, in two copies with a generation counter.
- 2026-10-26 (attack round 2): a save clears the destination's magic byte first (only when it is valid) and writes
  it last, so any number of interrupted saves in a row leaves the old or the new settings. Checked by simulation
  over every cut position and torn value for up to three consecutive saves.
- 2026-10-26: `SCAN` refuses a scan whose photo estimate exceeds 300 (RealityScan's limit). After attack round 2,
  the estimate assumes a clock up to 1 % slow and the phone starting 10 s before the table.
- 2026-10-26: ring heights are a saved list (`RINGS`), and `SCAN <rings> ...` uses the first `<rings>` of them.
- 2026-10-26: configuring through the LCD1602 (which has an I2C backpack) is the next phase, built on these same
  settings; A4/A5 stay free for it.
- 2026-10-26 (proposed, for confirmation at the freeze):
  - arm reduction 8:1 provisional in the default `ARATIO`;
  - homing at 4 ms per half-step with an 80° search, and a 5° lift when the switch starts closed;
  - at most 4 rings in whole degrees;
  - `STOP` halts console moves too;
  - the arm motor calibrated on the platter's driver (no raw arm-step command);
  - spec location `docs/specs/phase-3/`.
