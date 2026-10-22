# Phase 0 research (2026-10-22)

Scope: viable free STLs for the turntable, whether the ATmega328PB Nano clone is usable with Rust, and what a
command-line-only build and flash workflow needs on this Mac (macOS 27, Apple Silicon). Everything in
"Verified on this machine" was executed here; web facts carry their source. Pages on Thingiverse, Printables,
MakerWorld, MyMiniFactory and Hackster sit behind a bot challenge and could not be read directly, so licenses
from those sites are marked *unverified* and must be checked by hand before downloading.

## 1. The MCU: an ATmega328PB on a Nano-shaped clone

**Verdict: usable.** Two routes both build here; the project defaults to route A.

| | Route A (default) | Route B (native 328PB) |
|---|---|---|
| HAL crate | `arduino-hal`, feature `arduino-nano` (maps to `atmega-hal/atmega328p`) | `atmega-hal`, feature `atmega328pb` (exists; no `arduino-hal` board for it) |
| `-C target-cpu` | `atmega328p` | `atmega328pb` (listed by `rustc --print target-cpus --target avr-none`) |
| Linker | Homebrew `avr-gcc` alias = GCC 9.5 | `avr-gcc@14` (keg-only). GCC 9's avr-libc has no `crtatmega328pb.o` / `libatmega328pb.a`: link fails |
| Conveniences | `pins!` (d0..d13, a0..a7), `default_serial!`, 21 Uno examples | raw port pins (`pb5`), `#[avr_device::entry]`, `Delay<MHz16>` |
| 328PB extras (USART1, SPI1, TWI1, TC3, TC4, PE0..3) | not addressable | available (`dp.USART1` compiled) |
| Build result here | 1934 B flash (1902 `.text` + 32 `.data`), 33 B SRAM (release), ELF `avr:5` | 338 B flash blink, `-mmcu=atmega328pb` passed to the linker |

Why A is safe: Microchip's AT15007 ("Differences between ATmega328/P and ATmega328PB", AN42559) describes the
328PB as backward compatible with 328/P code: existing code runs without configuration changes, the reverse is
not guaranteed. Differences that matter to us: the full-swing crystal oscillator option is gone (Arduino fuses use
the low-power oscillator, so clones ship fine), one VCC and one GND pin became PE0/PE1 on the bare chip, ADC6/7
became PE2/PE3 (digital-capable), and the signature is `1E 95 16` instead of `1E 95 0F`. One Arduino-forum report
(thread 1151664) of 328P code "doing nothing" on a 328PB was a custom board with unresolved fuse and wiring
questions, not a clone with a shipped bootloader.

Flashing facts (serial bootloader over USB, which is what a Nano clone offers):

