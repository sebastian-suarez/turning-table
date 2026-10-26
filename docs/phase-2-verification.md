# Phase 2 implementation verification

DOR/FE/UPE correction verified on 2026-10-22 against `docs/phase-2-spec.md`
**v3.2** at `54aaa40`, including the v3.1 atomic-acceptance requirement at
`c0e2c38`. This working tree starts at HEAD `774e46b`, with the preceding
firmware fix pass at `b876942`. The full suite also rechecks that pass's
atomic acceptance, absolute NEXT targeting and ISR-owned LED behaviour.

All requested host/build checks pass. Hardware acceptance and independent
verification/reviews remain with the maintainer. This pass ran no reviews or
review scripts, created no commit, and performed no flashing, bootloader probing
or serial-device access.

## Behavioural changes

1. Every hardware-error or overflow flush increments a wrapping **u32 RX loss
   generation**. Each dequeued byte carries its generation through assembly,
   parsing and preparation. `RxBuffer::accept_if_current` compares it while
   firmware holds the same interrupt mutex that applies the controller command
   and commits the planner. A loss during blocking DONE output, parsing or
   fixed-point preparation therefore invalidates the pending command, including
   ZERO/STOPS/SPEED and POS/STATUS replies. A stale command produces no reply or
   motion; the loss notice supplies its ERR LINE. Already accepted moves and
   their position-bearing DONE events remain valid.
2. Flushes have separate notices, including two faults before main drains either
   notice, two faults without an accepted clean line between, and a second fault
   before recovery LF. Each produces exactly one ERR LINE. Consuming recovery LF
   does not duplicate it. A notice immediately clears the assembler's partial
   line; input is discarded through a trustworthy post-loss LF, then the first
   following clean line can execute. Overlong lines still produce one length
   error and discard through LF.
3. Controller application, planner commit and reply snapshots are bounded state
   operations inside a short critical section. Parsing, i64 planning, formatting
   and every UART write run with interrupts enabled. POS/STATUS print the copied
   motion snapshot and main-only planner values after releasing the mutex.
4. TC2 now drives activity LED PB5 next to the coil write. Main configures D13
   before enabling interrupts and never writes PORTB afterwards. The LED follows
   the timer's active state, including energise/hold/settle, at 1 ms resolution.
   This avoids reverting future PB0/PB2 arm-coil writes with a main-loop PORTB
   read-modify-write. All pin assignments are unchanged.
5. NEXT now moves by `origin + target(k+1) - position`, as required by v3.2.
   Manual STEP/ROT no longer shifts the stop grid. ZERO still resets position,
   origin and k; STOPS still rebases the origin at the current position and
   resets k. A correction can be backwards or zero; a zero-distance NEXT still
   advances k and produces one immediate DONE after OK.
6. Parser and dispatcher use separate function frames to reduce AVR register
   spills and flash use. Replies use the prepared-command tag, avoiding retention
   of the original parser result across planning. These are size/layout changes
   with no extra protocol behaviour.
7. USART RX now passes DOR0 separately from FE0/UPE0 to the production ring.
   DOR-only means a gap **before an intact byte**: flush the old queue, increment
   the loss generation once, then deliver that byte normally after its loss
   notice. If it is LF, it immediately ends discard, so a lost `0` in
   `STEP 10\n` cannot swallow the immediately following `STEP 3\n`. A mid-line
   DOR-only byte reaches the assembler in discard mode; only its subsequent LF
   recovers, and popping that byte does not clear the sticky loss early.
   FE0/UPE0, with or without DOR0, corrupt the byte itself: drop it, even if it
   reads as LF, and discard through the next intact LF. One UDR0 read with any
   combination of these flags produces **one ERR LINE**, with no additional
   framing/parity reply: LINE is the spec's transport-error response. Recovery
   LF produces no duplicate. The generation acceptance guard is unchanged.
   Ordinary damaged-line tails are still filtered in the ISR to avoid filling
   the ring during discard. Combining the three side-effect-free flush flags
   with boolean bitwise OR avoids a duplicated AVR flush path and keeps flash
   within 8 KiB.

## Verification results

Inside `turntable-core/`, plain `cargo test` passes **37 tests**, **0 failed**,
and 0 doc tests. The crate remains dependency-free, no_std and independent of the
firmware's target and nightly. These commands also pass:

```sh
# Inside turntable-core/, stable host Rust:
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Inside firmware/, pinned nightly:
cargo build --release
avr-size -A target/avr-none/release/turntable-firmware.elf
cargo clippy -- -D warnings
cargo fmt --check
cargo build --release --no-default-features --features uno
avr-size -A target/avr-none/release/turntable-firmware.elf
cargo clippy --no-default-features --features uno -- -D warnings
```

