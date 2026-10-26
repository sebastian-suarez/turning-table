# Turning table

A 3D-printed turntable for photogrammetry. The platter turns an object by a fixed angle at each stop, a phone on a
counterweighted arc arm photographs it from several heights, and RealityScan on the phone reconstructs the 3D
model. The controller is an Arduino Nano clone (ATmega328PB) running Rust firmware built and flashed from the
command line; the Arduino IDE is not used.

## Start here

- [`docs/PLAN.md`](docs/PLAN.md): the phases, the decisions (D1-D4) and where each phase stands. Read it first.
- [`CLAUDE.md`](CLAUDE.md): the working agreements and the firmware conventions (pins, timers, power rules,
  version pins) for anyone, person or agent, who changes the repository.
- [`docs/bring-up.md`](docs/bring-up.md): the first build and flash, step by step, for a first Arduino project.
- [`docs/board/`](docs/board/): the current work, one card file per remaining phase; B3 (Phase 3, scan mode) is
  next.

## Hardware

| Part | Role |
|---|---|
| Arduino Nano clone, ATmega328PB (fallback: Arduino UNO, ATmega328P) | controller and USB serial console |
| 28BYJ-48 stepper on a ULN2003 driver board | platter drive, IN1..IN4 on D2..D5 |
| Second 28BYJ-48 on a ULN2003 board through a printed ~8:1 reduction | swings the counterweighted camera arm, IN1..IN4 on D6, D7, D8, D10 |
| 18650 power-bank shield (5 V 3 A boost output with cell protection), ground common with the Nano | powers the motors; never the Nano's 5 V pin |
| Phone running RealityScan (Object Mode, capture interval timer) | capture and reconstruction |
| Creality Hi printer (260 × 260 × 300 mm) | prints the parametric OpenSCAD design |

## Layout

| Path | Holds |
|---|---|
| `firmware/` | The `no_std` Rust crate flashed to the board. It owns the nightly pin (`rust-toolchain.toml`), the AVR target settings (`.cargo/config.toml`) and the flashing preset (`Ravedude.toml`); nothing outside this directory builds for AVR. |
| `turntable-core/` | Hardware-independent logic (line assembly, command parser, stop planner, motion controller), `no_std` without an allocator, tested on the host with stable Rust. |
| `docs/` | [`PLAN.md`](docs/PLAN.md) (phases and decisions), [`phase-0-research.md`](docs/phase-0-research.md) (MCU, toolchain and design research with sources), [`bring-up.md`](docs/bring-up.md) (first build and flash), [`phase-2-spec.md`](docs/phase-2-spec.md) (stepper drive and console protocol), [`phase-2-acceptance.md`](docs/phase-2-acceptance.md) and [`phase-2-verification.md`](docs/phase-2-verification.md) (hardware acceptance and its results), [`phase-2-protractor-spec.md`](docs/phase-2-protractor-spec.md) (the printed measuring aids). |
| `docs/board/` | The build board's source of truth: one card file per block (`B3.md` ... `B6.md`) and `board.json`, which names the GitHub Project. |
| `hardware/` | [`BOM.md`](hardware/BOM.md), `cad/` (OpenSCAD sources and the Makefile that exports them) and [`measure/`](hardware/measure/) (the printable protractor disc and shaft pointer for the stepper acceptance test). The rig's CAD and the wiring come in Phase 4. |

## Status

Research, board bring-up, a capture trial with RealityScan and the Phase 2 stepper drive are done: the drive
passed its hardware acceptance on 2026-10-26 ([`docs/phase-2-acceptance.md`](docs/phase-2-acceptance.md), results
in [`docs/phase-2-verification.md`](docs/phase-2-verification.md)). Scan mode is next, then the mechanical design,
the scan procedure and hardening; see the plan.

## Develop

One-time setup on macOS with Homebrew:

```sh
brew tap osx-cross/avr && brew trust osx-cross/avr
brew install avr-gcc avrdude
cargo +stable install --locked ravedude
brew install --cask openscad@snapshot   # only for hardware/cad/
```

The pinned nightly and its components install themselves on the first `cargo` call inside `firmware/`.

```sh
cd firmware
cargo build --release
avr-size -A target/avr-none/release/turntable-firmware.elf   # .text + .data <= 30720 B, .data + .bss <= 2048 B
cargo clippy && cargo fmt
ls /dev/cu.*                                                 # find the board's serial port
RAVEDUDE_PORT=/dev/cu.usbserial-XXXX cargo run               # flash through avrdude, then open the 57600 8N1 console
cargo build --no-default-features --features uno             # Arduino UNO fallback
```

The firmware has no test harness (there is none on AVR). Pure logic is tested on the host:

```sh
cd turntable-core
cargo test
```

The printed parts are exported from the command line:

```sh
make -C hardware/cad          # export the STLs into hardware/measure/
make -C hardware/cad check    # export, then check the STL bounds
```

[`docs/bring-up.md`](docs/bring-up.md) walks through identifying the board, probing its bootloader and the first
flash, with the output this board produced. The console protocol is line-based ASCII, one command per line, so it
can be typed into the `cargo run` console.

## How work is tracked

The GitHub Project [Turning table](https://github.com/users/sebastian-suarez/projects/3) is the build board: one
card per build block (label `block`, titled `B<n> — <name>`, keys matching the plan's phase numbers) and one per
open decision (label `decision`). `docs/board/` is its source of truth: each card is a file `docs/board/<KEY>.md`
whose front matter holds the status, build order and labels and whose body is the step list, ticked only after a
step is verified. The two-way sync workflow that turns these files into issues and keeps them equal to the board
is not installed yet; until it is, the files are authoritative and the board shows only what has been synced.
Design decisions are recorded in [`docs/PLAN.md`](docs/PLAN.md); the board's own README explains its fields,
views and rules.

## License

[MIT](LICENSE).