- avrdude checks the signature that the **bootloader** returns. Optiboot and urboot hard-code `SIGNATURE_0..2`
  at build time, so the reported signature tells you what the bootloader was built for, not what the silicon
  is. A Sept 2026 forum thread (1460819) documents exactly this clone: uploads work as a plain Nano with the
  *normal* (115200, not "old bootloader") setting (`Device signature = 0x1e950f (probably m328p)`, the avrdude 7
  output format quoted in the thread; avrdude 8 prints `1E 95 0F (ATmega328P, ...)`). The poster
  later re-burned a 328PB bootloader (`1E 95 16`) through an ISP programmer (Arduino-as-ISP, MiniCore 3.1.3,
  urboot) only because they wanted the IDE's 328PB board variant; that is not needed here. Application code reaches USART1 and the other 328PB peripherals whatever bootloader is installed,
  so route B is flashed through the existing bootloader with the partno the probe showed (avrdude's partno must
  match the bootloader's reported signature, not the compile target). ISP wiring is only needed if we deliberately
  change the bootloader or fuses.
- avrdude 8.3 knows `m328pb` / `ATmega328PB` (dry run returns `0x1e 0x95 0x16`) and, tellingly, `lgt328p`:
  some "Nano" clones carry an **LGT8F328P**, a 32 MHz near-clone that is not supported by avr-hal. Read the chip
  marking before anything else: `ATMEGA328PB-AU` or `-MU` is what we want.
- ravedude 0.2.2 presets: `nano` = programmer `arduino`, partno `atmega328p`, 57600 baud ("old bootloader");
  `nano-new` = same at 115200 (Optiboot). Which one a given clone has is unknown until probed; the one documented
  328PB clone used 115200, so `firmware/Ravedude.toml` defaults to `nano-new` (a guess to confirm in Phase 1).
  Neither preset can auto-detect the port of a clone (`Not able to guess port`), hence `RAVEDUDE_PORT` or `port =`
  in `Ravedude.toml`. ravedude supports `inherit = "<preset>"` under `[board]`, so the 328PB case is a custom
  board of `partno = "atmega328pb"` plus `baudrate` (which ravedude requires in any `[board.avrdude]` block),
  drafted (commented) in `firmware/Ravedude.toml` and validated with `ravedude --dump-config`. avr-hal PR #462
  (Dec 2023, issue #461) added the 328PB signature to ravedude's bundled avrdude config, so that path is
  supported upstream.
- Probe command, no write: `avrdude -c arduino -p m328p -P /dev/cu.usbserial-XXXX -b 57600 -v`, retry `-b 115200`.
- UNO fallback (ATmega328P, real Arduino or clone): same firmware with feature `arduino-uno`, preset `uno`
  (115200, auto port by VID/PID). Nothing else changes.

## 2. Rust for AVR: what the command-line workflow needs

Facts (sources: avr-hal README, avr-hal-template, rustc platform-support page for `avr-none`):

- Nightly only. `avr-none` is a tier-3 target with no prebuilt `core`, so `-Z build-std=core` and the
  `rust-src` component. `-C target-cpu=<mcu>` is mandatory (`need-explicit-cpu: true`); the target spec names
  `avr-gcc` as the linker and links `-lgcc`, so avr-gcc must be on `PATH` (or set as `linker`).
- avr-hal and its template pin `nightly-2025-04-27` (rustc 1.88.0-nightly, 2025-04-26). `arduino-hal` is a git
  dependency pinned by rev `e5c8f37fe48419956e722490a82b9ca9b9fc61a2` (not on crates.io).
- Snag found here: with that nightly, the fresh `Cargo.lock` resolved `encoding_rs 0.8.42`, a *host* build
  dependency of `avr-device 0.8.1` (through `yaml-rust2`, used by its `svd2rust` build step). It fails with E0658
  `slice_as_chunks` (stabilised after that nightly). `cargo update -p encoding_rs --precise 0.8.35` fixes it;
  the pin lives in `Cargo.lock`.
- `cargo run` = ravedude = avrdude flash + serial console. Install host tools with `cargo +stable`.

Verified on this machine (all exit 0):

| Step | Result |
|---|---|
| `brew tap osx-cross/avr && brew trust osx-cross/avr` | Homebrew 7.0.10 requires the trust step |
| `brew install avr-gcc avrdude` | avr-gcc 9.5.0 (alias of `avr-gcc@9`, linked), avrdude 8.3, both bottled for this macOS |
| `brew install osx-cross/avr/avr-gcc@14` | 14.4.0, keg-only (`/opt/homebrew/opt/avr-gcc@14/bin`), only needed for route B |
| `rustup toolchain install nightly-2025-04-27 --component rust-src` | later extended to clippy, rustfmt, rust-analyzer through `rust-toolchain.toml` |
| `cargo +stable install --locked ravedude` | ravedude v0.2.2 |
| `cargo build` / `--release` / `clippy` / `fmt --check` in `firmware/` | clean; release ELF 1902 B `.text`, 32 B `.data`, 1 B `.bss` (33 B SRAM: string literals live in `.data`) |
| `cargo build --release --no-default-features --features uno` | UNO variant of the same firmware builds: 1900 B `.text`, 32 B `.data`, 1 B `.bss` |
| Route B throwaway crate (scratchpad, not committed) | builds and links with `-mmcu=atmega328pb`; fails with GCC 9 as expected |

Not verified: flashing. No board was connected (`ls /dev/cu.*` shows only Bluetooth and debug ports). Phase 1.

## 3. Driving a hobby servo from avr-hal

- avr-hal has no servo abstraction. The upstream example `examples/arduino-uno/src/bin/uno-manual-servo.rs`
  configures TC1 (16-bit) in fast PWM mode 14 with `ICR1` as TOP: prescaler 64 and `ICR1 = 4999` give a 50 Hz
  frame with 4 µs per count (OCR1A 250..500 for 1..2 ms). Prescaler 8 with `ICR1 = 39999` gives 0.5 µs per count.
  Hardware outputs: OC1A = D9 (PB1), OC1B = D10 (PB2) only. The `simple_pwm` API cannot reach 50 Hz.
- Calibrate per servo: the example notes that real limits lie outside the nominal 1..2 ms.
- Current: an SG90 stalls at roughly 650 mA, a 28BYJ-48 draws ~240 mA. Neither belongs on the Nano's 5 V pin
  while it is USB-powered (500 mA budget shared with the board); Brocken's build needed a separate buck converter.

## 4. Free STL candidates

No free design was found that drives an *indexed* platter with a hobby servo. The ecosystem is 28BYJ-48 (small
tables) or NEMA17 (OpenScan, Ciclop). Servo appears either as the camera-shutter presser or as a
continuous-rotation drive for constant-speed video turntables. This is decision D1 in `PLAN.md`.

| # | Design | Drive / controller | Files | License | Fit |
|---|---|---|---|---|---|
| 1 | **Turntable for display / photography / 3D scanning using 28BYJ-48** (Printables 539153) | 28BYJ-48 + ULN2003, Arduino **Nano** clone, potentiometer | STEP (export STL yourself) | *unverified* | Closest electrical match; STEP makes a servo mount remix easy |
| 2 | **Scanner 3D & Turntable** (MakerWorld 204524) | 28BYJ-48 + ULN2003, UNO; slot for the driver board, USB hole | STL + print profiles | *unverified*; MakerWorld's default licence usually forbids redistribution | Purpose-built for photogrammetry |
| 3 | **Arduino controlled photogrammetry 3D-scanner**, Brian Brocken (Thingiverse 3958326, Hackaday 168301, MyMiniFactory 104755) | 28BYJ-48 (platter), **SG90 presses a Bluetooth shutter**, 1602 LCD, joystick, UNO, print-in-place bearing, 2..200 photos/rev | STL; Fusion 360 sources sold on CGTrader | a search snippet of the Hackster listing says CC BY-NC-ND (NoDerivatives would forbid remixing the mount); *unverified* on Hackster and Thingiverse | Only candidate that already uses a servo (as trigger); proven for photogrammetry (ReCap) |
| 4 | **IKEA SNUDDA motorized turntable with Arduino** (Printables 499102) | 28BYJ-48 + ULN2003, UNO, on a 39 cm IKEA lazy susan | STL for the motor mount | *unverified* | Large platter cheaply; author used it for photogrammetry |
| 5 | **Arduino Turntable** (Instructables) | 28BYJ-48 + ULN2003A, **Nano**, sketch included | STL base + lid | *unverified* (Instructables default is CC BY-NC-SA) | Small; author suggests it for 3D scanning |
| 6 | **Williams College Makerspace DIY turntable** (GitHub tashrique/DIY-Turntable-Makerspace-Resources) | stepper + bearings, Arduino | STL + Arduino code | **no LICENSE file** (all rights reserved by default; ask) | Well-documented 3-part tutorial |
| 7 | **OpenScan Classic / Mini / Midi** (GitHub OpenScan-org/OpenScan-Design, `files/Classic/V1`; Printables 83322/83333 are old mirrors) | 2x NEMA17 (turntable + rocking arm), Raspberry Pi shield; the Arduino kit is deprecated; OpenScan-Lite is a community Arduino/ESP32 branch | STL, BOM in OpenScan-Doc | **no licence declared in the repo**; firmware OpenScan3 is GPL-3.0 | Best results in this space (object rotates *and* tilts), but a NEMA17 + Pi project; heavy for servos |
| 8 | **Ciclop** (GitHub LibreScanner/ciclop, Thingiverse 740357) | NEMA17 + A4988 on a ZUM/UNO shield, two line lasers, Logitech C270, 200 mm acrylic platter | STL/STEP/DXF, BOM | **CC BY-SA 4.0** (README) | The UNO fallback's natural project, but laser scanning and Horus (its software) is unmaintained |
| 9 | **Adafruit Motorized Turntable** (Thingiverse 2899821) | **continuous-rotation micro servo** (FS90R class), constant speed, reverse switch | STL + Fusion (parametric) | *unverified* (Adafruit guides are normally CC BY-SA) | The only servo-driven table found; no indexing, would need an encoder for scanning |
| 10 | **ArUco Photogrammetry Turntable 190 mm+** (MakerWorld 3175311) | none (hand-indexed), marker collar gives real-world scale | STL | *unverified* | Idea to borrow: a marker ring on whatever platter we pick |

Recommendation for Phase 4, pending D1: with a 28BYJ-48, print #1 (Nano-native, editable STEP) or #2
(scanner-specific); with servos, remix #1's platter onto a 2:1 gear + 180° servo, or a feedback-360 servo on
#9's base. Either way add a #10-style marker ring.

## 5. Reconstruction and camera trigger on the Mac side

- Apple's **"Creating a photogrammetry command-line app"** sample (RealityKit `PhotogrammetrySession`, macOS
  13+, Xcode 15.2+) runs on Apple Silicon and takes a folder of JPEG/HEIC/PNG; fits "command line only".
- Meshroom has no official macOS build and its dense depth maps need CUDA (a CPU-only draft mode exists).
  OpenScanCloud is a free web API. COLMAP builds with Homebrew (sparse on CPU; dense needs CUDA).
- Camera trigger: Brocken used an SG90 on a Bluetooth remote because iOS refuses HC-05/HC-06 modules; the
  cleaner path is host-driven capture (gphoto2 for DSLRs, `imagesnap` or Continuity Camera for webcams/iPhone),
  which also keeps the Nano firmware trivial. Decision D2.

## Sources

- avr-hal: https://github.com/Rahix/avr-hal (README; `mcu/atmega-hal/Cargo.toml`; `arduino-hal/Cargo.toml`;
  `ravedude/README.md`; `ravedude/src/boards.toml`; `examples/arduino-uno/src/bin/uno-manual-servo.rs`;
  issue 461 and PR 462; repo `rust-toolchain.toml`)
