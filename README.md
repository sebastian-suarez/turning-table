# Turning table

A 3D-printed turntable for photogrammetry. The platter turns an object by a fixed angle at each stop, a phone on a
counterweighted arc arm photographs it from several heights, and RealityScan on the phone reconstructs the 3D
model. The controller is an Arduino Nano clone (ATmega328PB) running Rust firmware built and flashed from the
command line; the Arduino IDE is not used.

Phases, decisions and progress live in [`docs/PLAN.md`](docs/PLAN.md).

## Hardware

| Part | Role |
|---|---|
| Arduino Nano clone, ATmega328PB (fallback: Arduino UNO, ATmega328P) | controller and USB serial console |
| 28BYJ-48 stepper on a ULN2003 driver board | platter drive, IN1..IN4 on D2..D5 |
| Second 28BYJ-48 on a ULN2003 board through a printed ~8:1 reduction | swings the counterweighted camera arm, IN1..IN4 on D6, D7, D8, D10 |
| 18650 power-bank shield (5 V 3 A boost output with cell protection), ground common with the Nano | powers the motors; never the Nano's 5 V pin |
| Phone running RealityScan (Object Mode, capture interval timer) | capture and reconstruction |
| Creality Hi printer (260 × 260 × 300 mm) | prints the parametric OpenSCAD design |

## Repository layout

- `firmware/`: the `no_std` Rust crate flashed to the board. It owns the nightly pin (`rust-toolchain.toml`), the
  AVR target settings (`.cargo/config.toml`) and the flashing preset (`Ravedude.toml`); nothing outside this
  directory builds for AVR.
- `turntable-core/`: hardware-independent logic (line assembly, command parser, stop planner, motion controller),
  `no_std` without an allocator, tested on the host with stable Rust.
- `docs/`: [`PLAN.md`](docs/PLAN.md) (phases and decisions), [`phase-0-research.md`](docs/phase-0-research.md)
  (MCU, toolchain and design research with sources), [`bring-up.md`](docs/bring-up.md) (first build and flash, step
  by step, for a first Arduino project) and [`phase-2-spec.md`](docs/phase-2-spec.md) (stepper drive and console
  protocol).
- `hardware/`: CAD sources, exported STLs, bill of materials and wiring, filled in during Phase 4.
  [`hardware/measure/`](hardware/measure/) holds the printable protractor disc and shaft pointer for the stepper
  acceptance test.

## Status

Research, board bring-up and a capture trial with RealityScan are done. The Phase 2 stepper drive is implemented
and verified on the host against [`docs/phase-2-spec.md`](docs/phase-2-spec.md); its hardware acceptance
([`docs/phase-2-acceptance.md`](docs/phase-2-acceptance.md)) is next. Scan mode, the mechanical design, the scan
procedure and hardening follow; see the plan.

## Build and flash

One-time setup on macOS with Homebrew:

```sh
brew tap osx-cross/avr && brew trust osx-cross/avr
brew install avr-gcc avrdude
cargo +stable install --locked ravedude
```

The pinned nightly and its components install themselves on the first `cargo` call inside `firmware/`.

```sh
cd firmware
cargo build --release
avr-size -A target/avr-none/release/turntable-firmware.elf   # .text + .data <= 30720 B, .data + .bss <= 2048 B
ls /dev/cu.*                                                 # find the board's serial port
RAVEDUDE_PORT=/dev/cu.usbserial-XXXX cargo run               # flash through avrdude, then open the 57600 8N1 console
cargo build --no-default-features --features uno             # Arduino UNO fallback
```

The firmware has no test harness (there is none on AVR). Pure logic is tested on the host:

```sh
cd turntable-core
cargo test
```

[`docs/bring-up.md`](docs/bring-up.md) walks through identifying the board, probing its bootloader and the first
flash, with the output this board produced. The console protocol is line-based ASCII, one command per line, so it
can be typed into the `cargo run` console.

## License

[MIT](LICENSE).
