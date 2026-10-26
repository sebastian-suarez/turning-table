# Spec: phase-3

Version: 1
Status: draft

## Problem

Phase 2 drives the platter from the console, one move per command. A photogrammetry scan needs K rings × N
stops, each stop held still while the phone's interval timer fires, with the camera arm raised between rings, and
it should run without the Mac. Two measured facts from the Phase 2 acceptance shape it: the board's clock runs
0.70 % slow, which would put a 3-ring scan about 2.5 s out of step with the phone, and the arm motor's gearbox
(63.68:1 or 64:1) is not measured yet. This is Phase 3 of `docs/PLAN.md` (board card B3).

## Scope

In:
- The arm stepper on its own ULN2003 as a second `Controller` instance in the same 1 kHz TC2 tick, with `ARM <deg>`.
- `HOME`: a push button used as a home switch sets the arm's reference angle.
- `SCAN <rings> <stops> <period>` on a fixed period corrected for the board's clock, with the arm climbing
  between rings in hops that fit between two phone shots; `STOP`.
- A start/stop push button for use without the Mac.
- Settings in EEPROM: calibration (gear ratios, clock trim, home angle) and scan settings (ring heights, last
  scan), with defaults built into the code and console commands that override them.
- The new logic in `turntable-core/`, tested on the host; the firmware glue in `firmware/`.

Out: the platter's Phase 2 motion behaviour (half-step table, move lifecycle and timings, cumulative stop grid, RX
loss handling), which stays as it is; the CAD; the display.

Non-goals: the LCD1602 menu (next phase; its I2C pins A4/A5 stay free); a host CLI; the home switch as a limit
during ordinary moves; acceleration ramps; a `DEFAULTS` command (type the values listed below instead); a
manual-advance scan mode; homing the platter; the SG90.

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
| `MIN_SCAN_PERIOD_MS`, `MAX_SCAN_PERIOD_MS` | 1000, 60000 | `SCAN` period range |
| `SHOT_MARGIN_MS` | 1000 | part of every period that no move may use (see Scan schedule) |
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
| Clock trim | `TRIM <ppm>` | how slow the board's clock runs, ppm, −20000..20000 | 6968 (82.663 s measured for 82091 ticks in the Phase 2 acceptance) |
| Home angle | `AHOME <deg>` | arm elevation at which the home switch closes, one decimal, 0.0..30.0 | 5.0 (provisional, from the Phase 4 CAD) |
| Ring heights | `RINGS <h1> [h2] [h3] [h4]` | 1..4 whole degrees, each 0..90 | 10 35 60 |
| Saved scan | `SCAN <r> <s> <p>` | rings 1..4, stops 1..720, period 1000..60000 ms | 3 36 3000 |

