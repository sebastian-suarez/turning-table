# Plan: phase-3

The spec is `spec.md`; this file says how.

## Approach

Phase 2's split stays: everything that can be wrong is pure logic in `turntable-core/` and tested on the host;
`firmware/` is the glue (registers, ISRs, EEPROM, UART, formatting). The new core pieces are small and
independent: parser variants, the settings record codec, `Controller::halt`, a debouncer, the admission and
button tables, the homing state machine and the scan sequence. The firmware reuses the Phase 2 `Controller`,
`Planner` (one instance per motor, the arm's with `aratio`), `accept()` transaction and `round_half_away`.

The firmware's main loop gains a scan runner: it holds the `Scan` and, each time both motors are `Idle`, starts
its next move (an arm move at the `SPEED` period, or a platter `NEXT` at the scan's turn period through
`Controller::start_with_period`). Homing runs the same way from `Homing` steps. Nothing is timed against the
phone, so the blocking UART writes cannot spoil a scan.

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

- **Photos during motion.** The platter never stops while the phone shoots. At 36 photos × 3 s a turn takes about
  107 s: the rim of a 25 cm object moves about 7 mm/s, about one pixel of blur at 1/60 s. Good light (short
  exposures) matters; W9 and W10 check that real scans build. Arm climbs at 5.5°/s may blur a photo or two per
  ring change (the user's choice).
- **Auto-capture.** The user reports RealityScan's auto-capture in Object Mode; what triggers it is undocumented.
  The interval timer is the documented turntable route and needs nothing from the table.
- **Coil heat.** A turn keeps the platter's coils energised for 1-4 minutes (Phase 2 moves were seconds). A
  28BYJ-48 runs warm but within rating at 5 V; W10 runs three turns back to back.
- **ISR length.** Two controllers and two debouncers in TC2. The Phase 2 ISR body was 16 µs; the budget is 50 µs.
  F3 counts cycles; W6 checks for lost ticks on the board.
- **Flash.** Phase 2 uses 8178 B. Parser growth, the settings codec with CRC and two slots, homing and the scan
  must fit 16 KiB. If not, the first lever is avoiding new generic `i64` division helpers, as Phase 2 did.
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
