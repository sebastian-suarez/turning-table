# Phase 2 spec: stepper drive (v2, revised after the Codex plan attack of 2026-10-22)

Acceptance suite first; an implementation must pass it on the host before it is flashed. v1 → v2 changes: cumulative
closure targets, buffered RX with corruption rejection, one-move-in-flight policy, numeric bounds, a host-testable
motion controller, `DONE` defined as "released and settled", rate-independent gearbox calibration.

## Scope

Drive the 28BYJ-48 (5 V, unipolar) through the ULN2003 board from the Nano, with console commands to move by
half-steps or degrees and a stop planner, so the Phase 3 scan mode can be built on it. Out of scope: scan mode,
servo (`ARM`), EEPROM, homing sensor, host CLI.

## Hardware interface

| Signal | Nano pin | AVR |
|---|---|---|
| ULN2003 IN1..IN4 | D2..D5 | PD2..PD5 |
| activity LED | D13 | PB5 |
| console | D0/D1, 57600 8N1 | USART0 |

Motor and ULN2003 `+` from an external 5 V supply, GND common with the Nano.

## Architecture

- `turntable-core/` (`no_std`, no allocator, no `f32`, host-tested): `LineAssembler`, `Command` parser, `Planner`
  (ratio, degree conversion, cumulative stop targets), `Controller` (tick-driven motion state machine that emits
  coil patterns and events), `Sequencer` table. Everything that can be wrong lives here.
- `firmware/`: USART0 RX interrupt → ring buffer; TC2 1 kHz ISR → `Controller::tick()` → PORTD bits; main loop
  drains the ring buffer, feeds the parser, executes commands, prints replies and events. The ISRs never print.
  Shared state crosses the ISR boundary only through `avr_device::interrupt::Mutex<Cell<_>>` or `RefCell`, and
  no wide arithmetic (i64 division) happens inside an ISR or a critical section.

## Motion controller (`turntable-core::Controller`, driven by a 1 kHz tick)

- Half-step table on IN1..IN4: `1000 1100 0100 0110 0010 0011 0001 1001`; direction = index ±1 mod 8. The index
  persists across moves and across release, so re-energising resumes the same phase.
- Timer: TC2 CTC, prescaler 128 at 16 MHz → 125 kHz, `OCR2A = 124` → 1.000 kHz. One half-step every `period_ms`
  ticks (1..50, default 2 = 500 half-steps/s ≈ 7.4 rpm). No acceleration ramp in Phase 2; the speed range stays
  provisional until the loaded platter passes Phase 4 (lower speed or add a ramp then).
- Move lifecycle: `Idle → Energise (hold the current pattern 20 ms) → Stepping → Hold (50 ms) → Release (all
  outputs low) → Settle (100 ms) → Idle`, and the `Done` event fires on entering `Idle`. So **`DONE` means: motor
  de-energised and 100 ms of settling elapsed**, the moment Phase 3 may take the photo. A zero-length move goes
  `Idle → Idle` and still produces exactly one `Done`. Position is `i32` half-steps, updated per emitted step.
- One move in flight: `STEP`, `ROT`, `NEXT`, `ZERO`, `STOPS` while not `Idle` → `ERR BUSY`; `POS`, `STATUS`,
  `SPEED` are always accepted (`SPEED` applies from the next move). Exactly one `DONE` per accepted move, none
  for rejected commands.

## Planner (`turntable-core::Planner`)

- `ratio_x100: u32` half-steps per revolution in hundredths; provisional `407577` (63.68395:1), alternative
  `409600` (64:1); compile-time constant in Phase 2, runtime-configurable in Phase 3.
- `degrees_to_halfsteps(tenths: i32) -> Option<i32>`: `|tenths| ≤ 36000` (±10 turns), computed in `i64` as
  `round_half_away(tenths × ratio_x100 / 3600 / 100)`; `None` when out of range.
- Cumulative stop targets: with `N` stops (1..720) and global stop counter `k` since the planner origin,
  `target(k) = round(k × ratio_x100 / (N × 100))` in `i64`, and `NEXT` moves `target(k+1) − target(k)`. Hence
  after 10 revolutions the position is `round(10 × 4075.77) = 40758`, the first revolution totals 4076 and some
  later ones 4075. `ZERO` sets position = 0 and origin = 0, `k = 0`; `STOPS n` sets `k = 0` with origin = current
  position. Position is checked arithmetic: a move that would leave `|position| > 2^30` is rejected `ERR RANGE`.

## Console protocol

