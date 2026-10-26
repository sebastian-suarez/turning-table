//! Phase 2: interrupt-driven stepper motion and a line-oriented serial console.
//!
//! The ATmega328PB Nano runs the ATmega328P instruction/peripheral subset. Keep
//! that target: the installed avr-gcc 9 cannot link a native 328PB executable.
//! D2..D5 are signals only; motor power comes from external 5 V, common ground.

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::{pac, prelude::*};
use avr_device::interrupt::{self, Mutex};
use core::cell::{Cell, RefCell};
use panic_halt as _;
use turntable_core::{
    accept, Command, Controller, Error, LineAssembler, Planner, PreparedCommand, RxBuffer, State,
    RATIO_X100,
};

mod console_format;
use console_format::{ConsoleWriter, Number};

// RefCell gives temporary mutable access under the interrupt mutex. On AVR an
// i32 position takes four separate byte reads; Cell/RefCell alone would not make
// those reads atomic. Every access from main *and* either ISR uses this mutex.
static MOTION: Mutex<RefCell<Controller>> = Mutex::new(RefCell::new(Controller::new()));
static RX: Mutex<RefCell<RxBuffer>> = Mutex::new(RefCell::new(RxBuffer::new()));
static DONE: Mutex<Cell<Option<i32>>> = Mutex::new(Cell::new(None));

/// Map the core's IN1..IN4 notation (bits 3..0) to the wiring PD2..PD5.
/// Preserve PD0/PD1 (USART) and PD6/PD7: writing an entire raw nibble to PORTD
/// would otherwise affect unrelated pins. This function is called only before
/// enabling interrupts or from the timer ISR, so its read/modify/write cannot
/// race another PORTD writer. No main-loop GPIO code writes these four pins.
fn write_coils(pattern: u8) {
    let bits = ((pattern & 0b1000) >> 1)
        | ((pattern & 0b0100) << 1)
        | ((pattern & 0b0010) << 3)
        | ((pattern & 0b0001) << 5);
    // SAFETY: the PAC register block contains volatile, interior-mutable
    // registers. We only touch the motor output bits under the ownership rule
    // above; the pin tokens are retained in main and never used as writers.
    let port = unsafe { &*pac::PORTD::ptr() };
    port.portd()
        .modify(|r, w| unsafe { w.bits((r.bits() & !0b0011_1100) | bits) });
}

#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    // SAFETY: the HAL transmitter owns only the TX path after split(). This
    // ISR is the sole RX reader. UDR0 has separate RX/TX hardware buffers, so
    // reading here does not consume or overwrite a byte being transmitted.
    let usart = unsafe { &*pac::USART0::ptr() };
    // DOR0 means frames were lost BEFORE this byte; the byte itself is intact
    // unless FE0 (bad stop bit) or UPE0 (bad parity) also marks it corrupt.
    // Read status FIRST: reading UDR0 advances the FIFO and its error bits.
    // Do not use the HAL reader here: it discards this error information.
    let status = usart.ucsr0a().read();
    let byte = usart.udr0().read().bits();
    let overrun = status.dor0().bit_is_set();
    let corrupt_frame = status.fe0().bit_is_set() || status.upe0().bit_is_set();
    // Worst case: flush queue indices, increment the u32 loss generation, and
    // enqueue one byte. No loop, buffer copy, formatting or UART write.
    // Release bound: 83 cycles = 5.1875 us at 16 MHz (verification report).
    interrupt::free(|cs| {
        RX.borrow(cs)
            .borrow_mut()
            .receive(byte, overrun, corrupt_frame)
    });
    // Never print in an ISR. The 32-byte queue gives main time for ufmt's
    // blocking TX; overflow is sticky and rejects input instead of changing it.
}