Both firmware feature sets are clippy-clean with warnings denied. Each ELF was
saved immediately after its build, before the next feature set replaced the
shared output path.

| Release build | .text | .data | .bss | Flash (.text + .data) | Static SRAM (.data + .bss) |
|---|---:|---:|---:|---:|---:|
| Nano (default) | 7942 B | 192 B | 67 B | 8134 / 8192 B | 259 / 384 B |
| UNO | 7942 B | 192 B | 67 B | 8134 / 8192 B | 259 / 384 B |

Flash headroom is **58 B** and static-SRAM headroom is **125 B** within the
Phase 2 budgets. Compared with `b876942`'s 8122 B flash and 259 B static SRAM,
this correction adds **12 B flash and 0 B SRAM**.
additional to static SRAM: assembler, planner, parser/formatter temporaries,
call frames and interrupt saves require hardware high-water measurement.

No dependency, manifest, lockfile or toolchain/configuration file changed:
`nightly-2025-04-27`, `arduino-hal` rev
`e5c8f37fe48419956e722490a82b9ca9b9fc61a2`, and
`encoding_rs = 0.8.35` are preserved. No root Cargo manifest/workspace or new
dependency was introduced. Both boards still target the ATmega328P subset.

## Interrupt-latency budget

Each explicit `interrupt::free` has a comment listing its worst-case contents
and release bound. The bounds below come from the built ELF's
`avr-objdump -d -C` control-flow paths, using the AVRe instruction timings in
Microchip's [AVR Instruction Set Manual](https://onlinedocs.microchip.com/oxy/GUID-0B644D8F-67E7-49E6-82C9-1B2B9ABE6A0D-en-US-23/GUID-BA59618D-4850-490B-B176-0BCC3D9438A1.html).
They conservatively select the longer branch paths and count from CLI through
the matching SREG restoration, excluding panic/halt paths that require violated
queue/borrow invariants. The relevant machine code is identical on both board
builds; their startup UART configuration differs outside these sections.

At 16 MHz, one cycle is 0.0625 us. The target is 20 us (320 cycles), comfortably
below the nominal 57600 8N1 byte interval `10 / 57600 = 173.611 us`.

| Critical section | Worst-case contents | CLI → SREG restore (ELF byte addresses) | Cycle bound | Time bound |
|---|---|---|---:|---:|
| Dispatch snapshot | Copy state, i32 position and period (6 bytes), borrow check | `0x36e` → `0x396` | 21 | 1.3125 us |
| Dispatch acceptance | Detach DONE; compare u32 RX generation; bounded i32 controller checks/updates; copy planner k/origin/stops; copy motion snapshot | `0x3d6` → `0x8ec` | 293 | 18.3125 us |
| Main dequeue | Detach DONE; pop one byte+generation or one loss notice; update queue metadata; clear sticky loss only on LF | `0xf50` / `0x10f0` → `0x1028` | 97 / 91 | 6.0625 / 5.6875 us |
| USART RX | Flush indices/increment generation if needed; discard corrupt frame or enqueue intact DOR-only byte/recovery LF | `0xb50` → `0xbf6` | 83 | 5.1875 us |
| TC2 tick | Small countdowns, one i32 step, table lookup, DONE mailbox, activity flag | `0xc4e` → `0xdda` | 135 | 8.4375 us |

No normal critical-section path contains a loop, formatting call, UART write,
wide multiply or division. Full ISR bodies, including compiler register saves,
GPIO writes and RETI, have conservative bounds of **155 cycles / 9.6875 us** for
USART RX and **257 cycles / 16.0625 us** for TC2; hardware interrupt entry and the
vector jump are additional. Compiler-generated stack-pointer masking sequences
are 3 cycles / 0.1875 us; the peripheral-take startup section is 10 cycles /
0.625 us and runs before global interrupts are enabled.

These are static bounds for these release binaries, not measured timing under
traffic. The maintainer still needs hardware acceptance 5 (idle and STATUS-loaded
ROT 3600 timing) to check the physical clock, timer and serial interaction.

## Acceptance coverage

The previous parser, degree conversion, ten-revolution planner, sequencer,
controller lifecycle, decimal and integer-division checks remain. The new/updated
checks use the production RxBuffer, LineAssembler, Command::parse, generation
guard, Planner and Controller, with the simulated ISR paused at explicit points:

| Case | Required outcome checked |
|---|---|
| Completing LF dequeued, RX loss injected while printing previous DONE | Stale NEXT never moves or advances k; old DONE stays intact; one ERR LINE; next clean STEP executes |
| Loss after parse/preparation, before acceptance, for all eight commands | No controller/planner mutation or stale acceptance/reply; one ERR LINE; next clean STEP executes |
| Lost `-` of STEP -5, lost digit of STEP 10, lost LF after STEP 1, with three complete commands queued ahead | All queued/pre-loss/altered commands rejected; lost LF detected by DOR on the next line's first byte; one ERR LINE; recovery executes |
| Lost `0` of `STEP 10\n`, DOR-only on its LF, immediately followed by `STEP 3\n`; prefix queued or already assembled | Exactly one ERR LINE; no STEP 10/altered STEP 1; STEP 3 executes to position 3, with no extra recovery newline; existing queued-command matrix also removes that extra newline |
| Same lost digit, but FE/UPE on LF, with and without DOR; then `STEP 3\nSTEP 4\n` | Corrupt LF is dropped; STEP 3 discarded through its LF; STEP 4 executes to position 4; exactly one ERR LINE, no separate framing/parity reply |
| DOR-only on mid-line `5` after a lost `-` in `STEP -50\n` | Ring delivers normal `5` after one loss notice; assembler discards it and the tail through LF; sticky loss survives popping `5`; next STEP 3 executes |
| POS consumed and STEP prefix already assembled before lost `-` | Earlier accepted POS retained; STEP 5 cannot execute; one ERR LINE; recovery executes |
| STEP 1 prefix consumed; LF lost; next line's first byte carries DOR | STEP 1 cannot execute; damaged following line dropped through LF; recovery executes |
| Two faults, no accepted clean line between; notices drained or still pending | Two ERR LINE notices; no stale motion; recovery executes |
| Second fault before recovery LF; first notice drained or still pending | Two ERR LINE notices even while assembler is discarding; recovery executes |
| u32 loss-generation wrap with two pending notices | Stale generation rejected; both notices retained across wrap; recovery LF adds no duplicate |
| Manual STEP 5 between NEXTs, both ratios and nine multi-stop counts, nonzero origin | Following NEXT returns to absolute target; revolution closes at origin + round(ratio); immediate zero-distance completion handled |

Existing transport checks also cover 32-byte FIFO wrap, overflow without LF,
overflow on LF itself, stale queued delimiters, FE/UPE-damaged LF, overlong
lines and interleaved clean DONE output. The former test asserting that a fully
dequeued line survives a later RX loss has been replaced with the opposite v3.1
regressions above.

## Remaining interpretations and limits

- ERR LINE is still emitted as soon as loss/length damage is observed, before
  recovery LF if necessary. This differs from reading the transport paragraph
  as requiring a deferred reply, but satisfies acceptance 5's explicit 40-byte
  burst **without newline** case. Recovery LF itself emits no duplicate error.
- Absolute NEXT deliberately supersedes v3.1's relative-delta wording, following
  the accepted v3.2 requirement. Hardware acceptance 4 likewise uses **200
  lockstep commands**, each sent after the preceding reply. A free-running host
  can overflow the 32-byte ring while replies block main and is expected to
  produce ERR LINE bursts by design; it is not the lockstep acceptance test.
- The existing signed-i32 move interface retains an extreme v3.2 boundary:
  a NEXT correction from exactly `-2^30` to `+2^30` needs `+2^31` half-steps and
  returns ERR RANGE before changing k, although both endpoints are in bounds.
  This was confirmed separately with origin `2^30 - 113`, STOPS 36 and the
  provisional ratio. Expanding the controller's signed move representation is
  outside the listed fix scope; ordinary grid corrections and all specified
  acceptance cases pass.
- The initial stop count remains 36 because the spec leaves it unspecified.
  The illustrative 0.05-degree rounding example is below the public API's
  integer-tenths precision; its rational value and actual positive/negative
  ties are tested with the production rounding helper.
- Generation equality/notice counting assumes fewer than `2^32` unreported
  flushes or flushes during one byte-to-acceptance interval. Normal wrap is
  tested; a full undrained counter lap is a finite-counter limit. At the UART's
  byte rate, that requires more than eight days of continuous faults.
- No hardware acceptance was run. Gearbox calibration, released-position
  accuracy, loaded motion, physical closure, lockstep serial traffic, measured
  timing under traffic and stack high-water use remain for the maintainer.

## Backlog from the round-2 reviews (not blocking hardware acceptance)

- `sticky_error` / `has_error()` in `turntable-core/src/rx.rs` have no production reader: gate them behind
  `#[cfg(test)]` and drop the branch from `pop()` (saves one `.bss` byte and ISR instructions).