- Lines end in `\n` (`\r` ignored), at most 24 bytes, ASCII, case-insensitive verb, decimal arguments with
  optional sign; `ROT` accepts one decimal (`ROT -7.5`), everything else integers. Commands: `STEP <±n>`
  (|n| ≤ 1 000 000), `ROT <±deg>` (|deg| ≤ 3600), `STOPS <n>` (1..720), `NEXT`, `ZERO`, `POS`, `SPEED <ms>`
  (1..50), `STATUS`.
- Replies: `OK` on accept, `ERR <BUSY|RANGE|SYNTAX|LINE>` on reject, `DONE <position>` asynchronously when a move
  reaches `Idle`, `POS <position> <k>`; `STATUS` prints state, period, ratio, stops, k. No `HELP` (SRAM).
- Transport: USART0 RX interrupt fills a 32-byte ring buffer. The ISR records hardware errors (`DOR0`, `FE0`,
  `UPE0` in `UCSR0A`, read before `UDR0`) and ring overflow in a sticky flag. The `LineAssembler` drops any line
  that is longer than 24 bytes or that was received while the flag was set, discards through the next `\n`, clears
  the flag and replies `ERR LINE` once. Rationale: at 57600 a byte arrives every 174 µs and the blocking `ufmt`
  writer can hold the main loop longer than the 2-byte hardware FIFO survives; a corrupted line must never be
  executed as a different valid command.

## Acceptance suite (host `cargo test` in `turntable-core/`, frozen before the rival implementations start)

1. Parser: every command round-trips; rejects unknown verbs, missing/extra/oversized/non-numeric arguments,
   values just outside each range, lines of 25 bytes; accepts `rot -7.5`, `ROT 10`, `step +5`, trailing `\r`,
   `STEP -1000000`, `ROT 3600`, `ROT -3600`.
2. Degrees: for both ratios, 360° → 4076 / 4096, 90° → 1019 / 1024, 10° → 113 / 114, 3600° → 40758 / 40960,
   −x mirrors x, 0 → 0, rounding ties documented and tested (e.g. 0.05° × ratio), 3600.1° → `None`.
3. Planner: for N in {1, 2, 3, 5, 8, 24, 36, 72, 360, 720} and both ratios, 10 × N consecutive `NEXT` deltas sum
   to `round(10 × ratio)`, every delta is within one half-step of the ideal, the first revolution sums to
   `round(ratio)`; `ZERO` and `STOPS` reset `k` as specified; a double `NEXT` while busy is rejected and does not
   advance `k`.
4. Controller: using the production sequencer and a simulated 1 kHz tick, for periods 1, 2 and 50 and both
   directions: the emitted patterns follow the table in order with exactly one coil changing per step, the number
   of emitted steps equals the request, the final position is correct, energise/hold/release/settle durations
   match (20/50/100 ms), outputs are all-low after release, a new move accepted during `Settle` is rejected and a
   move accepted after `Idle` restarts from the retained phase, a zero-length move yields exactly one `Done`, and
   every accepted move yields exactly one `Done`.
5. Transport: the assembler drops a line marked corrupted and the next clean line executes; a 40-byte burst
   without newline yields one `ERR LINE`; interleaved `DONE` output does not change parsing (simulated).
6. Build: `nano` and `uno` features build with clippy clean; release `.text + .data` ≤ 8 KiB, `.data + .bss`
   ≤ 384 B (ring buffer included).

## Hardware acceptance (after the suite passes; the first real measurements)

1. Gearbox calibration, rate-independent: `ZERO`, then `STEP 40760` at `SPEED 10`, read the pointer; repeat at
   `SPEED 4` and `SPEED 2`, twice each, same direction. All six readings must agree within ±0.5°; if the faster
   runs read short, that is lost steps, not the gearbox: lower the default speed and repeat. The agreed reading
   decides the constant (10 turns of 4076 = 40760 steps: a 63.68:1 unit overshoots the mark by about 2°, a 64:1
   unit stops about 17.8° short). Record supply voltage and load.
2. Positioning: protractor disc, 150 mm pointer, same-direction approach: `ROT 90`, `ROT 10` within ±0.5°,
   measured **after** `DONE` (released); the pointer must not move during a 10 s dwell nor across 10 release/restart
   cycles.
3. Closure: `STOPS 36`, 360 × `NEXT` (10 turns) returns within ±0.5°; the index mark passes exactly 10 times.
4. Serial robustness: from the Mac, send 200 commands back to back through the port while moves complete; no
   command executes with a different meaning, every accepted move gets one `DONE`, corrupted lines get `ERR LINE`.

## Delivery

Per the routing rule for a spec'd feature with an acceptance suite: freeze this spec and the suite in a commit,
then one implementation by Claude and one by Codex, each in its own worktree branched from that commit; run the
suite and the build checks on both; flash and run the hardware acceptance on the survivor(s); keep the one that
passes, Codex's if both do. One retry each if neither passes, then stop and ask.