#[avr_device::interrupt(atmega328p)]
fn TIMER2_COMPA() {
    // Worst case: one tick's small counters, i32 step and table lookup, a
    // position-bearing DONE mailbox write, and a one-bit activity snapshot.
    // Release bound: 135 cycles = 8.4375 us; full ISR body is 16.0625 us.
    let (output, active) = interrupt::free(|cs| {
        let mut motion = MOTION.borrow(cs).borrow_mut();
        let output = motion.tick();
        if let Some(position) = output.done {
            // There can be only one in-flight move. main detaches an old event
            // atomically before it accepts a new move, so this cannot overwrite
            // an unreported completion. Retain the *completed* position.
            DONE.borrow(cs).set(Some(position));
        }
        (output, motion.snapshot().state != State::Idle)
    });
    write_coils(output.pattern);
    // PB0/PB2 will be the arm coils. Keep PB5's read/modify/write in this ISR
    // too: main must never restore an old PORTB value over a newer coil write.
    // The retained D13 pin token only configures direction before interrupts.
    // SAFETY: this ISR is the only PORTB writer after interrupts are enabled.
    let port = unsafe { &*pac::PORTB::ptr() };
    port.portb()
        .modify(|r, w| unsafe { w.bits((r.bits() & !(1 << 5)) | (u8::from(active) << 5)) });
    // Controller::tick contains only counters, i32 +/-1 and table lookup.
    // Degree/stop i64 multiplication/division is exclusively in main.
}

fn configure_timer(timer: pac::TC2) {
    // Timer2 has its own prescaler, including /128 (Timer0/1's choices differ).
    // Stop the clock while programming it, select CTC (clear on compare A),
    // and leave COM2A/COM2B disconnected: GPIO, not timer PWM, drives the coils.
    timer.tccr2b().write(|w| w.cs2().no_clock());
    timer.tccr2a().write(|w| w.wgm2().ctc());
    timer.tcnt2().write(|w| unsafe { w.bits(0) });
    // 16,000,000 / 128 = 125,000 counter increments per second.
    // The counter visits 0..=124, i.e. 125 counts, so 125,000/125 = 1,000 Hz.
    timer.ocr2a().write(|w| unsafe { w.bits(124) });
    // Timer flags clear by writing ONE. Clear any stale match before unmasking
    // OCIE2A, then start the prescaler. Global interrupts are still off here.
    timer.tifr2().write(|w| w.ocf2a().set_bit());
    timer.timsk2().write(|w| w.ocie2a().set_bit());
    timer.tccr2b().write(|w| w.cs2().prescale_128());
    // TC0 remains available for later timing; TC1 remains free for servo PWM.
}

fn print_done<W: ufmt::uWrite>(serial: &mut W, position: i32) {
    // Deliberately ignore only a transport write error. The actual HAL writer
    // is Infallible; generic helpers avoid spelling its board-specific type.
    let _ = ufmt::uwriteln!(serial, "DONE {}\r", Number::signed(position));
}

fn print_error<W: ufmt::uWrite>(serial: &mut W, error: Error) {
    let _ = ufmt::uwriteln!(serial, "ERR {}\r", error.as_str());
}