Arm angles are elevations: 0° is horizontal, positive is up. The arm position is an `i32` count of arm half-steps
from 0°, and an angle converts as `round_half_away(tenths × aratio_x100 / 360000)` (the Phase 2 rounding). With the
defaults the home angle 5.0° is position 455, and 10°, 35°, 60° are 910, 3186, 5461. Positive arm half-steps are
"up"; on the bench, by convention, up is clockwise seen from the shaft side (as the platter's `STEP +`). If the
assembled rig moves the wrong way, swapping the jumpers IN1↔IN4 and IN2↔IN3 reverses the motor with no code change.

### EEPROM record

25 bytes at EEPROM address 0, little-endian:

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | magic `0x54` |
| 1 | 1 | layout version `1` |
| 2 | 4 | platter ratio `u32` |
| 6 | 4 | arm ratio `u32` |
| 10 | 2 | clock trim `i16` |
| 12 | 2 | home angle, tenths, `i16` |
| 14 | 1 | ring count 1..4 |
| 15 | 4 | ring heights `u8`, unused entries 0 |
| 19 | 1 | saved scan rings |
| 20 | 2 | saved scan stops `u16` |
| 22 | 2 | saved scan period ms `u16` |
| 24 | 1 | checksum: sum of bytes 0..23 modulo 256 |

A record is valid when magic, version and checksum match and every field is in its range from the settings table
(unused ring entries must be 0). Cross-field rules (a ring below the home angle, saved rings above the ring
count) are not decode errors; `SCAN` checks them when it runs. At boot an invalid or blank record means the
built-in defaults; nothing is written at boot.

### Console protocol additions

Lines, case, the 24-byte limit, `ERR LINE` handling and every Phase 2 command keep their Phase 2 behaviour, except
`SPEED`, which now sets the half-step period of both motors and is refused while a scan or homing runs, and
`STATUS`, which gains the arm and scan fields at the end of its line.

| Command | Reply | Then |
|---|---|---|
| `ARM <deg>` (one decimal) | `OK` | `ADONE <arm position>` |
| `HOME` | `OK` | `ADONE <home position>` on success, `ERR HOME` on failure |
| `SCAN <rings> <stops> <period>` or `SCAN` | `OK` | `DONE`/`ADONE` per move, then `END <moves>` |
| `STOP` | `OK` | ends a scan or homing (see below); does not interrupt a console `STEP`, `ROT`, `NEXT` or `ARM` |
| `RINGS`, `PRATIO`, `ARATIO`, `TRIM`, `AHOME` | `OK` after the EEPROM write | |
| `CFG` | `CFG <pratio> <aratio> <trim> <ahome tenths> <scan rings> <scan stops> <scan period> <h1> [h2 h3 h4]` | |
| `STATUS` | `STATUS <state> <period> <pratio> <stops> <k> <arm state> <arm position> <homed 0/1> <scan>` | |

`<scan>` is `-` when no scan runs, otherwise the number of scan moves started. `RINGS` checks each height only
against 0..90; `SCAN` checks the heights it uses against the arm's travel, so a later `AHOME` cannot slip past. New errors: `ERR HOME` (arm not
homed, or homing failed) and `ERR SAVE` (EEPROM read-back mismatch). Boot prints the Phase 2 banner, then
`DEFAULTS` if the EEPROM record was invalid, then a `CFG` line.

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
4. A failure, or a `STOP` during step 1 or 2 (which halts the arm and fails once it is `Idle`), prints `ERR HOME`
   and leaves the arm not homed. The lift in step 1 prints nothing.

### Halt

`Controller::halt()` ends a move early: no half-step is emitted after it. The coils stay on the last pattern and
go all-low on the 50th tick after the halt; `Done`, carrying the unchanged position, fires on the 150th. A halt
during `Energise` emits no half-step at all. A halt in `Idle`, `Hold`, `Release` or `Settle` changes nothing.
Every accepted move still produces exactly one `Done`.

### Scan schedule

Inputs: the first `R` ring heights, `N` stops, period `T` ms, the trim, the current `SPEED` period `p`, the arm
position `a0`. `SCAN` first rebases the platter planner like `STOPS N` (origin = platter position, k = 0). A
platter scan move is a `NEXT`.

- Ticks for `x` real milliseconds: `ticks(x) = round_half_away(x × 10^6 / (10^6 + trim))`.
- Move budget `B = ticks(T − SHOT_MARGIN_MS)`. The platter move must fit:
  `MOVE_OVERHEAD_TICKS + ceil(pratio_x100 / (100 × N)) × p ≤ B`. Hop size `H = floor((B − MOVE_OVERHEAD_TICKS) / p)`
  arm half-steps, at least 1.
- Moves, in order: the arm hops from `a0` to ring 1's position (`ceil(|Δ| / H)` hops, none if `Δ = 0`); for each
  ring, `N − 1` platter moves; between ring r and r + 1, `max(1, ceil(|Δ| / H))` slots, the first of which also
  makes the platter move that closes ring r's revolution, and the arm moves in hops of `H` toward the next height
  with the remainder last. After the last ring the platter stays at its last stop. `M` is the number of moves.
