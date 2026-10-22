# Phase 2 implementation verification

Verified on 2026-10-22 against the frozen v2 specification, starting from
`bfaea880c452b4004efdfed1b7c168514497eb0e`. Hardware acceptance remains for the
maintainer: no flashing, bootloader probing or serial-port access was performed.
No commit was created.

The requested scope is **v2 at bfaea88**. During implementation an external
process advanced HEAD through `437f446` to `de94c98`, updating `CLAUDE.md` and the
specification to v3. Those commits are preserved. This report and implementation
are assessed against the originally requested v2, retrieved from `bfaea88`, rather
than silently expanding scope to the newly committed v3.

## Results

Inside `turntable-core/`, plain `cargo test` passes **26 tests**, with **0 failed**,
and 0 doc tests. The crate uses stable host Rust, has no dependencies, and does
not inherit the firmware's AVR configuration. There is no root Cargo manifest or
workspace.

Both firmware release builds pass:

```sh
cd firmware
cargo build --release
avr-size -A target/avr-none/release/turntable-firmware.elf
cargo build --release --no-default-features --features uno
avr-size -A target/avr-none/release/turntable-firmware.elf
```

| Build | .text | .data | .bss | Flash (.text + .data) | Static SRAM (.data + .bss) |
|---|---:|---:|---:|---:|---:|
| Nano (default) | 7850 B | 192 B | 60 B | 8042 / 8192 B | 252 / 384 B |
| UNO | 7850 B | 192 B | 60 B | 8042 / 8192 B | 252 / 384 B |

Flash has 150 B of headroom within the Phase 2 budget. Static SRAM includes the
32-byte receive ring and its metadata, controller and completion mailbox. Stack
usage is additional: it includes the 24-byte line buffer, planner, formatter
buffers, call frames and interrupt saves; it is not part of `.data + .bss`.

These checks also pass, with no warnings:

```sh
# Inside firmware/, using its pinned nightly:
cargo clippy -- -D warnings
cargo clippy --no-default-features --features uno -- -D warnings
cargo fmt --check

# Inside turntable-core/, using stable:
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The nightly, `arduino-hal` git revision and AVR target configuration are unchanged.
This implementation does not edit the specification. Every existing external lockfile package/version/checksum is
preserved, including `encoding_rs = 0.8.35`. The existing `avr-device = 0.8.1`
package is now also declared directly so firmware can name its interrupt API;
the only added package is the local `turntable-core` crate. Linking uses the
installed avr-gcc 9.5.0 and the `atmega328p` CPU for both boards.

## Acceptance coverage

- Parser: all eight commands round-trip; signs, mixed case, CR, boundary values,
  missing/extra/unknown/non-numeric/oversized inputs and 25-byte lines.
- Degrees: both ratios, every specified angle and sign, out-of-range values,
  exhaustive sign symmetry across the allowed tenths, and exact rounding ties.
- Planner: all ten specified stop counts for ten revolutions with both ratios,
  per-delta error, cumulative targets, origin/counter resets, busy and range
  rejection without mutation, counter/product overflow, and manual-motion
  interleaving.
- Controller: production sequencer, both directions, periods 1/2/50, exact step
  count/position, one-coil transitions, 20/50/100 ms lifecycle timing, released
  outputs, retained phase, settle rejection, next-move SPEED, zero moves and
  exactly one position-bearing completion per accepted move.
- Transport: production 32-byte ring, FIFO wrap, sticky hardware/overflow error,
  corrupted and overlong lines, overflow on the delimiter itself, stale queued
  delimiters, damaged newline, corruption after complete-line dequeue, recovery
  to the following clean command, and simulated interleaved DONE output.
- Size helpers: decimal conversion against host formatting over boundaries and
  10,000 sampled u32 values; integer long division against an independent i128
  oracle over i64 endpoints and 20,000 sampled numerator/divisor pairs.

## Interpretations and differences from the frozen text

1. The initial stop count is unspecified. It starts at **36**, matching the
   closure exercise; `STOPS` sets any count in 1..720 and rebases the origin.
2. The transport paragraph places `ERR LINE` after the next newline, while the
   acceptance suite requires an error for a 40-byte burst **without** newline.
   The implementation follows that explicit acceptance case: report once as
   soon as damage/overflow is observed, then discard through a trustworthy LF.
   The sticky transport flag is cleared only when main consumes that recovery
   delimiter; the delimiter does not generate a duplicate error.
3. A receive fault conservatively purges queued, undispatched bytes, discards
   through a trustworthy newline, and inserts an ordered recovery marker before
   following clean bytes. This may also drop buffered clean commands, which have
   not been accepted. It prevents clearing a sticky fault at an older queued
   newline and executing a truncated command with a different meaning. A newline
   carrying a hardware error is not trusted as a recovery boundary.
4. The rounding example `0.05°` cannot be passed through an integer-tenths API,
   and neither production ratio makes that angle an exact half-step tie. Its
   rational value is tested directly; exact positive/negative ties use the same
   rounding helper and a synthetic 1800-step/revolution ratio. Input precision
   remains exactly one decimal place.

Energising begins at the first timer boundary after acceptance (less than one
tick later), then lasts a full 20 ms. From that boundary, the last step is at
`20 + |steps| × period` ms, release is 50 ms later, and DONE is another 100 ms
later. Release is a transition within the settle interval, not an extra tick.
Integer long division and the small decimal adapter preserve the specified
numeric results and ufmt protocol while meeting the flash budget.

## Review record

Two fresh-context implementation reviews covered all files listed below, the
requested v2 text, build configuration and verification logs. Both returned no
material findings against v2. Their arithmetic checks confirmed 1 kHz, the GPIO
sequence `4, 12, 8, 24, 16, 48, 32, 36`, the ten-turn totals and the size sums.

The configured second-vendor review could not run because its CLI reported
`Not logged in`. The updated project's named review helper was attempted, but
the filesystem sandbox denied its required cache-directory write. The available
fresh-context fallback reviews do not constitute that missing vendor review or
successful execution of the blocked helper.

An initial snapshot accidentally included the concurrently committed v3 text.
Both reviewers received a corrected snapshot containing v2 from `bfaea88`
before their final conclusions. A provisional missing lost-minus/multiple-queued-
lines acceptance-matrix finding was withdrawn: those explicit cases were added
in v3, while v2 section 5 requires corruption rejection, the 40-byte burst and
interleaved DONE. The implementation additionally tests the production ring's
lost-digit, stale-delimiter and overflow-delimiter paths.

The pre-implementation plan review's event-retention, timer-boundary,
transactional-counter and ordered-recovery concerns were addressed in code and
host tests. The rounding-example ambiguity is resolved explicitly above.
Reviewers did not independently run commands or test hardware. The actual AVR
dispatcher/PAC operations are verified by builds and source review; host tests
exercise the production pure logic. Interrupt latency, stack high-water use and
physical motion still require hardware measurement. A parent-side disassembly
inspection found no wide-arithmetic or printing-helper calls in either ISR.

## Files

| File | Change |
|---|---|
| `turntable-core/Cargo.toml` | Independent, dependency-free no_std library manifest. |
| `turntable-core/Cargo.lock` | Lockfile containing only this local crate. |
| `turntable-core/src/lib.rs` | Modules, public types and protocol errors; std only under cfg(test). |
| `turntable-core/src/line.rs` | Fixed 24-byte assembler, corruption rejection and newline recovery. |
| `turntable-core/src/command.rs` | Bounded ASCII command parser and signed integer/tenths conversion. |
| `turntable-core/src/planner.rs` | Hundredths ratio, i64 intermediates, cumulative targets and prepare/commit. |
| `turntable-core/src/controller.rs` | Tick-driven lifecycle, position limits, phase retention and Done outputs. |
| `turntable-core/src/sequencer.rs` | Frozen eight-phase half-step table and direction handling. |
| `turntable-core/src/rx.rs` | Production 32-byte ring and ordered sticky-error recovery. |
| `turntable-core/src/decimal.rs` | Division-free decimal conversion, used by the ufmt adapter. |
| `turntable-core/src/tests.rs` | Complete host acceptance cases and arithmetic/transport regressions. |
| `firmware/Cargo.toml` | Core path dependency and direct declaration of existing avr-device 0.8.1. |
| `firmware/Cargo.lock` | Local core package and direct dependency links; external pins unchanged. |
| `firmware/src/main.rs` | USART0/TC2 interrupts, protected shared state, GPIO mapping, dispatcher and main loop. |
| `firmware/src/console_format.rs` | Small ufmt numeric adapter and shared blocking transmitter. |
| `docs/phase-2-verification.md` | This report and explicit spec interpretations. |