// Keep parser locals out of the planner/acceptance stack frame. On an 8-bit
// AVR, inlining both paths spills many registers and costs flash as well as
// instructions inside the critical section.
#[inline(never)]
fn dispatch<W: ufmt::uWrite>(
    serial: &mut W,
    planner: &mut Planner,
    command: Result<Command, Error>,
    generation: u32,
) {
    // Worst case: copy state, i32 position and the next-move period (6 bytes).
    // Release bound: 21 cycles = 1.3125 us at 16 MHz.
    let snapshot = interrupt::free(|cs| MOTION.borrow(cs).borrow().snapshot());
    // This is intentionally OUTSIDE every critical section. AVR's software
    // i64 division takes many instructions; masking RX for that long can lose
    // bytes at 57600 baud (one incoming byte about every 174 microseconds).
    let prepared = command.and_then(|command| planner.prepare(command, snapshot));
    // Worst case: detach DONE, compare one u32 generation, apply one prepared
    // command (bounded i32 checks/updates), commit k/origin/stops and copy the
    // 6-byte motion snapshot. No parsing, i64 arithmetic, formatting or TX.
    // Release bound: 293 cycles = 18.3125 us, below the 20 us target.
    let (previous_done, accepted) = interrupt::free(|cs| {
        // Check/apply and event detachment are one transaction. The ISR may
        // have completed between the earlier snapshot and this lock. Idle is
        // still available; a pending event must not produce an extra BUSY.
        let previous_done = DONE.borrow(cs).take();
        // Generation check, controller apply, planner commit and snapshot,
        // shared with the host tests. MOTION is borrowed only if the
        // generation still matches; both borrows end inside this section.
        let accepted = accept(
            &RX.borrow(cs).borrow(),
            || MOTION.borrow(cs).borrow_mut(),
            planner,
            generation,
            prepared,
        );
        (previous_done, accepted)
    });
    if let Some(position) = previous_done {
        print_done(serial, position);
    }
    // The queued loss notice will print exactly one ERR LINE for this flush.
    // Printing another error here would count the same loss twice.
    let Some((result, snapshot)) = accepted else {
        return;
    };
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            print_error(serial, error);
            return;
        }
    };
    // Everything below uses local snapshots/main-only planner words. Blocking
    // POS/STATUS/DONE formatting and UART TX always leave interrupts enabled.
    // Use the prepared tag for replies too, so the original parser result need
    // not survive the planner's arithmetic in AVR's limited register set.
    match prepared {
        Ok(PreparedCommand::Pos) => {
            let _ = ufmt::uwriteln!(
                serial,
                "POS {} {}\r",
                Number::signed(snapshot.position),
                Number::unsigned(planner.k())
            );
        }
        Ok(PreparedCommand::Status) => {
            let _ = ufmt::uwriteln!(
                serial,
                "STATUS {} {} {} {} {}\r",
                snapshot.state.as_str(),
                Number::unsigned(u32::from(snapshot.period_ms)),
                Number::unsigned(planner.ratio_x100()),
                Number::unsigned(u32::from(planner.stops())),
                Number::unsigned(planner.k())
            );
        }
        _ => {
            let _ = ufmt::uwriteln!(serial, "OK\r");
        }
    }
    // Zero-distance STEP/ROT/NEXT produces its event immediately, after its OK.
    // This value stays local so consecutive zero moves cannot overwrite it.
    if let Some(position) = output.done {
        print_done(serial, position);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    // The HAL sets asynchronous USART0, 8 data bits, no parity, one stop bit.
    // RXCIE0 enables RX-complete interrupts; TX stays a blocking main-only
    // ufmt writer. Retain the unused reader token so nobody polls RX as well.
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    serial.listen(arduino_hal::hal::usart::Event::RxComplete);
    let (_receiver, serial) = serial.split();
    let mut serial = ConsoleWriter(serial);
    let _led = pins.d13.into_output();
    let _motor_pins = (
        pins.d2.into_output(),
        pins.d3.into_output(),
        pins.d4.into_output(),
        pins.d5.into_output(),
    );
    write_coils(0);
    configure_timer(dp.TC2);

    // The spec leaves the initial stop count open; 36 matches its closure
    // exercise. STOPS changes this at runtime and rebases the planner origin.
    let mut planner = Planner::new(RATIO_X100, 36).unwrap();
    let mut assembler = LineAssembler::new();
    // SAFETY: shared statics and peripherals are fully initialised. Neither
    // ISR enables nested interrupts, prints, or borrows main's stack objects.
    unsafe { interrupt::enable() };
    ufmt::uwriteln!(&mut serial, "turntable-firmware boot\r").unwrap_infallible();

    loop {
        // Worst case: detach the 5-byte DONE mailbox and pop one byte+u32
        // generation or one loss notice. No assembler, parser or UART TX.
        // Release bound: 97 cycles = 6.0625 us at 16 MHz.
        let (done, item) =
            interrupt::free(|cs| (DONE.borrow(cs).take(), RX.borrow(cs).borrow_mut().pop()));
        if let Some(position) = done {
            print_done(&mut serial, position);
        }
        if let Some(item) = item {
            if let Some(line) = assembler.push(item.byte, item.corrupted) {
                if item.corrupted {
                    // Notices survive later losses too. Never generation-gate
                    // them or coalesce two faults during the same recovery.
                    print_error(&mut serial, Error::Line);
                } else {
                    dispatch(
                        &mut serial,
                        &mut planner,
                        line.and_then(Command::parse),
                        item.generation,
                    );
                }
            }
        }
    }
}