- Move `j` (1..M) starts at tick `t0 + ticks((j − 1) × T)`, where `t0` is the tick at which the scan was accepted.
  So the first move starts at once. The platter and arm parts of one move start on the same tick. `END M` is
  printed at `t0 + ticks(M × T)`. Each due tick is computed from `j`, never by accumulating, so rounding cannot
  drift. A move whose due tick passes while a motor is still busy starts as soon as both are `Idle`; later due
  ticks do not shift.
- `STOP` during a scan: no further move starts, and `END <moves started>` is printed once both motors are `Idle`.
- Procedure this serves: set the phone's interval timer to `T`, start it, and start the scan right after a shot.
  The shot before the start shows the start pose, and each later shot falls after the move that started just
  before it. The margin leaves a full second for the press and the shutter. The phone should end with `M + 1`
  photos.

Worked example (defaults, `SPEED 2`, arm homed at 455, `SCAN 3 36 3000`): `B = 1986`, `H = 907`. Moves: one arm hop
+455; 35 platter moves; three slots (platter + arm +907, arm +907, arm +462); 35 platter moves; three slots
(platter + arm +907, arm +907, arm +461); 35 platter moves. So `M = 112`, 107 platter moves, the arm ends at 5461.
Due ticks start 0, 2979, 5958, 8938, and `END 112` comes at tick 333675. `SCAN 1 36 1300` is `ERR RANGE`
(`B = 298 < 171 + 114 × 2`). `SCAN 1 36 1500` is accepted (`B = 497`).

### Start/stop button

The press is registered after `BUTTON_DEBOUNCE_TICKS` closed samples that follow at least as many open ones;
holding the button produces no repeats. The debouncer starts as released.

| State | Press does |
|---|---|
| a scan or homing runs | `STOP` |
| a console move runs (`STEP`, `ROT`, `NEXT`, `ARM`) | nothing |
| idle, arm not homed | `HOME` |
| idle, arm homed | `SCAN` with the saved settings; a refusal prints its `ERR` line |

### Core interface (what the host suite tests)

New public items in `turntable-core`, re-exported from the crate root. The implementer may add items but not
change these without a spec change.

- `Command` gains `Arm(i16)` (tenths), `Home`, `Rings(Rings)`, `Scan(Option<ScanSettings>)`, `Stop`,
  `PRatio(u32)`, `ARatio(u32)`, `Trim(i16)`, `AHome(i16)` (tenths) and `Cfg`; `Error` gains `Home` and `Save`
  (`as_str` gives `HOME`, `SAVE`).
- `Rings { heights: [u8; 4], len: u8 }`, `ScanSettings { rings: u8, stops: u16, period_ms: u16 }`,
  `Settings { pratio_x100: u32, aratio_x100: u32, trim_ppm: i16, ahome_tenths: i16, rings: Rings, scan: ScanSettings }`
  with public fields, `Settings::DEFAULT`, `SETTINGS_BYTES = 25`, `Settings::encode(&self) -> [u8; 25]` and
  `Settings::decode(&[u8; 25]) -> Option<Settings>`.
- `Controller::halt(&mut self)`.
- `Debouncer::new(ticks: u8)`, `Debouncer::sample(&mut self, closed: bool) -> bool` (true on the sample that
  registers a press), `Debouncer::closed(&self) -> bool` (the debounced level).
- `Activity { platter_idle: bool, arm_idle: bool, scanning: bool, homing: bool, homed: bool }`,
  `admit(&Command, Activity) -> Result<(), Error>`, `button_action(Activity) -> ButtonAction` with
  `ButtonAction::{Stop, Ignore, Home, Scan}`.
- `Homing::start(switch_closed: bool, settings: &Settings) -> (Homing, HomingStep)`,
  `Homing::done(&mut self, switch_closed: bool, halted_by_switch: bool) -> HomingStep`, `Homing::stop(&mut self)`,
  with `HomingStep::{Move { steps: i32, period_ms: u8, halt_on_switch: bool }, Homed(i32), Failed}`.
