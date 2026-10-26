# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

A 3D-printed turntable for 3D scanning (photogrammetry), driven by an Arduino Nano clone whose MCU is an
**ATmega328PB** (fallback board on hand: Arduino UNO, ATmega328P). Firmware is Rust on `avr-hal`, built and
flashed from the command line only; the Arduino IDE is never used. Platter drive is a 28BYJ-48 stepper on a ULN2003
board; a second 28BYJ-48 swings a counterweighted camera arc arm through a printed ~8:1 reduction (the SG90 from
the kit is a spare); capture and reconstruction are RealityScan on the phone (Object Mode, interval timer); the
mechanical design is custom, as parametric OpenSCAD in `hardware/cad/`, printed on a Creality Hi
(260 × 260 × 300 mm). Decisions and phases: `docs/PLAN.md`; parts: `hardware/BOM.md`.

- `firmware/` is the `no_std` crate flashed to the board. It owns its `rust-toolchain.toml`, `.cargo/config.toml`
  and `Ravedude.toml`. Keep those inside `firmware/`: the planned host-side crates (`turntable-core/`, `host/`)
  must build for the Mac with stable Rust and must not inherit the AVR target.
- `docs/PLAN.md` holds the phases and decisions D1-D3 (drive, capture, reconstruction; recorded 2026-10-22).
  `docs/phase-0-research.md` holds the Phase 0 findings with sources: STL candidates, MCU viability, toolchain.
- `hardware/` receives the CAD sources, exported STLs, BOM and wiring from Phase 4.

## One-time toolchain setup (macOS, Homebrew 7)

```sh
brew tap osx-cross/avr && brew trust osx-cross/avr   # Homebrew 7 refuses to load formulae from untrusted taps
brew install avr-gcc avrdude                           # avr-gcc = GCC 9 (rustc's linker + libgcc), avrdude 8
cargo +stable install --locked ravedude                # avrdude wrapper that `cargo run` uses
brew install --cask openscad@snapshot                  # `openscad` CLI for hardware/cad/ (the plain `openscad` cask is disabled since 2026-09-01)
```

The pinned nightly and its components (`rust-src`, `clippy`, `rustfmt`, `rust-analyzer`) install themselves from
`firmware/rust-toolchain.toml` on the first cargo call inside `firmware/`. Host tools are always installed with
`cargo +stable`, never with the nightly.

## Commands (run inside `firmware/`)

```sh
cargo build                                               # dev profile is already opt-level = "s"
cargo build --release
cargo build --no-default-features --features uno          # UNO fallback (cargo features `nano` | `uno` pick the arduino-hal board)
avr-size -A target/avr-none/release/turntable-firmware.elf  # .text+.data <= 30720 B on a Nano (both bootloaders), 32256 B on the UNO; .data+.bss <= 2048 B
cargo clippy && cargo fmt
ls /dev/cu.*                                              # CH340 clones appear as cu.usbserial-* or cu.wchusbserial*
RAVEDUDE_PORT=/dev/cu.usbserial-XXXX cargo run            # build, flash through avrdude, open the 57600 8N1 console
```

There is no `cargo test` for the firmware (`[[bin]] test = false`; no test harness on AVR). Pure logic goes into
`turntable-core/` (planned, `no_std`, no allocator) and is tested there on the host with plain `cargo test`.

## Flashing an ATmega328PB Nano

- The firmware is compiled for `atmega328p` (arduino-hal feature `arduino-nano`, `-C target-cpu=atmega328p`).
  The 328PB executes 328P code unchanged (Microchip app note AT15007). The 328PB-only peripherals (USART1, SPI1,
  TWI1, TC3, TC4, PE0..PE3) are simply not addressable on this path, and nothing here needs them.
- avrdude checks the signature the **bootloader** reports; Optiboot and urboot hard-code it at build time. This
  board was probed on 2026-10-22: FTDI port `/dev/cu.usbserial-A5069RR4` (set in `Ravedude.toml`), answers at
  115200 with `1E 95 0F`, silent at 57600, so the `nano-new` preset is the confirmed choice; `nano` is the 57600
  "old bootloader" variant for other clones. Wrong baud looks like `programmer is not responding` / `not in sync`;
  a port held by `screen` or the `cargo run` console looks like `cannot open port ...: Resource busy`. If avrdude prints `Device signature = 1E 95 16 (ATmega328PB)` or
  `expected signature for ATmega328P is 1E 95 0F`, switch to the custom board block in `Ravedude.toml`
  (`inherit = "nano-new"`, `partno = "atmega328pb"`).
- Probe without writing: `avrdude -c arduino -p m328p -P /dev/cu.usbserial-XXXX -b 115200 -v`, then retry with
  `-b 57600`. A timeout means the wrong baud; a signature mismatch names the right part. Before any of this, read
  the chip marking: `ATMEGA328PB-AU`/`-MU` is what we want; an `LGT8F328P` is a 32 MHz near-clone that avr-hal does
  not support, and its bootloader can still answer the probe with a 328P signature.