- The NEXT absorption test covers stops 2..720; add N = 1 over two revolutions.
- The host tests re-implement the firmware's acceptance sequence (`RxConsole::process`); move the pure
  acceptance step into `turntable-core` and call it from both `dispatch` and the tests to remove drift risk.
- Phase 2 leaves 58 B under its 8 KiB guideline; the arm's second controller instance needs the Phase 3 budget
  (16 KiB proposed, hardware limit 30720 B).

## Hardware acceptance log

Readings from the walkthrough in `docs/phase-2-acceptance.md`, in the order they were taken.

| Date | Section | Result |
|---|---|---|
| 2026-10-26 | 0, setup | Printed disc, pointer and coupon fit the motor at the default `flat_fit = 3.0`; Nano on the breadboard with IN1..IN4 on D2..D5, driver and motor powered from the 18650 shield's 5 V (HOLD), common ground on the blue rail |
| 2026-10-26 | 1, flash and console | `cargo run` flashed the Phase 2 firmware; `STATUS` answered; `STEP 512` and `STEP -512` moved the shaft out and back. Positive steps turn the shaft **clockwise** seen from the shaft side, the same direction in which the disc's numbers increase |
| 2026-10-26 | 3, gearbox calibration | Ten commanded revolutions (`STEP 40760`) in one console session, start read at 0°: ends at 343°, 350°, 343°, 343°, 343°, 343° for `SPEED` 10, 4, 2, 10, 4, 2. Five of six are 17° short, the same at every speed, so this unit is the exact **64:1** gearbox (prediction 17.58° short; readings to the nearest degree). The 350° outlier at `SPEED 4` was not repeated in the second `SPEED 4` run, and steps lost at speed can only shorten a move, so it is a reading or start-alignment slip, not the motor. `RATIO_X100` set to `409_600`; host tests 37/37, Nano and UNO builds 8134 B flash. Supply voltage under load not yet recorded |
| 2026-10-26 | 3, supply | 5.17 V at the driver's `+`/`−` pins while the motor runs, the same as the shield's unloaded output: the 18650 shield holds regulation under the stepper's load |
| 2026-10-26 | 4, positioning | After reflashing with the 64:1 ratio: from `ZERO` on 0°, `ROT 90` lands on 90° and `ROT -100`, `ROT 20` lands on 10°, both read within the ±0.5° tolerance |
| 2026-10-26 | 4, holding and restarts | With the coils released after `DONE`, the pointer stays put for 10 s; ten `STEP 8` advance evenly with no jump at any restart. The released shaft can still be turned by hand through the gearbox, as expected without holding current, so nothing may touch the platter between a reading and the next move. Section 4 passes |
| 2026-10-26 | 5, closure | Fresh console, start approached with multiples of 8 half-steps only, then `lockstep.py ... "STOPS 36" --repeat 360 NEXT`: start 1.5°, end 2.0°, last reply `DONE 40960` (the planner's ten-turn total at 64:1). The +0.5° difference is at the edge of the ±0.5° tolerance and equals the reading resolution; it is in the long direction, which lost steps cannot produce. Section 3's runs show the same small excess (343° read against 342.4° predicted). Over one scan revolution it amounts to about 0.05°. Section 5 passes |
| 2026-10-26 | 6, serial and timing | `--repeat 200 "STEP 10"`: 200/200, last `DONE 2000`. `ROT 3600` idle: `OK` at 2.007 s, `DONE 40960` at 84.670 s, **82.663 s**. `ROT 3600` under `--spam-status`: **82.66 s**, 406 `STATUS` sent and 406 answered, 0 `ERR LINE`. Identical times with and without traffic: no timer ticks are lost to masked interrupts. The absolute time misses the walkthrough's 81.9 s ± 0.5 s: the expected value omitted 0.171 s of energise, hold and settle (82.091 s at an exact 16 MHz clock), and the remaining 0.57 s is the board's clock running 0.70 % slow (about 15.89 MHz, typical of a ceramic resonator, not of a crystal). The maintainer accepted this on 2026-10-26: the criterion is now the idle-versus-traffic comparison (spec v3.4), and the 0.70 % clock error is a measured board property that Phase 3 corrects for. A re-run of the 200-move test captured in full: 200 `OK`, 200 `DONE`, 0 `ERR LINE`. Section 6 passes |

**Phase 2 hardware acceptance complete on 2026-10-26.** Measured properties carried into Phase 3: the platter motor's gearbox is 64:1 (`RATIO_X100 = 409_600`), positive steps turn clockwise seen from the shaft side, the board's clock runs 0.70 % slow, the shield delivers 5.17 V under load, and the released shaft holds against vibration but not against a hand.