- `Scan::new(settings: &Settings, run: ScanSettings, speed_ms: u8, arm_position: i32) -> Result<Scan, Error>`,
  `Scan::next(&mut self) -> Option<ScanMove>`, `Scan::end_tick(&self) -> u32`, with
  `ScanMove { due_tick: u32, platter: bool, arm_steps: i32 }` (`due_tick` relative to `t0`).

### Firmware rules

- The Phase 2 concurrency rules hold for both motors. PORTB and PORTD (coils and LED) are written only by the
  TC2 ISR (or before interrupts are enabled). Shared state crosses the ISR boundary only through
  `interrupt::Mutex`. No `i64` arithmetic happens in an ISR or a critical section. Each motor has its own `DONE`
  mailbox.
- The TC2 ISR ticks both controllers, samples the home switch and the button, counts a `u32` millisecond tick
  (wrapping arithmetic) and halts the arm on a debounced closure while a homing search is armed.
- EEPROM writes run in main, write only bytes that changed and are read back. Before each byte, wait with
  interrupts enabled until `EEPE` clears. Then mask interrupts only around the `EEMPE`→`EEPE` strobe: an
  interrupt between the two lets the 4-cycle window lapse and the write is silently dropped. The HAL's
  `write_byte` does not mask interrupts itself. A read-back mismatch replies `ERR SAVE` and keeps the new value
  in RAM. Settings commands are refused during scans and moves, so EEPROM writes never delay a scan move.
- Strings stay terse (they live in SRAM); `ufmt` only.

## Acceptance criteria

### P: parser (host)

- P1 `ARM 10` → `Arm(100)`, `arm 47.5` → `Arm(475)`, `ARM 0` and `ARM 90` accepted; `ARM 90.1`, `ARM -0.1` →
  `RANGE`; `ARM`, `ARM 1.25`, `ARM 1 2` → `SYNTAX`.
- P2 `HOME`, `STOP`, `CFG`, `SCAN` parse with no argument; `HOME 1`, `STOP 1`, `CFG 1` → `SYNTAX`.
- P3 `RINGS 10` → heights `[10]`; `RINGS 10 35 60` and `RINGS 60 35 10 5` keep their order; `RINGS` and five
  heights → `SYNTAX`; `RINGS 10.5` → `SYNTAX`; `RINGS 91` → `RANGE`.
- P4 `SCAN 3 36 3000` → `Scan(Some(..))`; `SCAN 3 36` and `SCAN 1 2 3 4` → `SYNTAX`; `SCAN 0 36 3000`,
  `SCAN 5 36 3000`, `SCAN 1 0 3000`, `SCAN 1 721 3000`, `SCAN 1 36 999`, `SCAN 1 36 60001` → `RANGE`.
- P5 `PRATIO 100000` and `1000000` accepted, `99999` and `1000001` → `RANGE`; `ARATIO 100000` and `20000000`
  accepted, `99999` and `20000001` → `RANGE`; `TRIM -20000`, `TRIM 0`, `TRIM 20000` accepted, `TRIM 20001` →
  `RANGE`; `AHOME 0`, `AHOME 5.5`, `AHOME 30` accepted, `AHOME 30.1` and `AHOME -0.1` → `RANGE`.
- P6 The Phase 2 suite (`turntable-core/src/tests.rs`) passes. Its single change is that `ARM 10` leaves the
  list of rejected lines, since it is now a valid command.

### S: settings and EEPROM record (host)

- S1 `Settings::DEFAULT` holds exactly the defaults in the settings table.
- S2 `decode(encode(s)) == Some(s)` for the defaults and for settings with every field at its minimum and at its
  maximum.
- S3 `decode` of 25 bytes of `0xFF` (erased EEPROM) and of 25 bytes of `0x00` → `None`.
- S4 For every byte position of a valid encoding, replacing that byte with `byte ^ 0x01` and with `byte ^ 0xFF`
  makes `decode` return `None`.
