# Bring-up guide: from a blank Mac to a blinking, talking Nano

For a first Arduino project. Every command below is the real one; the expected output is what this board produced
on 2026-10-22. Run the `cargo` commands inside `firmware/`.

## 1. Words you will meet

- **MCU**: the chip that runs your program, here an ATmega328PB with 32 KiB of flash (program memory) and 2 KiB
  of SRAM (variables). The Nano is just that chip plus a USB-to-serial converter, a 16 MHz crystal and a 5 V
  regulator on a small board.
- **Bootloader**: a tiny program that lives in the last 2 KiB of flash. After every reset it listens on the serial
  line for about a second; if the Mac sends a new program, it writes it into flash, otherwise it starts whatever
  program is there. It is what lets us program over plain USB without a dedicated programmer.
- **avrdude**: the Mac-side program that talks to the bootloader (protocol `arduino`, a.k.a. STK500v1) and writes
  the flash. ravedude, used by `cargo run`, is a thin wrapper around it.
- **Baud**: the serial speed. Two different speeds matter and must not be confused: the bootloader's (115200 on
  this board) and your program's console (57600, chosen in `src/main.rs` and mirrored in `Ravedude.toml`).
- **Signature**: three bytes that identify the chip model. The bootloader reports them and avrdude refuses to
  flash when they do not match the part you asked for (`-p`). Bootloaders hard-code the value they were built
  with, so this board's bootloader says "ATmega328P" even though the silicon is a 328PB.
- **Serial console**: the text your program prints over USB. Opening the port on the Mac usually resets the board
  (the USB-serial chip pulls the reset line), so you see the boot banner each time you connect.
- **Rust nightly, `avr-none`, `build-std`**: Rust's AVR support is experimental, so it needs the nightly compiler,
  the `avr-none` target with an explicit CPU, and the `core` library built from source. `rust-toolchain.toml` and
  `.cargo/config.toml` in `firmware/` take care of all three.

## 2. One-time setup (already done on this Mac)

```sh
brew tap osx-cross/avr && brew trust osx-cross/avr   # the AVR tools live in a third-party tap; Homebrew 7 needs the trust step
brew install avr-gcc avrdude                           # avr-gcc 9 (the linker rustc uses) and avrdude 8.3
cargo +stable install --locked ravedude                # the flash-and-console helper behind `cargo run`
```

Check them: `which avr-gcc avrdude ravedude` prints three paths, and `rustup toolchain list` shows
`nightly-2025-04-27-aarch64-apple-darwin` after the first `cargo build`.

## 3. Identify the board

1. Read the big chip's marking with a magnifier: `ATMEGA328PB` (ours). A board marked `LGT8F328P` is a lookalike
   that this toolchain does not support.
2. Plug it in with a data cable and list the serial ports:

   ```sh
   ls /dev/cu.*
   ```

   A new entry appears: `/dev/cu.usbserial-A5069RR4`. The suffix is the FTDI chip's serial number, so it stays the
   same for this board on any port. (CH340-based boards show up as `cu.usbserial-1xx0` or `cu.wchusbserial...`.)
   No new entry means a charge-only cable or a dead board; try another cable first.

## 4. Probe the bootloader (reads only, writes nothing)

```sh
avrdude -c arduino -p m328p -P /dev/cu.usbserial-A5069RR4 -b 115200 -v
```

What this board answered (trimmed):

```
Using port            : /dev/cu.usbserial-A5069RR4
Setting baud rate     : 115200
Programmer type       : Arduino
Device signature = 1E 95 0F (ATmega328P, ATA6614Q, LGT8F328P)
```

`1E 95 0F` means the bootloader was built for a 328P: we flash as `atmega328p`, which is exactly what the
`nano-new` preset in `Ravedude.toml` does. The same probe at `-b 57600` answered
`Error: programmer is not responding` / `not in sync: resp=0x00` ten times: that is simply the wrong speed.
If you ever see `expected signature for ATmega328P is 1E 95 0F` next to `1E 95 16`, the bootloader is a 328PB one;
`Ravedude.toml` has a commented block for that case.

## 5. Build

```sh
cd firmware
cargo build            # first time: downloads the nightly, builds `core`, then the firmware (a few minutes); later runs take ~1 s
cargo build --release  # same, smaller/faster code
```

The result is an ELF file, `target/avr-none/debug/turntable-firmware.elf` (`cargo run` flashes this debug build;
`--release` lands in `target/avr-none/release/`). See how much of the chip it uses:

```sh
avr-size -A target/avr-none/debug/turntable-firmware.elf
```

Trimmed to the three sections that matter (the full listing also shows a header, `.note.gnu.avr.deviceinfo`,
ten `.debug_*` sections and a `Total` over 300 KB; none of those are flashed, they only live in the file):

```
.data     32     # initialised variables and string literals: stored in flash, copied to SRAM at boot
.text   2056     # the code: flash
.bss       1     # zero-initialised variables: SRAM
```

`.text + .data` = 2088 bytes, exactly the "2088 bytes" avrdude reports in section 6. The release build is
1902 + 32. Flash budget: `.text + .data` under 30720 bytes (32 KiB minus the 2 KiB bootloader). SRAM budget:
`.data + .bss` under 2048 bytes, minus what the stack needs while running.

## 6. Flash and watch: `cargo run`

```sh
cargo run              # port comes from Ravedude.toml; or RAVEDUDE_PORT=/dev/cu.usbserial-XXXX cargo run
```

The real output:

```
     Running `ravedude target/avr-none/debug/turntable-firmware.elf`
       Board Arduino Nano (New Bootloader)
 Programming target/avr-none/debug/turntable-firmware.elf => /dev/cu.usbserial-A5069RR4
Reading 2088 bytes for flash from input file turntable-firmware.elf
Writing 2088 bytes to flash
Writing | ################################################## | 100% 0.33s
Reading | ################################################## | 100% 0.32s
2088 bytes of flash verified
  Programmed target/avr-none/debug/turntable-firmware.elf
     Console /dev/cu.usbserial-A5069RR4 at 57600 baud
             CTRL+C to exit.

�urntable-firmware boot
tick 0
tick 1
```

Line by line: ravedude picked the preset, told avrdude to write the ELF, avrdude read the flash back and
compared it ("verified"), then ravedude opened the console at the program's baud. The board resets when the
console opens, so the first byte of the banner is usually garbled; that is harmless. `tick n` arrives once a
second and the on-board LED marked `L` (next to the USB connector) blinks at the same rate.

Ctrl-C closes the console only; the program keeps running on the board, with or without the Mac. Running
`cargo run` again re-flashes without unplugging anything. The small button on the board resets it and prints
the banner again.

## 7. The same thing by hand (what ravedude does for you)

Flash with avrdude directly:

```sh
avrdude -c arduino -p atmega328p -P /dev/cu.usbserial-A5069RR4 -b 115200 \
        -U flash:w:target/avr-none/debug/turntable-firmware.elf:e
```

`-c` programmer protocol, `-p` part, `-P` port, `-b` bootloader baud, `-U memory:operation:file:format`
(`w` = write, `e` = the file is an ELF). avrdude verifies after writing by default. A single
`Warning: attempt 1 of 10: not in sync: resp=0x60` is normal: the running program was still printing when
avrdude sent its first sync byte; attempt 2 succeeds. Ten failed attempts mean a real problem (section 8).

Read the console with a plain terminal (pick one):

```sh
screen /dev/cu.usbserial-A5069RR4 57600         # leave with Ctrl-A, then K, then y
```

```sh
# plain shell: the port must be held open (fd 3) while stty configures it, or macOS forgets the settings
exec 3<>/dev/cu.usbserial-A5069RR4 && stty -f /dev/cu.usbserial-A5069RR4 57600 cs8 -cstopb -parenb raw -echo && cat <&3
# Ctrl-C to stop, then `exec 3>&-` to release the port. Expect a burst of garbage before the banner (reset + baud switch).
```

`screen` and the `cargo run` console open the port exclusively, so while one of them is running avrdude fails at
once with `cannot open port /dev/cu.usbserial-A5069RR4: Resource busy`: close the console first. The fd-3 shell
trick does not lock the port; with it open, avrdude and `cat` both read the same bytes and avrdude reports
`not in sync` instead. (`cu` is also installed but needs a lock directory it cannot write without `sudo`.)

## 8. When something goes wrong

| Symptom | Meaning | Do |
|---|---|---|
| `cannot open port ...: Resource busy` | `screen` or the `cargo run` console holds the port | close it, then flash |
| `programmer is not responding` / `not in sync` (10 attempts) | wrong bootloader baud, another reader on the port (fd-3 trick), or wrong port | try the other preset (`nano` = 57600), stop the `cat`, re-check `ls /dev/cu.*` |
| `Device signature = ... expected signature ...` | bootloader built for another chip | use the 328PB block in `Ravedude.toml` |
| `avr-gcc: command not found` at link time | Homebrew tools missing | section 2 |
| `error[E0658] ... slice_as_chunks` in `encoding_rs` | a `cargo update` moved a host build dependency past the pinned nightly | `cargo update -p encoding_rs --precise 0.8.35` |
| console shows garbage every line | console baud mismatch | `default_serial!(…, 57600)` in `src/main.rs` and `serial-baudrate` in `Ravedude.toml` must agree |
| LED blinks but console is silent | console opened at the wrong baud or wrong port | same check |
| `No such file or directory` for the port | board unplugged or re-enumerated | `ls /dev/cu.*` again |

## 9. Next: wiring the stepper and the servo (Phase 2)

Wire with the USB cable unplugged. Pin plan (also in `CLAUDE.md`):

| Part | Wire | Nano pin |
|---|---|---|
| ULN2003 board | IN1, IN2, IN3, IN4 | D2, D3, D4, D5 |
| ULN2003 board | `-` (GND) | GND |
| ULN2003 board | `+` (5 V) | external 5 V supply (see below), never the Nano's `5V` pin |
| 28BYJ-48 | white 5-pin plug | the white socket on the ULN2003 board |
| SG90 | brown (GND), red (5 V), orange (signal) | GND, external 5 V (same supply), D9 |

Power: the stepper draws about 240 mA and the SG90 spikes to about 650 mA when it starts or stalls. The Nano's
`5V` pin is fed from the USB port through a diode and shares the Mac's 500 mA budget with the board itself, so
even a short test can brown out the Nano (random resets, garbage on the console) or trip the Mac's USB
protection. Use a regulated 5 V supply of at least 2 A for both actuators: a breadboard power module set to 5 V
(often in starter kits) or a 5 V USB charger with a breakout cable. Its GND must be wired to a Nano GND pin
(common ground); only the signal wires and GND go between the supply side and the Nano. Read the voltage printed
on the 28BYJ-48 label: 5 V and 12 V variants exist and look identical; kits normally ship the 5 V one, and 12 V
into a 5 V motor burns it. The Phase 2 firmware adds the half-step sequencer and the `STEP`/`ROT` console commands;
`docs/PLAN.md` has the acceptance test.
