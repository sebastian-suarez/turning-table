# Plan: phase-3

The spec is `spec.md`; this file says how.

## Approach

Phase 2's split stays: everything that can be wrong is pure logic in `turntable-core/` and tested on the host;
`firmware/` is the glue (registers, ISRs, EEPROM, UART, formatting). The new core pieces are small and
independent: parser variants, the settings record codec, `Controller::halt`, a debouncer, the admission and
button tables, the homing state machine and the scan schedule. The firmware reuses the Phase 2 `Controller`,
`Planner` (one instance per motor, the arm's with `aratio`), `accept()` transaction and `round_half_away`.

The firmware's main loop gains a scan runner: it holds the `Scan`, compares the ISR's millisecond counter with
the next move's due tick, and starts the platter `NEXT` and the arm hop in one critical section. Homing runs the
same way from `Homing` steps. Due ticks are absolute from `t0`, so the blocking UART writes delay a move by a
few milliseconds at most and never shift the schedule.

The acceptance suite goes into `turntable-core/tests/phase3.rs` (an integration test, so it can only use the
public interface the spec names). The suite task also amends the one Phase 2 test line that lists `ARM 10` as
an unknown command.

## Files

| Path | Route | Writer |
| --- | --- | --- |
| `turntable-core/tests/phase3.rs` | coding.tests | tests writer (protected after `spec.sh ready`) |
| `turntable-core/src/tests.rs` (drop `ARM 10` from the rejected list) | coding.tests | tests writer |
| `turntable-core/src/{command,controller,planner,lib}.rs`, new `settings.rs`, `scan.rs`, `homing.rs`, `input.rs` | coding.feature | implementer |
| `firmware/src/main.rs`, `firmware/src/console_format.rs` | coding.feature | implementer |
| `docs/PLAN.md`, `docs/board/B3.md`, `CLAUDE.md` (pins, protocol, budgets) | hub | after verification |

One writer owns all the implementation (core and firmware): it is one artifact with one interface between its
halves.

## Risks

- **The phone's real interval.** If RealityScan's timer runs a few ms off its setting, or waits for each capture
  to finish, 100+ periods add up to more than the 1 s margin. W9 measures it, and the period takes milliseconds,
  so the measured value can be typed in. Unknown until measured.
- **ISR length.** Two controllers, two debouncers and a tick counter in TC2. The Phase 2 ISR body was 16 µs; the
  budget is 50 µs. F3 counts cycles; W7 checks for lost ticks on the board.
- **Flash.** Phase 2 uses 8178 B. Parser growth, the scan schedule (`u64` maths through `round_half_away`) and the
  EEPROM code must fit 16 KiB. If not, the first lever is avoiding new generic `i64` division helpers, as Phase 2
  did.
- **EEPROM strobe.** The HAL's `write_byte` leaves interrupts enabled across the 4-cycle `EEMPE`→`EEPE` window;
  the 1 kHz tick would silently drop writes. The spec mandates the critical section and a read-back.
- **Homing on the bench.** The home switch is pressed by hand in Phase 3; real repeatability is a Phase 4 rig
  measurement.
- **Provisional constants.** The 8:1 reduction and the 5.0° home angle are placeholders until the Phase 4 CAD;
  both are `ARATIO`/`AHOME` settings, so no reflash is needed to change them.

## Verification

- `cd turntable-core && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- `cd firmware && cargo build --release && cargo build --release --no-default-features --features uno && cargo clippy -- -D warnings && cargo fmt --check`
- `avr-size -A firmware/target/avr-none/release/turntable-firmware.elf` against F2.
- `avr-objdump -d` cycle counts for F3, written in `log.md`.
- The W criteria on the bench with the user, results in `log.md` and on board card B3.