- S5 Bytes laid out as in the record table with a correct checksum but one field out of range (each field once,
  including a nonzero unused ring entry and a wrong version) → `None`.

### C: controller halt (host, production sequencer, simulated 1 kHz tick)

- C1 Halt during `Stepping` at periods 1, 2 and 50, both directions: no half-step after the halt; outputs keep
  the last pattern until the 50th tick after the halt and are all-low from then; `Done` fires on the 150th tick
  with the position at the halt; the next move resumes from the retained phase.
- C2 Halt during `Energise`: no half-step is ever emitted; exactly one `Done`; position unchanged.
- C3 Halt in `Idle`, `Hold`, `Release` and `Settle` leaves the tick-by-tick outputs and the `Done` tick identical
  to a run without the halt.

### D: debouncing and button actions (host)

- D1 `Debouncer::new(20)`: 19 closed samples register nothing; the 20th consecutive closed sample after 20 open
  ones returns true exactly once; holding closed for 1000 more samples returns nothing; samples alternating
  closed/open for 1000 ticks register nothing; after a release a new press needs 20 open then 20 closed samples;
  a fresh debouncer registers a press after 20 closed samples.
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

### N: scan schedule (host)

- N1 With the defaults and speed 2, `Scan::new(.., ScanSettings{3, 36, 3000}, 2, 455)` yields exactly the worked
  example: 112 moves, the listed arm hops in order, `platter` true on 107 moves, due ticks 0, 2979, 5958, 8938
  for the first four, `end_tick` 333675.
- N2 For every `j`, `due_tick(j) = round_half_away((j − 1) × T × 10^6, 10^6 + trim)`, and
  `end_tick = round_half_away(M × T × 10^6, 10^6 + trim)`. Checked for trims −20000, 0, 6968, 20000 and periods
  1000..60000 on scans up to `SCAN 4 720 60000`. With trim 0 the due ticks are exactly `(j − 1) × T`.
- N3 Every move fits: `171 + max(platter half-steps, |arm_steps|) × p ≤ B`, where the platter half-steps of a move
  come from the Phase 2 planner's cumulative targets.
- N4 Hops: each `|arm_steps| ≤ H`, signed toward the target; the hops of each transition sum to the height
  difference exactly; counts follow the schedule; descending ring lists hop downwards.
- N5 Edge cases: an arm already at ring 1 makes no positioning hop; `R = 1`, `N = 1` with the arm at the ring
  gives `M = 0` and `end_tick 0`; two equal consecutive heights give a one-slot transition with `arm_steps 0`.
- N6 `Scan::new` → `Err(Range)` when rings exceed the ring count, when one of the first `R` heights is below the
  home angle or above 65°, when the platter move does not fit (`SCAN 1 36 1300` at speed 2 with the defaults)
  and when `H < 1`. `SCAN 1 36 1500` is accepted.

### F: firmware build and budgets (hub-verified)

- F1 In `firmware/`: `cargo build --release`, `cargo build --release --no-default-features --features uno`,
  `cargo clippy -- -D warnings` and `cargo fmt --check` succeed; in `turntable-core/`: `cargo test`,
  `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` succeed.
- F2 `avr-size -A` on the release ELF: `.text + .data ≤ 16384` and `.data + .bss ≤ 640`.
- F3 Cycle counts from the disassembly, written in `docs/specs/phase-3/log.md`: the TC2 ISR's worst case is at most
  800 cycles (50 µs); every critical section in main and in the USART ISR stays at most 320 cycles (20 µs) as in
  Phase 2. The EEPROM rule above holds (a reviewer checks the code).
- F4 Reply formats match the protocol tables (a reviewer checks the code; W-criteria check them on the board).

### W: hardware acceptance (on the bench, after the host suite passes; results in the log)