- avr-hal template: https://github.com/Rahix/avr-hal-template (`rust-toolchain.toml`, `.cargo/config.toml`, `Cargo.toml`)
- rustc `avr-none`: https://doc.rust-lang.org/rustc/platform-support/avr-none.html
- Homebrew AVR tap: https://github.com/osx-cross/homebrew-avr
- Microchip AT15007 / AN42559: https://www.microchip.com/en-us/application-notes/an42559
- Arduino forum, Nano clone with ATmega328PB (Sept 2026): https://forum.arduino.cc/t/nano-clone-with-atmega328pb-and-invalid-signature/1460819
- Arduino forum, 328PB signature on upload: https://forum.arduino.cc/t/unable-to-send-sketches-to-atmega328pb/613299
- Arduino forum, 328P code on 328PB (unresolved): https://forum.arduino.cc/t/atmega328pb-doesnt-seem-to-be-compatible-with-atmega328p-code/1151664
- LGT8F328P board package (to recognise the impostor): https://github.com/dbuezas/lgt8fx
- OpenScan: https://openscan.eu/openscan-classic , https://github.com/OpenScan-org/OpenScan-Design ,
  https://openscan-org.github.io/OpenScan-Doc/hardware/OpenScanClassic/ , https://github.com/OogaBooga21/OpenScan-Lite