- Native 328PB build, only if a 328PB-only peripheral is ever needed: `atmega-hal` with feature `atmega328pb`
  instead of `arduino-hal`, `-C target-cpu=atmega328pb`, and
  `[target.avr-none] linker = "/opt/homebrew/opt/avr-gcc@14/bin/avr-gcc"` after
  `brew install osx-cross/avr/avr-gcc@14` (keg-only). The Homebrew `avr-gcc` alias (GCC 9) ships no
  `crtatmega328pb.o`. This path is verified to build (see the research doc) but loses `arduino_hal::pins!` and
  `default_serial!`. Flash it with the same preset the probe selected: avrdude's partno must match the
  bootloader's reported signature, not the compile target, and no bootloader change is needed to use USART1.
- UNO fallback: `cargo build --no-default-features --features uno` (or `cargo run ...` with the same flags),
  `board = "uno"` in `Ravedude.toml`, and comment out its `port` line (or export `RAVEDUDE_PORT`): a configured
  port overrides the UNO preset's VID/PID auto-detection. Pins are the same (D9/D10 for TC1 PWM, D13 LED, D0/D1
  serial).

## Version pins (change deliberately, together)

- `nightly-2025-04-27` is the nightly avr-hal itself pins; other nightlies have hit LLVM AVR regressions.
- `arduino-hal` is a git dependency pinned by rev (not on crates.io); the rev is the one avr-hal-template uses.
- `Cargo.lock` pins `encoding_rs = 0.8.35`. It is a *host* build dependency (avr-device → yaml-rust2 →
  encoding_rs); 0.8.42 uses `slice::as_chunks`, unstable on this nightly, and fails with E0658. After any
  `cargo update`, re-pin with `cargo update -p encoding_rs --precise 0.8.35`. `Cargo.lock` belongs in git.

## Working agreements (2026-10-22)

- **Learning repository.** The user is new to Arduino and asked for this: comment the code generously (register
  choices, timer arithmetic, interrupt safety, fixed-point reasoning, why a construct is needed on AVR). This is
  the user's explicit exception to their no-comments rule.
- **Commits on `main` are authorized without asking** (spec freezes, review fixes, merges). No AI attribution
  anywhere, per the user's global rule.
- **Who writes code.** Codex GPT-6.1 Sol implements from a spec that has survived an adversarial attack
  (`docs/phase-N-spec.md`), at `max` effort, launched with `codex exec` directly because the plugin wrapper caps
  effort at `xhigh`; Codex GPT-6 Astra attacks the spec and reviews the result (`~/.claude/scripts/astra-review.sh`);
  Claude verifies (tests, builds, hardware acceptance) and runs the artifact-only review; no rival implementations.

## Firmware conventions

- Servo output (only if the spare SG90 is ever wired): avr-hal has no servo driver. Use TC1 (16-bit) in fast PWM
  mode 14 with ICR1 as TOP for a 50 Hz frame; the hardware output is D9 (OC1A) (D10/OC1B is taken by the arm
  stepper). Reference:
  `examples/arduino-uno/src/bin/uno-manual-servo.rs` in avr-hal (prescaler 64, ICR1 = 4999, 4 µs per count).
  Prescaler 8 with ICR1 = 39999 gives 0.5 µs resolution. TC0 stays free for a millis tick; TC2 drives the stepper
  (next bullet).
- Steppers: 28BYJ-48 through ULN2003, platter IN1..IN4 on D2..D5 (PD2..PD5), arm IN1..IN4 on D6, D7, D8, D10
  (PD6, PD7, PB0, PB2; D9/OC1A stays reserved for an optional servo), both as instances of the same controller
  stepped from one TC2 CTC interrupt (1 ms tick, step period in ticks), coils released when idle. Half-steps per revolution is one
  named constant, measured in Phase 2, not assumed: 28BYJ-48 units ship with either a 63.68395:1 gearbox
  (4075.77 half-steps) or an exact 64:1 one (4096); the wrong constant drifts 1.78° per revolution. Accumulate
  the fractional remainder in integers across stops so N stops close exactly. Measured 2026-10-26: the platter
  motor is 64:1 (`RATIO_X100 = 409_600`); the arm motor is not yet measured. This board's clock runs 0.70 % slow
  (about 15.89 MHz, most likely a ceramic resonator; not inspected): step counts are exact, durations are not, so anything timed against the
  phone needs the correction. Motor and servo power from the
  18650 power-bank shield's 5 V output (`hardware/BOM.md` item 2; slide switch on HOLD) with common ground, never
  the Nano's 5 V pin, not even for a test.
- The console protocol is line-based ASCII, one command per `\n`, so it can be typed into the `cargo run` console.
  Format with `ufmt`, never `core::fmt` (flash and RAM).
- `panic = "abort"` plus `panic-halt`: a panic freezes the board and the LED stops toggling. No heap.
- Every string literal lives in SRAM (`.data`): Rust has no progmem by default, and the two banner strings already
  cost 32 of the 2048 bytes. Keep protocol strings terse, watch `.data` in `avr-size -A`, and reach for the
  `avr-progmem` crate before adding help text or long error messages.
- Motors and servos are never powered from the Nano's 5 V pin while on USB; the shield's 5 V, common ground.
