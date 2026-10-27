//! Main-only EEPROM access. Timer and RX must keep running during a 3.4 ms write.
use arduino_hal::pac;
use avr_device::interrupt;
use turntable_core::{load, save_plan, Error, Settings};

fn registers() -> &'static pac::eeprom::RegisterBlock {
    // SAFETY: main is the sole EEPROM owner; neither ISR accesses these registers.
    unsafe { &*pac::EEPROM::ptr() }
}

fn ready() {
    // Never mask interrupts while the hardware finishes its erase/write cycle.
    while registers().eecr().read().eepe().bit_is_set() {}
}

fn read(address: u16) -> u8 {
    ready();
    let ee = registers();
    ee.eear().write(|w| unsafe { w.bits(address) });
    ee.eecr().modify(|_, w| w.eere().set_bit());
    ee.eedr().read().bits()
}

fn slots() -> ([u8; 24], [u8; 24]) {
    let mut a = [0; 24];
    let mut b = [0; 24];
    for i in 0..24 {
        a[i] = read(i as u16);
        b[i] = read(32 + i as u16);
    }
    (a, b)
}

#[inline(never)]
pub fn boot() -> (Settings, bool) {
    let (a, b) = slots();
    match load(&a, &b) {
        Some(current) => (current.settings, false),
        None => (Settings::DEFAULT, true),
    }
}

pub fn save(settings: &Settings) -> Result<(), Error> {
    let (a, b) = slots();
    for (address, value) in save_plan(&a, &b, settings) {
        ready();
        if read(address) == value {
            continue;
        }
        let ee = registers();
        ee.eear().write(|w| unsafe { w.bits(address) });
        ee.eedr().write(|w| unsafe { w.bits(value) });
        // Erase+write mode, ready interrupt disabled. Set up with interrupts
        // enabled; only the timed strobe below needs an atomic instruction pair.
        ee.eecr().write(|w| unsafe { w.bits(0) });
        interrupt::free(|_| {
            // SAFETY: EECR is I/O address 0x1f on the 328P/PB. Two adjacent
            // SBI instructions (2 cycles each) meet EEMPE's four-cycle window.
            // No compiler-generated load/store or ISR can split this pair.
            unsafe { core::arch::asm!("sbi 0x1f, 2", "sbi 0x1f, 1", options(nostack)) };
        });
        // read waits for EEPE with interrupts enabled, including the final
        // magic write. A mismatch stops before any later byte is touched.
        if read(address) != value {
            return Err(Error::Save);
        }
    }
    Ok(())
}
