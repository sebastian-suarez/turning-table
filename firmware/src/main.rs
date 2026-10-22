//! Turntable firmware, phase 1 bring-up: heartbeat LED + serial banner.
//!
//! Serial console: 57600 baud, 8N1 (`cargo run` opens it through ravedude).

#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    let mut led = pins.d13.into_output();

    ufmt::uwriteln!(&mut serial, "turntable-firmware boot\r").unwrap_infallible();

    let mut tick: u32 = 0;
    loop {
        led.toggle();
        ufmt::uwriteln!(&mut serial, "tick {}\r", tick).unwrap_infallible();
        tick = tick.wrapping_add(1);
        arduino_hal::delay_ms(300);
    }
}