- W1 Phase 2 smoke test after flashing: `STATUS`, `STEP 512`, `STEP -512`, `NEXT` → `DONE 512`, `DONE 0`,
  `DONE 114`. Boot prints `DEFAULTS` on a fresh EEPROM, then `CFG 409600 3276800 6968 50 3 36 3000 10 35 60`.
- W2 Arm gearbox: plug the arm motor into the platter's driver and run the Phase 2 calibration procedure (section
  3 of `docs/phase-2-acceptance.md`). Set `ARATIO` to the measured half-steps × 100 × 8, check it with `CFG`, and
  reconnect the arm motor to its own driver.
- W3 Arm channel: protractor disc on the arm motor. `HOME`, then press the home switch by hand during the search
  → `ADONE` with the home position (455 at `ARATIO 3276800`). `ARM 50` is 45° of arm, so with `ARATIO` = the
  W2 ratio × 8 the shaft turns exactly one revolution clockwise (pointer back at its start ±0.5°); `ARM 5`
  returns it to its start ±0.5°.
- W4 Homing failures: `HOME` with the switch held closed throughout → `ERR HOME` after the lift; `HOME` with
  nobody pressing → `ERR HOME` after about 29 s; `STATUS` shows homed 0 after each.
- W5 EEPROM: `TRIM 7000` and `RINGS 10 40` → `OK`; after a power cycle and after a `cargo run` reflash, `CFG`
  shows both; then `TRIM 6968` and `RINGS 10 35 60` restore them.
- W6 Clock correction: after homing, `SCAN 1 36 3000` timed by the Mac from `OK` to `END 36` takes
  108.00 s ± 0.10 s. With `TRIM 0` the same scan takes about 108.75 s. Restore `TRIM 6968`.
- W7 Timing under traffic: W6 repeated with `STATUS` sent every 200 ms gives the same time within 0.10 s.
- W8 Button: a press with the arm not homed starts `HOME`; a press on the home switch completes it; a press
  starts the saved scan; a press mid-scan prints `END` after the current move.
- W9 Phone interval: with RealityScan's interval timer, time 100 shots with a stopwatch. If the real interval
  differs from the set one by more than 5 ms, use the measured interval as the scan period.
- W10 Exit: start RealityScan's interval timer, start `SCAN 1 36 <period>` right after a shot, and leave the
  table alone. The scan runs unattended to `END 36`, the phone takes 37 photos, none while the LED is on, and
  RealityScan builds a model from them.

## Decisions

- 2026-10-26: the third `SCAN` number is a fixed period, not a pause after each move; the phone's interval timer
  takes the same value, and the period is corrected for the board's clock.
- 2026-10-26: between rings the arm climbs in hops that fit between two shots, so every photo is sharp and the scan
  stays unattended (extra photos at in-between heights).
- 2026-10-26: a push button mounted as a home switch on A1 gives the arm its reference (`HOME`); a tilt ball
  switch was rejected as too imprecise.
- 2026-10-26: a start/stop button on A0: when idle it homes if needed, else it starts the saved scan; during a
  scan or homing it stops.
- 2026-10-26: calibration values are built-in defaults that console commands override and the board keeps in
  EEPROM; the clock correction is one of them (default from the Phase 2 measurement).
- 2026-10-26: ring heights are a saved list (`RINGS`), and `SCAN <rings> <stops> <period>` uses the first
  `<rings>` of them; `SCAN` alone repeats the saved scan.
- 2026-10-26: configuring through the LCD1602 (which has an I2C backpack) is the next phase, built on these same
  settings; A4/A5 stay free for it.
- 2026-10-26 (proposed, for confirmation at the freeze): arm reduction 8:1 provisional in the default `ARATIO`;
  the 1000 ms shot margin; homing at 4 ms per half-step with an 80° search; a 5° lift when the switch starts
  closed; at most 4 rings in whole degrees; `END <moves>` and per-move `DONE`/`ADONE` during scans; arm motor
  calibration done on the platter's driver (no raw arm-step command); spec location `docs/specs/phase-3/`.