- Ciclop: https://github.com/LibreScanner/ciclop , https://www.thingiverse.com/thing:740357 , https://reprap.org/wiki/Ciclop
- Brocken: https://hackaday.io/project/168301-arduino-controlled-photogrammetry-3d-scanner ,
  https://www.thingiverse.com/thing:3958326 , https://www.hackster.io/bribro12/arduino-controlled-photogrammetry-3d-scanner-a8fff7 ,
  build log https://jpralves.net/post/2019/11/13/arduino-controlled-photogrammetry-3d-scanner.html
- Printables 539153: https://www.printables.com/model/539153-turntable-for-display-photography-3d-scanning-usin
- Printables 499102: https://www.printables.com/model/499102-ikea-snudda-motorized-turntable-with-arduino
- MakerWorld 204524: https://makerworld.com/en/models/204524 ; 3175311: https://makerworld.com/models/3175311
- Instructables: https://www.instructables.com/Arduino-Turntable/
- Williams Makerspace: https://github.com/tashrique/DIY-Turntable-Makerspace-Resources , https://sites.williams.edu/makerspace/?p=1536
- Adafruit turntable: https://learn.adafruit.com/motorized-turntable-circuitpython , https://www.thingiverse.com/thing:2899821
- Apple Object Capture CLI sample: https://developer.apple.com/documentation/realitykit/creating-a-photogrammetry-command-line-app
