# Phase 2 spec: stepper drive (short plan, to be attacked before implementation)

Written 2026-10-22. Acceptance suite first; the implementation must pass it before the hardware test.

## Scope

Drive the 28BYJ-48 (5 V, unipolar) through the ULN2003 board from the Nano, with console commands to move by
half-steps or degrees, so the Phase 3 scan mode can be built on it. Out of scope: scan mode, servo, EEPROM,
homing sensor, host CLI.

## Hardware interface

| Signal | Nano pin | AVR |
|---|---|---|
| ULN2003 IN1..IN4 | D2..D5 | PD2..PD5 |
| activity LED | D13 | PB5 |
| console | D0/D1, 57600 8N1 | USART0 |

Motor and ULN2003 `+` from an external 5 V supply, GND common with the Nano.

## Motion engine (firmware, `firmware/src/stepper.rs`)

- 8-state half-step table on IN1..IN4: `1000 1100 0100 0110 0010 0011 0001 1001`; direction = index ±1 mod 8.
  The table index persists across moves and across coil release so re-energising never jumps a step.
- Timer TC2, CTC mode, prescaler 128 → 125 kHz, `OCR2A = 124` → 1 kHz tick. The ISR advances one half-step
  every `period_ms` ticks while `remaining > 0`, then decrements. Default `period_ms = 2` (500 half-steps/s,
  about 7 rpm); `SPEED` sets 1..50 ms. No acceleration ramp in Phase 2 (28BYJ-48 turntables run fine without one
  at 2 ms; add a ramp only if the loaded platter stalls or skips).
- Coil release: 50 ms after a move completes, all four outputs low (no holding torque, no heating; platter friction
  holds position). The `remaining` counter and `position` are shared with the main loop through
  `avr_device::interrupt::Mutex<Cell<_>>` sections; the ISR never prints.
- Position: `i32` half-steps from the last `ZERO`; motion math and parsing live in `turntable-core`.

## Motion math and parser (`turntable-core/`, `no_std`, no allocator, tested on the host)

- `HalfStepsPerRev`: fixed-point hundredths, provisional `407577` (63.68395:1); alternative `409600` (64:1).
  Compile-time constant in Phase 2, chosen after the Phase 2 ratio measurement; made runtime-configurable in Phase 3.
- `degrees_to_halfsteps(tenths_of_deg) -> i32` with round-half-away-from-zero.
- Stop planner: for `N` stops, target of stop `i` is `round(i × hs_per_rev / N)`; each `NEXT` moves the difference,
  so N stops sum exactly to `round(hs_per_rev)`. The fractional residual of a revolution (0.77 half-step for
  4075.77) carries in the fixed-point accumulator across revolutions.
- Parser: one ASCII line per command, `\n`-terminated (`\r` ignored), max 24 bytes, case-insensitive verb.
  Commands: `STEP <±n>`, `ROT <±deg[.d]>`, `STOPS <n>` (1..720), `NEXT`, `ZERO`, `POS`, `SPEED <ms>`, `STATUS`.
  Replies: `OK` on accept, `ERR <short reason>` on reject, `DONE <position>` when a move finishes (asynchronous,
  so Phase 3 and a host can wait for it). No `HELP` text (every literal costs SRAM).
- Console RX is polled in the main loop in Phase 2 (fine for typed commands; a host sending long lines fast may
  lose bytes until an RX interrupt with a ring buffer is added, planned for Phase 3).

## Acceptance suite (host `cargo test` in `turntable-core/`, must pass before flashing)

1. Parser: every command round-trips; rejects unknown verbs, missing/extra arguments, out-of-range values,
   lines over 24 bytes; accepts `rot -7.5`, `ROT 10`, trailing `\r`.
2. `degrees_to_halfsteps`: 360° → 4076 (ratio 407577) and 4096 (ratio 409600); 90° → 1019 / 1024; 10° → 113 / 114;
   negative mirrors positive; 0 → 0.
3. Stop planner closure: for N in {1, 2, 3, 5, 8, 24, 36, 72, 360, 720} and both ratios, the N deltas sum to
   `round(hs_per_rev)`, every delta is within one half-step of the ideal, and after 10 full revolutions the
   accumulated position is within one half-step of `10 × hs_per_rev`.
4. Sequencer: 8 forward then 8 backward indices return to the start; the table has exactly one or two coils on
   per state and adjacent states differ in exactly one coil.
5. Firmware builds for `nano` and `uno` features with clippy clean; release `.text + .data` ≤ 8 KiB, `.data + .bss`
   ≤ 256 B (headroom for Phase 3).

## Hardware acceptance (docs/PLAN.md Phase 2)

Protractor disc with a 150 mm pointer, same-direction approach: `ROT 90` and `ROT 10` within ±0.5°; 10 commanded
revolutions = 10 observed index passes, return within ±0.5°. First run decides the ratio constant (17.8° short
over 10 turns = the 64:1 unit).

## Delivery

Per the user's routing rule for a spec'd feature with an acceptance suite: one attempt by Claude and one by Codex,
each in its own git worktree, suite run on both, the passing one kept (Codex's if both pass). Requires an initial
commit first.
